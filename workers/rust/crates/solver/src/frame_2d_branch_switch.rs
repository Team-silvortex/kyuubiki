use crate::frame_2d_corotational::{correct_corotational_equilibrium, normalized_residual};
use crate::frame_2d_corotational_element::{assemble_internal, assemble_tangent_and_internal};
use crate::frame_2d_stability::Frame2dStabilitySystem;
use crate::linear_algebra::{reduce_sparse_system, sparse_to_dense};
use crate::linear_dense::solve_linear_system;
use crate::symmetric_critical_mode::SymmetricCriticalMode;
use kyuubiki_protocol::{
    Frame2dBranchDirection, Frame2dBranchModeComponent, Frame2dBranchSwitchProbeResult,
    Frame2dBranchSwitchSelection, Frame2dElementInput,
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
    mode_index: usize,
    mode_eigenvalue: f64,
    amplitude: f64,
    selection: Frame2dBranchSwitchSelection,
) -> Vec<Frame2dBranchSwitchProbeResult> {
    let components = vec![Frame2dBranchModeComponent {
        mode_index,
        normalized_eigenvalue: Some(mode_eigenvalue),
        weight: 1.0,
    }];
    let component_modes = [critical_mode];
    probe_direction_family(
        context,
        critical_displacement,
        primary_displacement,
        critical_load_factor,
        critical_mode,
        mode_index,
        Some(mode_eigenvalue),
        &components,
        &component_modes,
        amplitude,
        selection,
    )
}

pub(crate) fn probe_pairwise_branch_switches(
    context: &BranchSwitchContext<'_>,
    critical_displacement: &[f64],
    primary_displacement: &[f64],
    critical_load_factor: f64,
    critical_modes: &[SymmetricCriticalMode],
    amplitude: f64,
    selection: Frame2dBranchSwitchSelection,
) -> Vec<Frame2dBranchSwitchProbeResult> {
    let mut probes = Vec::new();
    for left_index in 0..critical_modes.len() {
        for right_index in left_index + 1..critical_modes.len() {
            for relative_sign in [1.0, -1.0] {
                let Some((shape, left_weight, right_weight)) = combine_modes(
                    &critical_modes[left_index].shape,
                    &critical_modes[right_index].shape,
                    relative_sign,
                ) else {
                    continue;
                };
                let components = vec![
                    Frame2dBranchModeComponent {
                        mode_index: left_index,
                        normalized_eigenvalue: Some(
                            critical_modes[left_index].normalized_eigenvalue,
                        ),
                        weight: left_weight,
                    },
                    Frame2dBranchModeComponent {
                        mode_index: right_index,
                        normalized_eigenvalue: Some(
                            critical_modes[right_index].normalized_eigenvalue,
                        ),
                        weight: right_weight,
                    },
                ];
                let component_modes = [
                    critical_modes[left_index].shape.as_slice(),
                    critical_modes[right_index].shape.as_slice(),
                ];
                probes.extend(probe_direction_family(
                    context,
                    critical_displacement,
                    primary_displacement,
                    critical_load_factor,
                    &shape,
                    left_index,
                    None,
                    &components,
                    &component_modes,
                    amplitude,
                    selection,
                ));
            }
        }
    }
    probes
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn probe_weighted_branch_switches(
    context: &BranchSwitchContext<'_>,
    critical_displacement: &[f64],
    primary_displacement: &[f64],
    critical_load_factor: f64,
    critical_modes: &[SymmetricCriticalMode],
    weights: &[f64],
    amplitude: f64,
    selection: Frame2dBranchSwitchSelection,
) -> Vec<Frame2dBranchSwitchProbeResult> {
    let Some(normalized_weights) = normalize_weights(weights) else {
        return Vec::new();
    };
    if normalized_weights.len() != critical_modes.len() || critical_modes.is_empty() {
        return Vec::new();
    }
    let mut shape = vec![0.0; critical_modes[0].shape.len()];
    for (mode, weight) in critical_modes.iter().zip(&normalized_weights) {
        if mode.shape.len() != shape.len() {
            return Vec::new();
        }
        for (value, mode_value) in shape.iter_mut().zip(&mode.shape) {
            *value += weight * mode_value;
        }
    }
    let shape_norm = shape.iter().map(|value| value * value).sum::<f64>().sqrt();
    if !(shape_norm.is_finite() && shape_norm > f64::EPSILON) {
        return Vec::new();
    }
    for value in &mut shape {
        *value /= shape_norm;
    }
    let active = normalized_weights
        .iter()
        .enumerate()
        .filter(|(_, weight)| **weight != 0.0)
        .collect::<Vec<_>>();
    let mode_index = active[0].0;
    let components = active
        .iter()
        .map(|(index, weight)| Frame2dBranchModeComponent {
            mode_index: *index,
            normalized_eigenvalue: Some(critical_modes[*index].normalized_eigenvalue),
            weight: **weight / shape_norm,
        })
        .collect::<Vec<_>>();
    let component_modes = active
        .iter()
        .map(|(index, _)| critical_modes[*index].shape.as_slice())
        .collect::<Vec<_>>();
    probe_direction_family(
        context,
        critical_displacement,
        primary_displacement,
        critical_load_factor,
        &shape,
        mode_index,
        None,
        &components,
        &component_modes,
        amplitude,
        selection,
    )
}

#[allow(clippy::too_many_arguments)]
fn probe_direction_family(
    context: &BranchSwitchContext<'_>,
    critical_displacement: &[f64],
    primary_displacement: &[f64],
    critical_load_factor: f64,
    critical_mode: &[f64],
    mode_index: usize,
    mode_eigenvalue: Option<f64>,
    mode_components: &[Frame2dBranchModeComponent],
    component_modes: &[&[f64]],
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
                mode_index,
                mode_eigenvalue,
                mode_components,
                component_modes,
                amplitude,
                direction,
            )
        })
        .collect()
}

pub(crate) fn unavailable_branch_switches(
    selection: Frame2dBranchSwitchSelection,
    mode_index: usize,
    amplitude: f64,
    detail: &str,
) -> Vec<Frame2dBranchSwitchProbeResult> {
    directions(selection)
        .into_iter()
        .map(|direction| {
            failed_result(
                direction,
                mode_index,
                None,
                vec![Frame2dBranchModeComponent {
                    mode_index,
                    normalized_eigenvalue: None,
                    weight: 1.0,
                }],
                amplitude,
                0,
                None,
                detail.to_string(),
            )
        })
        .collect()
}

pub(crate) fn unavailable_pairwise_branch_switches(
    selection: Frame2dBranchSwitchSelection,
    mode_count: usize,
    amplitude: f64,
    detail: &str,
) -> Vec<Frame2dBranchSwitchProbeResult> {
    let mut probes = Vec::new();
    for left_index in 0..mode_count {
        for right_index in left_index + 1..mode_count {
            for relative_sign in [1.0, -1.0] {
                let components = vec![
                    Frame2dBranchModeComponent {
                        mode_index: left_index,
                        normalized_eigenvalue: None,
                        weight: std::f64::consts::FRAC_1_SQRT_2,
                    },
                    Frame2dBranchModeComponent {
                        mode_index: right_index,
                        normalized_eigenvalue: None,
                        weight: relative_sign * std::f64::consts::FRAC_1_SQRT_2,
                    },
                ];
                probes.extend(directions(selection).into_iter().map(|direction| {
                    failed_result(
                        direction,
                        left_index,
                        None,
                        components.clone(),
                        amplitude,
                        0,
                        None,
                        detail.to_string(),
                    )
                }));
            }
        }
    }
    probes
}

pub(crate) fn unavailable_weighted_branch_switches(
    selection: Frame2dBranchSwitchSelection,
    weights: &[f64],
    amplitude: f64,
    detail: &str,
) -> Vec<Frame2dBranchSwitchProbeResult> {
    let Some(weights) = normalize_weights(weights) else {
        return Vec::new();
    };
    let components = weights
        .iter()
        .enumerate()
        .filter(|(_, weight)| **weight != 0.0)
        .map(|(mode_index, weight)| Frame2dBranchModeComponent {
            mode_index,
            normalized_eigenvalue: None,
            weight: *weight,
        })
        .collect::<Vec<_>>();
    let mode_index = components[0].mode_index;
    directions(selection)
        .into_iter()
        .map(|direction| {
            failed_result(
                direction,
                mode_index,
                None,
                components.clone(),
                amplitude,
                0,
                None,
                detail.to_string(),
            )
        })
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
    mode_index: usize,
    mode_eigenvalue: Option<f64>,
    mode_components: &[Frame2dBranchModeComponent],
    component_modes: &[&[f64]],
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
            mode_index,
            mode_eigenvalue,
            mode_components.to_vec(),
            component_modes,
            amplitude,
            target_projection,
            critical_displacement,
            primary_displacement,
            critical_mode,
            state,
        ),
        Ok(false) => failed_result(
            direction,
            mode_index,
            mode_eigenvalue,
            mode_components.to_vec(),
            amplitude,
            state.iterations,
            Some(state),
            "modal-constraint Newton iteration did not converge".into(),
        ),
        Err(error) => failed_result(
            direction,
            mode_index,
            mode_eigenvalue,
            mode_components.to_vec(),
            amplitude,
            state.iterations,
            Some(state),
            error,
        ),
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
    mode_index: usize,
    mode_eigenvalue: Option<f64>,
    mode_components: Vec<Frame2dBranchModeComponent>,
    component_modes: &[&[f64]],
    amplitude: f64,
    target_projection: f64,
    critical_displacement: &[f64],
    primary_displacement: &[f64],
    critical_mode: &[f64],
    state: BranchState,
) -> Frame2dBranchSwitchProbeResult {
    let projection = modal_projection(&state.displacement, critical_displacement, critical_mode);
    let mode_component_projections = component_modes
        .iter()
        .map(|mode| modal_projection(&state.displacement, critical_displacement, mode))
        .collect();
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
        mode_index,
        mode_eigenvalue,
        mode_components,
        mode_component_projections,
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
        continuation_steps: Vec::new(),
        continuation_converged: None,
        continuation_failure_detail: None,
    }
}

fn failed_result(
    direction: Frame2dBranchDirection,
    mode_index: usize,
    mode_eigenvalue: Option<f64>,
    mode_components: Vec<Frame2dBranchModeComponent>,
    amplitude: f64,
    iterations: usize,
    state: Option<BranchState>,
    detail: String,
) -> Frame2dBranchSwitchProbeResult {
    Frame2dBranchSwitchProbeResult {
        mode_index,
        mode_eigenvalue,
        mode_components,
        mode_component_projections: Vec::new(),
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
        continuation_steps: Vec::new(),
        continuation_converged: None,
        continuation_failure_detail: None,
    }
}

fn finite_value(value: f64) -> Option<f64> {
    value.is_finite().then_some(value)
}

fn combine_modes(left: &[f64], right: &[f64], relative_sign: f64) -> Option<(Vec<f64>, f64, f64)> {
    if left.len() != right.len() {
        return None;
    }
    let mut shape = left
        .iter()
        .zip(right)
        .map(|(left, right)| left + relative_sign * right)
        .collect::<Vec<_>>();
    let norm = shape.iter().map(|value| value * value).sum::<f64>().sqrt();
    if !(norm.is_finite() && norm > f64::EPSILON) {
        return None;
    }
    for value in &mut shape {
        *value /= norm;
    }
    Some((shape, 1.0 / norm, relative_sign / norm))
}

fn normalize_weights(weights: &[f64]) -> Option<Vec<f64>> {
    let scale = weights
        .iter()
        .map(|weight| weight.abs())
        .fold(0.0, f64::max);
    if !(scale.is_finite() && scale > 0.0) {
        return None;
    }
    let scaled = weights
        .iter()
        .map(|weight| weight / scale)
        .collect::<Vec<_>>();
    let norm = scaled
        .iter()
        .map(|weight| weight * weight)
        .sum::<f64>()
        .sqrt();
    (norm.is_finite() && norm > f64::EPSILON)
        .then(|| scaled.into_iter().map(|weight| weight / norm).collect())
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
#[path = "frame_2d_branch_switch_tests.rs"]
mod tests;
