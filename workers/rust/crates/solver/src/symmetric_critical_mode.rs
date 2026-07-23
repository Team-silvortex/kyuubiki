use crate::linear_algebra::{SparseMatrix, sparse_to_dense};

const MAX_DENSE_CRITICAL_MODE_DOFS: usize = 128;
const JACOBI_TOLERANCE: f64 = 1.0e-10;
const MAX_JACOBI_SWEEPS: usize = 40;

#[derive(Clone)]
pub(crate) struct SymmetricCriticalMode {
    pub(crate) normalized_eigenvalue: f64,
    pub(crate) normalized_residual: f64,
    pub(crate) shape: Vec<f64>,
}

#[cfg(test)]
pub(crate) fn extract_symmetric_critical_mode(
    matrix: &SparseMatrix,
    free_dofs: &[usize],
    full_dof_count: usize,
) -> Option<SymmetricCriticalMode> {
    extract_symmetric_critical_modes(matrix, free_dofs, full_dof_count, 1)
        .into_iter()
        .next()
}

pub(crate) fn extract_symmetric_critical_modes(
    matrix: &SparseMatrix,
    free_dofs: &[usize],
    full_dof_count: usize,
    mode_count: usize,
) -> Vec<SymmetricCriticalMode> {
    if matrix.size() == 0
        || matrix.size() > MAX_DENSE_CRITICAL_MODE_DOFS
        || free_dofs.len() != matrix.size()
        || mode_count == 0
    {
        return Vec::new();
    }
    let mut normalized = sparse_to_dense(matrix);
    if normalized.iter().flatten().any(|value| !value.is_finite()) {
        return Vec::new();
    }
    let scale = normalized
        .iter()
        .flatten()
        .map(|value| value.abs())
        .fold(0.0_f64, f64::max);
    if scale == 0.0 {
        return Vec::new();
    }
    for value in normalized.iter_mut().flatten() {
        *value /= scale;
    }
    let Some(eigenpairs) = absolute_eigenpairs(normalized.clone()) else {
        return Vec::new();
    };
    eigenpairs
        .into_iter()
        .take(mode_count)
        .filter_map(|(eigenvalue, mut reduced)| {
            normalize_and_canonicalize(&mut reduced)?;
            let residual = eigen_residual(&normalized, eigenvalue, &reduced);
            if !(eigenvalue.is_finite() && residual.is_finite()) {
                return None;
            }
            let mut shape = vec![0.0; full_dof_count];
            for (&dof, value) in free_dofs.iter().zip(reduced) {
                shape[dof] = value;
            }
            Some(SymmetricCriticalMode {
                normalized_eigenvalue: eigenvalue,
                normalized_residual: residual,
                shape,
            })
        })
        .collect()
}

fn absolute_eigenpairs(mut matrix: Vec<Vec<f64>>) -> Option<Vec<(f64, Vec<f64>)>> {
    let size = matrix.len();
    let mut vectors = vec![vec![0.0; size]; size];
    for (index, row) in vectors.iter_mut().enumerate() {
        row[index] = 1.0;
    }
    for _ in 0..MAX_JACOBI_SWEEPS {
        let mut maximum = 0.0_f64;
        for first in 0..size {
            for second in first + 1..size {
                let coupling = matrix[first][second];
                maximum = maximum.max(coupling.abs());
                if coupling.abs() <= JACOBI_TOLERANCE {
                    continue;
                }
                let tau = (matrix[second][second] - matrix[first][first]) / (2.0 * coupling);
                let tangent = if tau >= 0.0 {
                    1.0 / (tau + (1.0 + tau * tau).sqrt())
                } else {
                    -1.0 / (-tau + (1.0 + tau * tau).sqrt())
                };
                let cosine = 1.0 / (1.0 + tangent * tangent).sqrt();
                rotate(
                    &mut matrix,
                    &mut vectors,
                    first,
                    second,
                    cosine,
                    tangent * cosine,
                );
            }
        }
        if maximum <= JACOBI_TOLERANCE {
            break;
        }
    }
    let mut eigenpairs = (0..size)
        .map(|index| {
            (
                matrix[index][index],
                (0..size).map(|row| vectors[row][index]).collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<_>>();
    eigenpairs.sort_by(|left, right| left.0.abs().total_cmp(&right.0.abs()));
    Some(eigenpairs)
}

fn rotate(
    matrix: &mut [Vec<f64>],
    vectors: &mut [Vec<f64>],
    first: usize,
    second: usize,
    cosine: f64,
    sine: f64,
) {
    let first_diagonal = matrix[first][first];
    let second_diagonal = matrix[second][second];
    let coupling = matrix[first][second];
    matrix[first][first] = cosine * cosine * first_diagonal - 2.0 * sine * cosine * coupling
        + sine * sine * second_diagonal;
    matrix[second][second] = sine * sine * first_diagonal
        + 2.0 * sine * cosine * coupling
        + cosine * cosine * second_diagonal;
    matrix[first][second] = 0.0;
    matrix[second][first] = 0.0;
    for index in 0..matrix.len() {
        if index != first && index != second {
            let first_value = matrix[index][first];
            let second_value = matrix[index][second];
            matrix[index][first] = cosine * first_value - sine * second_value;
            matrix[first][index] = matrix[index][first];
            matrix[index][second] = sine * first_value + cosine * second_value;
            matrix[second][index] = matrix[index][second];
        }
        let first_vector = vectors[index][first];
        let second_vector = vectors[index][second];
        vectors[index][first] = cosine * first_vector - sine * second_vector;
        vectors[index][second] = sine * first_vector + cosine * second_vector;
    }
}

fn normalize_and_canonicalize(vector: &mut [f64]) -> Option<()> {
    let norm = vector.iter().map(|value| value * value).sum::<f64>().sqrt();
    if !(norm.is_finite() && norm > 0.0) {
        return None;
    }
    for value in vector.iter_mut() {
        *value /= norm;
    }
    let anchor = vector
        .iter()
        .copied()
        .max_by(|left, right| left.abs().total_cmp(&right.abs()))?;
    if anchor < 0.0 {
        for value in vector {
            *value = -*value;
        }
    }
    Some(())
}

fn eigen_residual(matrix: &[Vec<f64>], eigenvalue: f64, vector: &[f64]) -> f64 {
    matrix
        .iter()
        .zip(vector)
        .map(|(row, component)| {
            let applied = row
                .iter()
                .zip(vector)
                .map(|(value, direction)| value * direction)
                .sum::<f64>();
            (applied - eigenvalue * component).powi(2)
        })
        .sum::<f64>()
        .sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linear_algebra::{SparseMatrix, add_at};

    #[test]
    fn extracts_the_smallest_absolute_symmetric_mode() {
        let mut matrix = SparseMatrix::new(3);
        add_at(&mut matrix, 0, 0, 4.0);
        add_at(&mut matrix, 1, 1, -0.01);
        add_at(&mut matrix, 2, 2, 2.0);
        let mode = extract_symmetric_critical_mode(&matrix, &[0, 2, 3], 5).unwrap();
        assert!((mode.normalized_eigenvalue + 0.0025).abs() < 1.0e-12);
        assert!(mode.normalized_residual < 1.0e-12);
        assert_eq!(mode.shape, vec![0.0, 0.0, 1.0, 0.0, 0.0]);
    }

    #[test]
    fn rotates_equal_diagonal_coupled_terms() {
        let mut matrix = SparseMatrix::new(2);
        add_at(&mut matrix, 0, 0, 1.0);
        add_at(&mut matrix, 0, 1, 0.1);
        add_at(&mut matrix, 1, 0, 0.1);
        add_at(&mut matrix, 1, 1, 1.0);
        let mode = extract_symmetric_critical_mode(&matrix, &[0, 1], 2).unwrap();
        assert!((mode.normalized_eigenvalue - 0.9).abs() < 1.0e-12);
        assert!(mode.normalized_residual < 1.0e-12);
    }

    #[test]
    fn extracts_an_ordered_orthogonal_critical_subspace() {
        let mut matrix = SparseMatrix::new(3);
        add_at(&mut matrix, 0, 0, -0.02);
        add_at(&mut matrix, 1, 1, 0.01);
        add_at(&mut matrix, 2, 2, 4.0);
        let modes = extract_symmetric_critical_modes(&matrix, &[0, 1, 2], 3, 2);
        assert_eq!(modes.len(), 2);
        assert!((modes[0].normalized_eigenvalue - 0.0025).abs() < 1.0e-12);
        assert!((modes[1].normalized_eigenvalue + 0.005).abs() < 1.0e-12);
        assert!(modes.iter().all(|mode| mode.normalized_residual < 1.0e-12));
        let dot = modes[0]
            .shape
            .iter()
            .zip(&modes[1].shape)
            .map(|(left, right)| left * right)
            .sum::<f64>();
        assert!(dot.abs() < 1.0e-12);
    }

    #[test]
    fn declines_modes_beyond_the_dense_diagnostic_limit() {
        let matrix = SparseMatrix::new(MAX_DENSE_CRITICAL_MODE_DOFS + 1);
        assert!(
            extract_symmetric_critical_mode(&matrix, &vec![0; matrix.size()], matrix.size())
                .is_none()
        );
    }
}
