use crate::linear_algebra::{SparseMatrix, sparse_to_dense};
use crate::linear_banded::SymmetricBandCholesky;
use crate::linear_dense::solve_linear_system;

const MAX_BAND_FACTOR_ENTRIES: usize = 8_000_000;
const MIN_BACKWARD_ERROR_TOLERANCE: f64 = 1.0e-9;
const MAX_BACKWARD_ERROR_TOLERANCE: f64 = 1.0e-7;

pub(crate) const SYMMETRIC_BAND_CHOLESKY: &str = "symmetric_band_cholesky";
pub(crate) const DENSE_PIVOTED_FALLBACK: &str = "dense_pivoted_fallback";

pub(crate) struct SymmetricTangentSolution {
    pub(crate) solution: Vec<f64>,
    pub(crate) method: &'static str,
}

pub(crate) fn solve_symmetric_tangent(
    matrix: &SparseMatrix,
    rhs: &[f64],
    dense_fallback_limit: usize,
    context: &str,
) -> Result<SymmetricTangentSolution, String> {
    if matrix.size() != rhs.len() || matrix.size() == 0 {
        return Err(format!(
            "{context} tangent dimensions do not match a non-empty right-hand side"
        ));
    }
    if rhs.iter().any(|value| !value.is_finite()) {
        return Err(format!("{context} tangent right-hand side must be finite"));
    }
    if (0..matrix.size()).any(|row| {
        matrix
            .row_entries(row)
            .iter()
            .any(|(_, value)| !value.is_finite())
    }) {
        return Err(format!("{context} tangent matrix must be finite"));
    }

    let sparse_failure = match SymmetricBandCholesky::try_factor(matrix, MAX_BAND_FACTOR_ENTRIES) {
        Ok(Some(factor)) => {
            let solution = solve_refined(matrix, rhs, &factor, context)?;
            return Ok(SymmetricTangentSolution {
                solution,
                method: SYMMETRIC_BAND_CHOLESKY,
            });
        }
        Ok(None) => "symmetric band exceeds the factor memory budget".to_string(),
        Err(error) => error,
    };

    if rhs.len() <= dense_fallback_limit {
        let solution = solve_linear_system(sparse_to_dense(matrix), rhs.to_vec())
            .map_err(|error| format!("{context} tangent solve failed: {error}"))?;
        validate_solution(matrix, rhs, &solution, context)?;
        return Ok(SymmetricTangentSolution {
            solution,
            method: DENSE_PIVOTED_FALLBACK,
        });
    }

    Err(format!(
        "{context} tangent requires an indefinite or wide sparse solve beyond the {dense_fallback_limit}-dof dense fallback: {sparse_failure}"
    ))
}

fn solve_refined(
    matrix: &SparseMatrix,
    rhs: &[f64],
    factor: &SymmetricBandCholesky,
    context: &str,
) -> Result<Vec<f64>, String> {
    let mut solution = factor.solve(rhs)?;
    let tolerance = backward_error_tolerance(rhs.len());
    for refinement in 0..=4 {
        let residual = linear_residual(matrix, rhs, &solution);
        let backward_error = maximum_backward_error(matrix, rhs, &solution, &residual);
        if backward_error.is_finite() && backward_error <= tolerance {
            return Ok(solution);
        }
        if refinement == 4 {
            return Err(format!(
                "{context} tangent solution failed backward-error validation ({backward_error:.6e})"
            ));
        }
        let correction = factor.solve(&residual)?;
        for (value, correction) in solution.iter_mut().zip(correction) {
            *value += correction;
        }
    }
    unreachable!("refinement loop always returns")
}

fn linear_residual(matrix: &SparseMatrix, rhs: &[f64], solution: &[f64]) -> Vec<f64> {
    (0..matrix.size())
        .map(|row| {
            rhs[row]
                - matrix
                    .row_entries(row)
                    .iter()
                    .map(|&(column, value)| value * solution[column])
                    .sum::<f64>()
        })
        .collect()
}

fn validate_solution(
    matrix: &SparseMatrix,
    rhs: &[f64],
    solution: &[f64],
    context: &str,
) -> Result<(), String> {
    let residual = linear_residual(matrix, rhs, solution);
    let backward_error = maximum_backward_error(matrix, rhs, solution, &residual);
    if !backward_error.is_finite() || backward_error > backward_error_tolerance(rhs.len()) {
        return Err(format!(
            "{context} tangent solution failed backward-error validation ({backward_error:.6e})"
        ));
    }
    Ok(())
}

fn maximum_backward_error(
    matrix: &SparseMatrix,
    rhs: &[f64],
    solution: &[f64],
    residual: &[f64],
) -> f64 {
    let mut maximum = 0.0_f64;
    for row in 0..matrix.size() {
        let row_entries = matrix.row_entries(row);
        let row_norm = row_entries
            .iter()
            .map(|(_, value)| value.abs())
            .sum::<f64>();
        let connected_solution_scale = row_entries
            .iter()
            .map(|(column, _)| solution[*column].abs())
            .fold(0.0, f64::max);
        let equation_scale = rhs[row].abs() + row_norm * connected_solution_scale;
        let relative = if equation_scale == 0.0 {
            if residual[row] == 0.0 {
                0.0
            } else {
                f64::INFINITY
            }
        } else {
            residual[row].abs() / equation_scale
        };
        maximum = maximum.max(relative);
    }
    maximum
}

fn backward_error_tolerance(size: usize) -> f64 {
    (128.0 * f64::EPSILON * size.max(1) as f64)
        .clamp(MIN_BACKWARD_ERROR_TOLERANCE, MAX_BACKWARD_ERROR_TOLERANCE)
}

#[cfg(test)]
mod tests {
    use crate::linear_algebra::{SparseMatrix, add_at};

    use super::{DENSE_PIVOTED_FALLBACK, SYMMETRIC_BAND_CHOLESKY, solve_symmetric_tangent};

    #[test]
    fn reports_sparse_and_dense_solver_paths() {
        let mut positive = SparseMatrix::new(2);
        add_at(&mut positive, 0, 0, 2.0);
        add_at(&mut positive, 0, 1, -1.0);
        add_at(&mut positive, 1, 0, -1.0);
        add_at(&mut positive, 1, 1, 2.0);
        let sparse = solve_symmetric_tangent(&positive, &[1.0, 0.0], 2, "test")
            .expect("positive tangent should solve");
        assert_eq!(sparse.method, SYMMETRIC_BAND_CHOLESKY);

        let mut indefinite = SparseMatrix::new(2);
        add_at(&mut indefinite, 0, 1, 1.0);
        add_at(&mut indefinite, 1, 0, 1.0);
        let dense = solve_symmetric_tangent(&indefinite, &[1.0, 2.0], 2, "test")
            .expect("invertible indefinite tangent should use the dense fallback");
        assert_eq!(dense.method, DENSE_PIVOTED_FALLBACK);
        assert!((dense.solution[0] - 2.0).abs() < 1.0e-12);
        assert!((dense.solution[1] - 1.0).abs() < 1.0e-12);
    }

    #[test]
    fn preserves_tiny_scales_on_sparse_and_dense_tangent_paths() {
        let mut positive = SparseMatrix::new(2);
        for (row, column, value) in [
            (0, 0, 2.0e-24),
            (0, 1, -1.0e-24),
            (1, 0, -1.0e-24),
            (1, 1, 2.0e-24),
        ] {
            add_at(&mut positive, row, column, value);
        }
        let sparse = solve_symmetric_tangent(&positive, &[1.0e-24, 0.0], 2, "tiny SPD")
            .expect("tiny SPD tangent should solve without a dense fallback");
        assert_eq!(sparse.method, SYMMETRIC_BAND_CHOLESKY);
        assert!((sparse.solution[0] - 2.0 / 3.0).abs() < 1.0e-12);
        assert!((sparse.solution[1] - 1.0 / 3.0).abs() < 1.0e-12);

        let mut indefinite = SparseMatrix::new(2);
        add_at(&mut indefinite, 0, 1, 1.0e-24);
        add_at(&mut indefinite, 1, 0, 1.0e-24);
        let dense = solve_symmetric_tangent(&indefinite, &[1.0e-24, 2.0e-24], 2, "tiny indefinite")
            .expect("tiny indefinite tangent should use the dense fallback");
        assert_eq!(dense.method, DENSE_PIVOTED_FALLBACK);
        assert!((dense.solution[0] - 2.0).abs() < 1.0e-12);
        assert!((dense.solution[1] - 1.0).abs() < 1.0e-12);
    }
}
