use crate::buckling_math::{generalized_eigenpairs, reduce_dense};
use crate::modal_math::ensure_dense_modal_size;
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
    let modes = generalized_eigenpairs(&reduced_elastic, &reduced_geometric, mode_limit)?
        .into_iter()
        .enumerate()
        .map(|(index, pair)| {
            let shape = expand_and_normalize(&pair.vector, &free_dofs, dof_count);
            BucklingBeam1dModeResult {
                index,
                load_factor: pair.eigenvalue,
                residual_norm: pair.residual_norm,
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

fn expand_and_normalize(reduced: &[f64], free: &[usize], size: usize) -> Vec<f64> {
    let mut shape = vec![0.0; size];
    for (index, &dof) in free.iter().enumerate() {
        shape[dof] = reduced[index];
    }
    let norm = shape.iter().map(|value| value * value).sum::<f64>().sqrt();
    shape.iter_mut().for_each(|value| *value /= norm);
    shape
}
