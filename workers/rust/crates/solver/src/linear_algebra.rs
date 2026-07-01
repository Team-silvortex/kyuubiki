use crate::linear_dense::{solve_linear_system, zero_matrix};
use crate::linear_solver_profile::{SpdPreconditioner, SpdSolveOptions, SpdSolveProfile};

#[derive(Debug, Clone)]
pub(crate) struct SparseMatrix {
    rows: Vec<Vec<(usize, f64)>>,
}

#[derive(Debug, Clone)]
struct CompressedSparseMatrix {
    row_offsets: Vec<usize>,
    columns: Vec<usize>,
    values: Vec<f64>,
    diagonal: Vec<f64>,
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

    fn size(&self) -> usize {
        self.rows.len()
    }

    fn non_zero_count(&self) -> usize {
        self.rows.iter().map(Vec::len).sum()
    }

    fn average_row_non_zero_hint(&self) -> usize {
        let size = self.size().max(1);
        self.non_zero_count().div_ceil(size).max(1)
    }

    fn add_at(&mut self, row: usize, column: usize, value: f64) {
        if value.abs() <= 1.0e-18 {
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
                if last_value.abs() <= 1.0e-18 {
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
                if row_entries[index].1.abs() <= 1.0e-18 {
                    row_entries.remove(index);
                }
            }
            Err(index) => row_entries.insert(index, (column, value)),
        }
    }

    fn push_sorted_entry(&mut self, row: usize, column: usize, value: f64) {
        if value.abs() <= 1.0e-18 {
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
                if last_value.abs() <= 1.0e-18 {
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

    fn compress(&self) -> CompressedSparseMatrix {
        let size = self.size();
        let mut row_offsets = Vec::with_capacity(size + 1);
        let mut columns = Vec::new();
        let mut values = Vec::new();
        let mut diagonal = vec![0.0; size];

        row_offsets.push(0);
        for (row_index, row) in self.rows.iter().enumerate() {
            for &(column, value) in row {
                if column == row_index {
                    diagonal[row_index] = value;
                }
                columns.push(column);
                values.push(value);
            }
            row_offsets.push(columns.len());
        }

        CompressedSparseMatrix {
            row_offsets,
            columns,
            values,
            diagonal,
        }
    }
}

impl CompressedSparseMatrix {
    fn size(&self) -> usize {
        self.diagonal.len()
    }

    fn diagonal(&self, index: usize) -> f64 {
        self.diagonal[index]
    }

    fn multiply_vector_into(&self, vector: &[f64], result: &mut [f64]) {
        debug_assert_eq!(result.len(), self.size());
        for row in 0..self.size() {
            let start = self.row_offsets[row];
            let end = self.row_offsets[row + 1];
            let mut sum = 0.0;
            for index in start..end {
                sum += self.values[index] * vector[self.columns[index]];
            }
            result[row] = sum;
        }
    }

    fn apply_preconditioner_into(
        &self,
        kind: SpdPreconditioner,
        residual: &[f64],
        result: &mut [f64],
    ) {
        match kind {
            SpdPreconditioner::Jacobi => {
                for index in 0..self.size() {
                    result[index] = residual[index] / safe_diagonal(self.diagonal(index));
                }
            }
            SpdPreconditioner::SymmetricGaussSeidel => self.apply_sgs_into(residual, result),
        }
    }

    fn apply_sgs_into(&self, residual: &[f64], result: &mut [f64]) {
        let size = self.size();
        let mut forward = vec![0.0; size];
        for row in 0..size {
            let mut sum = residual[row];
            for index in self.row_offsets[row]..self.row_offsets[row + 1] {
                let column = self.columns[index];
                if column < row {
                    sum -= self.values[index] * forward[column];
                }
            }
            forward[row] = sum / safe_diagonal(self.diagonal(row));
        }

        for row in (0..size).rev() {
            let mut sum = self.diagonal(row) * forward[row];
            for index in self.row_offsets[row]..self.row_offsets[row + 1] {
                let column = self.columns[index];
                if column > row {
                    sum -= self.values[index] * result[column];
                }
            }
            result[row] = sum / safe_diagonal(self.diagonal(row));
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
    if size == 0 {
        return Ok(SpdSolveProfile {
            solution: Vec::new(),
            iterations: 0,
            residual_norm: 0.0,
        });
    }
    if size <= 1024 {
        return solve_linear_system(sparse_to_dense(matrix), rhs.to_vec()).map(|solution| {
            SpdSolveProfile {
                solution,
                iterations: 0,
                residual_norm: 0.0,
            }
        });
    }
    let scaling = diagonal_sparse_scaling(matrix);
    let scaled_rhs = scale_sparse_rhs(rhs, &scaling);
    let scaled_matrix = scale_sparse_matrix(matrix, &scaling);
    let diagonal_scale = average_diagonal_magnitude(&scaled_matrix).max(1.0);
    let compressed = scaled_matrix.compress();
    drop(scaled_matrix);

    match solve_spd_compressed(&compressed, &scaled_rhs, matrix, options.preconditioner) {
        Ok(profile) => Ok(profile),
        Err(error) => {
            let scaled_matrix = scale_sparse_matrix(matrix, &scaling);

            for factor in [1.0e-10, 1.0e-8, 1.0e-6] {
                let regularized =
                    regularize_sparse_diagonal(&scaled_matrix, diagonal_scale * factor);
                let compressed_regularized = regularized.compress();

                if let Ok(profile) = solve_spd_compressed(
                    &compressed_regularized,
                    &scaled_rhs,
                    &regularized,
                    options.preconditioner,
                ) {
                    return Ok(unscale_profile(profile, &scaling));
                }
            }

            Err(error)
        }
    }
    .map(|profile| unscale_profile(profile, &scaling))
}

fn solve_spd_compressed(
    matrix: &CompressedSparseMatrix,
    rhs: &[f64],
    fallback_source: &SparseMatrix,
    preconditioner: SpdPreconditioner,
) -> Result<SpdSolveProfile, String> {
    let size = rhs.len();

    let mut x = vec![0.0; size];
    let mut r = rhs.to_vec();
    let mut z = vec![0.0; size];
    let mut p = vec![0.0; size];
    let mut ap = vec![0.0; size];
    let mut ax = vec![0.0; size];
    matrix.apply_preconditioner_into(preconditioner, &r, &mut z);
    p.clone_from(&z);

    let rhs_norm = dot(rhs, rhs).sqrt().max(1.0);
    let tolerance = 1.0e-9 * rhs_norm;
    let mut rz_old = dot(&r, &z);
    if !rz_old.is_finite() || rz_old.abs() < 1.0e-20 {
        return Ok(SpdSolveProfile {
            solution: x,
            iterations: 0,
            residual_norm: dot(&r, &r).sqrt(),
        });
    }

    let max_iter = (size.saturating_mul(8)).clamp(256, 40_000);

    for iteration in 0..max_iter {
        matrix.multiply_vector_into(&p, &mut ap);
        let mut denom = dot(&p, &ap);
        if !denom.is_finite() || denom.abs() < 1.0e-20 {
            p.clone_from(&z);
            matrix.multiply_vector_into(&p, &mut ap);
            denom = dot(&p, &ap);
            if !denom.is_finite() || denom.abs() < 1.0e-20 {
                return solve_spd_fallback(fallback_source, rhs, "system is singular");
            }
        }

        let alpha = rz_old / denom;
        for index in 0..size {
            x[index] += alpha * p[index];
            r[index] -= alpha * ap[index];
        }

        if iteration % 32 == 31 {
            matrix.multiply_vector_into(&x, &mut ax);
            for index in 0..size {
                r[index] = rhs[index] - ax[index];
            }
        }

        let residual_norm = dot(&r, &r).sqrt();
        if residual_norm <= tolerance {
            return Ok(SpdSolveProfile {
                solution: x,
                iterations: iteration + 1,
                residual_norm,
            });
        }

        matrix.apply_preconditioner_into(preconditioner, &r, &mut z);

        let rz_new = dot(&r, &z);
        if !rz_new.is_finite() {
            return solve_spd_fallback(fallback_source, rhs, "iterative solver diverged");
        }
        let beta = if rz_old.abs() < 1.0e-20 {
            0.0
        } else {
            rz_new / rz_old
        };
        for index in 0..size {
            p[index] = z[index] + beta * p[index];
        }
        rz_old = rz_new;
    }

    solve_spd_fallback(fallback_source, rhs, "iterative solver did not converge")
}

fn dot(lhs: &[f64], rhs: &[f64]) -> f64 {
    lhs.iter().zip(rhs.iter()).map(|(a, b)| a * b).sum()
}

fn safe_diagonal(value: f64) -> f64 {
    if value.abs() < 1.0e-12 {
        1.0e-12
    } else {
        value
    }
}

fn solve_spd_fallback(
    matrix: &SparseMatrix,
    rhs: &[f64],
    reason: &str,
) -> Result<SpdSolveProfile, String> {
    if rhs.len() <= 1024 {
        solve_linear_system(sparse_to_dense(matrix), rhs.to_vec()).map(|solution| SpdSolveProfile {
            solution,
            iterations: 0,
            residual_norm: 0.0,
        })
    } else {
        Err(reason.to_string())
    }
}

fn sparse_to_dense(matrix: &SparseMatrix) -> Vec<Vec<f64>> {
    let size = matrix.size();
    let mut dense = zero_matrix(size);
    for (row_index, row) in matrix.rows.iter().enumerate() {
        for &(column, value) in row {
            dense[row_index][column] = value;
        }
    }
    dense
}

fn diagonal_sparse_scaling(matrix: &SparseMatrix) -> Vec<f64> {
    let size = matrix.size();
    let mut scaling = vec![1.0; size];

    for (index, row) in matrix.rows.iter().enumerate() {
        let diagonal = row
            .iter()
            .find_map(|(column, value)| (*column == index).then_some(*value))
            .unwrap_or(0.0)
            .abs();
        scaling[index] = if diagonal > 1.0e-12 {
            diagonal.sqrt().recip()
        } else {
            1.0
        };
    }

    scaling
}

fn scale_sparse_matrix(matrix: &SparseMatrix, scaling: &[f64]) -> SparseMatrix {
    let size = matrix.size();
    let mut scaled =
        SparseMatrix::with_uniform_row_capacity(size, matrix.average_row_non_zero_hint());
    for (row_index, row) in matrix.rows.iter().enumerate() {
        let row_scale = scaling[row_index];
        for &(column, value) in row {
            scaled.push_sorted_entry(row_index, column, value * row_scale * scaling[column]);
        }
    }
    scaled
}

fn scale_sparse_rhs(rhs: &[f64], scaling: &[f64]) -> Vec<f64> {
    rhs.iter()
        .enumerate()
        .map(|(index, value)| value * scaling[index])
        .collect()
}

fn unscale_solution(solution: &[f64], scaling: &[f64]) -> Vec<f64> {
    solution
        .iter()
        .enumerate()
        .map(|(index, value)| value * scaling[index])
        .collect()
}

fn unscale_profile(profile: SpdSolveProfile, scaling: &[f64]) -> SpdSolveProfile {
    SpdSolveProfile {
        solution: unscale_solution(&profile.solution, scaling),
        iterations: profile.iterations,
        residual_norm: profile.residual_norm,
    }
}

fn average_diagonal_magnitude(matrix: &SparseMatrix) -> f64 {
    let size = matrix.size().max(1);
    let diagonal_sum = matrix
        .rows
        .iter()
        .enumerate()
        .map(|(index, _)| matrix.diagonal_value(index).abs())
        .sum::<f64>();

    diagonal_sum / size as f64
}

fn regularize_sparse_diagonal(matrix: &SparseMatrix, epsilon: f64) -> SparseMatrix {
    let mut regularized = SparseMatrix::with_uniform_row_capacity(
        matrix.size(),
        matrix.average_row_non_zero_hint() + 1,
    );

    for (row_index, row) in matrix.rows.iter().enumerate() {
        regularized.rows[row_index].extend(row.iter().copied());
    }

    for row in 0..regularized.size() {
        regularized.add_at(row, row, epsilon);
    }

    regularized
}
