use super::finish_back_substitution;
use super::range_value::WorkValue;
use crate::linear_substitution::{SubnormalResult, solve_scaled_rhs_row_with_policy};
use crate::solver_control::{SolverStage, checkpoint, checkpoint_chunk};

const STAGE: SolverStage = SolverStage::TridiagonalForwardRange;

/// Exceptional replay with per-entry exponents, never a global RHS downscale.
/// The caller drops its ordinary workspace first and validates original rows after return.
pub(super) fn solve(
    diagonal: &[f64],
    upper: &[f64],
    second_upper: &[f64],
    factors: &[f64],
    swapped: &[bool],
    rhs: &[f64],
) -> Result<Vec<f64>, String> {
    checkpoint(STAGE, 0)?;
    let size = rhs.len();
    let offdiagonal = size.saturating_sub(1);
    if diagonal.len() != size
        || upper.len() != offdiagonal
        || factors.len() != offdiagonal
        || (!swapped.is_empty() && swapped.len() != offdiagonal)
        || (swapped.is_empty() && !second_upper.is_empty())
        || (!swapped.is_empty() && second_upper.len() != size.saturating_sub(2))
    {
        return Err("tridiagonal forward recovery dimensions must match".into());
    }
    let mut work = Vec::with_capacity(size);
    for row in 0..size {
        if !diagonal[row].is_finite()
            || diagonal[row] == 0.0
            || upper.get(row).is_some_and(|x| !x.is_finite())
            || second_upper.get(row).is_some_and(|x| !x.is_finite())
            || factors.get(row).is_some_and(|x| !x.is_finite())
        {
            return Err("tridiagonal forward recovery contains an invalid factor".into());
        }
        work.push(WorkValue::finite(rhs[row])?);
        checkpoint_chunk(STAGE, row + 1, size)?;
    }
    for row in 0..offdiagonal {
        if swapped.get(row) == Some(&true) {
            work.swap(row, row + 1);
        }
        work[row + 1] = work[row + 1].subtract_product(factors[row], work[row])?;
        checkpoint_chunk(STAGE, size + row + 1, size.saturating_mul(2))?;
    }
    checkpoint(STAGE, size.saturating_mul(2))?;

    let mut solution = vec![0.0; size];
    for row in (0..size).rev() {
        let width = if second_upper.is_empty() { 2 } else { 3 };
        let operands = (row + 1..(row + width).min(size)).map(|column| {
            let coefficient = if column == row + 1 {
                upper[row]
            } else {
                second_upper[row]
            };
            (coefficient, solution[column])
        });
        solution[row] = if let Some(rhs) = work[row].represented() {
            finish_back_substitution(rhs, diagonal[row], operands)?
        } else {
            solve_scaled_rhs_row_with_policy(
                (work[row].mantissa, work[row].exponent),
                operands,
                diagonal[row],
                SolverStage::TridiagonalSubstitutionRange,
                SubnormalResult::ValidateOriginalEquations,
            )?
        };
        checkpoint_chunk(STAGE, size * 2 + size - row, size.saturating_mul(3))?;
    }
    Ok(solution)
}
