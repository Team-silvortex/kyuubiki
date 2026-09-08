use super::SparseMatrix;
use crate::solver_control::{SolverStage, checkpoint, checkpoint_chunk};

type ReducedSystem = (SparseMatrix, Vec<f64>, Vec<usize>);

pub(crate) fn reduce_sparse_system(
    matrix: &SparseMatrix,
    force: &[f64],
    constrained: &[usize],
) -> Result<ReducedSystem, String> {
    checkpoint(SolverStage::ConstraintIndex, 0)?;
    let size = force.len();
    let mut is_constrained = vec![false; size];
    for (index, &dof) in constrained.iter().enumerate() {
        if dof < size {
            is_constrained[dof] = true;
        }
        checkpoint_chunk(SolverStage::ConstraintIndex, index + 1, constrained.len())?;
    }
    let (free, free_map) = free_map(size, |index| !is_constrained[index])?;
    checkpoint(SolverStage::ConstraintReduce, 0)?;
    let mut reduced =
        SparseMatrix::with_uniform_row_capacity(free.len(), matrix.average_row_non_zero_hint());
    let mut reduced_force = vec![0.0; free.len()];
    for (reduced_row, &global_row) in free.iter().enumerate() {
        reduced_force[reduced_row] = force[global_row];
        for &(global_col, value) in &matrix.rows[global_row] {
            let reduced_col = free_map[global_col];
            if reduced_col != usize::MAX {
                reduced.push_sorted_entry(reduced_row, reduced_col, value);
            }
        }
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
    let size = force.len();
    let mut prescribed_values = vec![None; size];
    for (index, &(dof, value)) in prescribed.iter().enumerate() {
        if dof < size {
            prescribed_values[dof] = Some(value);
        }
        checkpoint_chunk(SolverStage::ConstraintIndex, index + 1, prescribed.len())?;
    }
    let (free, free_map) = free_map(size, |index| prescribed_values[index].is_none())?;
    checkpoint(SolverStage::ConstraintReduce, 0)?;
    let mut reduced =
        SparseMatrix::with_uniform_row_capacity(free.len(), matrix.average_row_non_zero_hint());
    let mut reduced_force = vec![0.0; free.len()];
    for (reduced_row, &global_row) in free.iter().enumerate() {
        let mut rhs = force[global_row];
        for &(global_col, value) in &matrix.rows[global_row] {
            if let Some(prescribed_value) = prescribed_values[global_col] {
                rhs -= value * prescribed_value;
            } else {
                let reduced_col = free_map[global_col];
                if reduced_col != usize::MAX {
                    reduced.push_sorted_entry(reduced_row, reduced_col, value);
                }
            }
        }
        reduced_force[reduced_row] = rhs;
        checkpoint_chunk(SolverStage::ConstraintReduce, reduced_row + 1, free.len())?;
    }
    Ok((reduced, reduced_force, free))
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
