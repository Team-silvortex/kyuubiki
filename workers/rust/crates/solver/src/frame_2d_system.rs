use crate::frame_2d_element::{FrameElement, finite_fields};
use crate::linear_algebra::{
    SparseMatrix, add_at, reduce_sparse_system, solve_spd_system_profile_with_options,
};
use crate::linear_solver_profile::SpdSolveOptions;
use crate::solver_control::{SolverStage, checkpoint, checkpoint_chunk};
use crate::solver_postprocess::restore_solution;

pub(super) fn solve_system<'a>(
    nodes: impl ExactSizeIterator<Item = ([f64; 3], [bool; 3])>,
    elements: impl ExactSizeIterator<Item = Result<FrameElement<'a>, String>>,
    options: SpdSolveOptions,
) -> Result<Vec<f64>, String> {
    checkpoint(SolverStage::LinearPrepare, 0)?;
    let node_count = nodes.len();
    let dof_count = node_count * 3;
    let mut stiffness = SparseMatrix::new(dof_count);
    let mut forces = vec![0.0; dof_count];
    let mut constrained = Vec::new();
    for (index, (loads, supports)) in nodes.enumerate() {
        for dof in 0..3 {
            forces[index * 3 + dof] = loads[dof];
            if supports[dof] {
                constrained.push(index * 3 + dof);
            }
        }
        checkpoint_chunk(SolverStage::LinearPrepare, index + 1, node_count)?;
    }
    checkpoint(SolverStage::ElementAssembly, 0)?;
    let count = elements.len();
    for (index, element) in elements.enumerate() {
        let element = element?;
        let (matrix, load) = element.global_system()?;
        for (row, &global_row) in element.dofs.iter().enumerate() {
            forces[global_row] += load[row];
            finite_fields(element.id, "assembled load", &[forces[global_row]])?;
            for (column, &global_column) in element.dofs.iter().enumerate() {
                add_at(
                    &mut stiffness,
                    global_row,
                    global_column,
                    matrix[row][column],
                );
            }
        }
        checkpoint_chunk(SolverStage::ElementAssembly, index + 1, count)?;
    }
    // Validate constrained rows as well; reduction must not erase invalid assembly.
    checkpoint(SolverStage::SparseValidateMatrix, 0)?;
    for row in 0..dof_count {
        if stiffness
            .row_entries(row)
            .iter()
            .any(|(_, value)| !value.is_finite())
        {
            return Err("2d frame assembled stiffness is not representable".into());
        }
        checkpoint_chunk(SolverStage::SparseValidateMatrix, row + 1, dof_count)?;
    }
    let (reduced, force, free) = reduce_sparse_system(&stiffness, &forces, &constrained)?;
    let solution = solve_spd_system_profile_with_options(&reduced, &force, options)?.solution;
    restore_solution(dof_count, &[], &free, &solution)
}
