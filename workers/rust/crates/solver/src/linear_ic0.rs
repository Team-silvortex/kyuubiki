use crate::solver_control::{SolverStage, checkpoint, checkpoint_chunk};

#[derive(Debug, Clone)]
struct LowerTranspose {
    offsets: Vec<usize>,
    // Entries point into the lower factor's values; rows identify its source rows.
    factor_entries: Vec<u32>,
    source_rows: Vec<u32>,
}

#[derive(Debug, Clone)]
pub(crate) struct IncompleteCholesky {
    diagonal: Vec<f64>,
    lower_columns: Vec<u32>,
    lower_offsets: Vec<usize>,
    lower_values: Vec<f64>,
    transpose: LowerTranspose,
}

impl IncompleteCholesky {
    pub(crate) fn build(
        row_offsets: &[usize],
        lower_end_offsets: &[usize],
        columns: &[usize],
        values: &[f64],
        diagonal: &[f64],
    ) -> Result<Self, String> {
        checkpoint(SolverStage::IncompleteCholeskyFactor, 0)?;
        let size = diagonal.len();
        assert!(
            size <= u32::MAX as usize,
            "IC(0) index storage supports at most u32::MAX rows"
        );
        let lower_count = lower_end_offsets
            .iter()
            .enumerate()
            .map(|(row, &end)| end - row_offsets[row])
            .sum();
        let mut lower_columns = Vec::with_capacity(lower_count);
        let mut lower_offsets = Vec::with_capacity(size + 1);
        let mut lower_values = Vec::with_capacity(lower_count);
        let mut factor_diagonal = vec![0.0; size];
        lower_offsets.push(0);

        for row in 0..size {
            let row_start = lower_columns.len();
            for entry in row_offsets[row]..lower_end_offsets[row] {
                let column = columns[entry];
                let correction = lower_dot_until(
                    row_start,
                    lower_columns.len(),
                    column,
                    &lower_offsets,
                    &lower_columns,
                    &lower_values,
                );
                lower_columns.push(column as u32);
                lower_values.push((values[entry] - correction) / factor_diagonal[column]);
            }

            let lower_square_sum = lower_values[row_start..]
                .iter()
                .map(|value| value * value)
                .sum::<f64>();
            // A small positive floor keeps the preconditioner usable for nearly
            // singular assembled systems while the outer solver still checks residuals.
            factor_diagonal[row] = (diagonal[row] - lower_square_sum).max(1.0e-18).sqrt();
            lower_offsets.push(lower_columns.len());
            checkpoint_chunk(SolverStage::IncompleteCholeskyFactor, row + 1, size)?;
        }

        let transpose = transpose_lower(size, &lower_offsets, &lower_columns)?;
        Ok(Self {
            diagonal: factor_diagonal,
            lower_columns,
            lower_offsets,
            lower_values,
            transpose,
        })
    }

    pub(crate) fn apply(
        &self,
        residual: &[f64],
        result: &mut [f64],
        forward: &mut [f64],
    ) -> Result<(), String> {
        checkpoint(SolverStage::Ic0Forward, 0)?;
        for row in 0..self.diagonal.len() {
            let mut sum = residual[row];
            for entry in self.lower_offsets[row]..self.lower_offsets[row + 1] {
                sum -= self.lower_values[entry] * forward[self.lower_columns[entry] as usize];
            }
            forward[row] = sum / self.diagonal[row];
            checkpoint_chunk(SolverStage::Ic0Forward, row + 1, self.diagonal.len())?;
        }

        checkpoint(SolverStage::Ic0Backward, 0)?;
        for row in (0..self.diagonal.len()).rev() {
            let mut sum = forward[row];
            for entry in self.transpose.offsets[row]..self.transpose.offsets[row + 1] {
                let factor_entry = self.transpose.factor_entries[entry] as usize;
                sum -= self.lower_values[factor_entry]
                    * result[self.transpose.source_rows[entry] as usize];
            }
            result[row] = sum / self.diagonal[row];
            checkpoint_chunk(
                SolverStage::Ic0Backward,
                self.diagonal.len() - row,
                self.diagonal.len(),
            )?;
        }
        Ok(())
    }
}

fn lower_dot_until(
    row_start: usize,
    row_end: usize,
    column: usize,
    lower_offsets: &[usize],
    lower_columns: &[u32],
    lower_values: &[f64],
) -> f64 {
    let mut left = row_start;
    let mut right = lower_offsets[column];
    let right_end = lower_offsets[column + 1];
    let mut sum = 0.0;
    while left < row_end && right < right_end {
        let left_column = lower_columns[left];
        let right_column = lower_columns[right];
        if left_column == right_column {
            sum += lower_values[left] * lower_values[right];
            left += 1;
            right += 1;
        } else if left_column < right_column {
            left += 1;
        } else {
            right += 1;
        }
    }
    sum
}

fn transpose_lower(
    size: usize,
    lower_offsets: &[usize],
    lower_columns: &[u32],
) -> Result<LowerTranspose, String> {
    checkpoint(SolverStage::IncompleteCholeskyTranspose, 0)?;
    let mut counts = vec![0usize; size];
    for row in 0..size {
        for entry in lower_offsets[row]..lower_offsets[row + 1] {
            counts[lower_columns[entry] as usize] += 1;
        }
        checkpoint_chunk(SolverStage::IncompleteCholeskyTranspose, row + 1, size)?;
    }
    let mut offsets = Vec::with_capacity(size + 1);
    offsets.push(0);
    for (index, count) in counts.iter().enumerate() {
        offsets.push(offsets.last().copied().unwrap_or(0) + count);
        checkpoint_chunk(
            SolverStage::IncompleteCholeskyTranspose,
            size + index + 1,
            size * 2,
        )?;
    }
    let mut next = offsets[..size].to_vec();
    let mut factor_entries = vec![0u32; offsets[size]];
    let mut source_rows = vec![0u32; offsets[size]];
    for row in 0..size {
        let start = lower_offsets[row];
        let end = lower_offsets[row + 1];
        for (offset, &lower_column) in lower_columns[start..end].iter().enumerate() {
            let entry = start + offset;
            let column = lower_column as usize;
            let target = next[column];
            factor_entries[target] = entry as u32;
            source_rows[target] = row as u32;
            next[column] += 1;
        }
        checkpoint_chunk(
            SolverStage::IncompleteCholeskyTranspose,
            size * 2 + row + 1,
            size * 3,
        )?;
    }
    Ok(LowerTranspose {
        offsets,
        factor_entries,
        source_rows,
    })
}

#[cfg(test)]
mod tests {
    use super::{IncompleteCholesky, transpose_lower};

    #[test]
    fn transpose_retains_factor_entry_and_source_row_mapping() {
        let transpose = transpose_lower(5, &[0, 0, 1, 2, 4, 6], &[0, 1, 0, 2, 1, 3]).unwrap();
        assert_eq!(transpose.offsets, [0, 2, 4, 5, 6, 6]);
        assert_eq!(transpose.factor_entries, [0, 2, 1, 4, 3, 5]);
        assert_eq!(transpose.source_rows, [1, 3, 2, 4, 3, 4]);
    }

    #[test]
    fn empty_and_diagonal_only_factors_have_empty_transposes() {
        for size in [0, 1, 4] {
            let offsets = vec![0; size + 1];
            let transpose = transpose_lower(size, &offsets, &[]).unwrap();
            assert_eq!(transpose.offsets, offsets);
            assert!(transpose.factor_entries.is_empty());
            assert!(transpose.source_rows.is_empty());
        }
    }

    #[test]
    fn factor_application_matches_known_solution_and_reuses_workspace() {
        // A = L L^T with L = [[2,0,0], [1,3,0], [-1,2,4]].
        let factor = IncompleteCholesky::build(
            &[0, 3, 6, 9],
            &[0, 4, 8],
            &[0, 1, 2, 0, 1, 2, 0, 1, 2],
            &[4.0, 2.0, -2.0, 2.0, 10.0, 5.0, -2.0, 5.0, 21.0],
            &[4.0, 10.0, 21.0],
        )
        .unwrap();
        let mut result = [123.0; 3];
        let mut forward = [-456.0; 3];
        for (residual, expected) in [
            ([10.0, 17.0, -13.0], [1.0, 2.0, -1.0]),
            ([-13.0, 16.0, 69.5], [-2.0, 0.5, 3.0]),
            ([0.0; 3], [0.0; 3]),
        ] {
            factor.apply(&residual, &mut result, &mut forward).unwrap();
            for (actual, expected) in result.iter().zip(expected) {
                assert!((actual - expected).abs() < 1.0e-12);
            }
        }
    }
}
