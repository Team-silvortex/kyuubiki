use crate::frame_2d_corotational_element::{assemble_internal, assemble_tangent_and_internal};
use crate::frame_2d_stability::Frame2dStabilitySystem;
use crate::linear_algebra::{SparseMatrix, reduce_sparse_system, sparse_to_dense};
use crate::linear_banded::SymmetricBandCholesky;
use crate::linear_dense::solve_linear_system;
use kyuubiki_protocol::{
    Frame2dElementInput, Frame2dPDeltaFailureReason, Frame2dPDeltaStepResult,
    SolveFrame2dPDeltaRequest,
};

const DEFAULT_MAX_ITERATIONS: usize = 32;
const DEFAULT_RESIDUAL_TOLERANCE: f64 = 1.0e-7;
const DEFAULT_MAX_STEP_CUTBACKS: usize = 12;
const MAX_DENSE_FALLBACK_DOFS: usize = 1_024;

struct EquilibriumAttempt {
    displacement: Vec<f64>,
    iterations: usize,
    residual_norm: f64,
    converged: bool,
    failure_reason: Option<Frame2dPDeltaFailureReason>,
    failure_detail: Option<String>,
}

struct AdaptiveStep {
    displacement: Vec<f64>,
    iterations: usize,
    residual_norm: f64,
    converged: bool,
    achieved_load_factor: f64,
    substeps: usize,
    cutbacks: usize,
    failure_reason: Option<Frame2dPDeltaFailureReason>,
    failure_detail: Option<String>,
}

pub(crate) fn solve_corotational_steps(
    request: &SolveFrame2dPDeltaRequest,
    system: &Frame2dStabilitySystem,
    initial_imperfection: &[f64],
    maximum_load_factor: f64,
    critical_factor: f64,
    load_steps: usize,
) -> Result<Vec<Frame2dPDeltaStepResult>, String> {
    let frame = &request.buckling.frame;
    let initial_positions = frame
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| {
            (
                node.x + initial_imperfection[index * 3],
                node.y + initial_imperfection[index * 3 + 1],
            )
        })
        .collect::<Vec<_>>();
    let mut displacement = vec![0.0; frame.nodes.len() * 3];
    let mut steps = Vec::with_capacity(load_steps);
    let max_iterations = request.max_iterations.unwrap_or(DEFAULT_MAX_ITERATIONS);
    let tolerance = request.tolerance.unwrap_or(DEFAULT_RESIDUAL_TOLERANCE);
    let max_cutbacks = request
        .max_step_cutbacks
        .unwrap_or(DEFAULT_MAX_STEP_CUTBACKS);
    let mut previous_load_factor = 0.0;

    for step in 1..=load_steps {
        let load_factor = maximum_load_factor * step as f64 / load_steps as f64;
        let adaptive = solve_adaptive_step(
            &initial_positions,
            &frame.elements,
            system,
            &displacement,
            previous_load_factor,
            load_factor,
            max_iterations,
            tolerance,
            max_cutbacks,
        )?;
        displacement = adaptive.displacement;

        steps.push(Frame2dPDeltaStepResult {
            step,
            load_factor,
            critical_factor_ratio: load_factor / critical_factor,
            iterations: adaptive.iterations,
            converged: adaptive.converged,
            achieved_load_factor: Some(adaptive.achieved_load_factor),
            substeps: adaptive.substeps,
            cutbacks: adaptive.cutbacks,
            failure_reason: adaptive.failure_reason,
            failure_detail: adaptive.failure_detail,
            arc_length_constraint_error: None,
            arc_length_radius: None,
            residual_norm: adaptive.residual_norm,
            imperfection_amplification: imperfection_amplification(
                initial_imperfection,
                &displacement,
            ),
            max_incremental_displacement: max_translation(&displacement),
            displacements: displacement.clone(),
        });
        if !adaptive.converged {
            break;
        }
        previous_load_factor = load_factor;
    }
    Ok(steps)
}

#[allow(clippy::too_many_arguments)]
fn solve_adaptive_step(
    positions: &[(f64, f64)],
    elements: &[Frame2dElementInput],
    system: &Frame2dStabilitySystem,
    initial_displacement: &[f64],
    initial_load_factor: f64,
    target_load_factor: f64,
    max_iterations: usize,
    tolerance: f64,
    max_cutbacks: usize,
) -> Result<AdaptiveStep, String> {
    let mut displacement = initial_displacement.to_vec();
    let mut achieved_load_factor = initial_load_factor;
    let mut pending = vec![target_load_factor];
    let mut total_iterations = 0;
    let mut residual_norm = f64::INFINITY;
    let mut substeps = 0;
    let mut cutbacks = 0;

    while let Some(attempted_load_factor) = pending.pop() {
        let attempt = solve_equilibrium(
            positions,
            elements,
            system,
            &displacement,
            attempted_load_factor,
            max_iterations,
            tolerance,
        )?;
        total_iterations += attempt.iterations;
        residual_norm = attempt.residual_norm;
        if attempt.converged {
            displacement = attempt.displacement;
            achieved_load_factor = attempted_load_factor;
            substeps += 1;
            continue;
        }
        let last_failure_reason = attempt.failure_reason;
        let last_failure_detail = attempt.failure_detail;
        if cutbacks >= max_cutbacks {
            return Ok(AdaptiveStep {
                displacement,
                iterations: total_iterations,
                residual_norm,
                converged: false,
                achieved_load_factor,
                substeps,
                cutbacks,
                failure_reason: Some(Frame2dPDeltaFailureReason::CutbackLimitExhausted),
                failure_detail: cutback_failure_detail(
                    last_failure_reason,
                    last_failure_detail.as_deref(),
                ),
            });
        }
        let midpoint = 0.5 * (achieved_load_factor + attempted_load_factor);
        if midpoint <= achieved_load_factor + f64::EPSILON * target_load_factor.abs().max(1.0) {
            return Ok(AdaptiveStep {
                displacement,
                iterations: total_iterations,
                residual_norm,
                converged: false,
                achieved_load_factor,
                substeps,
                cutbacks,
                failure_reason: Some(Frame2dPDeltaFailureReason::IncrementTooSmall),
                failure_detail: cutback_failure_detail(
                    last_failure_reason,
                    last_failure_detail.as_deref(),
                ),
            });
        }
        pending.push(attempted_load_factor);
        pending.push(midpoint);
        cutbacks += 1;
    }

    Ok(AdaptiveStep {
        displacement,
        iterations: total_iterations,
        residual_norm,
        converged: true,
        achieved_load_factor,
        substeps,
        cutbacks,
        failure_reason: None,
        failure_detail: None,
    })
}

fn cutback_failure_detail(
    reason: Option<Frame2dPDeltaFailureReason>,
    detail: Option<&str>,
) -> Option<String> {
    match (reason, detail) {
        (Some(reason), Some(detail)) => Some(format!("last attempt: {reason:?}: {detail}")),
        (Some(reason), None) => Some(format!("last attempt: {reason:?}")),
        (None, Some(detail)) => Some(detail.to_string()),
        (None, None) => None,
    }
}

#[allow(clippy::too_many_arguments)]
fn solve_equilibrium(
    positions: &[(f64, f64)],
    elements: &[Frame2dElementInput],
    system: &Frame2dStabilitySystem,
    initial_displacement: &[f64],
    load_factor: f64,
    max_iterations: usize,
    tolerance: f64,
) -> Result<EquilibriumAttempt, String> {
    let mut displacement = initial_displacement.to_vec();
    let mut residual_norm = f64::INFINITY;
    let mut iterations = 0;
    let mut converged = false;
    let mut failure_reason = None;
    let mut failure_detail = None;
    for iteration in 1..=max_iterations {
        iterations = iteration;
        let (tangent, internal) =
            assemble_tangent_and_internal(positions, elements, &displacement)?;
        let residual = residual(&system.reference_force, &internal, load_factor);
        let (reduced_tangent, reduced_residual, free) =
            reduce_sparse_system(&tangent, &residual, &system.constrained_dofs);
        residual_norm =
            normalized_residual(&reduced_residual, &system.reference_force, load_factor);
        if residual_norm <= tolerance {
            converged = true;
            break;
        }
        let delta = match solve_tangent(&reduced_tangent, &reduced_residual) {
            Ok(delta) => delta,
            Err(error) => {
                failure_reason = Some(Frame2dPDeltaFailureReason::TangentSolveFailed);
                failure_detail = Some(error);
                break;
            }
        };
        if !apply_backtracked_increment(
            positions,
            elements,
            &system.reference_force,
            &free,
            &delta,
            load_factor,
            residual_norm,
            &mut displacement,
        )? {
            failure_reason = Some(Frame2dPDeltaFailureReason::LineSearchFailed);
            break;
        }
    }
    if !converged && failure_reason.is_none() {
        failure_reason = Some(Frame2dPDeltaFailureReason::MaximumIterations);
    }
    Ok(EquilibriumAttempt {
        displacement,
        iterations,
        residual_norm,
        converged,
        failure_reason,
        failure_detail,
    })
}

fn apply_backtracked_increment(
    positions: &[(f64, f64)],
    elements: &[Frame2dElementInput],
    external: &[f64],
    free: &[usize],
    delta: &[f64],
    load_factor: f64,
    current_norm: f64,
    displacement: &mut Vec<f64>,
) -> Result<bool, String> {
    let mut scale = 1.0;
    for _ in 0..10 {
        let mut trial = displacement.clone();
        for (index, &dof) in free.iter().enumerate() {
            trial[dof] += scale * delta[index];
        }
        let Ok(internal) = assemble_internal(positions, elements, &trial) else {
            scale *= 0.5;
            continue;
        };
        let trial_residual = residual(external, &internal, load_factor);
        let reduced = free
            .iter()
            .map(|&dof| trial_residual[dof])
            .collect::<Vec<_>>();
        if normalized_residual(&reduced, external, load_factor) < current_norm {
            *displacement = trial;
            return Ok(true);
        }
        scale *= 0.5;
    }
    Ok(false)
}

pub(crate) fn solve_tangent(matrix: &SparseMatrix, rhs: &[f64]) -> Result<Vec<f64>, String> {
    if let Ok(Some(factor)) = SymmetricBandCholesky::try_factor(matrix, 8_000_000) {
        let mut solution = factor.solve(rhs)?;
        for _ in 0..2 {
            let residual = linear_residual(matrix, rhs, &solution);
            if normalized_linear_residual(&residual, rhs) <= 1.0e-12 {
                break;
            }
            let correction = factor.solve(&residual)?;
            for (value, correction) in solution.iter_mut().zip(correction) {
                *value += correction;
            }
        }
        return Ok(solution);
    }
    if rhs.len() <= MAX_DENSE_FALLBACK_DOFS {
        return solve_linear_system(sparse_to_dense(matrix), rhs.to_vec());
    }
    Err("corotational frame tangent is not positive definite at this load step".into())
}

fn linear_residual(matrix: &SparseMatrix, rhs: &[f64], solution: &[f64]) -> Vec<f64> {
    (0..matrix.size())
        .map(|row| {
            rhs[row]
                - matrix
                    .row_entries(row)
                    .iter()
                    .map(|&(column, value)| value * solution[column])
                    .sum::<f64>()
        })
        .collect()
}

fn normalized_linear_residual(residual: &[f64], rhs: &[f64]) -> f64 {
    let numerator = residual.iter().map(|value| value.abs()).fold(0.0, f64::max);
    let denominator = rhs.iter().map(|value| value.abs()).fold(1.0, f64::max);
    numerator / denominator
}

fn residual(external: &[f64], internal: &[f64], load_factor: f64) -> Vec<f64> {
    external
        .iter()
        .zip(internal)
        .map(|(external, internal)| load_factor * external - internal)
        .collect()
}

pub(crate) fn normalized_residual(residual: &[f64], external: &[f64], load_factor: f64) -> f64 {
    let numerator = residual.iter().map(|value| value.abs()).fold(0.0, f64::max);
    let denominator = external
        .iter()
        .map(|value| (load_factor * value).abs())
        .fold(1.0, f64::max);
    numerator / denominator
}

pub(crate) fn imperfection_amplification(initial: &[f64], displacement: &[f64]) -> f64 {
    let mut numerator = 0.0;
    let mut denominator = 0.0;
    for node in 0..initial.len() / 3 {
        for offset in 0..2 {
            numerator += initial[node * 3 + offset] * displacement[node * 3 + offset];
            denominator += initial[node * 3 + offset].powi(2);
        }
    }
    1.0 + numerator / denominator.max(f64::MIN_POSITIVE)
}

pub(crate) fn max_translation(displacements: &[f64]) -> f64 {
    (0..displacements.len() / 3)
        .map(|node| (displacements[node * 3].powi(2) + displacements[node * 3 + 1].powi(2)).sqrt())
        .fold(0.0_f64, f64::max)
}
