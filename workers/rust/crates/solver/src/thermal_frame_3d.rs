use crate::frame_3d_math::{
    add_vector_12, frame3d_dof_map, frame3d_local_stiffness, frame3d_rotation,
    frame3d_thermal_gradient_vector, frame3d_thermal_uniform_vector, frame3d_transform,
    multiply_matrix_vector_12x12, subtract_vector_12, transform_frame3d_stiffness, transpose_12x12,
};
use crate::linear_algebra::{SparseMatrix, add_at, reduce_sparse_system, solve_spd_system};
use kyuubiki_protocol::{
    SolveThermalFrame3dRequest, SolveThermalFrame3dResult, ThermalFrame3dElementResult,
    ThermalFrame3dNodeResult,
};

pub fn solve_thermal_frame_3d(
    request: &SolveThermalFrame3dRequest,
) -> Result<SolveThermalFrame3dResult, String> {
    validate_thermal_frame_3d_request(request)?;

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
        let local_stiffness = local_stiffness_for(element, length);
        let transform = frame3d_transform(&rotation);
        let global_element_stiffness = transform_frame3d_stiffness(&local_stiffness, &transform);
        let equivalent_local = equivalent_thermal_load(element, node_i, node_j);
        let equivalent_global =
            multiply_matrix_vector_12x12(&transpose_12x12(&transform), &equivalent_local);
        let map = frame3d_dof_map(element.node_i, element.node_j);

        for row in 0..12 {
            force_vector[map[row]] += equivalent_global[row];
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

    let constrained = constrained_thermal_frame_3d_dofs(request);
    let (reduced_stiffness, reduced_force, free) =
        reduce_sparse_system(&global_stiffness, &force_vector, &constrained);
    let reduced_displacements = solve_spd_system(&reduced_stiffness, &reduced_force)?;

    let mut displacements = vec![0.0; dof_count];
    for (index, &dof) in free.iter().enumerate() {
        displacements[dof] = reduced_displacements[index];
    }

    let nodes = build_thermal_frame_3d_nodes(request, &displacements);
    let elements = build_thermal_frame_3d_elements(request, &displacements);

    Ok(SolveThermalFrame3dResult {
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
        max_axial_force: elements
            .iter()
            .flat_map(|element| [element.axial_force_i.abs(), element.axial_force_j.abs()])
            .fold(0.0_f64, f64::max),
        max_temperature_delta: nodes
            .iter()
            .map(|node| node.temperature_delta.abs())
            .fold(0.0_f64, f64::max),
        max_temperature_gradient: elements
            .iter()
            .flat_map(|element| {
                [
                    element.temperature_gradient_y.abs(),
                    element.temperature_gradient_z.abs(),
                ]
            })
            .fold(0.0_f64, f64::max),
        nodes,
        elements,
    })
}

fn build_thermal_frame_3d_nodes(
    request: &SolveThermalFrame3dRequest,
    displacements: &[f64],
) -> Vec<ThermalFrame3dNodeResult> {
    request
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

            ThermalFrame3dNodeResult {
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
                temperature_delta: node.temperature_delta,
            }
        })
        .collect()
}

fn build_thermal_frame_3d_elements(
    request: &SolveThermalFrame3dRequest,
    displacements: &[f64],
) -> Vec<ThermalFrame3dElementResult> {
    request
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
                .expect("validated thermal 3d frame element should define a stable local axis");
            let local_stiffness = local_stiffness_for(element, length);
            let transform = frame3d_transform(&rotation);
            let map = frame3d_dof_map(element.node_i, element.node_j);
            let global_displacements = std::array::from_fn(|i| displacements[map[i]]);
            let local_displacements =
                multiply_matrix_vector_12x12(&transform, &global_displacements);
            let equivalent_local = equivalent_thermal_load(element, node_i, node_j);
            let local_forces = subtract_vector_12(
                &multiply_matrix_vector_12x12(&local_stiffness, &local_displacements),
                &equivalent_local,
            );
            let average_temperature_delta =
                0.5 * (node_i.temperature_delta + node_j.temperature_delta);
            let thermal_strain = element.thermal_expansion * average_temperature_delta;
            let total_strain = (local_displacements[6] - local_displacements[0]) / length;
            let axial_stress = local_forces[0].abs().max(local_forces[6].abs()) / element.area;
            let bending_stress_y =
                local_forces[4].abs().max(local_forces[10].abs()) / element.section_modulus_y;
            let bending_stress_z =
                local_forces[5].abs().max(local_forces[11].abs()) / element.section_modulus_z;
            let max_bending_stress = bending_stress_y + bending_stress_z;

            ThermalFrame3dElementResult {
                index,
                id: element.id.clone(),
                node_i: element.node_i,
                node_j: element.node_j,
                length,
                average_temperature_delta,
                thermal_strain,
                mechanical_strain: total_strain - thermal_strain,
                total_strain,
                temperature_gradient_y: element.temperature_gradient_y,
                temperature_gradient_z: element.temperature_gradient_z,
                thermal_curvature_y: element.thermal_expansion * element.temperature_gradient_y
                    / element.section_depth_y,
                thermal_curvature_z: element.thermal_expansion * element.temperature_gradient_z
                    / element.section_depth_z,
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
                max_combined_stress: axial_stress + max_bending_stress,
            }
        })
        .collect()
}

fn validate_thermal_frame_3d_request(request: &SolveThermalFrame3dRequest) -> Result<(), String> {
    if request.nodes.len() < 2 {
        return Err("thermal 3d frame must define at least two nodes".to_string());
    }
    if request.elements.is_empty() {
        return Err("thermal 3d frame must define at least one element".to_string());
    }

    let constrained_dofs = constrained_thermal_frame_3d_dofs(request).len();
    if constrained_dofs < 6 {
        return Err("thermal 3d frame must restrain at least six degrees of freedom".to_string());
    }

    for node in &request.nodes {
        if !node.temperature_delta.is_finite() {
            return Err("thermal 3d frame node temperature_delta must be finite".to_string());
        }
    }

    for element in &request.elements {
        validate_thermal_frame_3d_element(request, element)?;
    }

    Ok(())
}

fn validate_thermal_frame_3d_element(
    request: &SolveThermalFrame3dRequest,
    element: &kyuubiki_protocol::ThermalFrame3dElementInput,
) -> Result<(), String> {
    if element.node_i >= request.nodes.len() || element.node_j >= request.nodes.len() {
        return Err("thermal 3d frame element references an out-of-range node".to_string());
    }
    if element.node_i == element.node_j {
        return Err("thermal 3d frame element cannot connect a node to itself".to_string());
    }
    validate_positive_frame_properties(element)?;
    if !(element.thermal_expansion.is_finite() && element.thermal_expansion >= 0.0) {
        return Err("thermal 3d frame element thermal_expansion must be non-negative".to_string());
    }
    if !(element.section_depth_y.is_finite() && element.section_depth_y > 0.0) {
        return Err("thermal 3d frame element section_depth_y must be positive".to_string());
    }
    if !(element.section_depth_z.is_finite() && element.section_depth_z > 0.0) {
        return Err("thermal 3d frame element section_depth_z must be positive".to_string());
    }
    if !element.temperature_gradient_y.is_finite() {
        return Err("thermal 3d frame element temperature_gradient_y must be finite".to_string());
    }
    if !element.temperature_gradient_z.is_finite() {
        return Err("thermal 3d frame element temperature_gradient_z must be finite".to_string());
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

fn validate_positive_frame_properties(
    element: &kyuubiki_protocol::ThermalFrame3dElementInput,
) -> Result<(), String> {
    if !(element.area.is_finite() && element.area > 0.0) {
        return Err("thermal 3d frame element area must be positive".to_string());
    }
    if !(element.youngs_modulus.is_finite() && element.youngs_modulus > 0.0) {
        return Err("thermal 3d frame element youngs_modulus must be positive".to_string());
    }
    if !(element.shear_modulus.is_finite() && element.shear_modulus > 0.0) {
        return Err("thermal 3d frame element shear_modulus must be positive".to_string());
    }
    if !(element.torsion_constant.is_finite() && element.torsion_constant > 0.0) {
        return Err("thermal 3d frame element torsion_constant must be positive".to_string());
    }
    if !(element.moment_of_inertia_y.is_finite() && element.moment_of_inertia_y > 0.0) {
        return Err("thermal 3d frame element moment_of_inertia_y must be positive".to_string());
    }
    if !(element.moment_of_inertia_z.is_finite() && element.moment_of_inertia_z > 0.0) {
        return Err("thermal 3d frame element moment_of_inertia_z must be positive".to_string());
    }
    if !(element.section_modulus_y.is_finite() && element.section_modulus_y > 0.0) {
        return Err("thermal 3d frame element section_modulus_y must be positive".to_string());
    }
    if !(element.section_modulus_z.is_finite() && element.section_modulus_z > 0.0) {
        return Err("thermal 3d frame element section_modulus_z must be positive".to_string());
    }
    Ok(())
}

fn constrained_thermal_frame_3d_dofs(request: &SolveThermalFrame3dRequest) -> Vec<usize> {
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

fn local_stiffness_for(
    element: &kyuubiki_protocol::ThermalFrame3dElementInput,
    length: f64,
) -> [[f64; 12]; 12] {
    frame3d_local_stiffness(
        element.area,
        element.youngs_modulus,
        element.shear_modulus,
        element.torsion_constant,
        element.moment_of_inertia_y,
        element.moment_of_inertia_z,
        length,
    )
}

fn equivalent_thermal_load(
    element: &kyuubiki_protocol::ThermalFrame3dElementInput,
    node_i: &kyuubiki_protocol::ThermalFrame3dNodeInput,
    node_j: &kyuubiki_protocol::ThermalFrame3dNodeInput,
) -> [f64; 12] {
    let average_temperature_delta = 0.5 * (node_i.temperature_delta + node_j.temperature_delta);
    add_vector_12(
        &frame3d_thermal_uniform_vector(
            element.area,
            element.youngs_modulus,
            element.thermal_expansion,
            average_temperature_delta,
        ),
        &frame3d_thermal_gradient_vector(
            element.youngs_modulus,
            element.moment_of_inertia_y,
            element.moment_of_inertia_z,
            element.thermal_expansion,
            element.section_depth_y,
            element.section_depth_z,
            element.temperature_gradient_y,
            element.temperature_gradient_z,
        ),
    )
}
