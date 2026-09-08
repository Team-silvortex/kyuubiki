use self::linear_ic0::IncompleteCholesky;
use crate::linear_dense::{DenseLu, zero_matrix};
use crate::linear_solver_profile::{SpdPreconditioner, SpdSolveOptions, SpdSolveProfile};
use crate::linear_spd::solve_spd_compressed;
use crate::solver_control::{SolverStage, check_cancellation, checkpoint, checkpoint_chunk};
use std::time::Instant;

const DENSE_REFINEMENT_TARGET: f64 = 1.0e-12;
const SPARSE_RESIDUAL_TOLERANCE: f64 = 1.0e-8;

#[cfg(test)]
#[path = "linear_sparse_product_tests.rs"]
mod product_tests;

#[path = "linear_sparse_compression.rs"]
mod compression;
#[path = "linear_ic0.rs"]
mod linear_ic0;
#[path = "linear_spd_prepared.rs"]
mod prepared;
#[path = "linear_sparse_product.rs"]
mod product;
#[path = "linear_sparse_reduction.rs"]
mod reduction;
#[path = "linear_sparse_residual.rs"]
mod residual;
#[path = "linear_algebra_scaling.rs"]
mod scaling;
#[path = "linear_sparse_path.rs"]
mod sparse_path;

pub(crate) use prepared::PreparedSpdSolver;
pub(crate) use reduction::{reduce_sparse_system, reduce_sparse_system_with_prescribed};
use residual::{sparse_relative_residual, sparse_residual_vector};
pub(crate) use residual::{sparse_residual_norm, stable_l2_norm};
pub(crate) use sparse_path::solve_tridiagonal_system;

#[derive(Debug, Clone)]
pub(crate) struct SparseMatrix {
    rows: Vec<Vec<(usize, f64)>>,
}

#[derive(Debug, Clone)]
pub(crate) struct CompressedSparseMatrix {
    max_row_entries: usize,
    pub(crate) row_offsets: Vec<usize>,
    pub(crate) lower_end_offsets: Vec<usize>,
    pub(crate) upper_start_offsets: Vec<usize>,
    pub(crate) columns: Vec<usize>,
    pub(crate) values: Vec<f64>,
    pub(crate) diagonal: Vec<f64>,
    incomplete_cholesky: Option<IncompleteCholesky>,
    inverse_diagonal: Vec<f64>,
}

impl SparseMatrix {
    pub(crate) fn new(size: usize) -> Self {
        Self {
            rows: vec![Vec::new(); size],
        }
    }

    pub(crate) fn with_uniform_row_capacity(size: usize, row_capacity: usize) -> Self {
        Self {
            rows: (0..size)
                .map(|_| Vec::with_capacity(row_capacity))
                .collect(),
        }
    }

    pub(crate) fn size(&self) -> usize {
        self.rows.len()
    }

    pub(crate) fn non_zero_count(&self) -> usize {
        self.rows.iter().map(Vec::len).sum()
    }

    pub(crate) fn row_entries(&self, row: usize) -> &[(usize, f64)] {
        &self.rows[row]
    }

    fn average_row_non_zero_hint(&self) -> usize {
        let size = self.size().max(1);
        self.non_zero_count().div_ceil(size).max(1)
    }

    fn add_at(&mut self, row: usize, column: usize, value: f64) {
        if value == 0.0 {
            return;
        }

        let row_entries = &mut self.rows[row];
        if row_entries.is_empty() {
            row_entries.push((column, value));
            return;
        }

        if let Some((last_column, last_value)) = row_entries.last_mut() {
            if *last_column == column {
                *last_value += value;
                if *last_value == 0.0 {
                    row_entries.pop();
                }
                return;
            }

            if *last_column < column {
                row_entries.push((column, value));
                return;
            }
        }

        match row_entries.binary_search_by_key(&column, |(entry_column, _)| *entry_column) {
            Ok(index) => {
                row_entries[index].1 += value;
                if row_entries[index].1 == 0.0 {
                    row_entries.remove(index);
                }
            }
            Err(index) => row_entries.insert(index, (column, value)),
        }
    }

    fn push_sorted_entry(&mut self, row: usize, column: usize, value: f64) {
        if value == 0.0 {
            return;
        }

        let row_entries = &mut self.rows[row];
        if let Some((last_column, last_value)) = row_entries.last_mut() {
            debug_assert!(
                *last_column <= column,
                "push_sorted_entry requires non-decreasing columns"
            );
            if *last_column == column {
                *last_value += value;
                if *last_value == 0.0 {
                    row_entries.pop();
                }
                return;
            }
        }

        row_entries.push((column, value));
    }

    fn diagonal_value(&self, row: usize) -> f64 {
        self.rows[row]
            .binary_search_by_key(&row, |(column, _)| *column)
            .ok()
            .map(|index| self.rows[row][index].1)
            .unwrap_or(0.0)
    }
}

impl CompressedSparseMatrix {
    pub(crate) fn size(&self) -> usize {
        self.diagonal.len()
    }

    pub(crate) fn non_zero_count(&self) -> usize {
        self.values.len()
    }

    fn diagonal(&self, index: usize) -> f64 {
        self.diagonal[index]
    }

    fn inverse_diagonal(&self, index: usize) -> f64 {
        self.inverse_diagonal[index]
    }

    pub(crate) fn apply_preconditioner_into(
        &self,
        kind: SpdPreconditioner,
        residual: &[f64],
        result: &mut [f64],
        workspace: &mut [f64],
    ) -> Result<(), String> {
        match kind {
            SpdPreconditioner::IncompleteCholesky => self
                .incomplete_cholesky
                .as_ref()
                .expect("IC(0) preconditioner must be prepared")
                .apply(residual, result, workspace),
            SpdPreconditioner::Jacobi => {
                checkpoint(SolverStage::PreconditionerJacobi, 0)?;
                for index in 0..self.size() {
                    result[index] = residual[index] * self.inverse_diagonal(index);
                    checkpoint_chunk(SolverStage::PreconditionerJacobi, index + 1, self.size())?;
                }
                Ok(())
            }
            SpdPreconditioner::SymmetricGaussSeidel => {
                self.apply_sgs_into(residual, result, workspace)
            }
        }
    }

    fn apply_sgs_into(
        &self,
        residual: &[f64],
        result: &mut [f64],
        forward: &mut [f64],
    ) -> Result<(), String> {
        checkpoint(SolverStage::SgsForward, 0)?;
        let size = self.size();
        debug_assert_eq!(forward.len(), size);
        for row in 0..size {
            let mut sum = residual[row];
            for index in self.row_offsets[row]..self.lower_end_offsets[row] {
                let column = self.columns[index];
                sum -= self.values[index] * forward[column];
            }
            forward[row] = sum * self.inverse_diagonal(row);
            checkpoint_chunk(SolverStage::SgsForward, row + 1, size)?;
        }

        checkpoint(SolverStage::SgsBackward, 0)?;
        for row in (0..size).rev() {
            let mut sum = self.diagonal(row) * forward[row];
            for index in self.upper_start_offsets[row]..self.row_offsets[row + 1] {
                let column = self.columns[index];
                sum -= self.values[index] * result[column];
            }
            result[row] = sum * self.inverse_diagonal(row);
            checkpoint_chunk(SolverStage::SgsBackward, size - row, size)?;
        }
        Ok(())
    }
}

pub(crate) trait MatrixAssembler {
    fn add_entry(&mut self, row: usize, column: usize, value: f64);
}

impl MatrixAssembler for [Vec<f64>] {
    fn add_entry(&mut self, row: usize, column: usize, value: f64) {
        self[row][column] += value;
    }
}

impl MatrixAssembler for Vec<Vec<f64>> {
    fn add_entry(&mut self, row: usize, column: usize, value: f64) {
        self[row][column] += value;
    }
}

impl MatrixAssembler for SparseMatrix {
    fn add_entry(&mut self, row: usize, column: usize, value: f64) {
        self.add_at(row, column, value);
    }
}

pub(crate) fn add_at<M: MatrixAssembler + ?Sized>(
    matrix: &mut M,
    row: usize,
    column: usize,
    value: f64,
) {
    matrix.add_entry(row, column, value);
}

pub(crate) fn solve_spd_system(matrix: &SparseMatrix, rhs: &[f64]) -> Result<Vec<f64>, String> {
    solve_spd_system_profile(matrix, rhs).map(|profile| profile.solution)
}

pub(crate) fn solve_spd_system_profile(
    matrix: &SparseMatrix,
    rhs: &[f64],
) -> Result<SpdSolveProfile, String> {
    solve_spd_system_profile_with_options(matrix, rhs, SpdSolveOptions::default())
}

pub(crate) fn solve_spd_system_profile_with_options(
    matrix: &SparseMatrix,
    rhs: &[f64],
    options: SpdSolveOptions,
) -> Result<SpdSolveProfile, String> {
    checkpoint(SolverStage::LinearPrepare, 0)?;
    let size = rhs.len();
    if matrix.size() != size {
        return Err("matrix dimensions do not match vector".to_string());
    }
    scaling::validate_sparse_system_finite(matrix, rhs)?;
    if size == 0 {
        return Ok(SpdSolveProfile {
            solution: Vec::new(),
            iterations: 0,
            matrix_non_zero_count: 0,
            residual_norm: 0.0,
            stages: Vec::new(),
        });
    }
    if size <= 1024 {
        let factor = DenseLu::factor(sparse_to_dense(matrix))?;
        let solution = factor.solve(rhs)?;
        let solution = refine_dense_solution(matrix, rhs, solution, &factor)?;
        return validate_spd_solution(
            matrix,
            rhs,
            SpdSolveProfile {
                solution,
                iterations: 0,
                matrix_non_zero_count: matrix.non_zero_count(),
                residual_norm: 0.0,
                stages: Vec::new(),
            },
        );
    }
    let scaling = scaling::diagonal_sparse_scaling(matrix);
    let scaled_rhs = scaling::scale_sparse_rhs(rhs, &scaling);
    let diagonal_scale = scaling::average_scaled_diagonal_magnitude(matrix, &scaling).max(1.0);
    let setup_started = Instant::now();
    let compressed = matrix.compress_scaled(&scaling, options.preconditioner)?;
    let setup_elapsed_ms = setup_started.elapsed().as_secs_f64() * 1000.0;
    checkpoint(SolverStage::LinearPrepare, 1)?;

    let scaled_profile = match solve_spd_compressed(&compressed, &scaled_rhs, matrix, &options) {
        Ok(profile) => profile,
        Err(error) => {
            check_cancellation()?;
            let scaled_matrix = scaling::scale_sparse_matrix(matrix, &scaling);
            let mut recovered = None;

            for factor in [1.0e-10, 1.0e-8, 1.0e-6] {
                check_cancellation()?;
                let regularized =
                    scaling::regularize_sparse_diagonal(&scaled_matrix, diagonal_scale * factor);
                let compressed_regularized = regularized.compress(options.preconditioner)?;

                if let Ok(profile) = solve_spd_compressed(
                    &compressed_regularized,
                    &scaled_rhs,
                    &regularized,
                    &options,
                ) {
                    recovered = Some(profile);
                    break;
                }
            }

            check_cancellation()?;
            recovered.ok_or(error)?
        }
    };
    let mut profile = scaling::unscale_profile(scaled_profile, &scaling);
    profile
        .stages
        .push(crate::linear_solver_profile::SpdSolveStage {
            label: "solve_spd_preconditioner_setup",
            elapsed_ms: setup_elapsed_ms,
        });
    validate_spd_solution(matrix, rhs, profile)
}

fn refine_dense_solution(
    matrix: &SparseMatrix,
    rhs: &[f64],
    mut solution: Vec<f64>,
    factor: &DenseLu,
) -> Result<Vec<f64>, String> {
    for _ in 0..2 {
        let validation = sparse_relative_residual(matrix, rhs, &solution, DENSE_REFINEMENT_TARGET)?;
        if validation.relative <= DENSE_REFINEMENT_TARGET {
            return Ok(solution);
        }
        let residual = sparse_residual_vector(matrix, rhs, &solution)?;
        let correction = factor.solve(&residual)?;
        for (value, correction) in solution.iter_mut().zip(correction) {
            *value += correction;
            if !value.is_finite() {
                return Err("dense linear refinement diverged".to_string());
            }
        }
    }
    Ok(solution)
}

fn validate_spd_solution(
    matrix: &SparseMatrix,
    rhs: &[f64],
    mut profile: SpdSolveProfile,
) -> Result<SpdSolveProfile, String> {
    profile.residual_norm = sparse_residual_norm(matrix, rhs, &profile.solution)?;
    let validation =
        sparse_relative_residual(matrix, rhs, &profile.solution, SPARSE_RESIDUAL_TOLERANCE)?;
    if !profile.residual_norm.is_finite() || !validation.relative.is_finite() {
        return Err("linear system solution produced a non-finite residual".to_string());
    }
    if validation.relative > SPARSE_RESIDUAL_TOLERANCE {
        return Err(format!(
            "linear system solution failed residual validation ({:.6e} at row {}, residual={:.6e}, equation_scale={:.6e})",
            validation.relative, validation.row, validation.residual, validation.equation_scale
        ));
    }
    Ok(profile)
}

pub(crate) fn safe_diagonal(value: f64) -> f64 {
    if value == 0.0 { 1.0 } else { value }
}

pub(crate) fn sparse_to_dense(matrix: &SparseMatrix) -> Vec<Vec<f64>> {
    let size = matrix.size();
    let mut dense = zero_matrix(size);
    for (row_index, row) in matrix.rows.iter().enumerate() {
        for &(column, value) in row {
            dense[row_index][column] = value;
        }
    }
    dense
}

#[cfg(test)]
mod tests {
    use super::{SparseMatrix, add_at, solve_tridiagonal_system, stable_l2_norm};

    #[test]
    fn stable_norm_propagates_non_finite_values() {
        assert!(stable_l2_norm([1.0, f64::NAN]).is_nan());
        assert_eq!(stable_l2_norm([1.0, f64::INFINITY]), f64::INFINITY);
    }

    #[test]
    fn solves_a_tridiagonal_sparse_system_in_linear_time_path() {
        let mut matrix = SparseMatrix::new(3);
        for (row, column, value) in [
            (0, 0, 2.0),
            (0, 1, -1.0),
            (1, 0, -1.0),
            (1, 1, 2.0),
            (1, 2, -1.0),
            (2, 1, -1.0),
            (2, 2, 2.0),
        ] {
            add_at(&mut matrix, row, column, value);
        }

        let solution = solve_tridiagonal_system(&matrix, &[1.0, 0.0, 1.0])
            .expect("tridiagonal matrix should use the chain path")
            .expect("tridiagonal system should solve");
        assert!(solution.iter().all(|value| (value - 1.0).abs() < 1.0e-12));
    }

    #[test]
    fn solves_a_numbering_independent_sparse_path() {
        let mut matrix = SparseMatrix::with_uniform_row_capacity(4, 3);
        for index in 0..4 {
            add_at(&mut matrix, index, index, 4.0);
        }
        for (first, second) in [(0, 2), (2, 1), (1, 3)] {
            add_at(&mut matrix, first, second, -1.0);
            add_at(&mut matrix, second, first, -1.0);
        }

        let solution = solve_tridiagonal_system(&matrix, &[2.0, 6.0, 4.0, 13.0])
            .expect("permuted path should use the tridiagonal backend")
            .expect("permuted tridiagonal system should solve");
        for (actual, expected) in solution.iter().zip([1.0, 3.0, 2.0, 4.0]) {
            assert!((actual - expected).abs() < 1.0e-12);
        }
    }

    #[test]
    fn declines_non_tridiagonal_sparse_systems() {
        let mut matrix = SparseMatrix::new(4);
        for row in 0..4 {
            add_at(&mut matrix, row, row, 4.0);
        }
        for leaf in 1..4 {
            add_at(&mut matrix, 0, leaf, -1.0);
            add_at(&mut matrix, leaf, 0, -1.0);
        }

        assert!(solve_tridiagonal_system(&matrix, &[1.0; 4]).is_none());
    }

    #[test]
    fn retains_and_solves_uniformly_tiny_sparse_coefficients() {
        let mut matrix = SparseMatrix::new(2);
        for (row, column, value) in [
            (0, 0, 2.0e-24),
            (0, 1, -1.0e-24),
            (1, 0, -1.0e-24),
            (1, 1, 2.0e-24),
        ] {
            add_at(&mut matrix, row, column, value);
        }

        assert_eq!(matrix.non_zero_count(), 4);
        let solution = solve_tridiagonal_system(&matrix, &[1.0e-24, 0.0])
            .expect("tiny matrix should retain its tridiagonal shape")
            .expect("tiny tridiagonal system should solve");
        assert!((solution[0] - 2.0 / 3.0).abs() < 1.0e-12);
        assert!((solution[1] - 1.0 / 3.0).abs() < 1.0e-12);
    }
}
