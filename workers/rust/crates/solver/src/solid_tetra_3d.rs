use crate::linear_algebra::{SparseMatrix, reduce_sparse_system, solve_spd_system};
use crate::solid_tetra_3d_element::{SolidTetra3dElementKernel, element_dof_map};
use crate::solid_tetra_3d_validation::validate_request;
use kyuubiki_protocol::{
    SolidTetra3dNodeResult, SolveSolidTetra3dRequest, SolveSolidTetra3dResult,
};

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
        nodes,
        elements,
    })
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
