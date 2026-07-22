use crate::buckling_math::{generalized_eigenpairs, reduce_dense};
use crate::frame_2d::solve_frame_2d;
use crate::frame_2d_math::{frame_local_stiffness, frame_transform, transform_frame_stiffness};
use crate::modal_math::ensure_dense_modal_size;
use kyuubiki_protocol::{
    BucklingFrame2dElementPreloadResult, BucklingFrame2dModeResult, SolveBucklingFrame2dRequest,
    SolveBucklingFrame2dResult,
};

pub fn solve_buckling_frame_2d(
    request: &SolveBucklingFrame2dRequest,
) -> Result<SolveBucklingFrame2dResult, String> {
    let static_result = solve_frame_2d(&request.frame)?;
    let dof_count = request.frame.nodes.len() * 3;
    ensure_dense_modal_size(dof_count, "buckling frame 2d")?;
    let mut elastic = vec![vec![0.0; dof_count]; dof_count];
    let mut geometric = vec![vec![0.0; dof_count]; dof_count];
    let mut element_preloads = Vec::with_capacity(request.frame.elements.len());

    for (index, element) in request.frame.elements.iter().enumerate() {
        let node_i = &request.frame.nodes[element.node_i];
        let node_j = &request.frame.nodes[element.node_j];
        let dx = node_j.x - node_i.x;
        let dy = node_j.y - node_i.y;
        let length = (dx * dx + dy * dy).sqrt();
        let transform = frame_transform(dx / length, dy / length);
        let local_elastic = frame_local_stiffness(
            element.area,
            element.youngs_modulus,
            element.moment_of_inertia,
            length,
        );
        let global_elastic = transform_frame_stiffness(&local_elastic, &transform);
        let static_element = &static_result.elements[index];
        let signed_axial_force =
            0.5 * (static_element.axial_force_i - static_element.axial_force_j);
        let reference_compressive_force = signed_axial_force.max(0.0);
        let active = reference_compressive_force > 1.0e-12;
        let local_geometric = geometric_stiffness(reference_compressive_force, length);
        let global_geometric = transform_frame_stiffness(&local_geometric, &transform);
        let map = frame_dof_map(element.node_i, element.node_j);
        assemble(&mut elastic, &global_elastic, &map);
        assemble(&mut geometric, &global_geometric, &map);
        element_preloads.push(BucklingFrame2dElementPreloadResult {
            index,
            id: element.id.clone(),
            signed_axial_force,
            reference_compressive_force,
            active_in_geometric_stiffness: active,
        });
    }
    if !element_preloads
        .iter()
        .any(|preload| preload.active_in_geometric_stiffness)
    {
        return Err("buckling frame 2d reference load produces no compressive member force".into());
    }

    let constrained = constrained_dofs(request);
    let free_dofs = (0..dof_count)
        .filter(|dof| !constrained.contains(dof))
        .collect::<Vec<_>>();
    let reduced_elastic = reduce_dense(&elastic, &free_dofs);
    let reduced_geometric = reduce_dense(&geometric, &free_dofs);
    let mode_limit = request.mode_count.unwrap_or(3).max(1).min(free_dofs.len());
    let modes = generalized_eigenpairs(&reduced_elastic, &reduced_geometric, mode_limit)?
        .into_iter()
        .enumerate()
        .map(|(index, pair)| BucklingFrame2dModeResult {
            index,
            load_factor: pair.eigenvalue,
            residual_norm: pair.residual_norm,
            shape: expand_and_normalize(&pair.vector, &free_dofs, dof_count),
        })
        .collect::<Vec<_>>();

    Ok(SolveBucklingFrame2dResult {
        input: request.clone(),
        minimum_load_factor: modes[0].load_factor,
        static_result,
        element_preloads,
        modes,
        free_dofs,
    })
}

fn geometric_stiffness(force: f64, length: f64) -> [[f64; 6]; 6] {
    let l2 = length * length;
    let factor = force / (30.0 * length);
    let mut stiffness = [[0.0; 6]; 6];
    let bending = [
        [36.0, 3.0 * length, -36.0, 3.0 * length],
        [3.0 * length, 4.0 * l2, -3.0 * length, -l2],
        [-36.0, -3.0 * length, 36.0, -3.0 * length],
        [3.0 * length, -l2, -3.0 * length, 4.0 * l2],
    ];
    let bending_dofs = [1, 2, 4, 5];
    for row in 0..4 {
        for column in 0..4 {
            stiffness[bending_dofs[row]][bending_dofs[column]] = bending[row][column] * factor;
        }
    }
    stiffness
}

fn assemble(global: &mut [Vec<f64>], element: &[[f64; 6]; 6], map: &[usize; 6]) {
    for row in 0..6 {
        for column in 0..6 {
            global[map[row]][map[column]] += element[row][column];
        }
    }
}

fn frame_dof_map(node_i: usize, node_j: usize) -> [usize; 6] {
    [
        node_i * 3,
        node_i * 3 + 1,
        node_i * 3 + 2,
        node_j * 3,
        node_j * 3 + 1,
        node_j * 3 + 2,
    ]
}

fn constrained_dofs(request: &SolveBucklingFrame2dRequest) -> Vec<usize> {
    request
        .frame
        .nodes
        .iter()
        .enumerate()
        .flat_map(|(index, node)| {
            [
                node.fix_x.then_some(index * 3),
                node.fix_y.then_some(index * 3 + 1),
                node.fix_rz.then_some(index * 3 + 2),
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
    if norm > 0.0 {
        shape.iter_mut().for_each(|value| *value /= norm);
    }
    shape
}
