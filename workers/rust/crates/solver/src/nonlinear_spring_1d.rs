use crate::linear_algebra::{
    PrescribedSystemValidator, SparseMatrix, add_at, reduce_sparse_system, solve_spd_system,
    solve_tridiagonal_system,
};
use crate::nonlinear_spring_1d_validation::{validate_contact_request, validate_request};
use crate::solver_control::{SolverStage, check_cancellation, checkpoint, checkpoint_chunk};
use kyuubiki_protocol::{
    ContactGap1dContactInput, ContactGap1dContactResult, NonlinearSpring1dElementInput,
    NonlinearSpring1dElementResult, NonlinearSpring1dNodeResult, NonlinearSpring1dStepResult,
    SolveContactGap1dRequest, SolveContactGap1dResult, SolveNonlinearSpring1dRequest,
    SolveNonlinearSpring1dResult,
};

#[cfg(test)]
#[path = "nonlinear_spring_constraint_tests.rs"]
mod constraint_tests;

pub fn solve_nonlinear_spring_1d(
    request: &SolveNonlinearSpring1dRequest,
) -> Result<SolveNonlinearSpring1dResult, String> {
    check_cancellation()?;
    validate_request(request)?;
    solve_validated_nonlinear_spring_1d(request.clone())
}

pub fn solve_nonlinear_spring_1d_owned(
    request: SolveNonlinearSpring1dRequest,
) -> Result<SolveNonlinearSpring1dResult, String> {
    check_cancellation()?;
    validate_request(&request)?;
    solve_validated_nonlinear_spring_1d(request)
}

fn solve_validated_nonlinear_spring_1d(
    request: SolveNonlinearSpring1dRequest,
) -> Result<SolveNonlinearSpring1dResult, String> {
    let path = solve_load_path(
        &request.nodes,
        &request.elements,
        &[],
        request.load_steps,
        request.max_iterations,
        request.tolerance,
    )?;
    let displacement = path.displacement;

    let nodes = nonlinear_nodes(&request.nodes, &displacement)?;
    let elements = nonlinear_elements(&request.nodes, &request.elements, &displacement)?;
    let max_displacement = checked_max_abs(
        nodes.iter().map(|node| node.ux),
        SolverStage::ResultNodeSummary,
    )?;
    let max_force = checked_max_abs(
        elements.iter().map(|element| element.force),
        SolverStage::ResultElementSummary,
    )?;

    checkpoint(SolverStage::ResultTotals, 1)?;
    Ok(SolveNonlinearSpring1dResult {
        input: request,
        nodes,
        elements,
        steps: path.steps,
        converged: path.achieved_load_factor == 1.0,
        achieved_load_factor: Some(path.achieved_load_factor),
        residual_norm: path.residual_norm,
        max_displacement,
        max_force,
    })
}

pub fn solve_contact_gap_1d(
    request: &SolveContactGap1dRequest,
) -> Result<SolveContactGap1dResult, String> {
    check_cancellation()?;
    validate_contact_request(request)?;
    solve_validated_contact_gap_1d(request.clone())
}

pub fn solve_contact_gap_1d_owned(
    request: SolveContactGap1dRequest,
) -> Result<SolveContactGap1dResult, String> {
    check_cancellation()?;
    validate_contact_request(&request)?;
    solve_validated_contact_gap_1d(request)
}

fn solve_validated_contact_gap_1d(
    request: SolveContactGap1dRequest,
) -> Result<SolveContactGap1dResult, String> {
    let path = solve_load_path(
        &request.nodes,
        &request.elements,
        &request.contacts,
        request.load_steps,
        request.max_iterations,
        request.tolerance,
    )?;
    let displacement = path.displacement;

    let nodes = nonlinear_nodes(&request.nodes, &displacement)?;
    let elements = nonlinear_elements(&request.nodes, &request.elements, &displacement)?;
    let contacts = contact_results(&request, &displacement)?;
    let max_displacement = checked_max_abs(
        nodes.iter().map(|node| node.ux),
        SolverStage::ResultNodeSummary,
    )?;
    let max_force = checked_max_abs(
        elements.iter().map(|element| element.force),
        SolverStage::ResultElementSummary,
    )?;
    let mut active_contact_count = 0;
    let max_contact_force = checked_max_abs(
        contacts.iter().map(|contact| {
            active_contact_count += usize::from(contact.active);
            contact.force
        }),
        SolverStage::ResultElementSummary,
    )?;

    checkpoint(SolverStage::ResultTotals, 1)?;
    Ok(SolveContactGap1dResult {
        input: request,
        nodes,
        elements,
        contacts,
        steps: path.steps,
        converged: path.achieved_load_factor == 1.0,
        achieved_load_factor: Some(path.achieved_load_factor),
        residual_norm: path.residual_norm,
        max_displacement,
        max_force,
        max_contact_force,
        active_contact_count,
    })
}

struct SpringLoadPath {
    displacement: Vec<f64>,
    steps: Vec<NonlinearSpring1dStepResult>,
    achieved_load_factor: f64,
    residual_norm: f64,
}

struct SpringEquilibrium {
    tangent: SparseMatrix,
    residual: Vec<f64>,
    norm: f64,
}

impl SpringEquilibrium {
    fn validate_increment(
        &self,
        constrained: &[usize],
        free: &[usize],
        delta: &[f64],
        complete: &mut [f64],
    ) -> Result<(), String> {
        if complete.len() != self.tangent.size() || free.len() != delta.len() {
            return Err("original Newton increment dimensions must match".into());
        }
        checkpoint(SolverStage::ResultFreeDofs, 0)?;
        for (index, chunk) in complete.chunks_mut(1024).enumerate() {
            chunk.fill(0.0);
            checkpoint(SolverStage::ResultFreeDofs, index * 1024 + chunk.len())?;
        }
        for (index, (&dof, &value)) in free.iter().zip(delta).enumerate() {
            let target = complete.get_mut(dof).ok_or_else(|| {
                format!("original Newton increment has an out-of-range free DOF {dof}")
            })?;
            *target = value;
            checkpoint_chunk(SolverStage::ResultFreeDofs, index + 1, free.len())?;
        }
        PrescribedSystemValidator::new_homogeneous(&self.tangent, constrained)?
            .validate(&self.residual, complete)
    }
}

struct SpringStep<'a> {
    nodes: &'a [kyuubiki_protocol::NonlinearSpring1dNodeInput],
    elements: &'a [NonlinearSpring1dElementInput],
    contacts: &'a [ContactGap1dContactInput],
    load_factor: f64,
    tolerance: f64,
}

enum SpringCorrection {
    Accepted(SpringEquilibrium),
    Exhausted(f64),
}

const MAX_BACKTRACKS: usize = 32;
const SUFFICIENT_DECREASE: f64 = 1e-4;

impl SpringStep<'_> {
    fn evaluate(&self, displacement: &[f64]) -> Result<SpringEquilibrium, String> {
        self.validate_state(displacement)?;
        let (tangent, internal) = assemble_tangent_and_internal(
            self.nodes.len(),
            self.elements,
            self.contacts,
            displacement,
        )?;
        let residual = checked_residual(self.nodes, &internal, self.load_factor)?;
        let norm = checked_max_abs(residual.iter().copied(), SolverStage::ResidualValidate)?;
        Ok(SpringEquilibrium {
            tangent,
            residual,
            norm,
        })
    }

    fn validate_state(&self, displacement: &[f64]) -> Result<(), String> {
        checkpoint(SolverStage::StabilityRecovery, 0)?;
        if displacement.len() != self.nodes.len() {
            return Err("nonlinear spring original state dimensions must match".into());
        }
        for (index, (node, &value)) in self.nodes.iter().zip(displacement).enumerate() {
            if !value.is_finite() {
                return Err(format!(
                    "nonlinear spring original state has non-finite displacement at node {index}"
                ));
            }
            if node.fix_x && value != 0.0 {
                return Err(format!(
                    "nonlinear spring original state fixed value changed at node {index}"
                ));
            }
            checkpoint_chunk(SolverStage::StabilityRecovery, index + 1, self.nodes.len())?;
        }
        Ok(())
    }

    fn correct(
        &self,
        base: &[f64],
        candidate: &mut [f64],
        free: &[usize],
        delta: &[f64],
        initial_norm: f64,
    ) -> Result<SpringCorrection, String> {
        copy_displacement(base, candidate)?;
        let mut scale = 1.0;
        let mut last_norm = initial_norm;
        // Bound all merit evaluations, including the full Newton step. Errors and
        // cancellation propagate; a rejected trial never replaces the base state.
        for _ in 0..=MAX_BACKTRACKS {
            update_displacement(base, candidate, free, delta, scale)?;
            let state = self.evaluate(candidate)?;
            last_norm = state.norm;
            if state.norm <= self.tolerance
                || state.norm <= (1.0 - SUFFICIENT_DECREASE * scale) * initial_norm
            {
                return Ok(SpringCorrection::Accepted(state));
            }
            scale *= 0.5;
        }
        Ok(SpringCorrection::Exhausted(last_norm))
    }
}

fn solve_load_path(
    nodes: &[kyuubiki_protocol::NonlinearSpring1dNodeInput],
    elements: &[NonlinearSpring1dElementInput],
    contacts: &[ContactGap1dContactInput],
    load_steps: Option<usize>,
    max_iterations: Option<usize>,
    tolerance: Option<f64>,
) -> Result<SpringLoadPath, String> {
    let load_steps = load_steps.unwrap_or(8).clamp(1, 256);
    let max_iterations = max_iterations.unwrap_or(32).clamp(1, 256);
    // Validation accepts any finite positive absolute force tolerance. Do not
    // silently relax explicit small-scale requests into zero-displacement success.
    let tolerance = tolerance.unwrap_or(1.0e-8);
    let constrained = nodes
        .iter()
        .enumerate()
        .filter_map(|(index, node)| node.fix_x.then_some(index))
        .collect::<Vec<_>>();
    let mut path = SpringLoadPath {
        displacement: vec![0.0; nodes.len()],
        steps: Vec::with_capacity(load_steps),
        achieved_load_factor: 0.0,
        residual_norm: 0.0,
    };
    let mut trial = vec![0.0; nodes.len()];
    let mut candidate = vec![0.0; nodes.len()];

    for step in 1..=load_steps {
        checkpoint(SolverStage::StabilityStep, step)?;
        // Reuse three buffers, regardless of load-step and backtracking counts.
        copy_displacement(&path.displacement, &mut trial)?;
        let load_factor = step as f64 / load_steps as f64;
        let problem = SpringStep {
            nodes,
            elements,
            contacts,
            load_factor,
            tolerance,
        };
        let mut state = problem.evaluate(&trial)?;
        let mut iterations = 0;
        let (residual_norm, converged) = loop {
            // Constrained entries are zero; inspect the free norm before allocating a reduction.
            if state.norm <= tolerance || iterations == max_iterations {
                break (state.norm, state.norm <= tolerance);
            }
            let (reduced_tangent, reduced_residual, free) =
                reduce_sparse_system(&state.tangent, &state.residual, &constrained)?;
            let delta = solve_chain_tangent(&reduced_tangent, &reduced_residual)?;
            drop(reduced_tangent);
            drop(reduced_residual);
            // Admit the full undamped Newton increment, not K_t times the nonlinear state.
            // The line search overwrites this same scratch buffer after validation.
            state.validate_increment(&constrained, &free, &delta, &mut candidate)?;
            iterations += 1;
            let norm = state.norm;
            drop(state);
            match problem.correct(&trial, &mut candidate, &free, &delta, norm)? {
                SpringCorrection::Accepted(accepted) => {
                    std::mem::swap(&mut trial, &mut candidate);
                    state = accepted;
                }
                SpringCorrection::Exhausted(last_norm) => break (last_norm, false),
            }
        };
        path.steps.push(NonlinearSpring1dStepResult {
            step,
            load_factor,
            iterations,
            residual_norm,
            converged,
        });
        if !converged {
            break;
        }
        std::mem::swap(&mut path.displacement, &mut trial);
        path.achieved_load_factor = load_factor;
        path.residual_norm = residual_norm;
    }
    Ok(path)
}

// Chain benchmarks retain a tridiagonal tangent after constraints and contact penalties.
// Preserve the general sparse fallback for caller-provided non-chain topologies.
fn solve_chain_tangent(tangent: &SparseMatrix, residual: &[f64]) -> Result<Vec<f64>, String> {
    match solve_tridiagonal_system(tangent, residual) {
        Some(result) => result,
        None => solve_spd_system(tangent, residual),
    }
}

fn assemble_tangent_and_internal(
    node_count: usize,
    elements: &[NonlinearSpring1dElementInput],
    contacts: &[ContactGap1dContactInput],
    displacement: &[f64],
) -> Result<(SparseMatrix, Vec<f64>), String> {
    checkpoint(SolverStage::ElementAssembly, 0)?;
    let mut tangent = SparseMatrix::new(node_count);
    let mut internal_force = vec![0.0; node_count];

    for (index, element) in elements.iter().enumerate() {
        let extension = displacement[element.node_j] - displacement[element.node_i];
        let (force, tangent_stiffness) = spring_response(index, element, extension)?;
        let map = [element.node_i, element.node_j];
        let local = [
            [tangent_stiffness, -tangent_stiffness],
            [-tangent_stiffness, tangent_stiffness],
        ];

        internal_force[element.node_i] -= force;
        internal_force[element.node_j] += force;

        for row in 0..2 {
            for column in 0..2 {
                add_at(&mut tangent, map[row], map[column], local[row][column]);
            }
        }
        checkpoint_chunk(SolverStage::ElementAssembly, index + 1, elements.len())?;
    }

    for (index, contact) in contacts.iter().enumerate() {
        let (penetration, force) = contact_response(index, contact, displacement[contact.node])?;
        if penetration > 0.0 {
            internal_force[contact.node] += force;
            add_at(
                &mut tangent,
                contact.node,
                contact.node,
                contact.normal_stiffness,
            );
        }
        checkpoint_chunk(SolverStage::ElementAssembly, index + 1, contacts.len())?;
    }

    // Zero residual can skip the linear solver, so validate assembly before convergence.
    checkpoint(SolverStage::SparseValidateMatrix, 0)?;
    let mut entries = 0;
    for (row, force) in internal_force.iter().enumerate() {
        if !force.is_finite() {
            return Err(format!(
                "nonlinear spring internal force at node {row} is non-finite"
            ));
        }
        for &(column, value) in tangent.row_entries(row) {
            if !value.is_finite() {
                return Err(format!(
                    "nonlinear spring tangent at ({row}, {column}) is non-finite"
                ));
            }
            entries += 1;
            if entries % 64 == 0 {
                checkpoint(SolverStage::SparseValidateMatrix, entries)?;
            }
        }
        checkpoint_chunk(SolverStage::SparseValidateRhs, row + 1, node_count)?;
    }
    checkpoint(SolverStage::SparseValidateMatrix, entries)?;
    Ok((tangent, internal_force))
}

fn spring_response(
    index: usize,
    element: &NonlinearSpring1dElementInput,
    extension: f64,
) -> Result<(f64, f64), String> {
    // Horner form cancels softening terms before the final multiplication;
    // fused sums avoid intermediate overflow and rounding away the small force.
    let weighted_cubic = element.cubic_stiffness * extension;
    let force = weighted_cubic.mul_add(extension, element.stiffness) * extension;
    let cubic_secant = weighted_cubic * extension;
    let tangent = cubic_secant.mul_add(3.0, element.stiffness);
    if !(extension.is_finite() && force.is_finite() && tangent.is_finite()) {
        return Err(format!(
            "nonlinear spring element {index} ({}) has non-finite extension, force or tangent",
            element.id,
        ));
    }
    Ok((force, tangent))
}

fn contact_response(
    index: usize,
    contact: &ContactGap1dContactInput,
    displacement: f64,
) -> Result<(f64, f64), String> {
    let penetration = if displacement <= contact.gap {
        0.0
    } else {
        displacement - contact.gap
    };
    let force = contact.normal_stiffness * penetration;
    if !(displacement.is_finite() && penetration.is_finite() && force.is_finite()) {
        return Err(format!(
            "contact gap {index} ({}) has non-finite displacement, penetration or force",
            contact.id,
        ));
    }
    Ok((penetration, force))
}

fn checked_residual(
    nodes: &[kyuubiki_protocol::NonlinearSpring1dNodeInput],
    internal: &[f64],
    load_factor: f64,
) -> Result<Vec<f64>, String> {
    checkpoint(SolverStage::SparseResidual, 0)?;
    nodes
        .iter()
        .zip(internal)
        .enumerate()
        .map(|(index, (node, &force))| {
            // Support loads contribute reactions, not a free equilibrium equation.
            let residual = if node.fix_x {
                0.0
            } else {
                load_factor * node.load_x - force
            };
            if !residual.is_finite() {
                return Err(format!(
                    "nonlinear spring residual at node {index} is non-finite"
                ));
            }
            checkpoint_chunk(SolverStage::SparseResidual, index + 1, nodes.len())?;
            Ok(residual)
        })
        .collect()
}

fn checked_max_abs(
    values: impl ExactSizeIterator<Item = f64>,
    stage: SolverStage,
) -> Result<f64, String> {
    checkpoint(stage, 0)?;
    let count = values.len();
    let mut maximum = 0.0_f64;
    for (index, value) in values.enumerate() {
        if !value.is_finite() {
            return Err(format!(
                "nonlinear spring {stage:?} value {index} is non-finite"
            ));
        }
        maximum = maximum.max(value.abs());
        checkpoint_chunk(stage, index + 1, count)?;
    }
    Ok(maximum)
}

fn update_displacement(
    base: &[f64],
    displacement: &mut [f64],
    free: &[usize],
    delta: &[f64],
    scale: f64,
) -> Result<(), String> {
    checkpoint(SolverStage::PcgVectorUpdate, 0)?;
    for (index, &dof) in free.iter().enumerate() {
        let value = base[dof] + scale * delta[index];
        if !value.is_finite() {
            return Err(format!(
                "nonlinear spring displacement at node {dof} is non-finite"
            ));
        }
        displacement[dof] = value;
        checkpoint_chunk(SolverStage::PcgVectorUpdate, index + 1, free.len())?;
    }
    Ok(())
}

fn copy_displacement(base: &[f64], target: &mut [f64]) -> Result<(), String> {
    for (index, (&value, output)) in base.iter().zip(target).enumerate() {
        *output = value;
        checkpoint_chunk(SolverStage::StabilityRecovery, index + 1, base.len())?;
    }
    Ok(())
}

fn nonlinear_nodes(
    nodes: &[kyuubiki_protocol::NonlinearSpring1dNodeInput],
    displacement: &[f64],
) -> Result<Vec<NonlinearSpring1dNodeResult>, String> {
    checkpoint(SolverStage::ResultNodes, 0)?;
    nodes
        .iter()
        .enumerate()
        .map(|(index, node)| {
            checkpoint_chunk(SolverStage::ResultNodes, index + 1, nodes.len())?;
            Ok(NonlinearSpring1dNodeResult {
                index,
                id: node.id.clone(),
                x: node.x,
                ux: displacement[index],
            })
        })
        .collect()
}

fn nonlinear_elements(
    nodes: &[kyuubiki_protocol::NonlinearSpring1dNodeInput],
    elements: &[kyuubiki_protocol::NonlinearSpring1dElementInput],
    displacement: &[f64],
) -> Result<Vec<NonlinearSpring1dElementResult>, String> {
    checkpoint(SolverStage::ResultElements, 0)?;
    elements
        .iter()
        .enumerate()
        .map(|(index, element)| {
            let extension = displacement[element.node_j] - displacement[element.node_i];
            let (force, tangent_stiffness) = spring_response(index, element, extension)?;
            checkpoint_chunk(SolverStage::ResultElements, index + 1, elements.len())?;

            Ok(NonlinearSpring1dElementResult {
                index,
                id: element.id.clone(),
                node_i: element.node_i,
                node_j: element.node_j,
                length: (nodes[element.node_j].x - nodes[element.node_i].x).abs(),
                extension,
                force,
                tangent_stiffness,
            })
        })
        .collect()
}

fn contact_results(
    request: &SolveContactGap1dRequest,
    displacement: &[f64],
) -> Result<Vec<ContactGap1dContactResult>, String> {
    checkpoint(SolverStage::ResultElements, 0)?;
    request
        .contacts
        .iter()
        .enumerate()
        .map(|(index, contact)| {
            let (penetration, force) =
                contact_response(index, contact, displacement[contact.node])?;
            checkpoint_chunk(
                SolverStage::ResultElements,
                index + 1,
                request.contacts.len(),
            )?;
            Ok(ContactGap1dContactResult {
                index,
                id: contact.id.clone(),
                node: contact.node,
                gap: contact.gap,
                penetration,
                force,
                active: penetration > 0.0,
            })
        })
        .collect()
}

#[cfg(test)]
mod response_tests {
    use super::*;

    fn response(k: f64, c: f64, u: f64) -> Result<(f64, f64), String> {
        spring_response(
            0,
            &NonlinearSpring1dElementInput {
                id: "softening".into(),
                node_i: 0,
                node_j: 1,
                stiffness: k,
                cubic_stiffness: c,
            },
            u,
        )
    }

    #[test]
    fn finite_tangent_survives_overflow_in_the_separate_cubic_term() {
        let unit = 2.0_f64.powi(1022);
        for sign in [-1.0, 1.0] {
            let (force, tangent) = response(3.0 * unit, -1.5 * unit, sign).unwrap();
            assert_eq!(force, sign * 1.5 * unit);
            assert_eq!(tangent, -1.5 * unit);
        }
    }

    #[test]
    fn finite_force_survives_overflow_in_the_separate_linear_term() {
        let unit = 2.0_f64.powi(1022);
        for sign in [-1.0, 1.0] {
            let (force, tangent) = response(2.5 * unit, -0.25 * unit, sign * 2.0).unwrap();
            assert_eq!(force, sign * 3.0 * unit);
            assert_eq!(tangent, -0.5 * unit);
        }
    }

    #[test]
    fn softening_force_near_cancellation_matches_a_factored_reference() {
        // With c = -k and u = 1 + d, f/k = -d * (2 + 3*d + d*d).
        for exponent in [-26, -27, -30, -40, -50] {
            let delta = 2.0_f64.powi(exponent);
            for unit in [2.0_f64.powi(-500), 1.0, 2.0_f64.powi(500)] {
                for sign in [-1.0, 1.0] {
                    let expected = -sign * delta * (2.0 + 3.0 * delta + delta * delta);
                    let (force, _) = response(unit, -unit, sign * (1.0 + delta)).unwrap();
                    assert!(
                        ((force / unit) / expected - 1.0).abs() <= 4.0 * f64::EPSILON,
                        "exponent={exponent}, unit={unit}, sign={sign}, force={force}"
                    );
                }
            }
        }
    }

    #[test]
    fn genuinely_nonfinite_constitutive_values_still_fail() {
        let unit = 2.0_f64.powi(1022);
        for (k, c, u) in [
            (unit, unit, 1.0),
            (unit, 0.0, 4.0),
            (unit, -unit, 2.0),
            (1.0, 0.0, f64::INFINITY),
        ] {
            let error = response(k, c, u).unwrap_err();
            assert!(error.contains("softening") && error.contains("non-finite"));
        }
    }
}
