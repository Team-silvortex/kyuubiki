use crate::linear_algebra::{SparseMatrix, sparse_to_dense};
use kyuubiki_protocol::Frame2dTangentStability;

pub(crate) const MAX_DENSE_INERTIA_DOFS: usize = 256;
const PIVOT_TOLERANCE: f64 = 1.0e-10;

pub(crate) struct SymmetricInertia {
    pub(crate) stability: Frame2dTangentStability,
    pub(crate) negative_pivots: Option<usize>,
    pub(crate) near_zero_pivots: Option<usize>,
}

pub(crate) fn assess_symmetric_inertia(matrix: &SparseMatrix) -> SymmetricInertia {
    if matrix.size() > MAX_DENSE_INERTIA_DOFS {
        return unassessed_size_limit();
    }

    let mut dense = sparse_to_dense(matrix);
    if dense.iter().flatten().any(|value| !value.is_finite()) {
        return unknown_near_singular();
    }
    let scale = dense
        .iter()
        .flatten()
        .map(|value| value.abs())
        .fold(0.0_f64, f64::max);
    if scale == 0.0 {
        return SymmetricInertia {
            stability: Frame2dTangentStability::NearSingular,
            negative_pivots: Some(0),
            near_zero_pivots: Some(matrix.size()),
        };
    }
    for value in dense.iter_mut().flatten() {
        *value /= scale;
    }

    let (negative, near_zero) = factor_inertia(&mut dense);
    SymmetricInertia {
        stability: if near_zero > 0 {
            Frame2dTangentStability::NearSingular
        } else if negative > 0 {
            Frame2dTangentStability::Indefinite
        } else {
            Frame2dTangentStability::PositiveDefinite
        },
        negative_pivots: Some(negative),
        near_zero_pivots: Some(near_zero),
    }
}

fn factor_inertia(matrix: &mut [Vec<f64>]) -> (usize, usize) {
    let mut pivot = 0;
    let mut negative = 0;
    let mut near_zero = 0;
    while pivot < matrix.len() {
        let (diagonal_index, diagonal_magnitude) = largest_diagonal(matrix, pivot);
        let off_diagonal = largest_off_diagonal(matrix, pivot);
        if off_diagonal.is_none_or(|(_, _, magnitude)| diagonal_magnitude >= 0.5 * magnitude) {
            symmetric_swap(matrix, pivot, diagonal_index);
            let value = matrix[pivot][pivot];
            count_value(value, &mut negative, &mut near_zero);
            if value.abs() > PIVOT_TOLERANCE {
                schur_one_by_one(matrix, pivot, value);
            }
            pivot += 1;
        } else {
            let (first, second, _) = off_diagonal.unwrap();
            symmetric_swap(matrix, pivot, first);
            let relocated_second = if second == pivot { first } else { second };
            symmetric_swap(matrix, pivot + 1, relocated_second);
            let a = matrix[pivot][pivot];
            let b = matrix[pivot][pivot + 1];
            let c = matrix[pivot + 1][pivot + 1];
            count_two_by_two(a, b, c, &mut negative, &mut near_zero);
            let determinant = a * c - b * b;
            if determinant.abs() > PIVOT_TOLERANCE.powi(2) {
                schur_two_by_two(matrix, pivot, a, b, c, determinant);
            }
            pivot += 2;
        }
    }
    (negative, near_zero)
}

fn largest_diagonal(matrix: &[Vec<f64>], start: usize) -> (usize, f64) {
    (start..matrix.len())
        .map(|index| (index, matrix[index][index].abs()))
        .max_by(|left, right| left.1.total_cmp(&right.1))
        .unwrap_or((start, 0.0))
}

fn largest_off_diagonal(matrix: &[Vec<f64>], start: usize) -> Option<(usize, usize, f64)> {
    let mut largest = None;
    for row in start..matrix.len() {
        for column in row + 1..matrix.len() {
            let candidate = (row, column, matrix[row][column].abs());
            if largest.is_none_or(|current: (usize, usize, f64)| candidate.2 > current.2) {
                largest = Some(candidate);
            }
        }
    }
    largest.filter(|(_, _, magnitude)| *magnitude > PIVOT_TOLERANCE)
}

fn symmetric_swap(matrix: &mut [Vec<f64>], left: usize, right: usize) {
    if left == right {
        return;
    }
    matrix.swap(left, right);
    for row in matrix {
        row.swap(left, right);
    }
}

fn schur_one_by_one(matrix: &mut [Vec<f64>], pivot: usize, value: f64) {
    for row in pivot + 1..matrix.len() {
        for column in row..matrix.len() {
            let updated = matrix[row][column] - matrix[row][pivot] * matrix[column][pivot] / value;
            matrix[row][column] = updated;
            matrix[column][row] = updated;
        }
    }
}

fn schur_two_by_two(
    matrix: &mut [Vec<f64>],
    pivot: usize,
    a: f64,
    b: f64,
    c: f64,
    determinant: f64,
) {
    for row in pivot + 2..matrix.len() {
        let row_a = matrix[row][pivot];
        let row_b = matrix[row][pivot + 1];
        for column in row..matrix.len() {
            let column_a = matrix[column][pivot];
            let column_b = matrix[column][pivot + 1];
            let correction = (row_a * (c * column_a - b * column_b)
                + row_b * (a * column_b - b * column_a))
                / determinant;
            let updated = matrix[row][column] - correction;
            matrix[row][column] = updated;
            matrix[column][row] = updated;
        }
    }
}

fn count_two_by_two(a: f64, b: f64, c: f64, negative: &mut usize, near_zero: &mut usize) {
    let center = 0.5 * (a + c);
    let radius = (0.25 * (a - c).powi(2) + b * b).sqrt();
    count_value(center - radius, negative, near_zero);
    count_value(center + radius, negative, near_zero);
}

fn count_value(value: f64, negative: &mut usize, near_zero: &mut usize) {
    if value.abs() <= PIVOT_TOLERANCE {
        *near_zero += 1;
    } else if value < 0.0 {
        *negative += 1;
    }
}

fn unassessed_size_limit() -> SymmetricInertia {
    SymmetricInertia {
        stability: Frame2dTangentStability::UnassessedSizeLimit,
        negative_pivots: None,
        near_zero_pivots: None,
    }
}

fn unknown_near_singular() -> SymmetricInertia {
    SymmetricInertia {
        stability: Frame2dTangentStability::NearSingular,
        negative_pivots: None,
        near_zero_pivots: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linear_algebra::{SparseMatrix, add_at};

    fn sparse(values: &[&[f64]]) -> SparseMatrix {
        let mut matrix = SparseMatrix::new(values.len());
        for (row, entries) in values.iter().enumerate() {
            for (column, value) in entries.iter().copied().enumerate() {
                add_at(&mut matrix, row, column, value);
            }
        }
        matrix
    }

    #[test]
    fn positive_definite_matrix_has_no_negative_direction() {
        let result = assess_symmetric_inertia(&sparse(&[
            &[2.0, -1.0, 0.0],
            &[-1.0, 2.0, -1.0],
            &[0.0, -1.0, 2.0],
        ]));
        assert_eq!(result.stability, Frame2dTangentStability::PositiveDefinite);
        assert_eq!(result.negative_pivots, Some(0));
        assert_eq!(result.near_zero_pivots, Some(0));
    }

    #[test]
    fn two_by_two_pivot_counts_an_indefinite_direction() {
        let result = assess_symmetric_inertia(&sparse(&[
            &[0.0, 1.0, 0.0],
            &[1.0, 0.0, 0.0],
            &[0.0, 0.0, 2.0],
        ]));
        assert_eq!(result.stability, Frame2dTangentStability::Indefinite);
        assert_eq!(result.negative_pivots, Some(1));
        assert_eq!(result.near_zero_pivots, Some(0));
    }

    #[test]
    fn singular_matrix_reports_zero_and_negative_directions() {
        let result = assess_symmetric_inertia(&sparse(&[
            &[1.0, 0.0, 0.0],
            &[0.0, 0.0, 0.0],
            &[0.0, 0.0, -1.0],
        ]));
        assert_eq!(result.stability, Frame2dTangentStability::NearSingular);
        assert_eq!(result.negative_pivots, Some(1));
        assert_eq!(result.near_zero_pivots, Some(1));
    }

    #[test]
    fn oversized_matrix_is_explicitly_unassessed() {
        let matrix = SparseMatrix::new(MAX_DENSE_INERTIA_DOFS + 1);
        let result = assess_symmetric_inertia(&matrix);
        assert_eq!(
            result.stability,
            Frame2dTangentStability::UnassessedSizeLimit
        );
        assert_eq!(result.negative_pivots, None);
        assert_eq!(result.near_zero_pivots, None);
    }
}
