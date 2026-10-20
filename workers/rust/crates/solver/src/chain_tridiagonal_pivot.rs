use super::{ensure_usable_pivot, row_scales};
use crate::solver_control::{SolverStage, checkpoint};

/// Adjacent-row pivoted LU: U gains only one additional superdiagonal.
/// Original coefficients and backward-error validation stay with the caller.
pub(super) struct PivotedTridiagonal {
    diagonal: Vec<f64>,
    upper: Vec<f64>,
    second_upper: Vec<f64>,
    factors: Vec<f64>,
    swapped: Vec<bool>,
}

impl PivotedTridiagonal {
    pub(super) fn factor(diagonal: &[f64], lower: &[f64], upper: &[f64]) -> Result<Self, String> {
        checkpoint(SolverStage::TridiagonalFactor, 0)?;
        let size = diagonal.len();
        let mut scales = row_scales(diagonal, lower, upper);
        let mut result = Self {
            diagonal: diagonal.to_vec(),
            upper: upper.to_vec(),
            second_upper: vec![0.0; size.saturating_sub(2)],
            factors: lower.to_vec(),
            swapped: vec![false; size.saturating_sub(1)],
        };
        for row in 0..size.saturating_sub(1) {
            let next = row + 1;
            let subdiagonal = result.factors[row];
            let diagonal = result.diagonal[row];
            let factor;
            if diagonal.abs() >= subdiagonal.abs() {
                ensure_usable_pivot(diagonal, scales[row], size)?;
                factor = subdiagonal / diagonal;
                result.diagonal[next] = (-factor).mul_add(result.upper[row], result.diagonal[next]);
            } else {
                scales.swap(row, next);
                ensure_usable_pivot(subdiagonal, scales[row], size)?;
                factor = diagonal / subdiagonal;
                result.swapped[row] = true;
                result.diagonal[row] = subdiagonal;
                let next_diagonal = result.diagonal[next];
                result.diagonal[next] = (-factor).mul_add(next_diagonal, result.upper[row]);
                result.upper[row] = next_diagonal;
                if next + 1 < size {
                    result.second_upper[row] = result.upper[next];
                    result.upper[next] *= -factor;
                }
            }
            result.factors[row] = factor;
            if !factor.is_finite() || !result.diagonal[next].is_finite() {
                return Err("tridiagonal elimination diverged".into());
            }
            if next % 256 == 0 {
                checkpoint(SolverStage::TridiagonalFactor, next)?;
            }
        }
        if size > 0 {
            ensure_usable_pivot(result.diagonal[size - 1], scales[size - 1], size)?;
        }
        checkpoint(SolverStage::TridiagonalFactor, size)?;
        Ok(result)
    }

    pub(super) fn solve(&self, rhs: &[f64]) -> Result<Vec<f64>, String> {
        let size = rhs.len();
        let mut result = rhs.to_vec();
        for row in 0..size.saturating_sub(1) {
            let next = row + 1;
            if self.swapped[row] {
                let previous = result[row];
                result[row] = result[next];
                result[next] = (-self.factors[row]).mul_add(result[row], previous);
            } else {
                result[next] = (-self.factors[row]).mul_add(result[row], result[next]);
            }
            if !result[next].is_finite() {
                return Err("tridiagonal elimination diverged".into());
            }
            if next % 256 == 0 {
                checkpoint(SolverStage::TridiagonalSubstitution, next)?;
            }
        }
        for row in (0..size).rev() {
            let mut value = result[row];
            if row + 1 < size {
                value = (-self.upper[row]).mul_add(result[row + 1], value);
            }
            if row + 2 < size {
                value = (-self.second_upper[row]).mul_add(result[row + 2], value);
            }
            result[row] = value / self.diagonal[row];
            if row % 256 == 0 {
                checkpoint(SolverStage::TridiagonalSubstitution, size + size - row)?;
            }
        }
        checkpoint(SolverStage::TridiagonalSubstitution, size.saturating_mul(2))?;
        Ok(result)
    }
}
