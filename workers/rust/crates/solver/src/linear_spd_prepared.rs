use super::{
    CompressedSparseMatrix, SparseMatrix, refine_dense_solution, scaling,
    sparse_path::{SparseTridiagonal, solve_factored_tridiagonal, sparse_tridiagonal_coefficients},
    sparse_to_dense, validate_spd_solution,
};
use crate::chain_tridiagonal::PreparedTridiagonal;
use crate::linear_dense::DenseLu;
use crate::linear_solver_profile::{SpdSolveOptions, SpdSolveProfile};
use crate::linear_spd::solve_spd_compressed;

pub(crate) struct PreparedSpdSolver {
    matrix: SparseMatrix,
    backend: PreparedBackend,
}

enum PreparedBackend {
    Empty,
    Tridiagonal {
        factor: PreparedTridiagonal,
        order: Option<Vec<usize>>,
    },
    Dense(DenseLu),
    Iterative {
        scaling: Vec<f64>,
        compressed: Box<CompressedSparseMatrix>,
        diagonal_scale: f64,
        options: SpdSolveOptions,
    },
}

impl PreparedSpdSolver {
    pub(crate) fn factor(matrix: SparseMatrix) -> Result<Self, String> {
        scaling::validate_sparse_system_finite(&matrix, &[])?;
        let size = matrix.size();
        let backend = if size == 0 {
            PreparedBackend::Empty
        } else if let Some(SparseTridiagonal {
            diagonal,
            lower,
            upper,
            order,
        }) = sparse_tridiagonal_coefficients(&matrix)
        {
            PreparedBackend::Tridiagonal {
                factor: PreparedTridiagonal::factor(&diagonal, &lower, &upper)?,
                order,
            }
        } else if size <= 1024 {
            PreparedBackend::Dense(DenseLu::factor(sparse_to_dense(&matrix))?)
        } else {
            let options = SpdSolveOptions::default();
            let scaling = scaling::diagonal_sparse_scaling(&matrix);
            let diagonal_scale =
                scaling::average_scaled_diagonal_magnitude(&matrix, &scaling).max(1.0);
            let compressed = Box::new(matrix.compress_scaled(&scaling, options.preconditioner));
            PreparedBackend::Iterative {
                scaling,
                compressed,
                diagonal_scale,
                options,
            }
        };
        Ok(Self { matrix, backend })
    }

    pub(crate) fn solve(&self, rhs: &[f64]) -> Result<Vec<f64>, String> {
        if rhs.len() != self.matrix.size() {
            return Err("matrix dimensions do not match vector".to_string());
        }
        scaling::validate_sparse_system_finite(&self.matrix, rhs)?;

        let profile = match &self.backend {
            PreparedBackend::Empty => SpdSolveProfile {
                solution: Vec::new(),
                iterations: 0,
                matrix_non_zero_count: 0,
                residual_norm: 0.0,
                stages: Vec::new(),
            },
            PreparedBackend::Tridiagonal { factor, order } => SpdSolveProfile {
                solution: solve_factored_tridiagonal(factor, order.as_deref(), rhs)?,
                iterations: 0,
                matrix_non_zero_count: self.matrix.non_zero_count(),
                residual_norm: 0.0,
                stages: Vec::new(),
            },
            PreparedBackend::Dense(factor) => {
                let solution = factor.solve(rhs)?;
                SpdSolveProfile {
                    solution: refine_dense_solution(&self.matrix, rhs, solution, factor)?,
                    iterations: 0,
                    matrix_non_zero_count: self.matrix.non_zero_count(),
                    residual_norm: 0.0,
                    stages: Vec::new(),
                }
            }
            PreparedBackend::Iterative {
                scaling,
                compressed,
                diagonal_scale,
                options,
            } => self.solve_iterative(rhs, scaling, compressed, *diagonal_scale, options)?,
        };
        validate_spd_solution(&self.matrix, rhs, profile).map(|profile| profile.solution)
    }

    fn solve_iterative(
        &self,
        rhs: &[f64],
        scaling_factors: &[f64],
        compressed: &CompressedSparseMatrix,
        diagonal_scale: f64,
        options: &SpdSolveOptions,
    ) -> Result<SpdSolveProfile, String> {
        let scaled_rhs = scaling::scale_sparse_rhs(rhs, scaling_factors);
        let scaled_profile =
            match solve_spd_compressed(compressed, &scaled_rhs, &self.matrix, options) {
                Ok(profile) => profile,
                Err(error) => self
                    .solve_regularized(&scaled_rhs, scaling_factors, diagonal_scale, options)
                    .ok_or(error)?,
            };
        Ok(scaling::unscale_profile(scaled_profile, scaling_factors))
    }

    fn solve_regularized(
        &self,
        scaled_rhs: &[f64],
        scaling_factors: &[f64],
        diagonal_scale: f64,
        options: &SpdSolveOptions,
    ) -> Option<SpdSolveProfile> {
        let scaled_matrix = scaling::scale_sparse_matrix(&self.matrix, scaling_factors);
        for factor in [1.0e-10, 1.0e-8, 1.0e-6] {
            let regularized =
                scaling::regularize_sparse_diagonal(&scaled_matrix, diagonal_scale * factor);
            let compressed = regularized.compress(options.preconditioner);
            if let Ok(profile) =
                solve_spd_compressed(&compressed, scaled_rhs, &regularized, options)
            {
                return Some(profile);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::{PreparedBackend, PreparedSpdSolver};
    use crate::linear_algebra::{SparseMatrix, add_at};

    #[test]
    fn reuses_a_dense_factor_for_multiple_right_hand_sides() {
        let mut matrix = SparseMatrix::new(3);
        for (row, column, value) in [
            (0, 0, 4.0),
            (0, 1, 1.0),
            (0, 2, 1.0),
            (1, 0, 1.0),
            (1, 1, 3.0),
            (1, 2, 0.5),
            (2, 0, 1.0),
            (2, 1, 0.5),
            (2, 2, 2.0),
        ] {
            add_at(&mut matrix, row, column, value);
        }
        let solver = PreparedSpdSolver::factor(matrix).expect("dense matrix should factor");

        let first = solver.solve(&[9.0, 8.5, 8.0]).expect("first solve");
        let second = solver.solve(&[-2.0, 2.5, 1.5]).expect("second solve");
        assert_vector_close(&first, &[1.0, 2.0, 3.0]);
        assert_vector_close(&second, &[-1.0, 1.0, 1.0]);
    }

    #[test]
    fn reuses_a_compressed_large_system() {
        const SIZE: usize = 1025;
        let mut matrix = SparseMatrix::with_uniform_row_capacity(SIZE, 2);
        for index in 0..SIZE {
            add_at(&mut matrix, index, index, 2.0);
        }
        add_at(&mut matrix, 0, SIZE - 1, 0.25);
        add_at(&mut matrix, SIZE - 1, 0, 0.25);
        let solver = PreparedSpdSolver::factor(matrix).expect("large matrix should prepare");

        let mut rhs = vec![2.0; SIZE];
        rhs[0] += 0.25;
        rhs[SIZE - 1] += 0.25;
        let first = solver.solve(&rhs).expect("first iterative solve");
        let second = solver.solve(&rhs).expect("second iterative solve");
        assert_vector_close(&first, &vec![1.0; SIZE]);
        assert_vector_close(&second, &vec![1.0; SIZE]);
    }

    #[test]
    fn reuses_a_numbering_independent_path_factor() {
        let mut matrix = SparseMatrix::with_uniform_row_capacity(4, 3);
        for index in 0..4 {
            add_at(&mut matrix, index, index, 4.0);
        }
        for (first, second) in [(0, 2), (2, 1), (1, 3)] {
            add_at(&mut matrix, first, second, -1.0);
            add_at(&mut matrix, second, first, -1.0);
        }

        let solver = PreparedSpdSolver::factor(matrix).expect("permuted path should factor");
        assert!(matches!(
            &solver.backend,
            PreparedBackend::Tridiagonal { order: Some(_), .. }
        ));

        let first = solver.solve(&[2.0, 6.0, 4.0, 13.0]).expect("first solve");
        let second = solver.solve(&[3.0, 2.0, 2.0, 3.0]).expect("second solve");
        assert_vector_close(&first, &[1.0, 3.0, 2.0, 4.0]);
        assert_vector_close(&second, &[1.0; 4]);
    }

    fn assert_vector_close(actual: &[f64], expected: &[f64]) {
        assert_eq!(actual.len(), expected.len());
        for (actual, expected) in actual.iter().zip(expected) {
            assert!((actual - expected).abs() <= 1.0e-9);
        }
    }
}
