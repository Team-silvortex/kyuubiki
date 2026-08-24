use crate::linear_algebra::{SparseMatrix, reduce_sparse_system, solve_spd_system};
use crate::solid_tetra_3d_element::{SolidTetra3dElementKernel, element_dof_map};
use crate::solid_tetra_3d_validation::{mesh_component_count, validate_request};
use kyuubiki_protocol::{
    SolidTetra3dElementResult, SolidTetra3dEquilibriumResult, SolidTetra3dNodeResult,
    SolidTetra3dQualityResult, SolveSolidTetra3dRequest, SolveSolidTetra3dResult,
};

const DISTORTION_WATCH_THRESHOLD: f64 = 0.20;
const SEVERE_DISTORTION_THRESHOLD: f64 = 0.05;
const NEAR_INCOMPRESSIBLE_POISSON_THRESHOLD: f64 = 0.45;

pub fn solve_solid_tetra_3d(
    request: &SolveSolidTetra3dRequest,
) -> Result<SolveSolidTetra3dResult, String> {
    validate_request(request)?;

    let kernels = request
        .elements
        .iter()
        .map(|element| SolidTetra3dElementKernel::new(element_points(request, element), element))
        .collect::<Result<Vec<_>, _>>()?;
    let dof_count = request.nodes.len() * 3;
    let mut stiffness = SparseMatrix::with_uniform_row_capacity(dof_count, 36);
    let mut force = vec![0.0; dof_count];
    let zero_displacements = vec![0.0; dof_count];
    let mut zero_internal = vec![0.0; dof_count];

    for (index, node) in request.nodes.iter().enumerate() {
        force[index * 3] = node.load_x;
        force[index * 3 + 1] = node.load_y;
        force[index * 3 + 2] = node.load_z;
    }
    for (element, kernel) in request.elements.iter().zip(&kernels) {
        kernel.assemble(
            &element_dof_map(element),
            &zero_displacements,
            &mut zero_internal,
            &mut stiffness,
        );
    }

    let constrained = constrained_dofs(request);
    let (reduced_stiffness, reduced_force, free) =
        reduce_sparse_system(&stiffness, &force, &constrained);
    let reduced_displacements = solve_spd_system(&reduced_stiffness, &reduced_force)?;
    let mut displacements = vec![0.0; dof_count];
    for (index, &dof) in free.iter().enumerate() {
        displacements[dof] = reduced_displacements[index];
    }
    let (reactions, equilibrium) =
        recover_equilibrium(request, &stiffness, &force, &displacements, &constrained);

    let nodes = request
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| {
            let ux = displacements[index * 3];
            let uy = displacements[index * 3 + 1];
            let uz = displacements[index * 3 + 2];
            SolidTetra3dNodeResult {
                index,
                id: node.id.clone(),
                x: node.x,
                y: node.y,
                z: node.z,
                ux,
                uy,
                uz,
                displacement_magnitude: (ux * ux + uy * uy + uz * uz).sqrt(),
                reaction_x: reactions[index][0],
                reaction_y: reactions[index][1],
                reaction_z: reactions[index][2],
            }
        })
        .collect::<Vec<_>>();
    let elements = request
        .elements
        .iter()
        .zip(&kernels)
        .enumerate()
        .map(|(index, (element, kernel))| {
            kernel.result(index, element, &element_dof_map(element), &displacements)
        })
        .collect::<Vec<_>>();
    let quality = summarize_quality(request, &elements);

    Ok(SolveSolidTetra3dResult {
        input: request.clone(),
        total_volume: elements.iter().map(|element| element.volume).sum(),
        max_displacement: nodes
            .iter()
            .map(|node| node.displacement_magnitude)
            .fold(0.0_f64, f64::max),
        max_von_mises_stress: elements
            .iter()
            .map(|element| element.von_mises_stress)
            .fold(0.0_f64, f64::max),
        total_strain_energy: elements
            .iter()
            .map(|element| element.strain_energy_density * element.volume)
            .sum(),
        max_strain_energy_density: elements
            .iter()
            .map(|element| element.strain_energy_density.abs())
            .fold(0.0_f64, f64::max),
        equilibrium,
        quality,
        nodes,
        elements,
    })
}

fn summarize_quality(
    request: &SolveSolidTetra3dRequest,
    elements: &[SolidTetra3dElementResult],
) -> SolidTetra3dQualityResult {
    let minimum_mean_ratio_quality = elements
        .iter()
        .map(|element| element.mean_ratio_quality)
        .reduce(f64::min)
        .unwrap_or(0.0);
    let distorted_element_count = elements
        .iter()
        .filter(|element| element.mean_ratio_quality < DISTORTION_WATCH_THRESHOLD)
        .count();
    let severely_distorted_element_count = elements
        .iter()
        .filter(|element| element.mean_ratio_quality < SEVERE_DISTORTION_THRESHOLD)
        .count();
    let near_incompressible_element_count = request
        .elements
        .iter()
        .filter(|element| element.poisson_ratio >= NEAR_INCOMPRESSIBLE_POISSON_THRESHOLD)
        .count();
    let mut watch_terms = Vec::new();
    if distorted_element_count > 0 {
        watch_terms.push("distorted_constant_strain_tetrahedra".to_string());
    }
    if severely_distorted_element_count > 0 {
        watch_terms.push("severely_distorted_constant_strain_tetrahedra".to_string());
    }
    if near_incompressible_element_count > 0 {
        watch_terms.push("near_incompressible_volumetric_locking_risk".to_string());
    }
    SolidTetra3dQualityResult {
        connected_component_count: mesh_component_count(request),
        minimum_mean_ratio_quality,
        distortion_watch_threshold: DISTORTION_WATCH_THRESHOLD,
        severe_distortion_threshold: SEVERE_DISTORTION_THRESHOLD,
        near_incompressible_poisson_threshold: NEAR_INCOMPRESSIBLE_POISSON_THRESHOLD,
        distorted_element_count,
        severely_distorted_element_count,
        near_incompressible_element_count,
        watch_terms,
    }
}

fn recover_equilibrium(
    request: &SolveSolidTetra3dRequest,
    stiffness: &SparseMatrix,
    applied: &[f64],
    displacements: &[f64],
    constrained: &[usize],
) -> (Vec<[f64; 3]>, SolidTetra3dEquilibriumResult) {
    let mut internal = vec![0.0; stiffness.size()];
    for (row, value) in internal.iter_mut().enumerate() {
        *value = stiffness
            .row_entries(row)
            .iter()
            .map(|&(column, coefficient)| coefficient * displacements[column])
            .sum();
    }

    let mut constrained_mask = vec![false; stiffness.size()];
    for &dof in constrained {
        constrained_mask[dof] = true;
    }

    let mut reactions = vec![[0.0; 3]; request.nodes.len()];
    let mut reaction_force = [0.0; 3];
    let mut max_free_residual_force = 0.0_f64;
    for node in 0..request.nodes.len() {
        let mut free_residual = [0.0; 3];
        for axis in 0..3 {
            let dof = node * 3 + axis;
            let residual = internal[dof] - applied[dof];
            if constrained_mask[dof] {
                reactions[node][axis] = residual;
                reaction_force[axis] += residual;
            } else {
                free_residual[axis] = residual;
            }
        }
        max_free_residual_force = max_free_residual_force.max(vector_norm(free_residual));
    }

    let applied_force = request.nodes.iter().fold([0.0; 3], |mut total, node| {
        total[0] += node.load_x;
        total[1] += node.load_y;
        total[2] += node.load_z;
        total
    });
    let applied_force_scale = request
        .nodes
        .iter()
        .map(|node| vector_norm([node.load_x, node.load_y, node.load_z]))
        .sum::<f64>();
    let balance_error = std::array::from_fn(|axis| applied_force[axis] + reaction_force[axis]);
    let scale = applied_force_scale.max(1.0e-30);
    let equilibrium = SolidTetra3dEquilibriumResult {
        applied_force,
        reaction_force,
        balance_error,
        applied_force_scale,
        max_free_residual_force,
        free_residual_relative_error: max_free_residual_force / scale,
        force_balance_relative_error: vector_norm(balance_error) / scale,
    };
    (reactions, equilibrium)
}

fn vector_norm(vector: [f64; 3]) -> f64 {
    vector.iter().map(|value| value * value).sum::<f64>().sqrt()
}

fn element_points(
    request: &SolveSolidTetra3dRequest,
    element: &kyuubiki_protocol::SolidTetra3dElementInput,
) -> [[f64; 3]; 4] {
    [
        element.node_a,
        element.node_b,
        element.node_c,
        element.node_d,
    ]
    .map(|index| {
        let node = &request.nodes[index];
        [node.x, node.y, node.z]
    })
}

fn constrained_dofs(request: &SolveSolidTetra3dRequest) -> Vec<usize> {
    request
        .nodes
        .iter()
        .enumerate()
        .flat_map(|(index, node)| {
            [node.fix_x, node.fix_y, node.fix_z]
                .into_iter()
                .enumerate()
                .filter_map(move |(axis, fixed)| fixed.then_some(index * 3 + axis))
        })
        .collect()
}
