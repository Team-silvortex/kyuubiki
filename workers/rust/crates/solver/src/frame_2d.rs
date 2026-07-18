use crate::frame_2d_math::{
    add_vector_6, frame_local_stiffness, frame_thermal_gradient_vector,
    frame_thermal_uniform_vector, frame_transform, multiply_matrix_vector_6x6, subtract_vector_6,
    transform_frame_stiffness, transpose_6x6,
};
use crate::frame_2d_validation::{validate_frame_2d_request, validate_thermal_frame_2d_request};
use crate::frame_energy::{frame_strain_energy_6, thermal_frame2d_strain_energy};
use crate::linear_algebra::{
    SparseMatrix, add_at, reduce_sparse_system, solve_spd_system_profile_with_options,
};
use crate::linear_solver_profile::SpdSolveOptions;
use kyuubiki_protocol::{
    Frame2dElementResult, Frame2dNodeResult, SolveFrame2dRequest, SolveFrame2dResult,
    SolveThermalFrame2dRequest, SolveThermalFrame2dResult, ThermalFrame2dElementResult,
    ThermalFrame2dNodeResult,
};

pub fn solve_frame_2d(request: &SolveFrame2dRequest) -> Result<SolveFrame2dResult, String> {
    solve_frame_2d_with_options(request, SpdSolveOptions::default())
}

pub fn solve_frame_2d_with_options(
    request: &SolveFrame2dRequest,
    options: SpdSolveOptions,
) -> Result<SolveFrame2dResult, String> {
    validate_frame_2d_request(request)?;

    let dof_count = request.nodes.len() * 3;
    let mut global_stiffness = SparseMatrix::new(dof_count);
    let mut force_vector = vec![0.0; dof_count];

    for (index, node) in request.nodes.iter().enumerate() {
        force_vector[index * 3] = node.load_x;
        force_vector[index * 3 + 1] = node.load_y;
        force_vector[index * 3 + 2] = node.moment_z;
    }

    for element in &request.elements {
        let node_i = &request.nodes[element.node_i];
        let node_j = &request.nodes[element.node_j];
        let dx = node_j.x - node_i.x;
        let dy = node_j.y - node_i.y;
        let length = (dx * dx + dy * dy).sqrt();
        let c = dx / length;
        let s = dy / length;
        let local_stiffness = frame_local_stiffness(
            element.area,
            element.youngs_modulus,
            element.moment_of_inertia,
            length,
        );
        let transform = frame_transform(c, s);
        let global_element_stiffness = transform_frame_stiffness(&local_stiffness, &transform);
        let map = frame_dof_map(element.node_i, element.node_j);

        for row in 0..6 {
            for column in 0..6 {
                add_at(
                    &mut global_stiffness,
                    map[row],
                    map[column],
                    global_element_stiffness[row][column],
                );
            }
        }
    }

    let constrained = request
        .nodes
        .iter()
        .enumerate()
        .flat_map(|(index, node)| {
            let mut dofs = Vec::new();
            if node.fix_x {
                dofs.push(index * 3);
            }
            if node.fix_y {
                dofs.push(index * 3 + 1);
            }
            if node.fix_rz {
                dofs.push(index * 3 + 2);
            }
            dofs
        })
        .collect::<Vec<_>>();

    let (reduced_stiffness, reduced_force, free) =
        reduce_sparse_system(&global_stiffness, &force_vector, &constrained);
    let reduced_displacements =
        solve_spd_system_profile_with_options(&reduced_stiffness, &reduced_force, options)?
            .solution;

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
            let rz = displacements[index * 3 + 2];

            Frame2dNodeResult {
                index,
                id: node.id.clone(),
                x: node.x,
                y: node.y,
                ux,
                uy,
                rz,
                displacement_magnitude: (ux * ux + uy * uy).sqrt(),
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
            let length = (dx * dx + dy * dy).sqrt();
            let c = dx / length;
            let s = dy / length;
            let local_stiffness = frame_local_stiffness(
                element.area,
                element.youngs_modulus,
                element.moment_of_inertia,
                length,
            );
            let transform = frame_transform(c, s);
            let map = frame_dof_map(element.node_i, element.node_j);
            let global_displacements = [
                displacements[map[0]],
                displacements[map[1]],
                displacements[map[2]],
                displacements[map[3]],
                displacements[map[4]],
                displacements[map[5]],
            ];
            let local_displacements = multiply_matrix_vector_6x6(&transform, &global_displacements);
            let local_forces = multiply_matrix_vector_6x6(&local_stiffness, &local_displacements);
            let strain_energy = frame_strain_energy_6(&local_forces, &local_displacements);
            let axial_stress = local_forces[0].abs().max(local_forces[3].abs()) / element.area;
            let bending_stress =
                local_forces[2].abs().max(local_forces[5].abs()) / element.section_modulus;
            let max_combined_stress = axial_stress + bending_stress;

            Frame2dElementResult {
                index,
                id: element.id.clone(),
                node_i: element.node_i,
                node_j: element.node_j,
                length,
                axial_force_i: local_forces[0],
                shear_force_i: local_forces[1],
                moment_i: local_forces[2],
                axial_force_j: local_forces[3],
                shear_force_j: local_forces[4],
                moment_j: local_forces[5],
                axial_stress,
                max_bending_stress: bending_stress,
                max_combined_stress,
                strain_energy,
            }
        })
        .collect::<Vec<_>>();

    let max_displacement = nodes
        .iter()
        .map(|node| node.displacement_magnitude)
        .fold(0.0_f64, f64::max);
    let max_rotation = nodes
        .iter()
        .map(|node| node.rz.abs())
        .fold(0.0_f64, f64::max);
    let max_moment = elements
        .iter()
        .flat_map(|element| [element.moment_i.abs(), element.moment_j.abs()])
        .fold(0.0_f64, f64::max);
    let max_stress = elements
        .iter()
        .map(|element| element.max_combined_stress)
        .fold(0.0_f64, f64::max);
    let total_strain_energy = elements.iter().map(|element| element.strain_energy).sum();

    Ok(SolveFrame2dResult {
        input: request.clone(),
        nodes,
        elements,
        max_displacement,
        max_rotation,
        max_moment,
        max_stress,
        total_strain_energy,
    })
}

pub fn solve_thermal_frame_2d(
    request: &SolveThermalFrame2dRequest,
) -> Result<SolveThermalFrame2dResult, String> {
    solve_thermal_frame_2d_with_options(request, SpdSolveOptions::default())
}

pub fn solve_thermal_frame_2d_with_options(
    request: &SolveThermalFrame2dRequest,
    options: SpdSolveOptions,
) -> Result<SolveThermalFrame2dResult, String> {
    validate_thermal_frame_2d_request(request)?;

    let dof_count = request.nodes.len() * 3;
    let mut global_stiffness = SparseMatrix::new(dof_count);
    let mut force_vector = vec![0.0; dof_count];

    for (index, node) in request.nodes.iter().enumerate() {
        force_vector[index * 3] = node.load_x;
        force_vector[index * 3 + 1] = node.load_y;
        force_vector[index * 3 + 2] = node.moment_z;
    }

    for element in &request.elements {
        let node_i = &request.nodes[element.node_i];
        let node_j = &request.nodes[element.node_j];
        let dx = node_j.x - node_i.x;
        let dy = node_j.y - node_i.y;
        let length = (dx * dx + dy * dy).sqrt();
        let c = dx / length;
        let s = dy / length;
        let local_stiffness = frame_local_stiffness(
            element.area,
            element.youngs_modulus,
            element.moment_of_inertia,
            length,
        );
        let transform = frame_transform(c, s);
        let transform_t = transpose_6x6(&transform);
        let global_element_stiffness = transform_frame_stiffness(&local_stiffness, &transform);
        let average_temperature_delta = 0.5 * (node_i.temperature_delta + node_j.temperature_delta);
        let equivalent_local = add_vector_6(
            &frame_thermal_uniform_vector(
                element.area,
                element.youngs_modulus,
                element.thermal_expansion,
                average_temperature_delta,
            ),
            &frame_thermal_gradient_vector(
                element.youngs_modulus,
                element.moment_of_inertia,
                element.thermal_expansion,
                element.section_depth,
                element.temperature_gradient_y,
            ),
        );
        let equivalent_global = multiply_matrix_vector_6x6(&transform_t, &equivalent_local);
        let map = frame_dof_map(element.node_i, element.node_j);

        for row in 0..6 {
            force_vector[map[row]] += equivalent_global[row];
            for column in 0..6 {
                add_at(
                    &mut global_stiffness,
                    map[row],
                    map[column],
                    global_element_stiffness[row][column],
                );
            }
        }
    }

    let constrained = request
        .nodes
        .iter()
        .enumerate()
        .flat_map(|(index, node)| {
            let mut dofs = Vec::new();
            if node.fix_x {
                dofs.push(index * 3);
            }
            if node.fix_y {
                dofs.push(index * 3 + 1);
            }
            if node.fix_rz {
                dofs.push(index * 3 + 2);
            }
            dofs
        })
        .collect::<Vec<_>>();

    let (reduced_stiffness, reduced_force, free) =
        reduce_sparse_system(&global_stiffness, &force_vector, &constrained);
    let reduced_displacements =
        solve_spd_system_profile_with_options(&reduced_stiffness, &reduced_force, options)?
            .solution;

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
            let rz = displacements[index * 3 + 2];

            ThermalFrame2dNodeResult {
                index,
                id: node.id.clone(),
                x: node.x,
                y: node.y,
                ux,
                uy,
                rz,
                displacement_magnitude: (ux * ux + uy * uy).sqrt(),
                temperature_delta: node.temperature_delta,
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
            let length = (dx * dx + dy * dy).sqrt();
            let c = dx / length;
            let s = dy / length;
            let local_stiffness = frame_local_stiffness(
                element.area,
                element.youngs_modulus,
                element.moment_of_inertia,
                length,
            );
            let transform = frame_transform(c, s);
            let map = frame_dof_map(element.node_i, element.node_j);
            let global_displacements = [
                displacements[map[0]],
                displacements[map[1]],
                displacements[map[2]],
                displacements[map[3]],
                displacements[map[4]],
                displacements[map[5]],
            ];
            let local_displacements = multiply_matrix_vector_6x6(&transform, &global_displacements);
            let average_temperature_delta =
                0.5 * (node_i.temperature_delta + node_j.temperature_delta);
            let equivalent_local = add_vector_6(
                &frame_thermal_uniform_vector(
                    element.area,
                    element.youngs_modulus,
                    element.thermal_expansion,
                    average_temperature_delta,
                ),
                &frame_thermal_gradient_vector(
                    element.youngs_modulus,
                    element.moment_of_inertia,
                    element.thermal_expansion,
                    element.section_depth,
                    element.temperature_gradient_y,
                ),
            );
            let local_forces = subtract_vector_6(
                &multiply_matrix_vector_6x6(&local_stiffness, &local_displacements),
                &equivalent_local,
            );
            let thermal_strain = element.thermal_expansion * average_temperature_delta;
            let total_strain = (local_displacements[3] - local_displacements[0]) / length;
            let mechanical_strain = total_strain - thermal_strain;
            let thermal_curvature =
                element.thermal_expansion * element.temperature_gradient_y / element.section_depth;
            let strain_energy = thermal_frame2d_strain_energy(
                element.youngs_modulus,
                element.area,
                element.moment_of_inertia,
                length,
                &local_displacements,
                mechanical_strain,
                thermal_curvature,
            );
            let axial_stress = local_forces[0].abs().max(local_forces[3].abs()) / element.area;
            let bending_stress =
                local_forces[2].abs().max(local_forces[5].abs()) / element.section_modulus;
            let max_combined_stress = axial_stress + bending_stress;

            ThermalFrame2dElementResult {
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
                thermal_curvature,
                axial_force_i: local_forces[0],
                shear_force_i: local_forces[1],
                moment_i: local_forces[2],
                axial_force_j: local_forces[3],
                shear_force_j: local_forces[4],
                moment_j: local_forces[5],
                axial_stress,
                max_bending_stress: bending_stress,
                max_combined_stress,
                strain_energy,
            }
        })
        .collect::<Vec<_>>();

    let max_displacement = nodes
        .iter()
        .map(|node| node.displacement_magnitude)
        .fold(0.0_f64, f64::max);
    let max_rotation = nodes
        .iter()
        .map(|node| node.rz.abs())
        .fold(0.0_f64, f64::max);
    let max_moment = elements
        .iter()
        .flat_map(|element| [element.moment_i.abs(), element.moment_j.abs()])
        .fold(0.0_f64, f64::max);
    let max_stress = elements
        .iter()
        .map(|element| element.max_combined_stress)
        .fold(0.0_f64, f64::max);
    let max_axial_force = elements
        .iter()
        .flat_map(|element| [element.axial_force_i.abs(), element.axial_force_j.abs()])
        .fold(0.0_f64, f64::max);
    let max_temperature_delta = nodes
        .iter()
        .map(|node| node.temperature_delta.abs())
        .fold(0.0_f64, f64::max);
    let max_temperature_gradient = elements
        .iter()
        .map(|element| element.temperature_gradient_y.abs())
        .fold(0.0_f64, f64::max);
    let total_strain_energy = elements.iter().map(|element| element.strain_energy).sum();

    Ok(SolveThermalFrame2dResult {
        input: request.clone(),
        nodes,
        elements,
        max_displacement,
        max_rotation,
        max_moment,
        max_stress,
        max_axial_force,
        max_temperature_delta,
        max_temperature_gradient,
        total_strain_energy,
    })
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
