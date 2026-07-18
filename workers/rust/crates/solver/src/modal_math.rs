const MAX_DENSE_MODAL_DOFS: usize = 4_096;

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
    for (index, &dof) in free_dofs.iter().enumerate() {
        shape[dof] = vector[index] / mass[dof].sqrt();
    }
    normalize_shape(&mut shape);
    shape
}

fn normalize_shape(shape: &mut [f64]) {
    let norm = shape.iter().map(|value| value * value).sum::<f64>().sqrt();
    if norm > 0.0 {
        for value in shape {
            *value /= norm;
        }
    }
}

pub(crate) fn jacobi_eigenpairs(mut matrix: Vec<Vec<f64>>) -> Vec<(f64, Vec<f64>)> {
    let size = matrix.len();
    let mut vectors = vec![vec![0.0; size]; size];
    for (index, row) in vectors.iter_mut().enumerate() {
        row[index] = 1.0;
    }

    for _ in 0..(size * size * 40).max(80) {
        let (p, q, max_offdiag) = largest_offdiag(&matrix);
        if max_offdiag < 1.0e-8 {
            break;
        }
        let tau = (matrix[q][q] - matrix[p][p]) / (2.0 * matrix[p][q]);
        let t = tau.signum() / (tau.abs() + (1.0 + tau * tau).sqrt());
        let c = 1.0 / (1.0 + t * t).sqrt();
        let s = t * c;
        rotate(&mut matrix, &mut vectors, p, q, c, s);
    }

    let mut pairs = (0..size)
        .map(|index| {
            let vector = (0..size).map(|row| vectors[row][index]).collect::<Vec<_>>();
            (matrix[index][index], vector)
        })
        .collect::<Vec<_>>();
    pairs.sort_by(|left, right| left.0.total_cmp(&right.0));
    pairs
}

fn largest_offdiag(matrix: &[Vec<f64>]) -> (usize, usize, f64) {
    let mut best = (0, 1.min(matrix.len() - 1), 0.0);
    for row in 0..matrix.len() {
        for column in (row + 1)..matrix.len() {
            let value = matrix[row][column].abs();
            if value > best.2 {
                best = (row, column, value);
            }
        }
    }
    best
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
    use super::ensure_dense_modal_size;

    #[test]
    fn dense_modal_solver_rejects_unsafe_matrix_sizes() {
        assert!(ensure_dense_modal_size(4_096, "modal").is_ok());
        let error = ensure_dense_modal_size(4_097, "modal").unwrap_err();
        assert!(error.contains("sparse modal solver"));
    }
}
