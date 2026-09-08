//! Pre-cancellation loops retained only for numerical and paired performance tests.
use super::SparseMatrix;

pub(super) fn diagonal_scaling(matrix: &SparseMatrix) -> Vec<f64> {
    let mut scaling = vec![1.0; matrix.size()];
    for (index, row) in matrix.rows.iter().enumerate() {
        let diagonal = row
            .iter()
            .find_map(|(column, value)| (*column == index).then_some(*value))
            .unwrap_or(0.0)
            .abs();
        scaling[index] = if diagonal > 0.0 {
            diagonal.sqrt().recip()
        } else {
            1.0
        };
    }
    scaling
}

pub(super) fn scale_matrix(matrix: &SparseMatrix, scaling: &[f64]) -> SparseMatrix {
    let mut scaled =
        SparseMatrix::with_uniform_row_capacity(matrix.size(), matrix.average_row_non_zero_hint());
    for (row_index, row) in matrix.rows.iter().enumerate() {
        let row_scale = scaling[row_index];
        for &(column, value) in row {
            scaled.push_sorted_entry(row_index, column, value * row_scale * scaling[column]);
        }
    }
    scaled
}

pub(super) fn scale_vector(values: &[f64], scaling: &[f64]) -> Vec<f64> {
    values
        .iter()
        .enumerate()
        .map(|(index, value)| value * scaling[index])
        .collect()
}

pub(super) fn diagonal_magnitude(matrix: &SparseMatrix, scaling: &[f64]) -> f64 {
    let sum = matrix
        .rows
        .iter()
        .enumerate()
        .map(|(index, _)| matrix.diagonal_value(index).abs() * scaling[index] * scaling[index])
        .sum::<f64>();
    sum / matrix.size().max(1) as f64
}

pub(super) fn regularize(matrix: &SparseMatrix, epsilon: f64) -> SparseMatrix {
    let mut result = SparseMatrix::with_uniform_row_capacity(
        matrix.size(),
        matrix.average_row_non_zero_hint() + 1,
    );
    for (index, row) in matrix.rows.iter().enumerate() {
        result.rows[index].extend(row.iter().copied());
    }
    for row in 0..result.size() {
        result.add_at(row, row, epsilon);
    }
    result
}

pub(super) fn validate(matrix: &SparseMatrix, rhs: &[f64]) -> Result<(), String> {
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
