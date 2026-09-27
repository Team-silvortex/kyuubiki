use crate::frame_3d_element::finite_fields;
use crate::frame_3d_math::normalized_direction as normalize;
use crate::linear_algebra::{SparseMatrix, add_at};
use crate::solver_control::{SolverStage, checkpoint, checkpoint_chunk};
use crate::solver_postprocess::try_collect_results;
use crate::thermal_frame_3d_constraint_basis::{dot, free_basis, reaction_coefficients};
use kyuubiki_protocol::{
    SolveThermalFrame3dRequest, ThermalFrame3dDirectionalConstraintResult,
    ThermalFrame3dDirectionalRotationalConstraintResult,
};

pub(crate) struct ThermalFrame3dConstraintSystem {
    physical_to_reduced: Vec<Vec<(usize, f64)>>,
    block_constraints: Vec<Vec<[f64; 3]>>,
    translational_slots: Vec<(usize, usize)>,
    rotational_slots: Vec<(usize, usize)>,
    reduced_size: usize,
}

impl ThermalFrame3dConstraintSystem {
    pub(crate) fn build(request: &SolveThermalFrame3dRequest) -> Result<Self, String> {
        checkpoint(SolverStage::ConstraintIndex, 0)?;
        let mut block_constraints = vec![Vec::new(); request.nodes.len() * 2];
        for (node_index, node) in request.nodes.iter().enumerate() {
            push_fixed_axes(
                &mut block_constraints[node_index * 2],
                [node.fix_x, node.fix_y, node.fix_z],
            );
            push_fixed_axes(
                &mut block_constraints[node_index * 2 + 1],
                [node.fix_rx, node.fix_ry, node.fix_rz],
            );
            checkpoint_chunk(
                SolverStage::ConstraintIndex,
                node_index + 1,
                request.nodes.len(),
            )?;
        }

        let mut translational_slots = Vec::with_capacity(request.directional_constraints.len());
        for (index, constraint) in request.directional_constraints.iter().enumerate() {
            let block = constraint.node * 2;
            let slot = block_constraints[block].len();
            block_constraints[block].push(normalize(constraint.direction)?);
            translational_slots.push((block, slot));
            checkpoint_chunk(
                SolverStage::ConstraintIndex,
                index + 1,
                request.directional_constraints.len(),
            )?;
        }
        let mut rotational_slots =
            Vec::with_capacity(request.directional_rotational_constraints.len());
        for (index, constraint) in request
            .directional_rotational_constraints
            .iter()
            .enumerate()
        {
            let block = constraint.node * 2 + 1;
            let slot = block_constraints[block].len();
            block_constraints[block].push(normalize(constraint.direction)?);
            rotational_slots.push((block, slot));
            checkpoint_chunk(
                SolverStage::ConstraintIndex,
                index + 1,
                request.directional_rotational_constraints.len(),
            )?;
        }

        let mut physical_to_reduced = vec![Vec::new(); request.nodes.len() * 6];
        let mut reduced_size = 0;
        checkpoint(SolverStage::ConstraintMap, 0)?;
        for (block, constraints) in block_constraints.iter().enumerate() {
            let free_basis = free_basis(constraints)
                .map_err(|error| format!("thermal 3d frame constraint block {block} {error}"))?;
            let physical_offset = (block / 2) * 6 + (block % 2) * 3;
            for basis in free_basis {
                for component in 0..3 {
                    if basis[component].abs() > 1.0e-15 {
                        physical_to_reduced[physical_offset + component]
                            .push((reduced_size, basis[component]));
                    }
                }
                reduced_size += 1;
            }
            checkpoint_chunk(
                SolverStage::ConstraintMap,
                block + 1,
                block_constraints.len(),
            )?;
        }

        Ok(Self {
            physical_to_reduced,
            block_constraints,
            translational_slots,
            rotational_slots,
            reduced_size,
        })
    }

    pub(crate) fn project(
        &self,
        matrix: &SparseMatrix,
        force: &[f64],
    ) -> Result<(SparseMatrix, Vec<f64>), String> {
        checkpoint(SolverStage::ConstraintReduce, 0)?;
        let mut reduced = SparseMatrix::new(self.reduced_size);
        let mut reduced_force = vec![0.0; self.reduced_size];
        for (physical_row, &physical_force) in force.iter().enumerate() {
            for &(reduced_row, row_weight) in &self.physical_to_reduced[physical_row] {
                reduced_force[reduced_row] += row_weight * physical_force;
                for &(physical_column, stiffness) in matrix.row_entries(physical_row) {
                    for &(reduced_column, column_weight) in
                        &self.physical_to_reduced[physical_column]
                    {
                        add_at(
                            &mut reduced,
                            reduced_row,
                            reduced_column,
                            row_weight * stiffness * column_weight,
                        );
                    }
                }
            }
            checkpoint_chunk(SolverStage::ConstraintReduce, physical_row + 1, force.len())?;
        }
        finite_fields("constraint projection", "load", &reduced_force)?;
        Ok((reduced, reduced_force))
    }

    pub(crate) fn restore(&self, reduced: &[f64]) -> Result<Vec<f64>, String> {
        try_collect_results(
            SolverStage::ResultFreeDofs,
            self.physical_to_reduced.iter().map(|terms| {
                let value = terms
                    .iter()
                    .map(|(index, weight)| weight * reduced[*index])
                    .sum();
                finite_fields("constraint restore", "displacement", &[value])?;
                Ok(value)
            }),
        )
    }

    pub(crate) fn build_results(
        &self,
        request: &SolveThermalFrame3dRequest,
        matrix: &SparseMatrix,
        force: &[f64],
        displacement: &[f64],
    ) -> Result<
        (
            Vec<ThermalFrame3dDirectionalConstraintResult>,
            Vec<ThermalFrame3dDirectionalRotationalConstraintResult>,
        ),
        String,
    > {
        let residual = residual_vector(matrix, force, displacement)?;
        let block_reactions = try_collect_results(
            SolverStage::ResultElements,
            self.block_constraints
                .iter()
                .enumerate()
                .map(|(block, constraints)| {
                    reaction_coefficients(constraints, block_vector(&residual, block))
                }),
        )?;

        let translational = try_collect_results(
            SolverStage::ResultElements,
            request
                .directional_constraints
                .iter()
                .zip(&self.translational_slots)
                .enumerate()
                .map(|(index, (constraint, &(block, slot)))| {
                    let direction = normalize(constraint.direction)?;
                    let displacement = dot(direction, block_vector(displacement, block));
                    let reaction_force = block_reactions[block][slot];
                    finite_fields(
                        &constraint.id,
                        "constraint response",
                        &[displacement, reaction_force],
                    )?;
                    Ok(ThermalFrame3dDirectionalConstraintResult {
                        index,
                        id: constraint.id.clone(),
                        node: constraint.node,
                        direction,
                        displacement,
                        reaction_force,
                    })
                }),
        )?;
        let rotational = try_collect_results(
            SolverStage::ResultElements,
            request
                .directional_rotational_constraints
                .iter()
                .zip(&self.rotational_slots)
                .enumerate()
                .map(|(index, (constraint, &(block, slot)))| {
                    let direction = normalize(constraint.direction)?;
                    let rotation = dot(direction, block_vector(displacement, block));
                    let reaction_moment = block_reactions[block][slot];
                    finite_fields(
                        &constraint.id,
                        "constraint response",
                        &[rotation, reaction_moment],
                    )?;
                    Ok(ThermalFrame3dDirectionalRotationalConstraintResult {
                        index,
                        id: constraint.id.clone(),
                        node: constraint.node,
                        direction,
                        rotation,
                        reaction_moment,
                    })
                }),
        )?;
        Ok((translational, rotational))
    }
}

fn push_fixed_axes(constraints: &mut Vec<[f64; 3]>, fixed: [bool; 3]) {
    for (axis, is_fixed) in fixed.into_iter().enumerate() {
        if is_fixed {
            constraints.push(std::array::from_fn(|index| (index == axis) as u8 as f64));
        }
    }
}

fn residual_vector(
    matrix: &SparseMatrix,
    force: &[f64],
    displacement: &[f64],
) -> Result<Vec<f64>, String> {
    try_collect_results(
        SolverStage::ResidualValidate,
        (0..force.len()).map(|row| {
            let value = matrix
                .row_entries(row)
                .iter()
                .map(|(column, value)| value * displacement[*column])
                .sum::<f64>()
                - force[row];
            finite_fields("constraint reaction", "residual", &[value])?;
            Ok(value)
        }),
    )
}

fn block_vector(vector: &[f64], block: usize) -> [f64; 3] {
    let offset = (block / 2) * 6 + (block % 2) * 3;
    [vector[offset], vector[offset + 1], vector[offset + 2]]
}
