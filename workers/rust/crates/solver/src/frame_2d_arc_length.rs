use crate::frame_2d_branch_continuation::{BranchContinuationContext, continue_branch_probes};
use crate::frame_2d_branch_subspace::{
    probe_subspace_branch_switches, unavailable_subspace_branch_switches,
};
use crate::frame_2d_branch_switch::{
    BranchSwitchContext, probe_branch_switches, probe_pairwise_branch_switches,
    probe_weighted_branch_switches, unavailable_branch_switches,
    unavailable_pairwise_branch_switches, unavailable_weighted_branch_switches,
};
use crate::frame_2d_corotational::{
    imperfection_amplification, max_translation, normalized_residual, solve_tangent,
};
use crate::frame_2d_corotational_element::assemble_tangent_and_internal;
use crate::frame_2d_stability::Frame2dStabilitySystem;
use crate::frame_2d_transition_refinement::{
    TransitionRefinementContext, refine_tangent_transition,
};
use crate::linear_algebra::reduce_sparse_system;
use crate::symmetric_critical_mode::{SymmetricCriticalMode, extract_symmetric_critical_modes};
use crate::symmetric_inertia::assess_symmetric_inertia;
use kyuubiki_protocol::{
    Frame2dBranchSwitchSelection, Frame2dCriticalModeResult, Frame2dElementInput,
    Frame2dPDeltaFailureReason, Frame2dPDeltaStepResult, Frame2dTangentStability,
    SolveFrame2dPDeltaRequest,
};

const DEFAULT_MAX_ITERATIONS: usize = 32;
const DEFAULT_RESIDUAL_TOLERANCE: f64 = 1.0e-7;
const DEFAULT_TARGET_ITERATIONS: usize = 6;
const DEFAULT_TRANSITION_REFINEMENT_STEPS: usize = 12;
const MIN_CONSTRAINT_DENOMINATOR: f64 = 1.0e-14;

pub(crate) struct ArcLengthState {
    pub(crate) displacement: Vec<f64>,
    pub(crate) load_factor: f64,
    pub(crate) displacement_increment: Vec<f64>,
    pub(crate) load_increment: f64,
}

pub(crate) struct ArcLengthAttempt {
    pub(crate) state: ArcLengthState,
    pub(crate) iterations: usize,
    pub(crate) residual_norm: f64,
    pub(crate) constraint_error: f64,
    pub(crate) converged: bool,
    pub(crate) failure_reason: Option<Frame2dPDeltaFailureReason>,
    pub(crate) failure_detail: Option<String>,
    pub(crate) tangent_stability: Option<Frame2dTangentStability>,
    pub(crate) tangent_negative_pivots: Option<usize>,
    pub(crate) tangent_near_zero_pivots: Option<usize>,
}

pub(crate) fn solve_arc_length_steps(
    request: &SolveFrame2dPDeltaRequest,
    system: &Frame2dStabilitySystem,
    initial_imperfection: &[f64],
    reference_path_factor: f64,
    critical_factor: f64,
    load_steps: usize,
) -> Result<Vec<Frame2dPDeltaStepResult>, String> {
    let frame = &request.buckling.frame;
    let positions = frame
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
    let initial_displacement = vec![0.0; frame.nodes.len() * 3];
    let (initial_tangent, _) =
        assemble_tangent_and_internal(&positions, &frame.elements, &initial_displacement)?;
    let (reduced_tangent, reduced_reference, free) = reduce_sparse_system(
        &initial_tangent,
        &system.reference_force,
        &system.constrained_dofs,
    );
    let load_direction = solve_tangent(&reduced_tangent, &reduced_reference)?;
    let direction_norm = dot(&load_direction, &load_direction).sqrt();
    let load_scale = request
        .arc_length_load_scale
        .unwrap_or(direction_norm.max(1.0e-12));
    let nominal_load_increment = reference_path_factor / load_steps as f64;
    let radius = request.arc_length_radius.unwrap_or_else(|| {
        nominal_load_increment * (direction_norm.powi(2) + load_scale.powi(2)).sqrt()
    });
    let max_iterations = request.max_iterations.unwrap_or(DEFAULT_MAX_ITERATIONS);
    let tolerance = request.tolerance.unwrap_or(DEFAULT_RESIDUAL_TOLERANCE);
    let max_cutbacks = request.max_step_cutbacks.unwrap_or(12);
    let target_iterations = request
        .arc_length_target_iterations
        .unwrap_or(DEFAULT_TARGET_ITERATIONS.min(max_iterations));
    let transition_refinement_steps = request
        .tangent_transition_refinement_steps
        .unwrap_or(DEFAULT_TRANSITION_REFINEMENT_STEPS);
    let branch_mode_count = request.branch_switch_mode_count.unwrap_or(1);
    let mut state = ArcLengthState {
        displacement: initial_displacement,
        load_factor: 0.0,
        displacement_increment: vec![0.0; free.len()],
        load_increment: 0.0,
    };
    let mut steps = Vec::with_capacity(load_steps);
    let nominal_radius = radius;
    let mut current_radius = radius;
    let mut previous_negative_pivots = None;

    for step in 1..=load_steps {
        let mut cutbacks = 0;
        let mut total_iterations = 0;
        let attempt = loop {
            let mut attempt = solve_arc_length_step(
                &positions,
                &frame.elements,
                system,
                &free,
                &state,
                current_radius,
                load_scale,
                max_iterations,
                tolerance,
            )?;
            total_iterations += attempt.iterations;
            if attempt.converged {
                break attempt;
            }
            if cutbacks >= max_cutbacks {
                let detail =
                    cutback_detail(attempt.failure_reason, attempt.failure_detail.as_deref());
                attempt.failure_reason = Some(Frame2dPDeltaFailureReason::CutbackLimitExhausted);
                attempt.failure_detail = detail;
                break attempt;
            }
            let reduced_radius = current_radius * 0.5;
            if reduced_radius <= f64::EPSILON * nominal_radius.max(1.0) {
                let detail =
                    cutback_detail(attempt.failure_reason, attempt.failure_detail.as_deref());
                attempt.failure_reason = Some(Frame2dPDeltaFailureReason::IncrementTooSmall);
                attempt.failure_detail = detail;
                break attempt;
            }
            current_radius = reduced_radius;
            cutbacks += 1;
        };
        state = attempt.state;
        let transition_changed = attempt.converged
            && previous_negative_pivots.is_some()
            && previous_negative_pivots != attempt.tangent_negative_pivots;
        let mut refinement = if transition_changed {
            match (steps.last(), attempt.tangent_negative_pivots) {
                (Some(lower), Some(upper_negative_pivots)) => refine_tangent_transition(
                    &TransitionRefinementContext {
                        positions: &positions,
                        elements: &frame.elements,
                        system,
                        free_dofs: &free,
                        max_iterations,
                        tolerance,
                        refinement_steps: transition_refinement_steps,
                        mode_count: branch_mode_count,
                    },
                    lower,
                    state.load_factor,
                    &state.displacement,
                    upper_negative_pivots,
                )?,
                _ => None,
            }
        } else {
            None
        };
        let mut critical_load_factor = refinement
            .as_ref()
            .and_then(|refinement| refinement.critical_load_factor);
        let mut critical_displacement = refinement
            .as_ref()
            .and_then(|refinement| refinement.critical_displacement.clone());
        let mut critical_modes = refinement
            .as_mut()
            .map(|refinement| std::mem::take(&mut refinement.critical_modes))
            .unwrap_or_default();
        if transition_changed && critical_modes.is_empty() {
            critical_modes = extract_critical_modes(
                &positions,
                &frame.elements,
                system,
                &free,
                &state.displacement,
                branch_mode_count,
            )?;
            if !critical_modes.is_empty() {
                critical_load_factor = Some(state.load_factor);
                critical_displacement = Some(state.displacement.clone());
            }
        }
        let mut branch_switch_probes = match (
            request.branch_switch_amplitude,
            critical_load_factor,
            critical_displacement.as_deref(),
        ) {
            _ if request.branch_switch == Frame2dBranchSwitchSelection::Disabled => Vec::new(),
            (Some(amplitude), Some(load_factor), Some(displacement))
                if !critical_modes.is_empty() =>
            {
                let context = BranchSwitchContext {
                    positions: &positions,
                    elements: &frame.elements,
                    system,
                    free_dofs: &free,
                    max_iterations,
                    tolerance,
                };
                let mut probes = critical_modes
                    .iter()
                    .enumerate()
                    .flat_map(|(mode_index, mode)| {
                        probe_branch_switches(
                            &context,
                            displacement,
                            &state.displacement,
                            load_factor,
                            &mode.shape,
                            mode_index,
                            mode.normalized_eigenvalue,
                            amplitude,
                            request.branch_switch,
                        )
                    })
                    .collect::<Vec<_>>();
                if request.branch_switch_pairwise_combinations {
                    probes.extend(probe_pairwise_branch_switches(
                        &context,
                        displacement,
                        &state.displacement,
                        load_factor,
                        &critical_modes,
                        amplitude,
                        request.branch_switch,
                    ));
                }
                if let Some(weights) = &request.branch_switch_mode_weights {
                    probes.extend(probe_weighted_branch_switches(
                        &context,
                        displacement,
                        &state.displacement,
                        load_factor,
                        &critical_modes,
                        weights,
                        amplitude,
                        request.branch_switch,
                    ));
                }
                if let Some(sample_count) = request.branch_switch_subspace_sample_count {
                    probes.extend(probe_subspace_branch_switches(
                        &context,
                        displacement,
                        &state.displacement,
                        load_factor,
                        &critical_modes,
                        sample_count,
                        amplitude,
                        request.branch_switch,
                    ));
                }
                probes
            }
            (Some(amplitude), _, _) if transition_changed => {
                let mut probes = (0..branch_mode_count)
                    .flat_map(|mode_index| {
                        unavailable_branch_switches(
                            request.branch_switch,
                            mode_index,
                            amplitude,
                            "critical mode unavailable for branch switching; dense extraction is limited to 128 reduced degrees of freedom",
                        )
                    })
                    .collect::<Vec<_>>();
                if request.branch_switch_pairwise_combinations {
                    probes.extend(unavailable_pairwise_branch_switches(
                        request.branch_switch,
                        branch_mode_count,
                        amplitude,
                        "critical mode unavailable for branch switching; dense extraction is limited to 128 reduced degrees of freedom",
                    ));
                }
                if let Some(weights) = &request.branch_switch_mode_weights {
                    probes.extend(unavailable_weighted_branch_switches(
                        request.branch_switch,
                        weights,
                        amplitude,
                        "critical mode unavailable for branch switching; dense extraction is limited to 128 reduced degrees of freedom",
                    ));
                }
                if let Some(sample_count) = request.branch_switch_subspace_sample_count {
                    probes.extend(unavailable_subspace_branch_switches(
                        request.branch_switch,
                        branch_mode_count,
                        sample_count,
                        amplitude,
                        "critical mode unavailable for branch switching; dense extraction is limited to 128 reduced degrees of freedom",
                    ));
                }
                probes
            }
            _ => Vec::new(),
        };
        if let (Some(continuation_steps), Some(load_factor), Some(displacement)) = (
            request.branch_continuation_steps,
            critical_load_factor,
            critical_displacement.as_deref(),
        ) {
            continue_branch_probes(
                &BranchContinuationContext {
                    positions: &positions,
                    elements: &frame.elements,
                    system,
                    free_dofs: &free,
                    max_iterations,
                    tolerance,
                    max_cutbacks,
                    target_iterations,
                    initial_radius: request.branch_continuation_radius.unwrap_or(current_radius),
                    min_radius_ratio: request.branch_continuation_min_radius_ratio,
                    load_scale,
                },
                displacement,
                load_factor,
                continuation_steps,
                &mut branch_switch_probes,
            );
        }
        previous_negative_pivots = attempt.tangent_negative_pivots;
        steps.push(Frame2dPDeltaStepResult {
            step,
            load_factor: state.load_factor,
            critical_factor_ratio: state.load_factor / critical_factor,
            iterations: total_iterations,
            converged: attempt.converged,
            achieved_load_factor: Some(state.load_factor),
            substeps: usize::from(attempt.converged),
            cutbacks,
            failure_reason: attempt.failure_reason,
            failure_detail: attempt.failure_detail.or_else(|| {
                (!attempt.converged).then(|| {
                    format!(
                        "arc-length constraint relative error={:.6e}",
                        attempt.constraint_error
                    )
                })
            }),
            arc_length_constraint_error: Some(attempt.constraint_error),
            arc_length_radius: Some(current_radius),
            load_factor_increment: Some(if attempt.converged {
                state.load_increment
            } else {
                0.0
            }),
            path_event: None,
            tangent_stability: attempt.tangent_stability,
            tangent_negative_pivots: attempt.tangent_negative_pivots,
            tangent_near_zero_pivots: attempt.tangent_near_zero_pivots,
            tangent_negative_pivot_delta: None,
            tangent_critical_eigenvalue: critical_modes
                .first()
                .as_ref()
                .map(|mode| mode.normalized_eigenvalue),
            tangent_critical_mode_residual: critical_modes
                .first()
                .as_ref()
                .map(|mode| mode.normalized_residual),
            tangent_critical_mode: critical_modes.first().map(|mode| mode.shape.clone()),
            tangent_critical_modes: critical_modes
                .iter()
                .enumerate()
                .map(|(mode_index, mode)| Frame2dCriticalModeResult {
                    mode_index,
                    normalized_eigenvalue: mode.normalized_eigenvalue,
                    normalized_residual: mode.normalized_residual,
                    shape: mode.shape.clone(),
                })
                .collect(),
            tangent_transition_load_factor_min: refinement
                .as_ref()
                .map(|refinement| refinement.load_factor_min),
            tangent_transition_load_factor_max: refinement
                .as_ref()
                .map(|refinement| refinement.load_factor_max),
            tangent_transition_load_factor_width: refinement
                .as_ref()
                .map(|refinement| refinement.load_factor_max - refinement.load_factor_min),
            tangent_transition_refinements: refinement
                .as_ref()
                .map(|refinement| refinement.refinements),
            tangent_critical_load_factor: critical_load_factor,
            branch_switch_probes,
            residual_norm: attempt.residual_norm,
            imperfection_amplification: imperfection_amplification(
                initial_imperfection,
                &state.displacement,
            ),
            max_incremental_displacement: max_translation(&state.displacement),
            displacements: state.displacement.clone(),
        });
        if !attempt.converged {
            break;
        }
        let adaptation = (target_iterations as f64 / attempt.iterations as f64)
            .sqrt()
            .clamp(0.5, 2.0);
        current_radius = (current_radius * adaptation).min(nominal_radius);
    }
    Ok(steps)
}

fn extract_critical_modes(
    positions: &[(f64, f64)],
    elements: &[Frame2dElementInput],
    system: &Frame2dStabilitySystem,
    free: &[usize],
    displacement: &[f64],
    mode_count: usize,
) -> Result<Vec<SymmetricCriticalMode>, String> {
    let (tangent, _) = assemble_tangent_and_internal(positions, elements, displacement)?;
    let (reduced, _, current_free) =
        reduce_sparse_system(&tangent, &system.reference_force, &system.constrained_dofs);
    debug_assert_eq!(current_free, free);
    Ok(extract_symmetric_critical_modes(
        &reduced,
        free,
        displacement.len(),
        mode_count,
    ))
}

fn cutback_detail(
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
pub(crate) fn solve_arc_length_step(
    positions: &[(f64, f64)],
    elements: &[Frame2dElementInput],
    system: &Frame2dStabilitySystem,
    free: &[usize],
    previous: &ArcLengthState,
    radius: f64,
    load_scale: f64,
    max_iterations: usize,
    tolerance: f64,
) -> Result<ArcLengthAttempt, String> {
    let (base_tangent, _) =
        assemble_tangent_and_internal(positions, elements, &previous.displacement)?;
    let (reduced_tangent, reduced_reference, current_free) = reduce_sparse_system(
        &base_tangent,
        &system.reference_force,
        &system.constrained_dofs,
    );
    debug_assert_eq!(current_free, free);
    let load_direction = solve_tangent(&reduced_tangent, &reduced_reference)?;
    let denominator = (dot(&load_direction, &load_direction) + load_scale.powi(2)).sqrt();
    let mut load_increment = radius / denominator;
    if branch_orientation(previous, &load_direction, load_scale) < 0.0 {
        load_increment = -load_increment;
    }
    let mut displacement_increment = load_direction
        .iter()
        .map(|value| value * load_increment)
        .collect::<Vec<_>>();
    let mut displacement = previous.displacement.clone();
    add_reduced_increment(&mut displacement, free, &displacement_increment);
    let mut load_factor = previous.load_factor + load_increment;
    let mut residual_norm = f64::INFINITY;
    let mut constraint_error = f64::INFINITY;

    for iteration in 1..=max_iterations {
        let (tangent, internal) =
            assemble_tangent_and_internal(positions, elements, &displacement)?;
        let residual = system
            .reference_force
            .iter()
            .zip(&internal)
            .map(|(external, internal)| load_factor * external - internal)
            .collect::<Vec<_>>();
        let (reduced_tangent, reduced_residual, _) =
            reduce_sparse_system(&tangent, &residual, &system.constrained_dofs);
        residual_norm =
            normalized_residual(&reduced_residual, &system.reference_force, load_factor);
        let constraint = dot(&displacement_increment, &displacement_increment)
            + (load_scale * load_increment).powi(2)
            - radius.powi(2);
        constraint_error = constraint.abs() / radius.powi(2);
        if residual_norm <= tolerance && constraint_error <= tolerance {
            let inertia = assess_symmetric_inertia(&reduced_tangent);
            return Ok(ArcLengthAttempt {
                state: ArcLengthState {
                    displacement,
                    load_factor,
                    displacement_increment,
                    load_increment,
                },
                iterations: iteration,
                residual_norm,
                constraint_error,
                converged: true,
                failure_reason: None,
                failure_detail: None,
                tangent_stability: Some(inertia.stability),
                tangent_negative_pivots: inertia.negative_pivots,
                tangent_near_zero_pivots: inertia.near_zero_pivots,
            });
        }

        let residual_direction = match solve_tangent(&reduced_tangent, &reduced_residual) {
            Ok(direction) => direction,
            Err(error) => {
                return Ok(failed_attempt(
                    previous,
                    iteration,
                    residual_norm,
                    constraint_error,
                    Frame2dPDeltaFailureReason::TangentSolveFailed,
                    Some(error),
                ));
            }
        };
        let load_direction = match solve_tangent(&reduced_tangent, &reduced_reference) {
            Ok(direction) => direction,
            Err(error) => {
                return Ok(failed_attempt(
                    previous,
                    iteration,
                    residual_norm,
                    constraint_error,
                    Frame2dPDeltaFailureReason::TangentSolveFailed,
                    Some(error),
                ));
            }
        };
        let correction_denominator = 2.0
            * (dot(&displacement_increment, &load_direction) + load_scale.powi(2) * load_increment);
        let denominator_scale = radius * (load_direction.len() as f64).sqrt().max(1.0);
        if !correction_denominator.is_finite()
            || correction_denominator.abs()
                <= MIN_CONSTRAINT_DENOMINATOR * denominator_scale.max(1.0)
        {
            return Ok(failed_attempt(
                previous,
                iteration,
                residual_norm,
                constraint_error,
                Frame2dPDeltaFailureReason::ArcLengthConstraintSingular,
                Some("arc-length correction denominator is singular".into()),
            ));
        }
        let load_correction = -(constraint
            + 2.0 * dot(&displacement_increment, &residual_direction))
            / correction_denominator;
        if !load_correction.is_finite() {
            return Ok(failed_attempt(
                previous,
                iteration,
                residual_norm,
                constraint_error,
                Frame2dPDeltaFailureReason::ArcLengthConstraintSingular,
                Some("arc-length load correction is not finite".into()),
            ));
        }
        let correction = residual_direction
            .iter()
            .zip(&load_direction)
            .map(|(residual, load)| residual + load * load_correction)
            .collect::<Vec<_>>();
        add_reduced_increment(&mut displacement, free, &correction);
        for (increment, correction) in displacement_increment.iter_mut().zip(correction) {
            *increment += correction;
        }
        load_increment += load_correction;
        load_factor += load_correction;
    }

    Ok(failed_attempt(
        previous,
        max_iterations,
        residual_norm,
        constraint_error,
        Frame2dPDeltaFailureReason::MaximumIterations,
        None,
    ))
}

fn failed_attempt(
    previous: &ArcLengthState,
    iterations: usize,
    residual_norm: f64,
    constraint_error: f64,
    reason: Frame2dPDeltaFailureReason,
    detail: Option<String>,
) -> ArcLengthAttempt {
    ArcLengthAttempt {
        state: ArcLengthState {
            displacement: previous.displacement.clone(),
            load_factor: previous.load_factor,
            displacement_increment: previous.displacement_increment.clone(),
            load_increment: previous.load_increment,
        },
        iterations,
        residual_norm,
        constraint_error,
        converged: false,
        failure_reason: Some(reason),
        failure_detail: detail,
        tangent_stability: None,
        tangent_negative_pivots: None,
        tangent_near_zero_pivots: None,
    }
}

fn branch_orientation(previous: &ArcLengthState, load_direction: &[f64], load_scale: f64) -> f64 {
    if previous.load_increment == 0.0 {
        return 1.0;
    }
    dot(&previous.displacement_increment, load_direction)
        + load_scale.powi(2) * previous.load_increment
}

fn add_reduced_increment(displacement: &mut [f64], free: &[usize], increment: &[f64]) {
    for (&dof, value) in free.iter().zip(increment) {
        displacement[dof] += value;
    }
}

fn dot(left: &[f64], right: &[f64]) -> f64 {
    left.iter()
        .zip(right)
        .map(|(left, right)| left * right)
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn branch_orientation_preserves_the_previous_generalized_direction() {
        let previous = ArcLengthState {
            displacement: vec![0.0],
            load_factor: 1.0,
            displacement_increment: vec![-0.5, 0.25],
            load_increment: -0.1,
        };
        assert!(branch_orientation(&previous, &[1.0, 0.0], 1.0) < 0.0);
    }
}
