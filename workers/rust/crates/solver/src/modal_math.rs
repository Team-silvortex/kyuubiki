use crate::linear_algebra::stable_l2_norm;
use crate::solver_control::{SolverStage, checkpoint, checkpoint_chunk};

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
) -> Result<Vec<f64>, String> {
    checkpoint(SolverStage::ResultFreeDofs, 0)?;
    if vector.len() != free_dofs.len() || mass.len() != dof_count || vector.is_empty() {
        return Err("modal mode shape dimensions are inconsistent or empty".into());
    }
    let mut shape = vec![0.0; dof_count];
    let mut max_exponent = i32::MIN;
    for (index, &dof) in free_dofs.iter().enumerate() {
        if dof >= dof_count || shape[dof] != 0.0 {
            return Err("modal mode shape free dofs are invalid or duplicated".into());
        }
        if !vector[index].is_finite() || !mass[dof].is_finite() || mass[dof] <= 0.0 {
            return Err(
                "modal mode shape requires finite values and positive free-dof mass".into(),
            );
        }
        // Reuse the output as a visited map; the recovery pass replaces every marker.
        shape[dof] = 1.0;
        if vector[index] != 0.0 {
            let (_, exponent) = weighted_component(vector[index], mass[dof]);
            max_exponent = max_exponent.max(exponent);
        }
        checkpoint_chunk(SolverStage::ResultFreeDofs, index + 1, free_dofs.len())?;
    }
    if max_exponent == i32::MIN {
        return Err("modal mode shape eigenvector is zero".into());
    }
    checkpoint(SolverStage::ResultNodes, 0)?;
    for (index, &dof) in free_dofs.iter().enumerate() {
        shape[dof] = if vector[index] == 0.0 {
            0.0
        } else {
            let (mantissa, exponent) = weighted_component(vector[index], mass[dof]);
            scaled_component(mantissa, exponent - max_exponent).copysign(vector[index])
        };
        checkpoint_chunk(SolverStage::ResultNodes, index + 1, free_dofs.len())?;
    }
    let norm = checked_shape_norm(&shape)?;
    checkpoint(SolverStage::ResultTotals, 0)?;
    for (index, &dof) in free_dofs.iter().enumerate() {
        shape[dof] /= norm;
        checkpoint_chunk(SolverStage::ResultTotals, index + 1, free_dofs.len())?;
    }
    Ok(shape)
}

// Keep v / sqrt(m) in exponent form until a common, mode-local scale is known.
fn weighted_component(value: f64, mass: f64) -> (f64, i32) {
    let (v, ve) = binary_parts(value.abs());
    let (m, me) = binary_parts(mass.sqrt());
    let mantissa = v / m;
    if mantissa < 1.0 {
        (mantissa * 2.0, ve - me - 1)
    } else {
        (mantissa, ve - me)
    }
}

fn binary_parts(value: f64) -> (f64, i32) {
    let (normal, adjustment) = if value < f64::MIN_POSITIVE {
        (value * 4_503_599_627_370_496.0, -52)
    } else {
        (value, 0)
    };
    let bits = normal.to_bits();
    let exponent = ((bits >> 52) & 0x7ff) as i32 - 1023 + adjustment;
    let mantissa = f64::from_bits((bits & ((1_u64 << 52) - 1)) | (1023_u64 << 52));
    (mantissa, exponent)
}

fn scaled_component(mantissa: f64, exponent: i32) -> f64 {
    let normal_power = |exponent: i32| f64::from_bits(((exponent + 1023) as u64) << 52);
    if exponent >= -1022 {
        mantissa * normal_power(exponent)
    } else if exponent >= -1075 {
        // Round into the subnormal range only on the final multiplication.
        (mantissa * normal_power(-1021)) * normal_power(exponent + 1021)
    } else {
        0.0
    }
}

pub(crate) fn checked_shape_norm(shape: &[f64]) -> Result<f64, String> {
    checkpoint(SolverStage::ResultNodeSummary, 0)?;
    let mut norm = 0.0_f64;
    for (index, chunk) in shape.chunks(64).enumerate() {
        norm = norm.hypot(stable_l2_norm(chunk.iter().copied()));
        checkpoint(
            SolverStage::ResultNodeSummary,
            ((index + 1) * 64).min(shape.len()),
        )?;
    }
    if !norm.is_finite() || norm <= 0.0 {
        return Err("modal frame recovered a zero or non-finite mode shape".into());
    }
    Ok(norm)
}

pub(crate) fn jacobi_eigenpairs(matrix: Vec<Vec<f64>>) -> Result<Vec<(f64, Vec<f64>)>, String> {
    jacobi_eigenpairs_with_max_sweeps(matrix, MAX_JACOBI_SWEEPS)
}

pub(crate) fn relative_positive_eigenvalue_floor(values: impl IntoIterator<Item = f64>) -> f64 {
    JACOBI_RELATIVE_TOLERANCE * values.into_iter().map(f64::abs).fold(0.0_f64, f64::max)
}

fn jacobi_eigenpairs_with_max_sweeps(
    matrix: Vec<Vec<f64>>,
    max_sweeps: usize,
) -> Result<Vec<(f64, Vec<f64>)>, String> {
    let size = matrix.len();
    checkpoint(SolverStage::ModalSweep, 0)?;
    if size == 0 || matrix.iter().any(|row| row.len() != size) {
        return Err("symmetric Jacobi matrix must be square and non-empty".to_string());
    }
    if matrix.iter().flatten().any(|value| !value.is_finite()) {
        return Err("symmetric Jacobi matrix contains a non-finite value".to_string());
    }
    for row in 0..size {
        for column in row + 1..size {
            // An unrelated stiff DOF must not hide asymmetry in this local pair.
            let scale = matrix[row][row]
                .abs()
                .max(matrix[column][column].abs())
                .max(matrix[row][column].abs())
                .max(matrix[column][row].abs());
            if scale > 0.0
                && (matrix[row][column] / scale - matrix[column][row] / scale).abs() > 1.0e-10
            {
                return Err("symmetric Jacobi matrix is not symmetric".to_string());
            }
        }
        checkpoint_chunk(SolverStage::ModalSweep, row + 1, size)?;
    }
    let components = independent_components(&matrix)?;
    if components.len() == 1 {
        return jacobi_connected_eigenpairs(matrix, max_sweeps);
    }
    let mut pairs = Vec::with_capacity(size);
    for indices in components {
        checkpoint(SolverStage::ModalSweep, 0)?;
        let block = indices
            .iter()
            .map(|&row| indices.iter().map(|&column| matrix[row][column]).collect())
            .collect();
        for (value, vector) in jacobi_connected_eigenpairs(block, max_sweeps)? {
            let mut expanded = vec![0.0; size];
            for (&dof, component) in indices.iter().zip(vector) {
                expanded[dof] = component;
            }
            pairs.push((value, expanded));
        }
    }
    pairs.sort_by(|left, right| left.0.total_cmp(&right.0));
    Ok(pairs)
}

fn independent_components(matrix: &[Vec<f64>]) -> Result<Vec<Vec<usize>>, String> {
    let mut visited = vec![false; matrix.len()];
    let mut components = Vec::new();
    let mut completed = 0;
    for root in 0..matrix.len() {
        if visited[root] {
            continue;
        }
        visited[root] = true;
        let mut pending = vec![root];
        let mut indices = Vec::new();
        while let Some(row) = pending.pop() {
            indices.push(row);
            for column in 0..matrix.len() {
                // Split only exact zeros, never a tolerance-based weak coupling.
                if !visited[column] && (matrix[row][column] != 0.0 || matrix[column][row] != 0.0) {
                    visited[column] = true;
                    pending.push(column);
                }
            }
            completed += 1;
            checkpoint_chunk(SolverStage::ModalSweep, completed, matrix.len())?;
        }
        indices.sort_unstable();
        components.push(indices);
    }
    Ok(components)
}

fn jacobi_connected_eigenpairs(
    mut matrix: Vec<Vec<f64>>,
    max_sweeps: usize,
) -> Result<Vec<(f64, Vec<f64>)>, String> {
    let size = matrix.len();
    let matrix_scale = matrix
        .iter()
        .flatten()
        .map(|value| value.abs())
        .fold(0.0_f64, f64::max);
    if matrix_scale > 0.0 {
        for value in matrix.iter_mut().flatten() {
            let original = *value;
            *value /= matrix_scale;
            if original != 0.0 && *value == 0.0 {
                return Err("symmetric Jacobi component scaling is not representable".into());
            }
        }
    }
    let mut vectors = vec![vec![0.0; size]; size];
    for (index, row) in vectors.iter_mut().enumerate() {
        row[index] = 1.0;
    }

    let mut converged = couplings_resolved(&matrix);
    for sweep in 0..max_sweeps {
        if converged {
            break;
        }
        for p in 0..size {
            checkpoint(SolverStage::ModalSweep, sweep * size + p)?;
            for q in p + 1..size {
                let coupling = matrix[p][q];
                if coupling_resolved(coupling, matrix[p][p], matrix[q][q]) {
                    continue;
                }
                let half_difference = (matrix[q][q] - matrix[p][p]) * 0.5;
                let radius = half_difference.hypot(coupling);
                let t = coupling / (half_difference + radius.copysign(half_difference));
                let c = 1.0 / (1.0 + t * t).sqrt();
                rotate(&mut matrix, &mut vectors, p, q, c, t * c);
            }
        }
        converged = couplings_resolved(&matrix);
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

fn coupling_resolved(value: f64, left: f64, right: f64) -> bool {
    // Bound both modal column residuals, not their geometric-mean scale. A tiny
    // rotation may barely change the eigenvalues but still be essential to a soft mode.
    let scale = left.abs().min(right.abs());
    value == 0.0 || (scale > 0.0 && value.abs() / scale <= JACOBI_RELATIVE_TOLERANCE)
}

fn couplings_resolved(matrix: &[Vec<f64>]) -> bool {
    (0..matrix.len()).all(|row| {
        (row + 1..matrix.len()).all(|column| {
            coupling_resolved(
                matrix[row][column],
                matrix[row][row],
                matrix[column][column],
            )
        })
    })
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
#[path = "modal_math_component_tests.rs"]
mod component_tests;

#[cfg(test)]
#[path = "modal_mode_shape_tests.rs"]
mod shape_tests;

#[cfg(test)]
#[path = "modal_math_convergence_tests.rs"]
mod convergence_tests;

#[cfg(test)]
#[path = "modal_math_cluster_tests.rs"]
mod cluster_tests;

#[cfg(test)]
mod tests {
    use super::{
        ensure_dense_modal_size, expand_mode_shape, jacobi_eigenpairs,
        jacobi_eigenpairs_with_max_sweeps,
    };

    #[test]
    fn mode_shape_expansion_stays_finite_across_extreme_mass_scales() {
        let shape = expand_mode_shape(&[1.0, 1.0], &[1.0e-300, 1.0e300], &[0, 1], 2).unwrap();

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
