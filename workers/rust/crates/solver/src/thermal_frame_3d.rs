use crate::frame_3d_math::{
    add_vector_12, frame3d_dof_map, frame3d_local_stiffness, frame3d_rotation_with_local_y,
    frame3d_thermal_gradient_vector, frame3d_thermal_uniform_vector, frame3d_transform,
    multiply_matrix_vector_12x12, subtract_vector_12, transform_frame3d_stiffness, transpose_12x12,
};
use crate::frame_energy::thermal_frame3d_strain_energy;
use crate::linear_algebra::{
    SparseMatrix, add_at, reduce_sparse_system, solve_spd_system_profile_with_options,
};
use crate::linear_solver_profile::SpdSolveOptions;
use crate::thermal_frame_3d_validation::validate_request;
use kyuubiki_protocol::{
    SolveThermalFrame3dRequest, SolveThermalFrame3dResult, ThermalFrame3dDirectionalSpringResult,
    ThermalFrame3dElementResult, ThermalFrame3dNodeResult,
};

pub fn solve_thermal_frame_3d(
    request: &SolveThermalFrame3dRequest,
) -> Result<SolveThermalFrame3dResult, String> {
    solve_thermal_frame_3d_with_options(request, SpdSolveOptions::default())
}

pub fn solve_thermal_frame_3d_with_options(
    request: &SolveThermalFrame3dRequest,
    options: SpdSolveOptions,
) -> Result<SolveThermalFrame3dResult, String> {
    validate_request(request)?;

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

    for spring in &request.directional_springs {
        let direction = normalized_direction(spring.direction);
        for row in 0..3 {
            for column in 0..3 {
                add_at(
                    &mut global_stiffness,
                    spring.node * 6 + row,
                    spring.node * 6 + column,
                    spring.stiffness * direction[row] * direction[column],
                );
            }
        }
    }

    for element in &request.elements {
        let node_i = &request.nodes[element.node_i];
        let node_j = &request.nodes[element.node_j];
        let dx = node_j.x - node_i.x;
        let dy = node_j.y - node_i.y;
        let dz = node_j.z - node_i.z;
        let length = (dx * dx + dy * dy + dz * dz).sqrt();
        let rotation = frame3d_rotation_with_local_y(dx, dy, dz, length, element.local_y_axis)?;
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
    let reduced_displacements =
        solve_spd_system_profile_with_options(&reduced_stiffness, &reduced_force, options)?
            .solution;

    let mut displacements = vec![0.0; dof_count];
    for (index, &dof) in free.iter().enumerate() {
        displacements[dof] = reduced_displacements[index];
    }

    let nodes = build_thermal_frame_3d_nodes(request, &displacements);
    let elements = build_thermal_frame_3d_elements(request, &displacements);
    let directional_springs = build_directional_spring_results(request, &displacements);
    let total_strain_energy = elements
        .iter()
        .map(|element| element.strain_energy)
        .sum::<f64>()
        + directional_springs
            .iter()
            .map(|spring| spring.strain_energy)
            .sum::<f64>();

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
        total_strain_energy,
        nodes,
        elements,
        directional_springs,
    })
}

fn build_directional_spring_results(
    request: &SolveThermalFrame3dRequest,
    displacements: &[f64],
) -> Vec<ThermalFrame3dDirectionalSpringResult> {
    request
        .directional_springs
        .iter()
        .enumerate()
        .map(|(index, spring)| {
            let direction = normalized_direction(spring.direction);
            let offset = spring.node * 6;
            let displacement = direction[0] * displacements[offset]
                + direction[1] * displacements[offset + 1]
                + direction[2] * displacements[offset + 2];
            ThermalFrame3dDirectionalSpringResult {
                index,
                id: spring.id.clone(),
                node: spring.node,
                direction,
                displacement,
                reaction_force: -spring.stiffness * displacement,
                stiffness: spring.stiffness,
                strain_energy: 0.5 * spring.stiffness * displacement * displacement,
            }
        })
        .collect()
}

fn normalized_direction(direction: [f64; 3]) -> [f64; 3] {
    let norm = direction
        .iter()
        .map(|value| value * value)
        .sum::<f64>()
        .sqrt();
    [
        direction[0] / norm,
        direction[1] / norm,
        direction[2] / norm,
    ]
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
            let rotation = frame3d_rotation_with_local_y(dx, dy, dz, length, element.local_y_axis)
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
            let thermal_curvature_y = element.thermal_expansion * element.temperature_gradient_y
                / element.section_depth_y;
            let thermal_curvature_z = element.thermal_expansion * element.temperature_gradient_z
                / element.section_depth_z;
            let mechanical_strain = total_strain - thermal_strain;
            let strain_energy = thermal_frame3d_strain_energy(
                element.youngs_modulus,
                element.shear_modulus,
                element.area,
                element.torsion_constant,
                element.moment_of_inertia_y,
                element.moment_of_inertia_z,
                length,
                &local_displacements,
                mechanical_strain,
                thermal_curvature_y,
                thermal_curvature_z,
            );
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
                mechanical_strain,
                total_strain,
                temperature_gradient_y: element.temperature_gradient_y,
                temperature_gradient_z: element.temperature_gradient_z,
                thermal_curvature_y,
                thermal_curvature_z,
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
                strain_energy,
            }
        })
        .collect()
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
