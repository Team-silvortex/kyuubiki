use crate::frame_3d_math::{
    frame3d_dof_map, frame3d_local_stiffness, frame3d_rotation, frame3d_transform,
    multiply_matrix_vector_12x12, transform_frame3d_stiffness,
};
use crate::linear_algebra::{SparseMatrix, add_at, reduce_sparse_system, solve_spd_system};
use kyuubiki_protocol::{
    Frame3dElementResult, Frame3dNodeResult, SolveFrame3dRequest, SolveFrame3dResult,
};

pub fn solve_frame_3d(request: &SolveFrame3dRequest) -> Result<SolveFrame3dResult, String> {
    validate_frame_3d_request(request)?;

    let dof_count = request.nodes.len() * 6;
    let mut global_stiffness = SparseMatrix::new(dof_count);
    let mut force_vector = vec![0.0; dof_count];

    for (index, node) in request.nodes.iter().enumerate() {
        force_vector[index * 6] = node.load_x;
        force_vector[index * 6 + 1] = node.load_y;
        force_vector[index * 6 + 2] = node.load_z;
        force_vector[index * 6 + 3] = node.moment_x;
        force_vector[index * 6 + 4] = node.moment_y;
        force_vector[index * 6 + 5] = node.moment_z;
    }

    for element in &request.elements {
        let node_i = &request.nodes[element.node_i];
        let node_j = &request.nodes[element.node_j];
        let dx = node_j.x - node_i.x;
        let dy = node_j.y - node_i.y;
        let dz = node_j.z - node_i.z;
        let length = (dx * dx + dy * dy + dz * dz).sqrt();
        let rotation = frame3d_rotation(dx, dy, dz, length)?;
        let local_stiffness = frame3d_local_stiffness(
            element.area,
            element.youngs_modulus,
            element.shear_modulus,
            element.torsion_constant,
            element.moment_of_inertia_y,
            element.moment_of_inertia_z,
            length,
        );
        let transform = frame3d_transform(&rotation);
        let global_element_stiffness = transform_frame3d_stiffness(&local_stiffness, &transform);
        let map = frame3d_dof_map(element.node_i, element.node_j);

        for row in 0..12 {
            for column in 0..12 {
                add_at(
                    &mut global_stiffness,
                    map[row],
                    map[column],
                    global_element_stiffness[row][column],
                );
            }
        }
    }

    let constrained = constrained_frame_3d_dofs(request);
    let (reduced_stiffness, reduced_force, free) =
        reduce_sparse_system(&global_stiffness, &force_vector, &constrained);
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
            let ux = displacements[index * 6];
            let uy = displacements[index * 6 + 1];
            let uz = displacements[index * 6 + 2];
            let rx = displacements[index * 6 + 3];
            let ry = displacements[index * 6 + 4];
            let rz = displacements[index * 6 + 5];

            Frame3dNodeResult {
                index,
                id: node.id.clone(),
                x: node.x,
                y: node.y,
                z: node.z,
                ux,
                uy,
                uz,
                rx,
                ry,
                rz,
                displacement_magnitude: (ux * ux + uy * uy + uz * uz).sqrt(),
                rotation_magnitude: (rx * rx + ry * ry + rz * rz).sqrt(),
            }
        })
        .collect::<Vec<_>>();

    let elements = request
        .elements
        .iter()
        .enumerate()
        .map(|(index, element)| {
            let node_i = &request.nodes[element.node_i];
            let node_j = &request.nodes[element.node_j];
            let dx = node_j.x - node_i.x;
            let dy = node_j.y - node_i.y;
            let dz = node_j.z - node_i.z;
            let length = (dx * dx + dy * dy + dz * dz).sqrt();
            let rotation = frame3d_rotation(dx, dy, dz, length)
                .expect("validated 3d frame element should define a stable local axis");
            let local_stiffness = frame3d_local_stiffness(
                element.area,
                element.youngs_modulus,
                element.shear_modulus,
                element.torsion_constant,
                element.moment_of_inertia_y,
                element.moment_of_inertia_z,
                length,
            );
            let transform = frame3d_transform(&rotation);
            let map = frame3d_dof_map(element.node_i, element.node_j);
            let global_displacements = std::array::from_fn(|i| displacements[map[i]]);
            let local_displacements =
                multiply_matrix_vector_12x12(&transform, &global_displacements);
            let local_forces = multiply_matrix_vector_12x12(&local_stiffness, &local_displacements);
            let strain_energy = frame3d_strain_energy(&local_forces, &local_displacements);
            let axial_stress = local_forces[0].abs().max(local_forces[6].abs()) / element.area;
            let bending_stress_y =
                local_forces[4].abs().max(local_forces[10].abs()) / element.section_modulus_y;
            let bending_stress_z =
                local_forces[5].abs().max(local_forces[11].abs()) / element.section_modulus_z;
            let max_bending_stress = bending_stress_y + bending_stress_z;
            let max_combined_stress = axial_stress + max_bending_stress;

            Frame3dElementResult {
                index,
                id: element.id.clone(),
                node_i: element.node_i,
                node_j: element.node_j,
                length,
                axial_force_i: local_forces[0],
                shear_force_y_i: local_forces[1],
                shear_force_z_i: local_forces[2],
                torsion_i: local_forces[3],
                moment_y_i: local_forces[4],
                moment_z_i: local_forces[5],
                axial_force_j: local_forces[6],
                shear_force_y_j: local_forces[7],
                shear_force_z_j: local_forces[8],
                torsion_j: local_forces[9],
                moment_y_j: local_forces[10],
                moment_z_j: local_forces[11],
                axial_stress,
                max_bending_stress,
                max_combined_stress,
                strain_energy,
            }
        })
        .collect::<Vec<_>>();

    Ok(SolveFrame3dResult {
        input: request.clone(),
        max_displacement: nodes
            .iter()
            .map(|node| node.displacement_magnitude)
            .fold(0.0_f64, f64::max),
        max_rotation: nodes
            .iter()
            .map(|node| node.rotation_magnitude)
            .fold(0.0_f64, f64::max),
        max_moment: elements
            .iter()
            .flat_map(|element| {
                [
                    element.moment_y_i.abs(),
                    element.moment_z_i.abs(),
                    element.moment_y_j.abs(),
                    element.moment_z_j.abs(),
                ]
            })
            .fold(0.0_f64, f64::max),
        max_stress: elements
            .iter()
            .map(|element| element.max_combined_stress)
            .fold(0.0_f64, f64::max),
        total_strain_energy: elements.iter().map(|element| element.strain_energy).sum(),
        nodes,
        elements,
    })
}

pub(super) fn validate_frame_3d_request(request: &SolveFrame3dRequest) -> Result<(), String> {
    if request.nodes.len() < 2 {
        return Err("3d frame must define at least two nodes".to_string());
    }
    if request.elements.is_empty() {
        return Err("3d frame must define at least one element".to_string());
    }

    let constrained_dofs = request.nodes.iter().fold(0, |sum, node| {
        sum + usize::from(node.fix_x)
            + usize::from(node.fix_y)
            + usize::from(node.fix_z)
            + usize::from(node.fix_rx)
            + usize::from(node.fix_ry)
            + usize::from(node.fix_rz)
    });
    if constrained_dofs < 6 {
        return Err("3d frame must restrain at least six degrees of freedom".to_string());
    }

    for element in &request.elements {
        validate_frame_3d_element(request, element)?;
    }

    Ok(())
}

pub(super) fn constrained_frame_3d_dofs(request: &SolveFrame3dRequest) -> Vec<usize> {
    request
        .nodes
        .iter()
        .enumerate()
        .flat_map(|(index, node)| {
            [
                node.fix_x.then_some(index * 6),
                node.fix_y.then_some(index * 6 + 1),
                node.fix_z.then_some(index * 6 + 2),
                node.fix_rx.then_some(index * 6 + 3),
                node.fix_ry.then_some(index * 6 + 4),
                node.fix_rz.then_some(index * 6 + 5),
            ]
            .into_iter()
            .flatten()
        })
        .collect()
}

fn validate_frame_3d_element(
    request: &SolveFrame3dRequest,
    element: &kyuubiki_protocol::Frame3dElementInput,
) -> Result<(), String> {
    if element.node_i >= request.nodes.len() || element.node_j >= request.nodes.len() {
        return Err("3d frame element references an out-of-range node".to_string());
    }
    if element.node_i == element.node_j {
        return Err("3d frame element must connect two distinct nodes".to_string());
    }
    if !(element.area.is_finite() && element.area > 0.0) {
        return Err("3d frame element area must be positive".to_string());
    }
    if !(element.youngs_modulus.is_finite() && element.youngs_modulus > 0.0) {
        return Err("3d frame element youngs_modulus must be positive".to_string());
    }
    if !(element.shear_modulus.is_finite() && element.shear_modulus > 0.0) {
        return Err("3d frame element shear_modulus must be positive".to_string());
    }
    if !(element.torsion_constant.is_finite() && element.torsion_constant > 0.0) {
        return Err("3d frame element torsion_constant must be positive".to_string());
    }
    if !(element.moment_of_inertia_y.is_finite() && element.moment_of_inertia_y > 0.0) {
        return Err("3d frame element moment_of_inertia_y must be positive".to_string());
    }
    if !(element.moment_of_inertia_z.is_finite() && element.moment_of_inertia_z > 0.0) {
        return Err("3d frame element moment_of_inertia_z must be positive".to_string());
    }
    if !(element.section_modulus_y.is_finite() && element.section_modulus_y > 0.0) {
        return Err("3d frame element section_modulus_y must be positive".to_string());
    }
    if !(element.section_modulus_z.is_finite() && element.section_modulus_z > 0.0) {
        return Err("3d frame element section_modulus_z must be positive".to_string());
    }

    let node_i = &request.nodes[element.node_i];
    let node_j = &request.nodes[element.node_j];
    let dx = node_j.x - node_i.x;
    let dy = node_j.y - node_i.y;
    let dz = node_j.z - node_i.z;
    let length = (dx * dx + dy * dy + dz * dz).sqrt();
    frame3d_rotation(dx, dy, dz, length)?;

    Ok(())
}

fn frame3d_strain_energy(local_forces: &[f64; 12], local_displacements: &[f64; 12]) -> f64 {
    0.5 * (0..12)
        .map(|index| local_forces[index] * local_displacements[index])
        .sum::<f64>()
}
