use super::{SparseMatrix, product::visit_row_chunks};
use crate::linear_binary_scale::split_finite;
use crate::linear_substitution::{SubnormalResult, solve_scaled_rhs_row_with_policy};
use crate::solver_control::{SolverStage, checkpoint, checkpoint_chunk};

type ReducedSystem = (SparseMatrix, Vec<f64>, Vec<usize>);

#[cfg(test)]
#[path = "linear_sparse_reduction_tests.rs"]
mod tests;

pub(crate) fn reduce_sparse_system(
    matrix: &SparseMatrix,
    force: &[f64],
    constrained: &[usize],
) -> Result<ReducedSystem, String> {
    checkpoint(SolverStage::ConstraintIndex, 0)?;
    validate_inputs(matrix, force)?;
    let size = force.len();
    let mut is_constrained = vec![false; size];
    for (index, &dof) in constrained.iter().enumerate() {
        if dof >= size {
            return Err("sparse reduction received an out-of-range constraint".into());
        }
        is_constrained[dof] = true;
        checkpoint_chunk(SolverStage::ConstraintIndex, index + 1, constrained.len())?;
    }
    let (free, free_map) = free_map(size, |index| !is_constrained[index])?;
    checkpoint(SolverStage::ConstraintReduce, 0)?;
    let mut reduced =
        SparseMatrix::with_uniform_row_capacity(free.len(), matrix.average_row_non_zero_hint());
    let mut reduced_force = vec![0.0; free.len()];
    for (reduced_row, &global_row) in free.iter().enumerate() {
        reduced_force[reduced_row] = force[global_row];
        visit_row_chunks(
            &matrix.rows[global_row],
            SolverStage::ConstraintReduceRow,
            |_, chunk| {
                for &(global_col, value) in chunk {
                    let reduced_col = free_map[global_col];
                    if reduced_col != usize::MAX {
                        reduced.push_sorted_entry(reduced_row, reduced_col, value);
                    }
                }
            },
        )?;
        checkpoint_chunk(SolverStage::ConstraintReduce, reduced_row + 1, free.len())?;
    }
    Ok((reduced, reduced_force, free))
}

pub(crate) fn reduce_sparse_system_with_prescribed(
    matrix: &SparseMatrix,
    force: &[f64],
    prescribed: &[(usize, f64)],
) -> Result<ReducedSystem, String> {
    checkpoint(SolverStage::ConstraintIndex, 0)?;
    validate_inputs(matrix, force)?;
    let size = force.len();
    let prescribed_values = index_prescribed(size, prescribed)?;
    let (free, free_map) = free_map(size, |index| prescribed_values[index].is_none())?;
    checkpoint(SolverStage::ConstraintReduce, 0)?;
    let mut reduced =
        SparseMatrix::with_uniform_row_capacity(free.len(), matrix.average_row_non_zero_hint());
    let mut reduced_force = vec![0.0; free.len()];
    for (reduced_row, &global_row) in free.iter().enumerate() {
        let mut rhs = force[global_row];
        let mut needs_range = false;
        let mut prescribed_count = 0;
        visit_row_chunks(
            &matrix.rows[global_row],
            SolverStage::ConstraintReduceRow,
            |_, chunk| {
                for &(global_col, value) in chunk {
                    if let Some(prescribed_value) = prescribed_values[global_col] {
                        prescribed_count += 1;
                        let product = value * prescribed_value;
                        rhs -= product;
                        needs_range |= (value != 0.0 && prescribed_value != 0.0 && !product.is_normal())
                                || !rhs.is_finite()
                                || rhs.is_subnormal()
                                // Replay from the original row: zero can hide an earlier rounded tail.
                                || (rhs == 0.0 && product != 0.0);
                    } else {
                        let reduced_col = free_map[global_col];
                        if reduced_col != usize::MAX {
                            reduced.push_sorted_entry(reduced_row, reduced_col, value);
                        }
                    }
                }
            },
        )?;
        if needs_range {
            rhs = recover_rhs(
                &matrix.rows[global_row],
                &prescribed_values,
                prescribed_count,
                force[global_row],
            )
            .map_err(|error| format!("sparse boundary row {global_row}: {error}"))?;
        }
        reduced_force[reduced_row] = rhs;
        checkpoint_chunk(SolverStage::ConstraintReduce, reduced_row + 1, free.len())?;
    }
    Ok((reduced, reduced_force, free))
}

pub(super) fn index_prescribed(
    size: usize,
    prescribed: &[(usize, f64)],
) -> Result<Vec<Option<f64>>, String> {
    let mut prescribed_values = vec![None; size];
    for (index, &(dof, value)) in prescribed.iter().enumerate() {
        if dof >= size {
            return Err("sparse reduction received an out-of-range prescribed value".into());
        }
        if !value.is_finite() {
            return Err("sparse reduction received a non-finite prescribed value".into());
        }
        if prescribed_values[dof].is_some_and(|previous| previous != value) {
            return Err("sparse reduction received conflicting prescribed values".into());
        }
        prescribed_values[dof] = Some(value);
        checkpoint_chunk(SolverStage::ConstraintIndex, index + 1, prescribed.len())?;
    }
    Ok(prescribed_values)
}

fn recover_rhs(
    entries: &[(usize, f64)],
    prescribed: &[Option<f64>],
    count: usize,
    rhs: f64,
) -> Result<f64, String> {
    let mut operands = Vec::with_capacity(count);
    visit_row_chunks(entries, SolverStage::ConstraintReduceRow, |_, chunk| {
        for &(column, coefficient) in chunk {
            if let Some(value) = prescribed[column] {
                operands.push((coefficient, value));
            }
        }
    })?;
    // Preserve the free matrix and its symmetry; the reduced load must itself fit.
    solve_scaled_rhs_row_with_policy(
        split_finite(rhs),
        operands.into_iter(),
        1.0,
        SolverStage::ConstraintReduceRow,
        SubnormalResult::RequireExactRange,
    )
}

fn validate_inputs(matrix: &SparseMatrix, rhs: &[f64]) -> Result<(), String> {
    if matrix.size() != rhs.len() {
        return Err("sparse reduction matrix dimensions must match vector".into());
    }
    checkpoint(SolverStage::SparseValidateRhs, 0)?;
    for (index, chunk) in rhs.chunks(1024).enumerate() {
        if chunk.iter().any(|value| !value.is_finite()) {
            return Err("sparse reduction vector contains a non-finite value".into());
        }
        checkpoint(SolverStage::SparseValidateRhs, index * 1024 + chunk.len())?;
    }
    validate_matrix(matrix)
}

pub(super) fn validate_matrix(matrix: &SparseMatrix) -> Result<(), String> {
    checkpoint(SolverStage::SparseValidateMatrix, 0)?;
    for (row, entries) in matrix.rows.iter().enumerate() {
        let mut previous = None;
        let mut error = None;
        visit_row_chunks(entries, SolverStage::SparseValidateMatrixRow, |_, chunk| {
            if error.is_some() {
                return;
            }
            for &(column, value) in chunk {
                if column >= matrix.size() || previous.is_some_and(|last| last >= column) {
                    error = Some("sparse reduction requires sorted unique in-range columns");
                    break;
                }
                if !value.is_finite() {
                    error = Some("sparse reduction matrix contains a non-finite value");
                    break;
                }
                previous = Some(column);
            }
        })?;
        if let Some(error) = error {
            return Err(error.into());
        }
        checkpoint_chunk(SolverStage::SparseValidateMatrix, row + 1, matrix.size())?;
    }
    Ok(())
}

fn free_map(
    size: usize,
    is_free: impl Fn(usize) -> bool,
) -> Result<(Vec<usize>, Vec<usize>), String> {
    checkpoint(SolverStage::ConstraintMap, 0)?;
    let mut free = Vec::new();
    let mut map = vec![usize::MAX; size];
    for (global, reduced) in map.iter_mut().enumerate() {
        if is_free(global) {
            *reduced = free.len();
            free.push(global);
        }
        checkpoint_chunk(SolverStage::ConstraintMap, global + 1, size)?;
    }
    Ok((free, map))
}
