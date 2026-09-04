use crate::linear_algebra::SparseMatrix;

/// Reusable Cholesky factor for symmetric positive-definite matrices with a narrow band.
pub(crate) struct SymmetricBandCholesky {
    size: usize,
    width: usize,
    lower: Vec<f64>,
    row_scales: Vec<f64>,
}

impl SymmetricBandCholesky {
    pub(crate) fn try_factor(
        matrix: &SparseMatrix,
        max_entries: usize,
    ) -> Result<Option<Self>, String> {
        let size = matrix.size();
        if size == 0 {
            return Err("banded Cholesky matrix must be non-empty".to_string());
        }
        let width = (0..size)
            .flat_map(|row| {
                matrix
                    .row_entries(row)
                    .iter()
                    .filter(move |(column, _)| *column <= row)
                    .map(move |(column, _)| row - column)
            })
            .max()
            .unwrap_or(0);
        let stride = width + 1;
        let Some(entry_count) = size.checked_mul(stride) else {
            return Ok(None);
        };
        if entry_count > max_entries {
            return Ok(None);
        }
        if (0..size).any(|row| {
            matrix
                .row_entries(row)
                .iter()
                .any(|(_, value)| !value.is_finite())
        }) {
            return Err("banded Cholesky matrix contains a non-finite value".to_string());
        }
        let row_scales = (0..size)
            .map(|row| {
                matrix
                    .row_entries(row)
                    .iter()
                    .map(|(_, value)| value.abs())
                    .fold(0.0, f64::max)
            })
            .collect::<Vec<_>>();
        if row_scales.contains(&0.0) {
            return Err("banded Cholesky matrix contains a singular row".to_string());
        }

        let mut factor = Self {
            size,
            width,
            lower: vec![0.0; entry_count],
            row_scales,
        };
        for row in 0..size {
            for &(column, value) in matrix.row_entries(row) {
                if column <= row {
                    factor.set(row, column, value);
                }
            }
        }
        factor.factor_in_place()?;
        Ok(Some(factor))
    }

    pub(crate) fn solve(&self, rhs: &[f64]) -> Result<Vec<f64>, String> {
        if rhs.len() != self.size || rhs.iter().any(|value| !value.is_finite()) {
            return Err("banded Cholesky right-hand side is invalid".to_string());
        }
        let mut forward = vec![0.0; self.size];
        for row in 0..self.size {
            let start = row.saturating_sub(self.width);
            let correction = (start..row)
                .map(|column| self.get(row, column) * forward[column])
                .sum::<f64>();
            forward[row] = (rhs[row] - correction) / self.get(row, row);
        }

        let mut solution = vec![0.0; self.size];
        for row in (0..self.size).rev() {
            let end = (row + self.width + 1).min(self.size);
            let correction = ((row + 1)..end)
                .map(|other| self.get(other, row) * solution[other])
                .sum::<f64>();
            solution[row] = (forward[row] - correction) / self.get(row, row);
        }
        if solution.iter().any(|value| !value.is_finite()) {
            return Err("banded Cholesky solve produced a non-finite value".to_string());
        }
        Ok(solution)
    }

    fn factor_in_place(&mut self) -> Result<(), String> {
        let relative_floor = (64.0 * f64::EPSILON * self.size.max(1) as f64).min(1.0e-8);
        for row in 0..self.size {
            let row_start = row.saturating_sub(self.width);
            for column in row_start..=row {
                let overlap_start = row_start.max(column.saturating_sub(self.width));
                let correction = (overlap_start..column)
                    .map(|index| self.get(row, index) * self.get(column, index))
                    .sum::<f64>();
                let reduced = self.get(row, column) - correction;
                if row == column {
                    if !(reduced.is_finite()
                        && reduced > 0.0
                        && reduced / self.row_scales[row] > relative_floor)
                    {
                        return Err(format!(
                            "banded Cholesky matrix is not positive definite at row {row}"
                        ));
                    }
                    self.set(row, column, reduced.sqrt());
                } else {
                    let factored = reduced / self.get(column, column);
                    if !factored.is_finite() {
                        return Err("banded Cholesky factorization diverged".to_string());
                    }
                    self.set(row, column, factored);
                }
            }
        }
        Ok(())
    }

    fn get(&self, row: usize, column: usize) -> f64 {
        if column > row || row - column > self.width {
            0.0
        } else {
            self.lower[row * (self.width + 1) + row - column]
        }
    }

    fn set(&mut self, row: usize, column: usize, value: f64) {
        let index = row * (self.width + 1) + row - column;
        self.lower[index] = value;
    }
}

#[cfg(test)]
mod tests {
    use crate::linear_algebra::{SparseMatrix, add_at};

    use super::SymmetricBandCholesky;

    #[test]
    fn factors_and_reuses_a_narrow_spd_band() {
        let mut matrix = SparseMatrix::new(4);
        for row in 0..4 {
            add_at(&mut matrix, row, row, 2.0);
            if row > 0 {
                add_at(&mut matrix, row, row - 1, -1.0);
                add_at(&mut matrix, row - 1, row, -1.0);
            }
        }
        let factor = SymmetricBandCholesky::try_factor(&matrix, 100)
            .expect("banded matrix should factor")
            .expect("band should fit the budget");
        let solution = factor
            .solve(&[1.0, 0.0, 0.0, 1.0])
            .expect("factored matrix should solve");
        for value in solution {
            assert!((value - 1.0).abs() < 1.0e-12);
        }
    }

    #[test]
    fn declines_a_band_that_exceeds_the_memory_budget() {
        let mut matrix = SparseMatrix::new(4);
        for row in 0..4 {
            add_at(&mut matrix, row, row, 2.0);
        }
        add_at(&mut matrix, 0, 3, -0.1);
        add_at(&mut matrix, 3, 0, -0.1);
        assert!(
            SymmetricBandCholesky::try_factor(&matrix, 12)
                .expect("budget check should not fail")
                .is_none()
        );
    }

    #[test]
    fn factors_a_uniformly_tiny_positive_definite_band() {
        let mut matrix = SparseMatrix::new(2);
        for (row, column, value) in [
            (0, 0, 2.0e-24),
            (0, 1, -1.0e-24),
            (1, 0, -1.0e-24),
            (1, 1, 2.0e-24),
        ] {
            add_at(&mut matrix, row, column, value);
        }

        let factor = SymmetricBandCholesky::try_factor(&matrix, 10)
            .expect("tiny matrix factorization should be scale invariant")
            .expect("tiny band should fit the budget");
        let solution = factor
            .solve(&[1.0e-24, 0.0])
            .expect("tiny factored matrix should solve");
        assert!((solution[0] - 2.0 / 3.0).abs() < 1.0e-12);
        assert!((solution[1] - 1.0 / 3.0).abs() < 1.0e-12);
    }
}
