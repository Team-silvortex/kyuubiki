use crate::frame_2d_branch_constraints::{
    BranchState, ModalConstraint, modal_projection, solve_modal_constraints,
};
use crate::frame_2d_corotational::correct_corotational_equilibrium;
use crate::frame_2d_stability::Frame2dStabilitySystem;
use crate::symmetric_critical_mode::SymmetricCriticalMode;
use kyuubiki_protocol::{
    Frame2dBranchDirection, Frame2dBranchModeComponent, Frame2dBranchProbeOrigin,
    Frame2dBranchSwitchProbeResult, Frame2dBranchSwitchSelection, Frame2dElementInput,
};

const DEGENERATE_EIGENVALUE_TOLERANCE: f64 = 1.0e-8;

pub(crate) struct BranchSwitchContext<'a> {
    pub(crate) positions: &'a [(f64, f64)],
    pub(crate) elements: &'a [Frame2dElementInput],
    pub(crate) system: &'a Frame2dStabilitySystem,
    pub(crate) free_dofs: &'a [usize],
    pub(crate) max_iterations: usize,
    pub(crate) tolerance: f64,
}

pub(crate) fn mark_probe_origin(
    mut probes: Vec<Frame2dBranchSwitchProbeResult>,
    origin: Frame2dBranchProbeOrigin,
    refinement_level: Option<usize>,
) -> Vec<Frame2dBranchSwitchProbeResult> {
    for probe in &mut probes {
        probe.origin = origin;
        probe.subspace_refinement_level = refinement_level;
    }
    probes
}

pub(crate) fn probe_branch_switches(
    context: &BranchSwitchContext<'_>,
    critical_displacement: &[f64],
    primary_displacement: &[f64],
    critical_load_factor: f64,
    critical_modes: &[SymmetricCriticalMode],
    mode_index: usize,
    amplitude: f64,
    selection: Frame2dBranchSwitchSelection,
) -> Vec<Frame2dBranchSwitchProbeResult> {
    let Some(selected_mode) = critical_modes.get(mode_index) else {
        return Vec::new();
    };
    let components = vec![Frame2dBranchModeComponent {
        mode_index,
        normalized_eigenvalue: Some(selected_mode.normalized_eigenvalue),
        weight: 1.0,
    }];
    let component_modes = [selected_mode.shape.as_slice()];
    let constraint_cluster = critical_modes
        .iter()
        .enumerate()
        .filter(|mode| {
            (mode.1.normalized_eigenvalue - selected_mode.normalized_eigenvalue).abs()
                <= DEGENERATE_EIGENVALUE_TOLERANCE
        })
        .collect::<Vec<_>>();
    let constraint_modes = constraint_cluster
        .iter()
        .map(|(_, mode)| mode.shape.as_slice())
        .collect::<Vec<_>>();
    let constraint_weights = constraint_cluster
        .iter()
        .map(|(index, _)| usize::from(*index == mode_index) as f64)
        .collect::<Vec<_>>();
    probe_direction_family(
        context,
        critical_displacement,
        primary_displacement,
        critical_load_factor,
        &selected_mode.shape,
        mode_index,
        Some(selected_mode.normalized_eigenvalue),
        &components,
        &component_modes,
        &constraint_modes,
        &constraint_weights,
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
                let constraint_weights = [left_weight, right_weight];
                let degenerate = (critical_modes[left_index].normalized_eigenvalue
                    - critical_modes[right_index].normalized_eigenvalue)
                    .abs()
                    <= DEGENERATE_EIGENVALUE_TOLERANCE;
                if !degenerate {
                    probes.extend(mark_probe_origin(
                        directions(selection)
                            .into_iter()
                            .map(|direction| {
                                failed_result(
                                    direction,
                                    left_index,
                                    None,
                                    components.clone(),
                                    amplitude,
                                    0,
                                    None,
                                    "pairwise branch probing requires a degenerate critical eigenspace"
                                        .into(),
                                )
                            })
                            .collect(),
                        Frame2dBranchProbeOrigin::PairwiseCombination,
                        None,
                    ));
                    continue;
                }
                probes.extend(mark_probe_origin(
                    probe_direction_family(
                        context,
                        critical_displacement,
                        primary_displacement,
                        critical_load_factor,
                        &shape,
                        left_index,
                        None,
                        &components,
                        &component_modes,
                        &component_modes,
                        &constraint_weights,
                        amplitude,
                        selection,
                    ),
                    Frame2dBranchProbeOrigin::PairwiseCombination,
                    None,
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
    let constraint_modes = [shape.as_slice()];
    let constraint_weights = [1.0];
    mark_probe_origin(
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
            &constraint_modes,
            &constraint_weights,
            amplitude,
            selection,
        ),
        Frame2dBranchProbeOrigin::CallerWeighted,
        None,
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
    constraint_modes: &[&[f64]],
    constraint_weights: &[f64],
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
                constraint_modes,
                constraint_weights,
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
                probes.extend(mark_probe_origin(
                    directions(selection)
                        .into_iter()
                        .map(|direction| {
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
                        })
                        .collect(),
                    Frame2dBranchProbeOrigin::PairwiseCombination,
                    None,
                ));
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
    mark_probe_origin(
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
            .collect(),
        Frame2dBranchProbeOrigin::CallerWeighted,
        None,
    )
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
    constraint_modes: &[&[f64]],
    constraint_weights: &[f64],
    amplitude: f64,
    direction: Frame2dBranchDirection,
) -> Frame2dBranchSwitchProbeResult {
    let sign = match direction {
        Frame2dBranchDirection::Positive => 1.0,
        Frame2dBranchDirection::Negative => -1.0,
    };
    let target_projection = sign * amplitude;
    let constraints = constraint_modes
        .iter()
        .zip(constraint_weights)
        .map(|(mode, weight)| ModalConstraint {
            mode,
            target: target_projection * weight,
        })
        .collect::<Vec<_>>();
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

    let outcome = solve_modal_constraints(context, critical_displacement, &constraints, &mut state);
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
        origin: Frame2dBranchProbeOrigin::CriticalMode,
        subspace_refinement_level: None,
        subspace_parent_angle_radians: None,
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
        origin: Frame2dBranchProbeOrigin::CriticalMode,
        subspace_refinement_level: None,
        subspace_parent_angle_radians: None,
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

fn displacement_distance(displacement: &[f64], critical: &[f64]) -> f64 {
    displacement
        .iter()
        .zip(critical)
        .map(|(value, critical)| (value - critical).powi(2))
        .sum::<f64>()
        .sqrt()
}

#[cfg(test)]
#[path = "frame_2d_branch_switch_tests.rs"]
mod tests;
