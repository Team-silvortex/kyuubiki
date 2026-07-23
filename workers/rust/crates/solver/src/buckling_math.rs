use crate::modal_math::jacobi_eigenpairs;
use kyuubiki_protocol::{
    BUCKLING_MODE_CLUSTER_RELATIVE_TOLERANCE, BucklingModeDirectionAssessment,
};

pub(crate) struct GeneralizedEigenpair {
    pub eigenvalue: f64,
    pub vector: Vec<f64>,
    pub residual_norm: f64,
}

pub(crate) struct ModeDirectionDiagnostic {
    pub relative_gap_to_next: Option<f64>,
    pub assessment: BucklingModeDirectionAssessment,
}

pub(crate) fn mode_direction_diagnostics(factors: &[f64]) -> Vec<ModeDirectionDiagnostic> {
    (0..factors.len())
        .map(|index| {
            let previous_gap = index
                .checked_sub(1)
                .map(|previous| relative_gap(factors[previous], factors[index]));
            let relative_gap_to_next = factors
                .get(index + 1)
                .map(|next| relative_gap(factors[index], *next));
            let clustered = previous_gap
                .into_iter()
                .chain(relative_gap_to_next)
                .any(|gap| gap <= BUCKLING_MODE_CLUSTER_RELATIVE_TOLERANCE);
            let assessment = if clustered {
                BucklingModeDirectionAssessment::Clustered
            } else if relative_gap_to_next.is_some() {
                BucklingModeDirectionAssessment::Isolated
            } else {
                BucklingModeDirectionAssessment::Unassessed
            };
            ModeDirectionDiagnostic {
                relative_gap_to_next,
                assessment,
            }
        })
        .collect()
}

pub(crate) fn generalized_eigenpairs(
    stiffness: &[Vec<f64>],
    geometric: &[Vec<f64>],
    mode_count: usize,
) -> Result<Vec<GeneralizedEigenpair>, String> {
    validate_matrices(stiffness, geometric)?;
    let active = geometrically_active_indices(geometric);
    if !active.is_empty() && active.len() < stiffness.len() {
        return condensed_generalized_eigenpairs(stiffness, geometric, &active, mode_count);
    }
    uncondensed_generalized_eigenpairs(stiffness, geometric, mode_count)
}

fn uncondensed_generalized_eigenpairs(
    stiffness: &[Vec<f64>],
    geometric: &[Vec<f64>],
    mode_count: usize,
) -> Result<Vec<GeneralizedEigenpair>, String> {
    let mode_count = mode_count.max(1);

    let elastic_lower = cholesky(stiffness).map_err(|_| {
        "buckling elastic stiffness is not positive definite after constraints".to_string()
    })?;
    let normalized = symmetric_generalized_operator(geometric, &elastic_lower);
    let mut pairs = jacobi_eigenpairs(normalized)
        .into_iter()
        .filter(|(reciprocal, _)| reciprocal.is_finite() && *reciprocal > 1.0e-12)
        .map(|(reciprocal, normalized_shape)| {
            let eigenvalue = 1.0 / reciprocal;
            let vector = solve_upper_transpose(&elastic_lower, &normalized_shape);
            eigenpair(stiffness, geometric, eigenvalue, vector)
        })
        .filter(|pair| pair.eigenvalue.is_finite() && pair.eigenvalue > 1.0e-9)
        .collect::<Vec<_>>();
    pairs.sort_by(|left, right| left.eigenvalue.total_cmp(&right.eigenvalue));
    pairs.truncate(mode_count);
    if pairs.is_empty() {
        return Err("buckling reference load pattern has no positive finite mode".to_string());
    }
    Ok(pairs)
}

fn geometrically_active_indices(geometric: &[Vec<f64>]) -> Vec<usize> {
    geometric
        .iter()
        .enumerate()
        .filter_map(|(index, row)| row.iter().any(|value| *value != 0.0).then_some(index))
        .collect()
}

fn condensed_generalized_eigenpairs(
    stiffness: &[Vec<f64>],
    geometric: &[Vec<f64>],
    active: &[usize],
    mode_count: usize,
) -> Result<Vec<GeneralizedEigenpair>, String> {
    let inactive = (0..stiffness.len())
        .filter(|index| !active.contains(index))
        .collect::<Vec<_>>();
    let inactive_stiffness = submatrix(stiffness, &inactive, &inactive);
    let inactive_lower = cholesky(&inactive_stiffness)
        .map_err(|_| "buckling inactive elastic subspace is not positive definite".to_string())?;
    let inactive_active = submatrix(stiffness, &inactive, active);
    let mut recovery = vec![vec![0.0; active.len()]; inactive.len()];
    for column in 0..active.len() {
        let rhs = inactive_active
            .iter()
            .map(|row| row[column])
            .collect::<Vec<_>>();
        let solved = solve_upper_transpose(&inactive_lower, &solve_lower(&inactive_lower, &rhs));
        for (row, value) in solved.into_iter().enumerate() {
            recovery[row][column] = value;
        }
    }

    let mut condensed_stiffness = submatrix(stiffness, active, active);
    for (row, &physical_row) in active.iter().enumerate() {
        for column in 0..active.len() {
            let correction = inactive
                .iter()
                .enumerate()
                .map(|(index, &physical_column)| {
                    stiffness[physical_row][physical_column] * recovery[index][column]
                })
                .sum::<f64>();
            condensed_stiffness[row][column] -= correction;
        }
    }
    let condensed_geometric = submatrix(geometric, active, active);
    let pairs = uncondensed_generalized_eigenpairs(
        &condensed_stiffness,
        &condensed_geometric,
        mode_count.min(active.len()),
    )?;
    Ok(pairs
        .into_iter()
        .map(|pair| {
            let mut vector = vec![0.0; stiffness.len()];
            for (index, &physical) in active.iter().enumerate() {
                vector[physical] = pair.vector[index];
            }
            for (index, &physical) in inactive.iter().enumerate() {
                vector[physical] = -dot(&recovery[index], &pair.vector);
            }
            eigenpair(stiffness, geometric, pair.eigenvalue, vector)
        })
        .collect())
}

fn submatrix(matrix: &[Vec<f64>], rows: &[usize], columns: &[usize]) -> Vec<Vec<f64>> {
    rows.iter()
        .map(|&row| columns.iter().map(|&column| matrix[row][column]).collect())
        .collect()
}

fn relative_gap(left: f64, right: f64) -> f64 {
    (right - left).abs() / left.abs().max(right.abs()).max(f64::MIN_POSITIVE)
}

fn validate_matrices(stiffness: &[Vec<f64>], geometric: &[Vec<f64>]) -> Result<(), String> {
    let size = stiffness.len();
    if size == 0
        || geometric.len() != size
        || stiffness.iter().any(|row| row.len() != size)
        || geometric.iter().any(|row| row.len() != size)
    {
        return Err(
            "buckling generalized eigenproblem matrices must be square and non-empty".into(),
        );
    }
    Ok(())
}

fn eigenpair(
    stiffness: &[Vec<f64>],
    geometric: &[Vec<f64>],
    eigenvalue: f64,
    mut vector: Vec<f64>,
) -> GeneralizedEigenpair {
    normalize(&mut vector).expect("eigensolver produced a non-zero vector");
    GeneralizedEigenpair {
        residual_norm: generalized_residual(stiffness, geometric, &vector, eigenvalue),
        eigenvalue,
        vector,
    }
}

fn cholesky(matrix: &[Vec<f64>]) -> Result<Vec<Vec<f64>>, String> {
    let size = matrix.len();
    let mut lower = vec![vec![0.0; size]; size];
    for row in 0..size {
        for column in 0..=row {
            let sum = (0..column)
                .map(|index| lower[row][index] * lower[column][index])
                .sum::<f64>();
            if row == column {
                let diagonal = matrix[row][row] - sum;
                if !(diagonal.is_finite() && diagonal > 1.0e-14) {
                    return Err("matrix is not positive definite".to_string());
                }
                lower[row][column] = diagonal.sqrt();
            } else {
                lower[row][column] = (matrix[row][column] - sum) / lower[column][column];
            }
        }
    }
    Ok(lower)
}

fn symmetric_generalized_operator(matrix: &[Vec<f64>], lower: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let size = matrix.len();
    let mut left = vec![vec![0.0; size]; size];
    for column in 0..size {
        let rhs = (0..size).map(|row| matrix[row][column]).collect::<Vec<_>>();
        let solved = solve_lower(lower, &rhs);
        for row in 0..size {
            left[row][column] = solved[row];
        }
    }
    (0..size)
        .map(|row| solve_lower(lower, &left[row]))
        .collect()
}

fn solve_lower(lower: &[Vec<f64>], rhs: &[f64]) -> Vec<f64> {
    let mut result = vec![0.0; rhs.len()];
    for row in 0..rhs.len() {
        let sum = (0..row)
            .map(|column| lower[row][column] * result[column])
            .sum::<f64>();
        result[row] = (rhs[row] - sum) / lower[row][row];
    }
    result
}

fn solve_upper_transpose(lower: &[Vec<f64>], rhs: &[f64]) -> Vec<f64> {
    let mut result = vec![0.0; rhs.len()];
    for row in (0..rhs.len()).rev() {
        let sum = ((row + 1)..rhs.len())
            .map(|column| lower[column][row] * result[column])
            .sum::<f64>();
        result[row] = (rhs[row] - sum) / lower[row][row];
    }
    result
}

fn normalize(vector: &mut [f64]) -> Result<(), String> {
    let norm = l2_norm(vector);
    if !(norm.is_finite() && norm > f64::EPSILON) {
        return Err("buckling eigenvector normalization failed".to_string());
    }
    vector.iter_mut().for_each(|value| *value /= norm);
    Ok(())
}

fn dot(left: &[f64], right: &[f64]) -> f64 {
    left.iter().zip(right).map(|(a, b)| a * b).sum()
}

fn l2_norm(values: &[f64]) -> f64 {
    dot(values, values).sqrt()
}

fn generalized_residual(
    stiffness: &[Vec<f64>],
    geometric: &[Vec<f64>],
    shape: &[f64],
    factor: f64,
) -> f64 {
    (0..shape.len())
        .map(|row| {
            (0..shape.len())
                .map(|column| {
                    (stiffness[row][column] - factor * geometric[row][column]) * shape[column]
                })
                .sum::<f64>()
        })
        .map(|value| value * value)
        .sum::<f64>()
        .sqrt()
}

#[cfg(test)]
mod tests {
    use kyuubiki_protocol::BucklingModeDirectionAssessment;

    use super::{generalized_eigenpairs, mode_direction_diagnostics};

    #[test]
    fn direction_diagnostics_distinguish_clusters_and_unassessed_tail() {
        let diagnostics = mode_direction_diagnostics(&[1.0, 1.000_001, 4.0, 9.0]);
        assert_eq!(
            diagnostics[0].assessment,
            BucklingModeDirectionAssessment::Clustered
        );
        assert_eq!(
            diagnostics[1].assessment,
            BucklingModeDirectionAssessment::Clustered
        );
        assert_eq!(
            diagnostics[2].assessment,
            BucklingModeDirectionAssessment::Isolated
        );
        assert_eq!(
            diagnostics[3].assessment,
            BucklingModeDirectionAssessment::Unassessed
        );
    }

    #[test]
    fn dense_single_mode_uses_the_complete_symmetric_spectrum() {
        let stiffness = diagonal(&[1.0, 1.000_1, 20.0, 100.0]);
        let geometric = diagonal(&[1.0; 4]);
        let pairs = generalized_eigenpairs(&stiffness, &geometric, 1)
            .expect("dense clustered spectrum should use the complete decomposition");
        assert_eq!(pairs.len(), 1);
        assert!((pairs[0].eigenvalue - 1.0).abs() < 1.0e-10);
    }

    #[test]
    fn geometric_inactive_dof_condensation_recovers_the_full_mode() {
        let stiffness = vec![vec![2.0, -1.0], vec![-1.0, 1.0]];
        let geometric = vec![vec![1.0, 0.0], vec![0.0, 0.0]];
        let pairs = generalized_eigenpairs(&stiffness, &geometric, 1)
            .expect("semidefinite generalized problem should condense");
        assert!((pairs[0].eigenvalue - 1.0).abs() < 1.0e-12);
        assert!((pairs[0].vector[0] - pairs[0].vector[1]).abs() < 1.0e-12);
        assert!(pairs[0].residual_norm < 1.0e-12);
    }

    fn diagonal(values: &[f64]) -> Vec<Vec<f64>> {
        (0..values.len())
            .map(|row| {
                (0..values.len())
                    .map(|column| if row == column { values[row] } else { 0.0 })
                    .collect()
            })
            .collect()
    }
}
