use crate::linear_algebra::{SparseMatrix, sparse_to_dense};
use crate::linear_banded::SymmetricBandCholesky;
use crate::linear_dense::solve_linear_system;

const MAX_BAND_FACTOR_ENTRIES: usize = 8_000_000;

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

    let sparse_failure = match SymmetricBandCholesky::try_factor(matrix, MAX_BAND_FACTOR_ENTRIES) {
        Ok(Some(factor)) => {
            let solution = solve_refined(matrix, rhs, &factor)?;
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
) -> Result<Vec<f64>, String> {
    let mut solution = factor.solve(rhs)?;
    for _ in 0..2 {
        let residual = linear_residual(matrix, rhs, &solution);
        if normalized_linear_residual(&residual, rhs) <= 1.0e-12 {
            break;
        }
        let correction = factor.solve(&residual)?;
        for (value, correction) in solution.iter_mut().zip(correction) {
            *value += correction;
        }
    }
    Ok(solution)
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

fn normalized_linear_residual(residual: &[f64], rhs: &[f64]) -> f64 {
    let numerator = residual.iter().map(|value| value.abs()).fold(0.0, f64::max);
    let denominator = rhs.iter().map(|value| value.abs()).fold(1.0, f64::max);
    numerator / denominator
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
}
