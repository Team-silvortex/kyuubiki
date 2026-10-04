use crate::linear_algebra::cancellable_l2_norm;
use crate::modal_math::{checked_shape_norm, expand_mode_shape, jacobi_eigenpairs};
use crate::modal_sparse::{
    InverseIterationOptions, ReducedSparseModalSystem, inverse_power_iteration,
};
use crate::solver_control::{SolverStage, checkpoint, checkpoint_chunk};

// Bound the complete-spectrum check for single-mode requests; large models remain sparse.
const SINGLE_MODE_DENSE_LIMIT: usize = roundoff::MAX_DOFS;

pub(crate) struct FrameSpectrum {
    pub(crate) pairs: Vec<(f64, Vec<f64>)>,
    pub(crate) residual_tolerance: f64,
}

#[path = "modal_frame_refinement.rs"]
mod refinement;
pub(crate) use refinement::roundoff;

#[cfg(test)]
#[path = "modal_frame_spectrum_control_tests.rs"]
mod control_tests;

pub(crate) fn frame_eigenpairs(
    system: &ReducedSparseModalSystem,
    mode_count: Option<usize>,
) -> Result<FrameSpectrum, String> {
    let size = system.free_dofs.len();
    let options = InverseIterationOptions::default();
    let (mut pairs, residual_tolerance, dense) = if mode_count == Some(1) {
        if let Some(pair) = system
            .operator
            .smallest_tridiagonal_eigenpair(options.tolerance)
        {
            let pair = pair?;
            // The recognized long axial chain has its own bounded cancellation floor.
            (vec![(pair.eigenvalue, pair.vector)], 2.0e-4, false)
        } else if size <= SINGLE_MODE_DENSE_LIMIT {
            (dense_eigenpairs(system)?, 1.0e-8, true)
        } else {
            let inverse = system.prepare_normalized_inverse()?;
            let pair = inverse_power_iteration(
                size,
                options,
                |vector| system.operator.apply(vector),
                |rhs| inverse.solve(rhs),
            )?;
            (
                vec![(pair.eigenvalue, pair.vector)],
                options.tolerance,
                false,
            )
        }
    } else {
        (dense_eigenpairs(system)?, 1.0e-8, true)
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
    if dense {
        refinement::refine_dense_modes(system, &mut pairs, residual_tolerance)?;
    }
    for (index, (value, vector)) in pairs.iter().enumerate() {
        checkpoint(SolverStage::ModalValidation, index)?;
        if vector.len() != size {
            return Err("modal frame eigenvector dimensions are inconsistent".into());
        }
        let norm = cancellable_l2_norm(vector.iter().copied(), SolverStage::ModalSpectrumNorm)?;
        if !norm.is_finite() || norm <= 0.0 {
            return Err("modal frame eigenvector is zero or non-finite".into());
        }
        let applied = if dense {
            system.operator.apply_compensated(vector)?
        } else {
            system.operator.apply(vector)?
        };
        let residual = cancellable_l2_norm(
            applied.iter().zip(vector).map(|(a, v)| a - value * v),
            SolverStage::ModalSpectrumNorm,
        )?;
        let scale = cancellable_l2_norm(applied.iter().copied(), SolverStage::ModalSpectrumNorm)?
            .max(value * norm);
        if !residual.is_finite()
            || !scale.is_finite()
            || scale <= 0.0
            || residual > residual_tolerance * scale
        {
            return Err(format!(
                "modal frame mode {index} failed its relative residual check (relative={:.6e}, tolerance={residual_tolerance:.6e})",
                residual / scale
            ));
        }
    }
    Ok(FrameSpectrum {
        pairs,
        residual_tolerance,
    })
}

fn dense_eigenpairs(system: &ReducedSparseModalSystem) -> Result<Vec<(f64, Vec<f64>)>, String> {
    jacobi_eigenpairs(system.operator.dense_fallback_matrix()?)
}

pub(crate) fn checked_mode_shape(
    vector: &[f64],
    mass: &[f64],
    free_dofs: &[usize],
    dof_count: usize,
) -> Result<(Vec<f64>, f64), String> {
    let shape = expand_mode_shape(vector, mass, free_dofs, dof_count)?;
    let norm = checked_shape_norm(&shape)?;
    Ok((shape, norm))
}

pub(crate) fn checked_published_mode_shape(
    system: &ReducedSparseModalSystem,
    vector: &[f64],
    mass: &[f64],
    dof_count: usize,
    value: f64,
    tolerance: f64,
    allow_roundoff: bool,
) -> Result<(Vec<f64>, f64), String> {
    let (mut shape, _) = checked_mode_shape(vector, mass, &system.free_dofs, dof_count)?;
    let mut reduced = Vec::with_capacity(system.free_dofs.len());
    for (index, &dof) in system.free_dofs.iter().enumerate() {
        reduced.push(shape[dof]);
        checkpoint_chunk(SolverStage::ResultNodes, index + 1, system.free_dofs.len())?;
    }
    system.operator.polish_published_shape_with_recovery(
        value,
        &mut reduced,
        &system.mass,
        tolerance,
        allow_roundoff,
    )?;
    for (index, (&dof, value)) in system.free_dofs.iter().zip(reduced).enumerate() {
        shape[dof] = value;
        checkpoint_chunk(SolverStage::ResultNodes, index + 1, system.free_dofs.len())?;
    }
    let norm = checked_shape_norm(&shape)?;
    if (norm - 1.0).abs() > 1e-10 {
        return Err("published modal shape lost its unit participation norm".into());
    }
    Ok((shape, norm))
}
