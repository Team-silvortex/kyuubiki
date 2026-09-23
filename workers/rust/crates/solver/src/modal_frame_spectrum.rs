use crate::linear_algebra::stable_l2_norm;
use crate::modal_math::{ensure_dense_modal_size, expand_mode_shape, jacobi_eigenpairs};
use crate::modal_sparse::{
    InverseIterationOptions, ReducedSparseModalSystem, inverse_power_iteration,
};
use crate::solver_control::{SolverStage, checkpoint};

// Bound the complete-spectrum check for single-mode requests; large models remain sparse.
const SINGLE_MODE_DENSE_LIMIT: usize = 128;

pub(crate) fn frame_eigenpairs(
    system: &ReducedSparseModalSystem,
    mode_count: Option<usize>,
) -> Result<Vec<(f64, Vec<f64>)>, String> {
    let size = system.free_dofs.len();
    let options = InverseIterationOptions::default();
    let (mut pairs, residual_tolerance) = if mode_count == Some(1) {
        if let Some(pair) = system
            .operator
            .smallest_tridiagonal_eigenpair(options.tolerance)
        {
            let pair = pair?;
            // The recognized long axial chain has its own bounded cancellation floor.
            (vec![(pair.eigenvalue, pair.vector)], 2.0e-4)
        } else if size <= SINGLE_MODE_DENSE_LIMIT {
            (dense_eigenpairs(system)?, 1.0e-8)
        } else {
            let pair = inverse_power_iteration(
                size,
                options,
                |vector| system.operator.apply(vector),
                |rhs| system.solve_normalized_inverse(rhs),
            )?;
            (vec![(pair.eigenvalue, pair.vector)], options.tolerance)
        }
    } else {
        (dense_eigenpairs(system)?, 1.0e-8)
    };
    // These are restrained-frame solvers, not free-free solvers. Never conceal a
    // nonpositive mode or discard a resolved soft mode using the stiffest eigenvalue.
    if pairs.is_empty()
        || pairs
            .iter()
            .any(|(value, _)| !value.is_finite() || *value <= 0.0)
    {
        return Err("modal frame spectrum contains a nonpositive or non-finite eigenvalue".into());
    }
    pairs.truncate(mode_count.unwrap_or(6).max(1));
    for (index, (value, vector)) in pairs.iter().enumerate() {
        checkpoint(SolverStage::ModalValidation, index)?;
        let norm = stable_l2_norm(vector.iter().copied());
        if vector.len() != size || !norm.is_finite() || norm <= 0.0 {
            return Err("modal frame eigenvector is zero or non-finite".into());
        }
        let applied = system.operator.apply(vector)?;
        let residual = stable_l2_norm(applied.iter().zip(vector).map(|(a, v)| a - value * v));
        let scale = stable_l2_norm(applied.iter().copied()).max(value * norm);
        if !residual.is_finite()
            || !scale.is_finite()
            || scale <= 0.0
            || residual > residual_tolerance * scale
        {
            return Err(format!(
                "modal frame mode {index} failed its relative residual check"
            ));
        }
    }
    Ok(pairs)
}

fn dense_eigenpairs(system: &ReducedSparseModalSystem) -> Result<Vec<(f64, Vec<f64>)>, String> {
    ensure_dense_modal_size(system.free_dofs.len(), "modal frame")?;
    jacobi_eigenpairs(system.operator.dense_fallback_matrix()?)
}

pub(crate) fn checked_mode_shape(
    vector: &[f64],
    mass: &[f64],
    free_dofs: &[usize],
    dof_count: usize,
) -> Result<(Vec<f64>, f64), String> {
    let shape = expand_mode_shape(vector, mass, free_dofs, dof_count);
    let norm = stable_l2_norm(shape.iter().copied());
    if shape.iter().any(|value| !value.is_finite()) || !norm.is_finite() || norm <= 0.0 {
        return Err("modal frame recovered a zero or non-finite mode shape".into());
    }
    Ok((shape, norm))
}
