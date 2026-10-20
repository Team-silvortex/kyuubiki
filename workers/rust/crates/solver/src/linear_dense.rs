use crate::solver_control::{SolverStage, checkpoint};

#[path = "linear_dense_equilibration.rs"]
mod equilibration;
use equilibration::RowEquilibration;

#[path = "linear_dense_substitution.rs"]
mod substitution;

#[cfg(test)]
#[path = "linear_dense_range_tests.rs"]
mod range_tests;

#[cfg(test)]
#[path = "linear_dense_substitution_tests.rs"]
mod substitution_tests;

pub(crate) fn solve_linear_system(
    matrix: Vec<Vec<f64>>,
    vector: Vec<f64>,
) -> Result<Vec<f64>, String> {
    let size = vector.len();

    if matrix.len() != size || matrix.iter().any(|row| row.len() != size) {
        return Err("matrix dimensions do not match vector".to_string());
    }
    DenseLu::factor(matrix)?.solve(&vector)
}

pub(crate) struct DenseLu {
    factors: Vec<Vec<f64>>,
    permutation: Vec<usize>,
    equilibration: Option<RowEquilibration>,
}

impl DenseLu {
    pub(crate) fn factor(mut matrix: Vec<Vec<f64>>) -> Result<Self, String> {
        checkpoint(SolverStage::DenseFactor, 0)?;
        let size = matrix.len();
        if matrix.iter().any(|row| row.len() != size) {
            return Err("dense factor matrix must be square".to_string());
        }
        if matrix.iter().flatten().any(|value| !value.is_finite()) {
            return Err("linear system matrix contains non-finite value".to_string());
        }

        let mut row_scales = matrix
            .iter()
            .map(|row| row.iter().map(|value| value.abs()).fold(0.0, f64::max))
            .collect::<Vec<_>>();
        if row_scales.contains(&0.0) {
            return Err("system is singular".to_string());
        }
        let equilibration = RowEquilibration::prepare(&mut matrix, &mut row_scales)?;
        let relative_pivot_floor = 32.0 * f64::EPSILON * size.max(1) as f64;
        let mut permutation = (0..size).collect::<Vec<_>>();

        for pivot in 0..size {
            let max_row = (pivot..size)
                .max_by(|&left, &right| {
                    let left_score = matrix[left][pivot].abs() / row_scales[left];
                    let right_score = matrix[right][pivot].abs() / row_scales[right];
                    left_score.total_cmp(&right_score)
                })
                .expect("pivot range should not be empty");

            matrix.swap(pivot, max_row);
            row_scales.swap(pivot, max_row);
            permutation.swap(pivot, max_row);

            let pivot_value = matrix[pivot][pivot];
            if pivot_value.abs() / row_scales[pivot] <= relative_pivot_floor {
                return Err("system is singular".to_string());
            }

            let (leading_rows, trailing_rows) = matrix.split_at_mut(pivot + 1);
            let pivot_row = &leading_rows[pivot];
            for row in trailing_rows {
                let factor = row[pivot] / pivot_value;
                if !factor.is_finite() {
                    return Err("linear system elimination diverged".to_string());
                }
                row[pivot] = factor;
                for (value, pivot_value) in row[pivot + 1..].iter_mut().zip(&pivot_row[pivot + 1..])
                {
                    *value = (-factor).mul_add(*pivot_value, *value);
                    if !value.is_finite() {
                        return Err("linear system elimination diverged".to_string());
                    }
                }
            }
            checkpoint(SolverStage::DenseFactor, pivot + 1)?;
        }

        Ok(Self {
            factors: matrix,
            permutation,
            equilibration,
        })
    }

    pub(crate) fn solve(&self, vector: &[f64]) -> Result<Vec<f64>, String> {
        checkpoint(SolverStage::DenseSubstitution, 0)?;
        let size = self.factors.len();
        if vector.len() != size {
            return Err("matrix dimensions do not match vector".to_string());
        }
        if vector.iter().any(|value| !value.is_finite()) {
            return Err("linear system vector contains non-finite value".to_string());
        }

        let mut solution = match &self.equilibration {
            Some(equilibration) => equilibration.permute_rhs(vector, &self.permutation)?,
            None => self
                .permutation
                .iter()
                .map(|index| vector[*index])
                .collect(),
        };
        for row in 0..size {
            solution[row] = substitution::solve_row(
                solution[row],
                &self.factors[row][..row],
                &solution[..row],
                1.0,
            )?;
            checkpoint(SolverStage::DenseSubstitution, row + 1)?;
        }

        for row in (0..size).rev() {
            solution[row] = substitution::solve_row(
                solution[row],
                &self.factors[row][row + 1..],
                &solution[row + 1..],
                self.factors[row][row],
            )?;
            checkpoint(SolverStage::DenseSubstitution, size + (size - row))?;
        }

        Ok(solution)
    }
}

pub(crate) fn zero_matrix(size: usize) -> Vec<Vec<f64>> {
    vec![vec![0.0; size]; size]
}
