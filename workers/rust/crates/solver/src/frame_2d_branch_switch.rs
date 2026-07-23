use crate::frame_2d_corotational::{correct_corotational_equilibrium, normalized_residual};
use crate::frame_2d_corotational_element::{assemble_internal, assemble_tangent_and_internal};
use crate::frame_2d_stability::Frame2dStabilitySystem;
use crate::linear_algebra::{reduce_sparse_system, sparse_to_dense};
use crate::linear_dense::solve_linear_system;
use kyuubiki_protocol::{
    Frame2dBranchDirection, Frame2dBranchSwitchProbeResult, Frame2dBranchSwitchSelection,
    Frame2dElementInput,
};

const MAX_LINE_SEARCH_STEPS: usize = 12;
const MIN_STEP_SCALE: f64 = 1.0 / 4096.0;

pub(crate) struct BranchSwitchContext<'a> {
    pub(crate) positions: &'a [(f64, f64)],
    pub(crate) elements: &'a [Frame2dElementInput],
    pub(crate) system: &'a Frame2dStabilitySystem,
    pub(crate) free_dofs: &'a [usize],
    pub(crate) max_iterations: usize,
    pub(crate) tolerance: f64,
}

struct BranchState {
    displacement: Vec<f64>,
    load_factor: f64,
    iterations: usize,
    residual_norm: f64,
    constraint_error: f64,
}

pub(crate) fn probe_branch_switches(
    context: &BranchSwitchContext<'_>,
    critical_displacement: &[f64],
    primary_displacement: &[f64],
    critical_load_factor: f64,
    critical_mode: &[f64],
    amplitude: f64,
    selection: Frame2dBranchSwitchSelection,
) -> Vec<Frame2dBranchSwitchProbeResult> {
    directions(selection)
        .into_iter()
        .map(|direction| {
            solve_direction(
                context,
                critical_displacement,
                primary_displacement,
                critical_load_factor,
                critical_mode,
                amplitude,
                direction,
            )
        })
        .collect()
}

pub(crate) fn unavailable_branch_switches(
    selection: Frame2dBranchSwitchSelection,
    amplitude: f64,
    detail: &str,
) -> Vec<Frame2dBranchSwitchProbeResult> {
    directions(selection)
        .into_iter()
        .map(|direction| failed_result(direction, amplitude, 0, None, detail.to_string()))
        .collect()
}

fn directions(selection: Frame2dBranchSwitchSelection) -> Vec<Frame2dBranchDirection> {
    match selection {
        Frame2dBranchSwitchSelection::Disabled => Vec::new(),
        Frame2dBranchSwitchSelection::Positive => vec![Frame2dBranchDirection::Positive],
        Frame2dBranchSwitchSelection::Negative => vec![Frame2dBranchDirection::Negative],
        Frame2dBranchSwitchSelection::Both => vec![
            Frame2dBranchDirection::Positive,
            Frame2dBranchDirection::Negative,
        ],
    }
}

fn solve_direction(
    context: &BranchSwitchContext<'_>,
    critical_displacement: &[f64],
    primary_displacement: &[f64],
    critical_load_factor: f64,
    critical_mode: &[f64],
    amplitude: f64,
    direction: Frame2dBranchDirection,
) -> Frame2dBranchSwitchProbeResult {
    let sign = match direction {
        Frame2dBranchDirection::Positive => 1.0,
        Frame2dBranchDirection::Negative => -1.0,
    };
    let target_projection = sign * amplitude;
    let mut state = BranchState {
        displacement: critical_displacement
            .iter()
            .zip(critical_mode)
            .map(|(value, mode)| value + target_projection * mode)
            .collect(),
        load_factor: critical_load_factor,
        iterations: 0,
        residual_norm: f64::INFINITY,
        constraint_error: f64::INFINITY,
    };

    let outcome = solve_modal_constraint(
        context,
        critical_displacement,
        critical_mode,
        target_projection,
        &mut state,
    );
    match outcome {
        Ok(true) => successful_result(
            context,
            direction,
            amplitude,
            target_projection,
            critical_displacement,
            primary_displacement,
            critical_mode,
            state,
        ),
        Ok(false) => failed_result(
            direction,
            amplitude,
            state.iterations,
            Some(state),
            "modal-constraint Newton iteration did not converge".into(),
        ),
        Err(error) => failed_result(direction, amplitude, state.iterations, Some(state), error),
    }
}

fn solve_modal_constraint(
    context: &BranchSwitchContext<'_>,
    critical_displacement: &[f64],
    critical_mode: &[f64],
    target_projection: f64,
    state: &mut BranchState,
) -> Result<bool, String> {
    let reduced_mode = context
        .free_dofs
        .iter()
        .map(|&dof| critical_mode[dof])
        .collect::<Vec<_>>();
    let reduced_reference = context
        .free_dofs
        .iter()
        .map(|&dof| context.system.reference_force[dof])
        .collect::<Vec<_>>();
    let constraint_scale = target_projection.abs().max(1.0e-12);

    for iteration in 1..=context.max_iterations {
        state.iterations = iteration;
        let (tangent, internal) = assemble_tangent_and_internal(
            context.positions,
            context.elements,
            &state.displacement,
        )?;
        let residual = residual(
            &context.system.reference_force,
            &internal,
            state.load_factor,
        );
        let (reduced_tangent, reduced_residual, free) =
            reduce_sparse_system(&tangent, &residual, &context.system.constrained_dofs);
        debug_assert_eq!(free, context.free_dofs);
        state.residual_norm = normalized_residual(
            &reduced_residual,
            &context.system.reference_force,
            state.load_factor,
        );
        let projection =
            modal_projection(&state.displacement, critical_displacement, critical_mode);
        let constraint = target_projection - projection;
        state.constraint_error = constraint.abs() / constraint_scale;
        if state.residual_norm <= context.tolerance && state.constraint_error <= context.tolerance {
            return Ok(true);
        }

        let correction = solve_augmented_correction(
            sparse_to_dense(&reduced_tangent),
            &reduced_reference,
            &reduced_mode,
            reduced_residual,
            constraint,
        )?;
        if !apply_backtracked_correction(
            context,
            critical_displacement,
            critical_mode,
            target_projection,
            &correction,
            state,
        )? {
            return Err("branch-switch line search failed to reduce the coupled residual".into());
        }
    }
    Ok(false)
}

fn solve_augmented_correction(
    tangent: Vec<Vec<f64>>,
    reference_force: &[f64],
    mode: &[f64],
    residual: Vec<f64>,
    constraint: f64,
) -> Result<Vec<f64>, String> {
    let size = residual.len();
    let mut augmented = vec![vec![0.0; size + 1]; size + 1];
    for row in 0..size {
        augmented[row][..size].copy_from_slice(&tangent[row]);
        augmented[row][size] = -reference_force[row];
        augmented[size][row] = mode[row];
    }
    let mut right_hand_side = residual;
    right_hand_side.push(constraint);
    solve_linear_system(augmented, right_hand_side)
        .map_err(|error| format!("branch-switch augmented tangent solve failed: {error}"))
}

fn apply_backtracked_correction(
    context: &BranchSwitchContext<'_>,
    critical_displacement: &[f64],
    critical_mode: &[f64],
    target_projection: f64,
    correction: &[f64],
    state: &mut BranchState,
) -> Result<bool, String> {
    let current_objective = state.residual_norm.max(state.constraint_error);
    let displacement_correction = &correction[..context.free_dofs.len()];
    let load_correction = correction[context.free_dofs.len()];
    let mut scale = 1.0;
    for _ in 0..MAX_LINE_SEARCH_STEPS {
        let mut trial_displacement = state.displacement.clone();
        for (index, &dof) in context.free_dofs.iter().enumerate() {
            trial_displacement[dof] =
                state.displacement[dof] + scale * displacement_correction[index];
        }
        let trial_load_factor = state.load_factor + scale * load_correction;
        if !trial_load_factor.is_finite() {
            scale *= 0.5;
            continue;
        }
        let Ok(internal) =
            assemble_internal(context.positions, context.elements, &trial_displacement)
        else {
            scale *= 0.5;
            continue;
        };
        let trial_residual = residual(
            &context.system.reference_force,
            &internal,
            trial_load_factor,
        );
        let reduced_residual = context
            .free_dofs
            .iter()
            .map(|&dof| trial_residual[dof])
            .collect::<Vec<_>>();
        let residual_norm = normalized_residual(
            &reduced_residual,
            &context.system.reference_force,
            trial_load_factor,
        );
        let projection =
            modal_projection(&trial_displacement, critical_displacement, critical_mode);
        let constraint_error =
            (target_projection - projection).abs() / target_projection.abs().max(1.0e-12);
        if residual_norm.max(constraint_error) < current_objective {
            state.displacement = trial_displacement;
            state.load_factor = trial_load_factor;
            state.residual_norm = residual_norm;
            state.constraint_error = constraint_error;
            return Ok(true);
        }
        scale *= 0.5;
        if scale < MIN_STEP_SCALE {
            break;
        }
    }
    Ok(false)
}

fn successful_result(
    context: &BranchSwitchContext<'_>,
    direction: Frame2dBranchDirection,
    amplitude: f64,
    target_projection: f64,
    critical_displacement: &[f64],
    primary_displacement: &[f64],
    critical_mode: &[f64],
    state: BranchState,
) -> Frame2dBranchSwitchProbeResult {
    let projection = modal_projection(&state.displacement, critical_displacement, critical_mode);
    let distance = displacement_distance(&state.displacement, critical_displacement);
    let projection_matches =
        projection * target_projection > 0.0 && projection.abs() >= amplitude * 0.9;
    let primary = correct_corotational_equilibrium(
        context.positions,
        context.elements,
        context.system,
        primary_displacement,
        state.load_factor,
        context.max_iterations,
        context.tolerance,
    );
    let (primary_equilibrium_converged, primary_distance, classification_detail) = match primary {
        Ok(Some(primary)) => (
            true,
            Some(displacement_distance(&state.displacement, &primary)),
            None,
        ),
        Ok(None) => (
            false,
            None,
            Some("primary-path equilibrium correction did not converge".to_string()),
        ),
        Err(error) => (
            false,
            None,
            Some(format!(
                "primary-path equilibrium correction failed: {error}"
            )),
        ),
    };
    let distinct_branch =
        projection_matches && primary_distance.is_some_and(|distance| distance >= amplitude * 0.5);
    Frame2dBranchSwitchProbeResult {
        direction,
        seed_amplitude: amplitude,
        iterations: state.iterations,
        equilibrium_converged: true,
        primary_equilibrium_converged,
        distinct_branch,
        load_factor: Some(state.load_factor),
        residual_norm: Some(state.residual_norm),
        modal_constraint_error: Some(state.constraint_error),
        mode_projection: Some(projection),
        displacement_distance: Some(distance),
        primary_displacement_distance: primary_distance,
        displacements: Some(state.displacement),
        failure_detail: classification_detail,
    }
}

fn failed_result(
    direction: Frame2dBranchDirection,
    amplitude: f64,
    iterations: usize,
    state: Option<BranchState>,
    detail: String,
) -> Frame2dBranchSwitchProbeResult {
    Frame2dBranchSwitchProbeResult {
        direction,
        seed_amplitude: amplitude,
        iterations,
        equilibrium_converged: false,
        primary_equilibrium_converged: false,
        distinct_branch: false,
        load_factor: state
            .as_ref()
            .and_then(|state| finite_value(state.load_factor)),
        residual_norm: state
            .as_ref()
            .and_then(|state| finite_value(state.residual_norm)),
        modal_constraint_error: state
            .as_ref()
            .and_then(|state| finite_value(state.constraint_error)),
        mode_projection: None,
        displacement_distance: None,
        primary_displacement_distance: None,
        displacements: None,
        failure_detail: Some(detail),
    }
}

fn finite_value(value: f64) -> Option<f64> {
    value.is_finite().then_some(value)
}

fn modal_projection(displacement: &[f64], critical: &[f64], mode: &[f64]) -> f64 {
    displacement
        .iter()
        .zip(critical)
        .zip(mode)
        .map(|((value, critical), mode)| (value - critical) * mode)
        .sum()
}

fn displacement_distance(displacement: &[f64], critical: &[f64]) -> f64 {
    displacement
        .iter()
        .zip(critical)
        .map(|(value, critical)| (value - critical).powi(2))
        .sum::<f64>()
        .sqrt()
}

fn residual(external: &[f64], internal: &[f64], load_factor: f64) -> Vec<f64> {
    external
        .iter()
        .zip(internal)
        .map(|(external, internal)| load_factor * external - internal)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unavailable_bidirectional_probe_is_explicit() {
        let probes =
            unavailable_branch_switches(Frame2dBranchSwitchSelection::Both, 0.01, "unavailable");
        assert_eq!(probes.len(), 2);
        assert_eq!(probes[0].direction, Frame2dBranchDirection::Positive);
        assert_eq!(probes[1].direction, Frame2dBranchDirection::Negative);
        assert!(
            probes
                .iter()
                .all(|probe| !probe.equilibrium_converged && probe.failure_detail.is_some())
        );
    }
}
