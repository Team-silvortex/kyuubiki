use super::{TridiagonalSystem, backward_error_tolerance};
use crate::linear_algebra::prescribed_row_backward_error;
use crate::solver_control::{SolverStage, checkpoint, checkpoint_chunk};

#[cfg(test)]
#[path = "chain_tridiagonal_constraint_validation_tests.rs"]
mod tests;

pub(super) fn validate(
    system: &TridiagonalSystem<'_>,
    fixed: &[bool],
    prescribed: &[(usize, f64)],
    solution: &[f64],
) -> Result<(), String> {
    checkpoint(SolverStage::ConstraintValidate, 0)?;
    let size = system.diagonal.len();
    super::validate_dimensions(system.diagonal, system.lower, system.upper, system.rhs)?;
    if solution.len() != size || fixed.len() != size {
        return Err("original constrained-chain dimensions must match".into());
    }
    for (index, (&value, &rhs)) in solution.iter().zip(system.rhs).enumerate() {
        if !value.is_finite() || !rhs.is_finite() {
            return Err(format!(
                "original constrained-chain contains a non-finite value at DOF {index}"
            ));
        }
        checkpoint_chunk(SolverStage::ConstraintValidate, index + 1, size)?;
    }
    for (step, &(index, value)) in prescribed.iter().enumerate() {
        if solution.get(index).is_none_or(|&actual| actual != value) {
            return Err(format!(
                "original constrained-chain prescribed value changed at DOF {index}"
            ));
        }
        checkpoint_chunk(SolverStage::ConstraintIndex, step + 1, prescribed.len())?;
    }
    for row in 0..size {
        if !fixed[row] {
            let mut entries = [(row, system.diagonal[row]); 3];
            let mut length = 1;
            if row > 0 {
                entries[length] = (row - 1, system.lower[row - 1]);
                length += 1;
            }
            if row + 1 < size {
                entries[length] = (row + 1, system.upper[row]);
                length += 1;
            }
            let error = prescribed_row_backward_error(
                &entries[..length],
                system.rhs[row],
                solution,
                |column| !fixed[column],
            )
            .map_err(|error| format!("original constrained-chain row {row}: {error}"))?;
            if !error.is_finite() || error > backward_error_tolerance(size) {
                return Err(format!(
                    "original constrained-chain row {row} failed equilibrium validation ({error:.6e})"
                ));
            }
        }
        checkpoint_chunk(SolverStage::ConstraintValidate, size + row + 1, size * 2)?;
    }
    Ok(())
}
