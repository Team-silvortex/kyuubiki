use super::{
    CompressedSparseMatrix, SPARSE_RESIDUAL_TOLERANCE,
    product::visit_row_chunks,
    residual::{
        product_needs_range_check,
        range::{RangeError, scaled_entries},
        row_solution_roundoff, subnormal_solution_roundoff,
    },
};
use crate::solver_control::{SolverStage, checkpoint, checkpoint_chunk};

impl CompressedSparseMatrix {
    /// The global PCG norm is only a candidate gate; small rows still need equilibrium.
    pub(crate) fn satisfies_equilibrium(
        &self,
        rhs: &[f64],
        rhs_scale: f64,
        solution: &[f64],
    ) -> Result<bool, String> {
        checkpoint(SolverStage::ResidualValidate, 0)?;
        if rhs.len() != self.size() || solution.len() != self.size() {
            return Err("iterative equilibrium dimensions must match the complete system".into());
        }
        if !rhs_scale.is_finite() || rhs_scale <= 0.0 {
            return Err("iterative equilibrium scale must be finite and positive".into());
        }
        let mut accepted = true;
        for (row, &rhs_value) in rhs.iter().enumerate() {
            if !solution[row].is_finite() {
                return Err("iterative equilibrium contains a non-finite solution".into());
            }
            let expected = rhs_value / rhs_scale;
            if rhs_value != 0.0 && expected == 0.0 {
                return Err("iterative equilibrium normalization would lose a nonzero load".into());
            }
            let start = self.row_offsets[row];
            let end = self.row_offsets[row + 1];
            let columns = &self.columns[start..end];
            let values = &self.values[start..end];
            let mut actual = 0.0;
            let mut scale = expected.abs();
            let mut solution_roundoff = 0.0;
            let mut has_response = false;
            let mut needs_range_check = false;
            visit_row_chunks(
                columns,
                SolverStage::ResidualValidateRow,
                |offset, columns| {
                    for (&column, &coefficient) in columns.iter().zip(&values[offset..]) {
                        let term = coefficient * solution[column];
                        has_response |= term != 0.0;
                        needs_range_check |=
                            product_needs_range_check(coefficient, solution[column], term);
                        actual += term;
                        scale += term.abs();
                        solution_roundoff +=
                            subnormal_solution_roundoff(coefficient, solution[column]);
                    }
                },
            )?;
            let residual = (expected - actual).abs();
            let relative = if !residual.is_finite() || !scale.is_finite() || needs_range_check {
                match scaled_entries(
                    columns.iter().copied().zip(values.iter().copied()),
                    columns.len(),
                    expected,
                    solution,
                    SolverStage::ResidualValidateRow,
                ) {
                    Ok(value) => value.resolved_relative(),
                    // A candidate can be rejected and refined; final validation still errors.
                    Err(RangeError::Unresolved(_)) => f64::INFINITY,
                    Err(error) => return Err(error.into()),
                }
            } else if scale == 0.0 {
                0.0
            } else {
                (residual - row_solution_roundoff(expected, has_response, solution_roundoff))
                    .max(0.0)
                    / scale
            };
            accepted &= relative <= SPARSE_RESIDUAL_TOLERANCE;
            checkpoint_chunk(SolverStage::ResidualValidate, row + 1, self.size())?;
        }
        Ok(accepted)
    }
}
