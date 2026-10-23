use super::{ThermalFrame3dConstraintSystem, block_vector, free_basis};
use crate::linear_algebra::{
    SPARSE_RESIDUAL_TOLERANCE, SparseMatrix, prescribed_row_backward_error,
};
use crate::linear_binary_scale::{restore, split_finite};
use crate::solver_control::{SolverStage, checkpoint, checkpoint_chunk};

#[cfg(test)]
#[path = "thermal_frame_3d_constraint_validation_tests.rs"]
mod tests;

impl ThermalFrame3dConstraintSystem {
    pub(crate) fn validate_original(
        &self,
        matrix: &SparseMatrix,
        force: &[f64],
        displacement: &[f64],
    ) -> Result<(), String> {
        checkpoint(SolverStage::ConstraintValidate, 0)?;
        let size = self.physical_to_reduced.len();
        if matrix.size() != size || force.len() != size || displacement.len() != size {
            return Err(
                "original projected-system dimensions must match the complete system".into(),
            );
        }
        for row in 0..size {
            if !force[row].is_finite() || !displacement[row].is_finite() {
                return Err(format!(
                    "original projected-system non-finite value at DOF {row}"
                ));
            }
            if self.physical_to_reduced[row].is_empty() && displacement[row] != 0.0 {
                return Err(format!(
                    "original projected-system fixed value changed at DOF {row}"
                ));
            }
            let mut previous = None;
            for (index, &(column, value)) in matrix.row_entries(row).iter().enumerate() {
                if column >= size || !value.is_finite() || previous.is_some_and(|p| p >= column) {
                    return Err(format!(
                        "original projected-system invalid matrix row {row}"
                    ));
                }
                previous = Some(column);
                row_checkpoint(index + 1)?;
            }
            checkpoint_chunk(SolverStage::ConstraintValidate, row + 1, size)?;
        }
        for (block, constraints) in self.block_constraints.iter().enumerate() {
            let value = block_vector(displacement, block);
            for &direction in constraints {
                validate_direction(direction, value)
                    .map_err(|error| format!("original projected-system block {block}: {error}"))?;
            }
            checkpoint_chunk(
                SolverStage::ConstraintValidate,
                size + block * 3 + 3,
                2 * size,
            )?;
        }

        let mut entries = Vec::new();
        for (block, constraints) in self.block_constraints.iter().enumerate() {
            let offset = (block / 2) * 6 + (block % 2) * 3;
            // Rebuild admissible directions instead of trusting the solve map.
            for direction in free_basis(constraints)? {
                let expected = projected_row(matrix, force, offset, direction, &mut entries)?;
                let error =
                    prescribed_row_backward_error(&entries, expected, displacement, |dof| {
                        !self.physical_to_reduced[dof].is_empty()
                    })?;
                if !error.is_finite() || error > SPARSE_RESIDUAL_TOLERANCE {
                    return Err(format!(
                        "original projected-system block {block} failed free equilibrium validation ({error:.6e})"
                    ));
                }
            }
            checkpoint_chunk(
                SolverStage::ConstraintValidate,
                2 * size + block * 3 + 3,
                3 * size,
            )?;
        }
        Ok(())
    }
}

fn validate_direction(direction: [f64; 3], value: [f64; 3]) -> Result<(), String> {
    if direction.iter().filter(|&&weight| weight != 0.0).count() == 1 {
        if direction
            .into_iter()
            .zip(value)
            .any(|(w, v)| w != 0.0 && v != 0.0)
        {
            return Err("axis constraint value changed".into());
        }
        return Ok(());
    }
    let scale = direction
        .into_iter()
        .zip(value)
        .filter(|(weight, _)| *weight != 0.0)
        .map(|(_, value)| value.abs())
        .fold(0.0, f64::max);
    if scale == 0.0 {
        return Ok(());
    }
    let mut dot = Sum::default();
    let mut magnitude = 0.0;
    for (weight, value) in direction.into_iter().zip(value) {
        if weight == 0.0 {
            continue;
        }
        let normalized = value / scale;
        if value != 0.0 && normalized == 0.0 {
            return Err("constraint identity normalization would lose numerical range".into());
        }
        let term = checked_product(weight, normalized)?;
        dot.add(term);
        dot.add(weight.mul_add(normalized, -term));
        magnitude += term.abs();
    }
    if dot.value().abs() > 1.0e-10 * magnitude {
        return Err("directional constraint value changed".into());
    }
    Ok(())
}

fn projected_row(
    matrix: &SparseMatrix,
    force: &[f64],
    offset: usize,
    direction: [f64; 3],
    entries: &mut Vec<(usize, f64)>,
) -> Result<f64, String> {
    checkpoint(SolverStage::ConstraintValidateRow, 0)?;
    entries.clear();
    let rows = std::array::from_fn::<_, 3, _>(|axis| {
        if direction[axis] == 0.0 {
            &[][..]
        } else {
            matrix.row_entries(offset + axis)
        }
    });
    // Coordinate supports retain the shared original-row arithmetic and range recovery.
    if let Some(axis) = direction.iter().position(|&weight| weight == 1.0)
        && direction.iter().filter(|&&weight| weight != 0.0).count() == 1
    {
        for chunk in rows[axis].chunks(1024) {
            entries.extend_from_slice(chunk);
            checkpoint(SolverStage::ConstraintValidateRow, entries.len())?;
        }
        return Ok(force[offset + axis]);
    }

    let mut maximum = 0.0_f64;
    for axis in 0..3 {
        if direction[axis] != 0.0 {
            maximum = maximum.max(force[offset + axis].abs());
        }
        for (index, &(_, value)) in rows[axis].iter().enumerate() {
            maximum = maximum.max(value.abs());
            row_checkpoint(index + 1)?;
        }
    }
    let (_, exponent) = split_finite(maximum);
    let mut rhs = Sum::default();
    for (axis, weight) in direction.into_iter().enumerate() {
        if weight != 0.0 {
            add_weighted(&mut rhs, weight, force[offset + axis], exponent)?;
        }
    }
    let mut positions = [0; 3];
    let mut visited = 0;
    while let Some(column) = (0..3)
        .filter_map(|axis| rows[axis].get(positions[axis]).map(|entry| entry.0))
        .min()
    {
        let mut coefficient = Sum::default();
        for axis in 0..3 {
            if let Some(&(next, value)) = rows[axis].get(positions[axis])
                && next == column
            {
                add_weighted(&mut coefficient, direction[axis], value, exponent)?;
                positions[axis] += 1;
                visited += 1;
                row_checkpoint(visited)?;
            }
        }
        let coefficient = coefficient.value();
        if coefficient != 0.0 {
            entries.push((column, coefficient));
        }
    }
    checkpoint(SolverStage::ConstraintValidateRow, visited)?;
    Ok(rhs.value())
}

fn add_weighted(sum: &mut Sum, weight: f64, value: f64, exponent: i32) -> Result<(), String> {
    let normalized = restore(value, -exponent);
    if !normalized.is_finite() || restore(normalized, exponent) != value {
        return Err("original projected equation normalization would lose numerical range".into());
    }
    let high = checked_product(weight, normalized)?;
    sum.add(high);
    sum.add(weight.mul_add(normalized, -high));
    Ok(())
}

fn checked_product(left: f64, right: f64) -> Result<f64, String> {
    let value = left * right;
    if !value.is_finite() || (left != 0.0 && right != 0.0 && value.abs() < f64::MIN_POSITIVE) {
        return Err("original projected equation product would lose numerical range".into());
    }
    Ok(value)
}

fn row_checkpoint(completed: usize) -> Result<(), String> {
    if completed % 1024 == 0 {
        checkpoint(SolverStage::ConstraintValidateRow, completed)?;
    }
    Ok(())
}

#[derive(Default)]
struct Sum {
    value: f64,
    correction: f64,
}

impl Sum {
    fn add(&mut self, value: f64) {
        let next = self.value + value;
        self.correction += if self.value.abs() >= value.abs() {
            (self.value - next) + value
        } else {
            (value - next) + self.value
        };
        self.value = next;
    }

    fn value(self) -> f64 {
        self.value + self.correction
    }
}
