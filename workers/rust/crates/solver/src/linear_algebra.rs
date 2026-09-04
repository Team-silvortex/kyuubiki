use self::linear_ic0::IncompleteCholesky;
use crate::chain_tridiagonal::solve_tridiagonal;
use crate::linear_dense::{DenseLu, zero_matrix};
use crate::linear_solver_profile::{SpdPreconditioner, SpdSolveOptions, SpdSolveProfile};
use crate::linear_spd::solve_spd_compressed;
use std::time::Instant;

const DENSE_REFINEMENT_TARGET: f64 = 1.0e-12;
const SPARSE_RESIDUAL_TOLERANCE: f64 = 1.0e-8;

#[path = "linear_ic0.rs"]
mod linear_ic0;
#[path = "linear_spd_prepared.rs"]
mod prepared;
#[path = "linear_algebra_scaling.rs"]
mod scaling;

pub(crate) use prepared::PreparedSpdSolver;

#[derive(Debug, Clone)]
pub(crate) struct SparseMatrix {
    rows: Vec<Vec<(usize, f64)>>,
}

#[derive(Debug, Clone)]
pub(crate) struct CompressedSparseMatrix {
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

    pub(crate) fn compress(&self, preconditioner: SpdPreconditioner) -> CompressedSparseMatrix {
        let size = self.size();
        let mut row_offsets = Vec::with_capacity(size + 1);
        let mut lower_end_offsets = Vec::with_capacity(size);
        let mut upper_start_offsets = Vec::with_capacity(size);
        let mut columns = Vec::new();
        let mut values = Vec::new();
        let mut diagonal = vec![0.0; size];

        row_offsets.push(0);
        for (row_index, row) in self.rows.iter().enumerate() {
            let row_start = columns.len();
            lower_end_offsets
                .push(row_start + row.partition_point(|(column, _)| *column < row_index));
            upper_start_offsets
                .push(row_start + row.partition_point(|(column, _)| *column <= row_index));
            for &(column, value) in row {
                if column == row_index {
                    diagonal[row_index] = value;
                }
                columns.push(column);
                values.push(value);
            }
            row_offsets.push(columns.len());
        }

        let inverse_diagonal: Vec<f64> = diagonal
            .iter()
            .map(|value| safe_diagonal(*value).recip())
            .collect();
        let incomplete_cholesky = matches!(preconditioner, SpdPreconditioner::IncompleteCholesky)
            .then(|| {
                IncompleteCholesky::build(
                    &row_offsets,
                    &lower_end_offsets,
                    &columns,
                    &values,
                    &diagonal,
                )
            });
        CompressedSparseMatrix {
            row_offsets,
            lower_end_offsets,
            upper_start_offsets,
            columns,
            values,
            diagonal,
            incomplete_cholesky,
            inverse_diagonal,
        }
    }

    fn compress_scaled(
        &self,
        scaling: &[f64],
        preconditioner: SpdPreconditioner,
    ) -> CompressedSparseMatrix {
        let size = self.size();
        debug_assert_eq!(scaling.len(), size);

        let mut row_offsets = Vec::with_capacity(size + 1);
        let mut lower_end_offsets = Vec::with_capacity(size);
        let mut upper_start_offsets = Vec::with_capacity(size);
        let non_zero_hint = self.non_zero_count();
        let mut columns = Vec::with_capacity(non_zero_hint);
        let mut values = Vec::with_capacity(non_zero_hint);
        let mut diagonal = vec![0.0; size];

        row_offsets.push(0);
        for (row_index, row) in self.rows.iter().enumerate() {
            let row_start = columns.len();
            lower_end_offsets
                .push(row_start + row.partition_point(|(column, _)| *column < row_index));
            upper_start_offsets
                .push(row_start + row.partition_point(|(column, _)| *column <= row_index));

            let row_scale = scaling[row_index];
            for &(column, value) in row {
                let scaled_value = value * row_scale * scaling[column];
                if column == row_index {
                    diagonal[row_index] = scaled_value;
                }
                columns.push(column);
                values.push(scaled_value);
            }
            row_offsets.push(columns.len());
        }

        let inverse_diagonal: Vec<f64> = diagonal
            .iter()
            .map(|value| safe_diagonal(*value).recip())
            .collect();
        let incomplete_cholesky = matches!(preconditioner, SpdPreconditioner::IncompleteCholesky)
            .then(|| {
                IncompleteCholesky::build(
                    &row_offsets,
                    &lower_end_offsets,
                    &columns,
                    &values,
                    &diagonal,
                )
            });
        CompressedSparseMatrix {
            row_offsets,
            lower_end_offsets,
            upper_start_offsets,
            columns,
            values,
            diagonal,
            incomplete_cholesky,
            inverse_diagonal,
        }
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

    pub(crate) fn multiply_vector_into(&self, vector: &[f64], result: &mut [f64]) {
        debug_assert_eq!(result.len(), self.size());
        for (row, result_value) in result.iter_mut().enumerate() {
            let start = self.row_offsets[row];
            let end = self.row_offsets[row + 1];
            let mut sum = 0.0;
            let columns = &self.columns[start..end];
            let values = &self.values[start..end];
            for (&column, &value) in columns.iter().zip(values.iter()) {
                sum += value * vector[column];
            }
            *result_value = sum;
        }
    }

    pub(crate) fn apply_preconditioner_into(
        &self,
        kind: SpdPreconditioner,
        residual: &[f64],
        result: &mut [f64],
        workspace: &mut [f64],
    ) {
        match kind {
            SpdPreconditioner::IncompleteCholesky => self
                .incomplete_cholesky
                .as_ref()
                .expect("IC(0) preconditioner must be prepared")
                .apply(residual, result, workspace),
            SpdPreconditioner::Jacobi => {
                for index in 0..self.size() {
                    result[index] = residual[index] * self.inverse_diagonal(index);
                }
            }
            SpdPreconditioner::SymmetricGaussSeidel => {
                self.apply_sgs_into(residual, result, workspace)
            }
        }
    }

    fn apply_sgs_into(&self, residual: &[f64], result: &mut [f64], forward: &mut [f64]) {
        let size = self.size();
        debug_assert_eq!(forward.len(), size);
        for row in 0..size {
            let mut sum = residual[row];
            for index in self.row_offsets[row]..self.lower_end_offsets[row] {
                let column = self.columns[index];
                sum -= self.values[index] * forward[column];
            }
            forward[row] = sum * self.inverse_diagonal(row);
        }

        for row in (0..size).rev() {
            let mut sum = self.diagonal(row) * forward[row];
            for index in self.upper_start_offsets[row]..self.row_offsets[row + 1] {
                let column = self.columns[index];
                sum -= self.values[index] * result[column];
            }
            result[row] = sum * self.inverse_diagonal(row);
        }
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

pub(crate) fn reduce_sparse_system(
    matrix: &SparseMatrix,
    force: &[f64],
    constrained: &[usize],
) -> (SparseMatrix, Vec<f64>, Vec<usize>) {
    let size = force.len();
    let mut is_constrained = vec![false; size];
    for &dof in constrained {
        if dof < size {
            is_constrained[dof] = true;
        }
    }

    let free = (0..size)
        .filter(|index| !is_constrained[*index])
        .collect::<Vec<_>>();
    let mut free_map = vec![usize::MAX; size];
    for (reduced, &global) in free.iter().enumerate() {
        free_map[global] = reduced;
    }

    let mut reduced =
        SparseMatrix::with_uniform_row_capacity(free.len(), matrix.average_row_non_zero_hint());
    let mut reduced_force = vec![0.0; free.len()];

    for (reduced_row, &global_row) in free.iter().enumerate() {
        reduced_force[reduced_row] = force[global_row];
        for &(global_col, value) in &matrix.rows[global_row] {
            let reduced_col = free_map[global_col];
            if reduced_col != usize::MAX {
                reduced.push_sorted_entry(reduced_row, reduced_col, value);
            }
        }
    }

    (reduced, reduced_force, free)
}

pub(crate) fn reduce_sparse_system_with_prescribed(
    matrix: &SparseMatrix,
    force: &[f64],
    prescribed: &[(usize, f64)],
) -> (SparseMatrix, Vec<f64>, Vec<usize>) {
    let size = force.len();
    let mut prescribed_values = vec![None; size];
    for &(dof, value) in prescribed {
        if dof < size {
            prescribed_values[dof] = Some(value);
        }
    }

    let free = (0..size)
        .filter(|index| prescribed_values[*index].is_none())
        .collect::<Vec<_>>();
    let mut free_map = vec![usize::MAX; size];
    for (reduced, &global) in free.iter().enumerate() {
        free_map[global] = reduced;
    }

    let mut reduced =
        SparseMatrix::with_uniform_row_capacity(free.len(), matrix.average_row_non_zero_hint());
    let mut reduced_force = vec![0.0; free.len()];

    for (reduced_row, &global_row) in free.iter().enumerate() {
        let mut rhs = force[global_row];
        for &(global_col, value) in &matrix.rows[global_row] {
            if let Some(prescribed_value) = prescribed_values[global_col] {
                rhs -= value * prescribed_value;
            } else {
                let reduced_col = free_map[global_col];
                if reduced_col != usize::MAX {
                    reduced.push_sorted_entry(reduced_row, reduced_col, value);
                }
            }
        }
        reduced_force[reduced_row] = rhs;
    }

    (reduced, reduced_force, free)
}

pub(crate) fn solve_spd_system(matrix: &SparseMatrix, rhs: &[f64]) -> Result<Vec<f64>, String> {
    solve_spd_system_profile(matrix, rhs).map(|profile| profile.solution)
}

/// Solves a symmetric tridiagonal system in linear time when its sparse shape
/// proves it is safe to do so. Callers can retain the generic SPD solver for
/// arbitrary meshes by treating `None` as "not a chain".
pub(crate) fn solve_tridiagonal_system(
    matrix: &SparseMatrix,
    rhs: &[f64],
) -> Option<Result<Vec<f64>, String>> {
    if matrix.size() != rhs.len() {
        return Some(Err("tridiagonal system dimensions must match".to_string()));
    }
    if rhs.is_empty() {
        return Some(Ok(Vec::new()));
    }

    let size = rhs.len();
    let mut lower = vec![0.0; size.saturating_sub(1)];
    let mut diagonal = vec![0.0; size];
    let mut upper = vec![0.0; size.saturating_sub(1)];
    for (row_index, row) in matrix.rows.iter().enumerate() {
        for &(column, value) in row {
            if column + 1 < row_index || column > row_index + 1 {
                return None;
            }
            if column == row_index {
                diagonal[row_index] = value;
            } else if column < row_index {
                lower[row_index - 1] = value;
            } else {
                upper[row_index] = value;
            }
        }
    }
    Some(solve_tridiagonal(&diagonal, &lower, &upper, rhs))
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
    let compressed = matrix.compress_scaled(&scaling, options.preconditioner);
    let setup_elapsed_ms = setup_started.elapsed().as_secs_f64() * 1000.0;

    let scaled_profile = match solve_spd_compressed(&compressed, &scaled_rhs, matrix, &options) {
        Ok(profile) => profile,
        Err(error) => {
            let scaled_matrix = scaling::scale_sparse_matrix(matrix, &scaling);
            let mut recovered = None;

            for factor in [1.0e-10, 1.0e-8, 1.0e-6] {
                let regularized =
                    scaling::regularize_sparse_diagonal(&scaled_matrix, diagonal_scale * factor);
                let compressed_regularized = regularized.compress(options.preconditioner);

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
        let validation = sparse_relative_residual(matrix, rhs, &solution, DENSE_REFINEMENT_TARGET);
        if validation.relative <= DENSE_REFINEMENT_TARGET {
            return Ok(solution);
        }
        let residual = sparse_residual_vector(matrix, rhs, &solution);
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
    profile.residual_norm = sparse_residual_norm(matrix, rhs, &profile.solution);
    let validation =
        sparse_relative_residual(matrix, rhs, &profile.solution, SPARSE_RESIDUAL_TOLERANCE);
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

pub(crate) fn sparse_residual_norm(matrix: &SparseMatrix, rhs: &[f64], solution: &[f64]) -> f64 {
    let residuals = matrix.rows.iter().zip(rhs).map(|(row, expected)| {
        expected
            - row
                .iter()
                .map(|(column, value)| value * solution[*column])
                .sum::<f64>()
    });
    stable_l2_norm(residuals)
}

fn sparse_residual_vector(matrix: &SparseMatrix, rhs: &[f64], solution: &[f64]) -> Vec<f64> {
    matrix
        .rows
        .iter()
        .zip(rhs)
        .map(|(row, expected)| {
            expected
                - row
                    .iter()
                    .map(|(column, value)| value * solution[*column])
                    .sum::<f64>()
        })
        .collect()
}

pub(crate) fn stable_l2_norm(values: impl IntoIterator<Item = f64>) -> f64 {
    let mut scale = 0.0;
    let mut sum_squares = 1.0;
    for value in values.into_iter().map(f64::abs) {
        if !value.is_finite() {
            return value;
        }
        if value == 0.0 {
            continue;
        }
        if scale < value {
            sum_squares = 1.0 + sum_squares * (scale / value).powi(2);
            scale = value;
        } else {
            sum_squares += (value / scale).powi(2);
        }
    }
    if scale == 0.0 {
        0.0
    } else {
        scale * sum_squares.sqrt()
    }
}

struct SparseResidualValidation {
    relative: f64,
    row: usize,
    residual: f64,
    equation_scale: f64,
}

fn sparse_relative_residual(
    matrix: &SparseMatrix,
    rhs: &[f64],
    solution: &[f64],
    tolerance: f64,
) -> SparseResidualValidation {
    let mut rows = Vec::with_capacity(matrix.size());
    let mut global_scale = 0.0_f64;
    for (row, expected) in matrix.rows.iter().zip(rhs) {
        let mut actual = 0.0;
        let mut equation_scale = expected.abs();
        for (column, value) in row {
            let term = value * solution[*column];
            actual += term;
            equation_scale += term.abs();
        }
        let residual = (expected - actual).abs();
        global_scale = global_scale.max(equation_scale);
        rows.push((residual, equation_scale));
    }

    let roundoff_floor = global_scale * f64::EPSILON / tolerance;
    let mut worst = SparseResidualValidation {
        relative: 0.0,
        row: 0,
        residual: 0.0,
        equation_scale: 0.0,
    };
    for (row_index, (residual, equation_scale)) in rows.into_iter().enumerate() {
        let effective_scale = equation_scale.max(roundoff_floor);
        let relative = if effective_scale == 0.0 {
            if residual == 0.0 { 0.0 } else { f64::INFINITY }
        } else {
            residual / effective_scale
        };
        if relative > worst.relative {
            worst = SparseResidualValidation {
                relative,
                row: row_index,
                residual,
                equation_scale: effective_scale,
            };
        }
    }
    worst
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
    fn declines_non_tridiagonal_sparse_systems() {
        let mut matrix = SparseMatrix::new(3);
        for row in 0..3 {
            add_at(&mut matrix, row, row, 2.0);
        }
        add_at(&mut matrix, 0, 2, -1.0);
        add_at(&mut matrix, 2, 0, -1.0);

        assert!(solve_tridiagonal_system(&matrix, &[1.0, 0.0, 1.0]).is_none());
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
