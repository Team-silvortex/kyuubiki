use crate::frame_2d_branch_switch::BranchSwitchContext;
use crate::frame_2d_corotational::normalized_residual;
use crate::frame_2d_corotational_element::{assemble_internal, assemble_tangent_and_internal};
use crate::linear_algebra::{reduce_sparse_system, sparse_to_dense};
use crate::linear_dense::solve_linear_system;

const MAX_LINE_SEARCH_STEPS: usize = 12;
const MIN_STEP_SCALE: f64 = 1.0 / 4096.0;

pub(crate) struct ModalConstraint<'a> {
    pub(crate) mode: &'a [f64],
    pub(crate) target: f64,
}

pub(crate) struct BranchState {
    pub(crate) displacement: Vec<f64>,
    pub(crate) load_factor: f64,
    pub(crate) iterations: usize,
    pub(crate) residual_norm: f64,
    pub(crate) constraint_error: f64,
}

pub(crate) fn solve_modal_constraints(
    context: &BranchSwitchContext<'_>,
    critical_displacement: &[f64],
    constraints: &[ModalConstraint<'_>],
    state: &mut BranchState,
) -> Result<bool, String> {
    if constraints.is_empty() {
        return Err("branch-switch modal constraint set must not be empty".into());
    }
    let reduced_modes = constraints
        .iter()
        .map(|constraint| {
            context
                .free_dofs
                .iter()
                .map(|&dof| constraint.mode[dof])
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let reduced_reference = context
        .free_dofs
        .iter()
        .map(|&dof| context.system.reference_force[dof])
        .collect::<Vec<_>>();
    let constraint_scale = constraints
        .iter()
        .map(|constraint| constraint.target.abs())
        .fold(0.0_f64, f64::max)
        .max(1.0e-12);

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
        let constraint_residuals =
            constraint_residuals(&state.displacement, critical_displacement, constraints);
        state.constraint_error =
            normalized_constraint_error(&constraint_residuals, constraint_scale);
        if state.residual_norm <= context.tolerance && state.constraint_error <= context.tolerance {
            return Ok(true);
        }

        let correction = solve_augmented_correction(
            sparse_to_dense(&reduced_tangent),
            &reduced_reference,
            &reduced_modes,
            constraints,
            reduced_residual,
            constraint_residuals,
        )?;
        if !apply_backtracked_correction(
            context,
            critical_displacement,
            constraints,
            constraint_scale,
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
    modes: &[Vec<f64>],
    constraints: &[ModalConstraint<'_>],
    residual: Vec<f64>,
    constraint_residuals: Vec<f64>,
) -> Result<Vec<f64>, String> {
    let size = residual.len();
    let constraint_count = modes.len();
    let augmented_size = size + constraint_count;
    let gauges = orthogonal_gauge_modes(modes, constraints)?;
    let mut augmented = vec![vec![0.0; augmented_size]; augmented_size];
    for row in 0..size {
        augmented[row][..size].copy_from_slice(&tangent[row]);
        augmented[row][size] = -reference_force[row];
        for (gauge_index, gauge) in gauges.iter().enumerate() {
            augmented[row][size + 1 + gauge_index] = gauge[row];
        }
    }
    for (constraint_index, mode) in modes.iter().enumerate() {
        augmented[size + constraint_index][..size].copy_from_slice(mode);
    }
    let mut right_hand_side = residual;
    right_hand_side.extend(constraint_residuals);
    solve_linear_system(augmented, right_hand_side)
        .map_err(|error| format!("branch-switch augmented tangent solve failed: {error}"))
}

fn orthogonal_gauge_modes(
    modes: &[Vec<f64>],
    constraints: &[ModalConstraint<'_>],
) -> Result<Vec<Vec<f64>>, String> {
    if modes.len() <= 1 {
        return Ok(Vec::new());
    }
    let mut target = constraints
        .iter()
        .map(|constraint| constraint.target)
        .collect::<Vec<_>>();
    normalize(&mut target)?;
    let mut coefficient_gauges: Vec<Vec<f64>> = Vec::with_capacity(modes.len() - 1);
    for axis in 0..modes.len() {
        let mut gauge = vec![0.0; modes.len()];
        gauge[axis] = 1.0;
        subtract_projection(&mut gauge, &target);
        for existing in &coefficient_gauges {
            subtract_projection(&mut gauge, existing);
        }
        if normalize(&mut gauge).is_ok() {
            coefficient_gauges.push(gauge);
        }
        if coefficient_gauges.len() + 1 == modes.len() {
            break;
        }
    }
    if coefficient_gauges.len() + 1 != modes.len() {
        return Err("branch-switch gauge basis is rank deficient".into());
    }
    Ok(coefficient_gauges
        .iter()
        .map(|coefficients| {
            let mut gauge = vec![0.0; modes[0].len()];
            for (coefficient, mode) in coefficients.iter().zip(modes) {
                for (value, mode_value) in gauge.iter_mut().zip(mode) {
                    *value += coefficient * mode_value;
                }
            }
            gauge
        })
        .collect())
}

fn apply_backtracked_correction(
    context: &BranchSwitchContext<'_>,
    critical_displacement: &[f64],
    constraints: &[ModalConstraint<'_>],
    constraint_scale: f64,
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
        let constraint_residuals =
            constraint_residuals(&trial_displacement, critical_displacement, constraints);
        let constraint_error = normalized_constraint_error(&constraint_residuals, constraint_scale);
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

fn constraint_residuals(
    displacement: &[f64],
    critical: &[f64],
    constraints: &[ModalConstraint<'_>],
) -> Vec<f64> {
    constraints
        .iter()
        .map(|constraint| {
            constraint.target - modal_projection(displacement, critical, constraint.mode)
        })
        .collect()
}

fn normalized_constraint_error(residuals: &[f64], scale: f64) -> f64 {
    residuals
        .iter()
        .map(|residual| residual.abs() / scale)
        .fold(0.0_f64, f64::max)
}

pub(crate) fn modal_projection(displacement: &[f64], critical: &[f64], mode: &[f64]) -> f64 {
    displacement
        .iter()
        .zip(critical)
        .zip(mode)
        .map(|((value, critical), mode)| (value - critical) * mode)
        .sum()
}

fn normalize(values: &mut [f64]) -> Result<(), String> {
    let norm = values.iter().map(|value| value * value).sum::<f64>().sqrt();
    if !(norm.is_finite() && norm > 1.0e-12) {
        return Err("branch-switch gauge direction has zero norm".into());
    }
    for value in values {
        *value /= norm;
    }
    Ok(())
}

fn subtract_projection(values: &mut [f64], basis: &[f64]) {
    let projection = values
        .iter()
        .zip(basis)
        .map(|(value, basis)| value * basis)
        .sum::<f64>();
    for (value, basis) in values.iter_mut().zip(basis) {
        *value -= projection * basis;
    }
}

fn residual(external: &[f64], internal: &[f64], load_factor: f64) -> Vec<f64> {
    external
        .iter()
        .zip(internal)
        .map(|(external, internal)| load_factor * external - internal)
        .collect()
}
