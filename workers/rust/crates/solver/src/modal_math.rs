use crate::linear_algebra::stable_l2_norm;

const MAX_DENSE_MODAL_DOFS: usize = 4_096;
const JACOBI_RELATIVE_TOLERANCE: f64 = 1.0e-12;
const MAX_JACOBI_SWEEPS: usize = 40;

pub(crate) fn ensure_dense_modal_size(dof_count: usize, label: &str) -> Result<(), String> {
    if dof_count > MAX_DENSE_MODAL_DOFS {
        return Err(format!(
            "{label} has {dof_count} dofs; the dense modal solver supports at most {MAX_DENSE_MODAL_DOFS}. Use the sparse modal solver for larger models"
        ));
    }
    Ok(())
}

pub(crate) fn expand_mode_shape(
    vector: &[f64],
    mass: &[f64],
    free_dofs: &[usize],
    dof_count: usize,
) -> Vec<f64> {
    let mut shape = vec![0.0; dof_count];
    let inverse_mass_scale = free_dofs
        .iter()
        .map(|&dof| mass[dof].sqrt().recip())
        .fold(0.0_f64, f64::max);
    for (index, &dof) in free_dofs.iter().enumerate() {
        shape[dof] = vector[index] * mass[dof].sqrt().recip() / inverse_mass_scale;
    }
    normalize_shape(&mut shape);
    shape
}

fn normalize_shape(shape: &mut [f64]) {
    let norm = stable_l2_norm(shape.iter().copied());
    if norm > 0.0 {
        for value in shape {
            *value /= norm;
        }
    }
}

pub(crate) fn jacobi_eigenpairs(matrix: Vec<Vec<f64>>) -> Result<Vec<(f64, Vec<f64>)>, String> {
    jacobi_eigenpairs_with_max_sweeps(matrix, MAX_JACOBI_SWEEPS)
}

pub(crate) fn relative_positive_eigenvalue_floor(values: impl IntoIterator<Item = f64>) -> f64 {
    JACOBI_RELATIVE_TOLERANCE * values.into_iter().map(f64::abs).fold(0.0_f64, f64::max)
}

fn jacobi_eigenpairs_with_max_sweeps(
    mut matrix: Vec<Vec<f64>>,
    max_sweeps: usize,
) -> Result<Vec<(f64, Vec<f64>)>, String> {
    let size = matrix.len();
    if size == 0 || matrix.iter().any(|row| row.len() != size) {
        return Err("symmetric Jacobi matrix must be square and non-empty".to_string());
    }
    if matrix.iter().flatten().any(|value| !value.is_finite()) {
        return Err("symmetric Jacobi matrix contains a non-finite value".to_string());
    }
    let matrix_scale = matrix
        .iter()
        .flatten()
        .map(|value| value.abs())
        .fold(0.0_f64, f64::max);
    for row in 0..size {
        for column in row + 1..size {
            if (matrix[row][column] - matrix[column][row]).abs()
                > 1.0e-10 * matrix_scale.max(f64::MIN_POSITIVE)
            {
                return Err("symmetric Jacobi matrix is not symmetric".to_string());
            }
        }
    }
    if matrix_scale > 0.0 {
        for value in matrix.iter_mut().flatten() {
            *value /= matrix_scale;
        }
    }
    let mut vectors = vec![vec![0.0; size]; size];
    for (index, row) in vectors.iter_mut().enumerate() {
        row[index] = 1.0;
    }

    let mut converged = largest_offdiag_magnitude(&matrix) <= JACOBI_RELATIVE_TOLERANCE;
    for _ in 0..max_sweeps {
        if converged {
            break;
        }
        for p in 0..size {
            for q in p + 1..size {
                let coupling = matrix[p][q];
                if coupling.abs() <= JACOBI_RELATIVE_TOLERANCE {
                    continue;
                }
                let tau = (matrix[q][q] - matrix[p][p]) / (2.0 * coupling);
                let t = if tau >= 0.0 {
                    1.0 / (tau + tau.hypot(1.0))
                } else {
                    -1.0 / (-tau + tau.hypot(1.0))
                };
                let c = 1.0 / (1.0 + t * t).sqrt();
                rotate(&mut matrix, &mut vectors, p, q, c, t * c);
            }
        }
        converged = largest_offdiag_magnitude(&matrix) <= JACOBI_RELATIVE_TOLERANCE;
    }
    if !converged {
        return Err(format!(
            "symmetric Jacobi eigensolver did not converge within {max_sweeps} sweeps (relative off-diagonal={:.6e})",
            largest_offdiag_magnitude(&matrix)
        ));
    }

    let mut pairs = (0..size)
        .map(|index| {
            let vector = (0..size).map(|row| vectors[row][index]).collect::<Vec<_>>();
            (matrix[index][index] * matrix_scale, vector)
        })
        .collect::<Vec<_>>();
    pairs.sort_by(|left, right| left.0.total_cmp(&right.0));
    Ok(pairs)
}

fn largest_offdiag_magnitude(matrix: &[Vec<f64>]) -> f64 {
    let mut largest = 0.0_f64;
    for row in 0..matrix.len() {
        for column in (row + 1)..matrix.len() {
            largest = largest.max(matrix[row][column].abs());
        }
    }
    largest
}

fn rotate(matrix: &mut [Vec<f64>], vectors: &mut [Vec<f64>], p: usize, q: usize, c: f64, s: f64) {
    let app = matrix[p][p];
    let aqq = matrix[q][q];
    let apq = matrix[p][q];
    matrix[p][p] = c * c * app - 2.0 * s * c * apq + s * s * aqq;
    matrix[q][q] = s * s * app + 2.0 * s * c * apq + c * c * aqq;
    matrix[p][q] = 0.0;
    matrix[q][p] = 0.0;
    for index in 0..matrix.len() {
        if index != p && index != q {
            let aip = matrix[index][p];
            let aiq = matrix[index][q];
            matrix[index][p] = c * aip - s * aiq;
            matrix[p][index] = matrix[index][p];
            matrix[index][q] = s * aip + c * aiq;
            matrix[q][index] = matrix[index][q];
        }
        let vip = vectors[index][p];
        let viq = vectors[index][q];
        vectors[index][p] = c * vip - s * viq;
        vectors[index][q] = s * vip + c * viq;
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ensure_dense_modal_size, expand_mode_shape, jacobi_eigenpairs,
        jacobi_eigenpairs_with_max_sweeps,
    };

    #[test]
    fn mode_shape_expansion_stays_finite_across_extreme_mass_scales() {
        let shape = expand_mode_shape(&[1.0, 1.0], &[1.0e-300, 1.0e300], &[0, 1], 2);

        assert!(shape.iter().all(|value| value.is_finite()));
        assert!((shape[0] - 1.0).abs() < 1.0e-12);
        assert!(shape[1] > 0.0);
        assert!(shape[1] < 1.0e-250);
    }

    #[test]
    fn dense_modal_solver_rejects_unsafe_matrix_sizes() {
        assert!(ensure_dense_modal_size(4_096, "modal").is_ok());
        let error = ensure_dense_modal_size(4_097, "modal").unwrap_err();
        assert!(error.contains("sparse modal solver"));
    }

    #[test]
    fn jacobi_eigenpairs_preserve_uniformly_tiny_matrix_scale() {
        let scale = 1.0e-24;
        let pairs = jacobi_eigenpairs(vec![vec![2.0 * scale, scale], vec![scale, 2.0 * scale]])
            .expect("tiny symmetric matrix should converge");

        assert!((pairs[0].0 / scale - 1.0).abs() < 1.0e-10);
        assert!((pairs[1].0 / scale - 3.0).abs() < 1.0e-10);
    }

    #[test]
    fn jacobi_eigenpairs_report_invalid_input_and_iteration_exhaustion() {
        assert!(jacobi_eigenpairs(Vec::new()).is_err());
        assert!(jacobi_eigenpairs(vec![vec![1.0, f64::NAN], vec![f64::NAN, 1.0]]).is_err());
        assert!(jacobi_eigenpairs(vec![vec![1.0, 0.5], vec![0.0, 1.0]]).is_err());

        let error =
            jacobi_eigenpairs_with_max_sweeps(vec![vec![1.0, 0.5], vec![0.5, 2.0]], 0).unwrap_err();
        assert!(error.contains("did not converge within 0 sweeps"));
    }
}
