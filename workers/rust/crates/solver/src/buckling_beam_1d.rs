use crate::modal_math::{ensure_dense_modal_size, jacobi_eigenpairs};
use kyuubiki_protocol::{
    BucklingBeam1dModeResult, SolveBucklingBeam1dRequest, SolveBucklingBeam1dResult,
};
use std::collections::HashSet;

pub fn solve_buckling_beam_1d(
    request: &SolveBucklingBeam1dRequest,
) -> Result<SolveBucklingBeam1dResult, String> {
    validate(request)?;
    let dof_count = request.nodes.len() * 2;
    ensure_dense_modal_size(dof_count, "buckling beam 1d")?;
    let mut elastic = vec![vec![0.0; dof_count]; dof_count];
    let mut geometric = vec![vec![0.0; dof_count]; dof_count];

    for element in &request.elements {
        let length = (request.nodes[element.node_j].x - request.nodes[element.node_i].x).abs();
        let local_elastic =
            elastic_stiffness(element.youngs_modulus * element.moment_of_inertia, length);
        let local_geometric = geometric_stiffness(element.reference_compressive_force, length);
        let map = [
            element.node_i * 2,
            element.node_i * 2 + 1,
            element.node_j * 2,
            element.node_j * 2 + 1,
        ];
        for row in 0..4 {
            for column in 0..4 {
                elastic[map[row]][map[column]] += local_elastic[row][column];
                geometric[map[row]][map[column]] += local_geometric[row][column];
            }
        }
    }

    let constrained = constrained_dofs(request);
    let free_dofs = (0..dof_count)
        .filter(|dof| !constrained.contains(dof))
        .collect::<Vec<_>>();
    let reduced_elastic = reduce_dense(&elastic, &free_dofs);
    let reduced_geometric = reduce_dense(&geometric, &free_dofs);
    let mode_limit = request.mode_count.unwrap_or(3).max(1);
    let physical_eigenpairs = if mode_limit == 1 {
        vec![smallest_generalized_eigenpair(
            &reduced_elastic,
            &reduced_geometric,
        )?]
    } else {
        let cholesky = cholesky(&reduced_geometric)?;
        let normalized = symmetric_generalized_operator(&reduced_elastic, &cholesky);
        jacobi_eigenpairs(normalized)
            .into_iter()
            .map(|(load_factor, normalized_shape)| {
                (
                    load_factor,
                    solve_upper_transpose(&cholesky, &normalized_shape),
                )
            })
            .collect()
    };
    let modes = physical_eigenpairs
        .into_iter()
        .filter(|(value, _)| value.is_finite() && *value > 1.0e-9)
        .take(mode_limit)
        .enumerate()
        .map(|(index, (load_factor, reduced_shape))| {
            let residual_norm = generalized_residual(
                &reduced_elastic,
                &reduced_geometric,
                &reduced_shape,
                load_factor,
            );
            let shape = expand_and_normalize(&reduced_shape, &free_dofs, dof_count);
            BucklingBeam1dModeResult {
                index,
                load_factor,
                residual_norm,
                shape,
            }
        })
        .collect::<Vec<_>>();
    if modes.is_empty() {
        return Err("buckling beam 1d did not produce a positive finite mode".to_string());
    }
    Ok(SolveBucklingBeam1dResult {
        input: request.clone(),
        minimum_load_factor: modes[0].load_factor,
        modes,
        free_dofs,
    })
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
        let residual = elastic_product
            .iter()
            .zip(&next_geometric_product)
            .map(|(elastic, geometric)| elastic - load_factor * geometric)
            .collect::<Vec<_>>();
        let scale = l2_norm(&elastic_product)
            .max(load_factor.abs() * l2_norm(&next_geometric_product))
            .max(1.0);
        let relative_residual = l2_norm(&residual) / scale;
        let factor_change = (load_factor - previous_factor).abs() / load_factor.abs().max(1.0);
        shape = next;
        if relative_residual <= 1.0e-6 && factor_change <= 1.0e-8 {
            return Ok((load_factor, shape));
        }
        previous_factor = load_factor;
    }
    Err("buckling inverse iteration did not converge within 256 iterations".to_string())
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

fn validate(request: &SolveBucklingBeam1dRequest) -> Result<(), String> {
    if request.nodes.len() < 2 || request.elements.is_empty() {
        return Err("buckling beam 1d requires at least two nodes and one element".to_string());
    }
    if request.nodes.iter().any(|node| !node.x.is_finite()) {
        return Err("buckling beam 1d node coordinates must be finite".to_string());
    }
    let mut node_ids = HashSet::new();
    if request
        .nodes
        .iter()
        .any(|node| node.id.is_empty() || !node_ids.insert(node.id.as_str()))
    {
        return Err("buckling beam 1d node ids must be non-empty and unique".to_string());
    }
    let mut element_ids = HashSet::new();
    for element in &request.elements {
        if element.id.is_empty() || !element_ids.insert(element.id.as_str()) {
            return Err("buckling beam 1d element ids must be non-empty and unique".to_string());
        }
        if element.node_i >= request.nodes.len()
            || element.node_j >= request.nodes.len()
            || element.node_i == element.node_j
        {
            return Err("buckling beam 1d element topology is invalid".to_string());
        }
        let length = (request.nodes[element.node_j].x - request.nodes[element.node_i].x).abs();
        if !(length.is_finite() && length > 1.0e-12) {
            return Err("buckling beam 1d element length must be positive".to_string());
        }
        for (label, value) in [
            ("youngs_modulus", element.youngs_modulus),
            ("moment_of_inertia", element.moment_of_inertia),
            (
                "reference_compressive_force",
                element.reference_compressive_force,
            ),
        ] {
            if !(value.is_finite() && value > 0.0) {
                return Err(format!("buckling beam 1d {label} must be positive"));
            }
        }
    }
    if constrained_dofs(request).len() < 2 {
        return Err("buckling beam 1d must restrain at least two degrees of freedom".to_string());
    }
    Ok(())
}

fn elastic_stiffness(ei: f64, length: f64) -> [[f64; 4]; 4] {
    let l2 = length * length;
    let factor = ei / length.powi(3);
    [
        [12.0, 6.0 * length, -12.0, 6.0 * length],
        [6.0 * length, 4.0 * l2, -6.0 * length, 2.0 * l2],
        [-12.0, -6.0 * length, 12.0, -6.0 * length],
        [6.0 * length, 2.0 * l2, -6.0 * length, 4.0 * l2],
    ]
    .map(|row| row.map(|value| value * factor))
}

fn geometric_stiffness(force: f64, length: f64) -> [[f64; 4]; 4] {
    let l2 = length * length;
    let factor = force / (30.0 * length);
    [
        [36.0, 3.0 * length, -36.0, 3.0 * length],
        [3.0 * length, 4.0 * l2, -3.0 * length, -l2],
        [-36.0, -3.0 * length, 36.0, -3.0 * length],
        [3.0 * length, -l2, -3.0 * length, 4.0 * l2],
    ]
    .map(|row| row.map(|value| value * factor))
}

fn constrained_dofs(request: &SolveBucklingBeam1dRequest) -> Vec<usize> {
    request
        .nodes
        .iter()
        .enumerate()
        .flat_map(|(index, node)| {
            [
                node.fix_y.then_some(index * 2),
                node.fix_rz.then_some(index * 2 + 1),
            ]
            .into_iter()
            .flatten()
        })
        .collect()
}

fn reduce_dense(matrix: &[Vec<f64>], free: &[usize]) -> Vec<Vec<f64>> {
    free.iter()
        .map(|&row| free.iter().map(|&column| matrix[row][column]).collect())
        .collect()
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
                    return Err(
                        "buckling reference geometric stiffness is not positive definite"
                            .to_string(),
                    );
                }
                lower[row][column] = diagonal.sqrt();
            } else {
                lower[row][column] = (matrix[row][column] - sum) / lower[column][column];
            }
        }
    }
    Ok(lower)
}

fn symmetric_generalized_operator(stiffness: &[Vec<f64>], lower: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let size = stiffness.len();
    let mut left = vec![vec![0.0; size]; size];
    for column in 0..size {
        let rhs = (0..size)
            .map(|row| stiffness[row][column])
            .collect::<Vec<_>>();
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

fn expand_and_normalize(reduced: &[f64], free: &[usize], size: usize) -> Vec<f64> {
    let mut shape = vec![0.0; size];
    for (index, &dof) in free.iter().enumerate() {
        shape[dof] = reduced[index];
    }
    let norm = shape.iter().map(|value| value * value).sum::<f64>().sqrt();
    shape.iter_mut().for_each(|value| *value /= norm);
    shape
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
