use crate::modal_math::jacobi_eigenpairs;

pub(crate) struct GeneralizedEigenpair {
    pub eigenvalue: f64,
    pub vector: Vec<f64>,
    pub residual_norm: f64,
}

pub(crate) fn generalized_eigenpairs(
    stiffness: &[Vec<f64>],
    geometric: &[Vec<f64>],
    mode_count: usize,
) -> Result<Vec<GeneralizedEigenpair>, String> {
    validate_matrices(stiffness, geometric)?;
    let mode_count = mode_count.max(1);
    if mode_count == 1 {
        let (eigenvalue, vector) = smallest_generalized_eigenpair(stiffness, geometric)?;
        return Ok(vec![eigenpair(stiffness, geometric, eigenvalue, vector)]);
    }

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

pub(crate) fn reduce_dense(matrix: &[Vec<f64>], free: &[usize]) -> Vec<Vec<f64>> {
    free.iter()
        .map(|&row| free.iter().map(|&column| matrix[row][column]).collect())
        .collect()
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

fn smallest_generalized_eigenpair(
    stiffness: &[Vec<f64>],
    geometric: &[Vec<f64>],
) -> Result<(f64, Vec<f64>), String> {
    let lower = cholesky(stiffness).map_err(|_| {
        "buckling elastic stiffness is not positive definite after constraints".to_string()
    })?;
    let size = stiffness.len();
    let mut shape = (0..size)
        .map(|index| {
            let phase = std::f64::consts::PI * (index + 1) as f64 / (size + 1) as f64;
            phase.sin() + 0.173 * (2.0 * phase).cos()
        })
        .collect::<Vec<_>>();
    normalize(&mut shape)?;
    let mut previous_factor = f64::NAN;

    for _ in 0..256 {
        let geometric_product = matrix_vector(geometric, &shape);
        let forward = solve_lower(&lower, &geometric_product);
        let mut next = solve_upper_transpose(&lower, &forward);
        normalize(&mut next)?;
        let elastic_product = matrix_vector(stiffness, &next);
        let next_geometric_product = matrix_vector(geometric, &next);
        let denominator = dot(&next, &next_geometric_product);
        if !(denominator.is_finite() && denominator > 1.0e-18) {
            return Err("buckling reference load pattern has no positive modal work".to_string());
        }
        let load_factor = dot(&next, &elastic_product) / denominator;
        let scale = l2_norm(&elastic_product)
            .max(load_factor.abs() * l2_norm(&next_geometric_product))
            .max(1.0);
        let relative_residual =
            generalized_residual(stiffness, geometric, &next, load_factor) / scale;
        let factor_change = (load_factor - previous_factor).abs() / load_factor.abs().max(1.0);
        shape = next;
        if relative_residual <= 1.0e-6 && factor_change <= 1.0e-8 {
            return Ok((load_factor, shape));
        }
        previous_factor = load_factor;
    }
    Err("buckling inverse iteration did not converge within 256 iterations".to_string())
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

fn matrix_vector(matrix: &[Vec<f64>], vector: &[f64]) -> Vec<f64> {
    matrix
        .iter()
        .map(|row| {
            row.iter()
                .zip(vector)
                .map(|(value, item)| value * item)
                .sum()
        })
        .collect()
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
