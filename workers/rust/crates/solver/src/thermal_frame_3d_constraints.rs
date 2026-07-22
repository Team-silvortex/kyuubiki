use crate::linear_algebra::{SparseMatrix, add_at};
use crate::linear_dense::solve_linear_system;
use kyuubiki_protocol::{
    SolveThermalFrame3dRequest, ThermalFrame3dDirectionalConstraintResult,
    ThermalFrame3dDirectionalRotationalConstraintResult,
};

const DIRECTION_TOLERANCE: f64 = 1.0e-10;

pub(crate) struct ThermalFrame3dConstraintSystem {
    physical_to_reduced: Vec<Vec<(usize, f64)>>,
    block_constraints: Vec<Vec<[f64; 3]>>,
    translational_slots: Vec<(usize, usize)>,
    rotational_slots: Vec<(usize, usize)>,
    reduced_size: usize,
}

impl ThermalFrame3dConstraintSystem {
    pub(crate) fn build(request: &SolveThermalFrame3dRequest) -> Result<Self, String> {
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
        }

        let mut translational_slots = Vec::with_capacity(request.directional_constraints.len());
        for constraint in &request.directional_constraints {
            let block = constraint.node * 2;
            let slot = block_constraints[block].len();
            block_constraints[block].push(normalize(constraint.direction));
            translational_slots.push((block, slot));
        }
        let mut rotational_slots =
            Vec::with_capacity(request.directional_rotational_constraints.len());
        for constraint in &request.directional_rotational_constraints {
            let block = constraint.node * 2 + 1;
            let slot = block_constraints[block].len();
            block_constraints[block].push(normalize(constraint.direction));
            rotational_slots.push((block, slot));
        }

        let mut physical_to_reduced = vec![Vec::new(); request.nodes.len() * 6];
        let mut reduced_size = 0;
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
        }

        Ok(Self {
            physical_to_reduced,
            block_constraints,
            translational_slots,
            rotational_slots,
            reduced_size,
        })
    }

    pub(crate) fn project(&self, matrix: &SparseMatrix, force: &[f64]) -> (SparseMatrix, Vec<f64>) {
        let mut reduced = SparseMatrix::new(self.reduced_size);
        let mut reduced_force = vec![0.0; self.reduced_size];
        for physical_row in 0..force.len() {
            for &(reduced_row, row_weight) in &self.physical_to_reduced[physical_row] {
                reduced_force[reduced_row] += row_weight * force[physical_row];
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
        }
        (reduced, reduced_force)
    }

    pub(crate) fn restore(&self, reduced: &[f64]) -> Vec<f64> {
        self.physical_to_reduced
            .iter()
            .map(|terms| {
                terms
                    .iter()
                    .map(|(index, weight)| weight * reduced[*index])
                    .sum()
            })
            .collect()
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
        let residual = residual_vector(matrix, force, displacement);
        let block_reactions = self
            .block_constraints
            .iter()
            .enumerate()
            .map(|(block, constraints)| {
                reaction_coefficients(constraints, block_vector(&residual, block))
            })
            .collect::<Result<Vec<_>, _>>()?;

        let translational = request
            .directional_constraints
            .iter()
            .zip(&self.translational_slots)
            .enumerate()
            .map(|(index, (constraint, &(block, slot)))| {
                let direction = normalize(constraint.direction);
                ThermalFrame3dDirectionalConstraintResult {
                    index,
                    id: constraint.id.clone(),
                    node: constraint.node,
                    direction,
                    displacement: dot(direction, block_vector(displacement, block)),
                    reaction_force: block_reactions[block][slot],
                }
            })
            .collect();
        let rotational = request
            .directional_rotational_constraints
            .iter()
            .zip(&self.rotational_slots)
            .enumerate()
            .map(|(index, (constraint, &(block, slot)))| {
                let direction = normalize(constraint.direction);
                ThermalFrame3dDirectionalRotationalConstraintResult {
                    index,
                    id: constraint.id.clone(),
                    node: constraint.node,
                    direction,
                    rotation: dot(direction, block_vector(displacement, block)),
                    reaction_moment: block_reactions[block][slot],
                }
            })
            .collect();
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

fn free_basis(constraints: &[[f64; 3]]) -> Result<Vec<[f64; 3]>, String> {
    let mut constrained_basis = Vec::with_capacity(constraints.len());
    for &direction in constraints {
        let candidate = orthogonalize(direction, &constrained_basis);
        let norm = dot(candidate, candidate).sqrt();
        if norm <= DIRECTION_TOLERANCE {
            return Err("contains linearly dependent directions".to_string());
        }
        constrained_basis.push(scale(candidate, norm.recip()));
    }

    let mut full_basis = constrained_basis.clone();
    let mut free = Vec::with_capacity(3 - constrained_basis.len());
    for axis in 0..3 {
        let candidate = orthogonalize(
            std::array::from_fn(|index| (index == axis) as u8 as f64),
            &full_basis,
        );
        let norm = dot(candidate, candidate).sqrt();
        if norm > DIRECTION_TOLERANCE {
            let unit = scale(candidate, norm.recip());
            full_basis.push(unit);
            free.push(unit);
        }
    }
    Ok(free)
}

fn orthogonalize(mut vector: [f64; 3], basis: &[[f64; 3]]) -> [f64; 3] {
    for &axis in basis {
        let projection = dot(vector, axis);
        for index in 0..3 {
            vector[index] -= projection * axis[index];
        }
    }
    vector
}

fn reaction_coefficients(constraints: &[[f64; 3]], residual: [f64; 3]) -> Result<Vec<f64>, String> {
    if constraints.is_empty() {
        return Ok(Vec::new());
    }
    let gram = constraints
        .iter()
        .map(|&left| constraints.iter().map(|&right| dot(left, right)).collect())
        .collect();
    let projected = constraints
        .iter()
        .map(|&direction| dot(direction, residual))
        .collect();
    solve_linear_system(gram, projected)
        .map_err(|error| format!("could not recover exact constraint reactions: {error}"))
}

fn residual_vector(matrix: &SparseMatrix, force: &[f64], displacement: &[f64]) -> Vec<f64> {
    (0..force.len())
        .map(|row| {
            matrix
                .row_entries(row)
                .iter()
                .map(|(column, value)| value * displacement[*column])
                .sum::<f64>()
                - force[row]
        })
        .collect()
}

fn block_vector(vector: &[f64], block: usize) -> [f64; 3] {
    let offset = (block / 2) * 6 + (block % 2) * 3;
    [vector[offset], vector[offset + 1], vector[offset + 2]]
}

fn normalize(direction: [f64; 3]) -> [f64; 3] {
    scale(direction, dot(direction, direction).sqrt().recip())
}

fn dot(left: [f64; 3], right: [f64; 3]) -> f64 {
    left.into_iter().zip(right).map(|(a, b)| a * b).sum()
}

fn scale(vector: [f64; 3], scalar: f64) -> [f64; 3] {
    vector.map(|value| value * scalar)
}
