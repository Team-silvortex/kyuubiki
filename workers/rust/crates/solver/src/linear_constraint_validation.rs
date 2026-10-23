use super::{
    SPARSE_RESIDUAL_TOLERANCE, SparseMatrix, product::visit_row_chunks, reduction, residual,
};
use crate::solver_control::{SolverStage, checkpoint, checkpoint_chunk};

#[cfg(test)]
#[path = "linear_constraint_validation_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "linear_homogeneous_validation_tests.rs"]
mod homogeneous_tests;

/// Borrows the pre-elimination matrix; the fixed-value index is reused across RHS updates.
pub(crate) struct PrescribedSystemValidator<'a> {
    matrix: &'a SparseMatrix,
    prescribed: Vec<Option<f64>>,
}

impl<'a> PrescribedSystemValidator<'a> {
    pub(crate) fn new_homogeneous(
        matrix: &'a SparseMatrix,
        constrained: &[usize],
    ) -> Result<Self, String> {
        checkpoint(SolverStage::ConstraintIndex, 0)?;
        reduction::validate_matrix(matrix)?;
        let mut prescribed = vec![None; matrix.size()];
        for (index, &dof) in constrained.iter().enumerate() {
            let fixed = prescribed.get_mut(dof).ok_or_else(|| {
                format!(
                    "original constrained-system received out-of-range zero constraint at DOF {dof}"
                )
            })?;
            *fixed = Some(0.0);
            checkpoint_chunk(SolverStage::ConstraintIndex, index + 1, constrained.len())?;
        }
        Ok(Self { matrix, prescribed })
    }

    pub(crate) fn new(
        matrix: &'a SparseMatrix,
        prescribed: &[(usize, f64)],
    ) -> Result<Self, String> {
        checkpoint(SolverStage::ConstraintIndex, 0)?;
        reduction::validate_matrix(matrix)?;
        Ok(Self {
            matrix,
            prescribed: reduction::index_prescribed(matrix.size(), prescribed)?,
        })
    }

    pub(crate) fn validate(&self, rhs: &[f64], solution: &[f64]) -> Result<(), String> {
        checkpoint(SolverStage::ConstraintValidate, 0)?;
        let size = self.matrix.size();
        if rhs.len() != size || solution.len() != size {
            return Err(
                "original constrained-system dimensions must match the complete system".into(),
            );
        }
        for (index, (&value, &load)) in solution.iter().zip(rhs).enumerate() {
            if !value.is_finite() || !load.is_finite() {
                return Err(format!(
                    "original constrained-system contains a non-finite value at DOF {index}"
                ));
            }
            if self.prescribed[index].is_some_and(|fixed| fixed != value) {
                return Err(format!(
                    "original constrained-system prescribed value changed at DOF {index}"
                ));
            }
            checkpoint_chunk(SolverStage::ConstraintValidate, index + 1, size)?;
        }
        for (row, entries) in self.matrix.rows.iter().enumerate() {
            // Fixed equations contain reactions, not zero-residual equilibrium conditions.
            if self.prescribed[row].is_none() {
                let error = prescribed_row_backward_error(entries, rhs[row], solution, |column| {
                    self.prescribed[column].is_none()
                })
                .map_err(|error| format!("original constrained-system row {row}: {error}"))?;
                if !error.is_finite() || error > SPARSE_RESIDUAL_TOLERANCE {
                    return Err(format!(
                        "original constrained-system row {row} failed equilibrium validation ({error:.6e})"
                    ));
                }
            }
            checkpoint_chunk(SolverStage::ConstraintValidate, size + row + 1, size * 2)?;
        }
        Ok(())
    }
}

pub(crate) fn prescribed_row_backward_error(
    entries: &[(usize, f64)],
    expected: f64,
    solution: &[f64],
    roundable: impl Fn(usize) -> bool,
) -> Result<f64, String> {
    let mut actual = 0.0;
    let mut scale = expected.abs();
    let mut roundoff = 0.0;
    let mut has_response = false;
    let mut needs_range = false;
    visit_row_chunks(entries, SolverStage::ConstraintValidateRow, |_, chunk| {
        for &(column, coefficient) in chunk {
            let value = solution.get(column).copied().unwrap_or(f64::NAN);
            let product = coefficient * value;
            actual += product;
            scale += product.abs();
            has_response |= product != 0.0;
            needs_range |= residual::product_needs_range_check(coefficient, value, product);
            if roundable(column) {
                roundoff += residual::subnormal_solution_roundoff(coefficient, value);
            }
        }
    })?;
    let error = (expected - actual).abs();
    if needs_range || !error.is_finite() || !scale.is_finite() {
        return residual::range::scaled_entries_with_roundoff(
            entries.iter().copied(),
            entries.len(),
            expected,
            solution,
            SolverStage::ConstraintValidateRow,
            roundable,
        )
        .map_err(String::from)
        .and_then(|row| {
            if !row.residual.is_finite() {
                Err("original equation residual is outside the representable range".into())
            } else {
                Ok(row.resolved_relative())
            }
        });
    }
    let roundoff = residual::row_solution_roundoff(expected, has_response, roundoff);
    Ok(if scale == 0.0 {
        0.0
    } else {
        (error - roundoff).max(0.0) / scale
    })
}
