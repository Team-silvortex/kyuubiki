use crate::linear_solver_profile::SpdSolveProfile;

use super::SparseMatrix;

pub(super) fn diagonal_sparse_scaling(matrix: &SparseMatrix) -> Vec<f64> {
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

pub(super) fn scale_sparse_matrix(matrix: &SparseMatrix, scaling: &[f64]) -> SparseMatrix {
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

pub(super) fn scale_sparse_rhs(rhs: &[f64], scaling: &[f64]) -> Vec<f64> {
    rhs.iter()
        .enumerate()
        .map(|(index, value)| value * scaling[index])
        .collect()
}

pub(super) fn unscale_profile(profile: SpdSolveProfile, scaling: &[f64]) -> SpdSolveProfile {
    SpdSolveProfile {
        solution: unscale_solution(&profile.solution, scaling),
        iterations: profile.iterations,
        matrix_non_zero_count: profile.matrix_non_zero_count,
        residual_norm: profile.residual_norm,
        stages: profile.stages,
    }
}

pub(super) fn average_scaled_diagonal_magnitude(matrix: &SparseMatrix, scaling: &[f64]) -> f64 {
    let size = matrix.size().max(1);
    let diagonal_sum = matrix
        .rows
        .iter()
        .enumerate()
        .map(|(index, _)| matrix.diagonal_value(index).abs() * scaling[index] * scaling[index])
        .sum::<f64>();

    diagonal_sum / size as f64
}

pub(super) fn regularize_sparse_diagonal(matrix: &SparseMatrix, epsilon: f64) -> SparseMatrix {
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

pub(super) fn validate_sparse_system_finite(
    matrix: &SparseMatrix,
    rhs: &[f64],
) -> Result<(), String> {
    if rhs.iter().any(|value| !value.is_finite()) {
        return Err("linear system vector contains non-finite value".to_string());
    }
    if matrix
        .rows
        .iter()
        .flatten()
        .any(|(_, value)| !value.is_finite())
    {
        return Err("linear system matrix contains non-finite value".to_string());
    }
    Ok(())
}

fn unscale_solution(solution: &[f64], scaling: &[f64]) -> Vec<f64> {
    solution
        .iter()
        .enumerate()
        .map(|(index, value)| value * scaling[index])
        .collect()
}
