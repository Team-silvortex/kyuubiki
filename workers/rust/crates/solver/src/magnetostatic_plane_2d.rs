use crate::linear_algebra::{
    SparseMatrix, add_at, reduce_sparse_system_with_prescribed, solve_spd_system,
};
use crate::magnetostatic_plane_2d_element::{
    precompute_quad_element, precompute_triangle_element, scalar_gradient,
};
use crate::magnetostatic_plane_2d_validation::{
    validate_magnetostatic_plane_quad_request, validate_magnetostatic_plane_triangle_request,
};
use kyuubiki_protocol::{
    MagnetostaticPlaneNodeResult, MagnetostaticPlaneQuadElementResult,
    MagnetostaticPlaneTriangleElementResult, SolveMagnetostaticPlaneQuad2dRequest,
    SolveMagnetostaticPlaneQuad2dResult, SolveMagnetostaticPlaneTriangle2dRequest,
    SolveMagnetostaticPlaneTriangle2dResult,
};

pub fn solve_magnetostatic_plane_triangle_2d(
    request: &SolveMagnetostaticPlaneTriangle2dRequest,
) -> Result<SolveMagnetostaticPlaneTriangle2dResult, String> {
    validate_magnetostatic_plane_triangle_request(request)?;

    let dof_count = request.nodes.len();
    let mut global_stiffness = SparseMatrix::new(dof_count);
    let mut source_vector = vec![0.0; dof_count];
    let computed_elements = request
        .elements
        .iter()
        .map(|element| precompute_triangle_element(request, element))
        .collect::<Result<Vec<_>, String>>()?;

    for (index, node) in request.nodes.iter().enumerate() {
        source_vector[index] = node.current_density;
    }

    for (element, computed) in request.elements.iter().zip(computed_elements.iter()) {
        let map = [element.node_i, element.node_j, element.node_k];
        for row in 0..3 {
            for column in 0..3 {
                add_at(
                    &mut global_stiffness,
                    map[row],
                    map[column],
                    computed.stiffness[row][column],
                );
            }
        }
    }

    let prescribed = request
        .nodes
        .iter()
        .enumerate()
        .filter_map(|(index, node)| {
            node.fix_vector_potential
                .then_some((index, node.vector_potential))
        })
        .collect::<Vec<_>>();

    let (reduced_stiffness, reduced_source, free) =
        reduce_sparse_system_with_prescribed(&global_stiffness, &source_vector, &prescribed);
    let reduced_potentials = solve_spd_system(&reduced_stiffness, &reduced_source)?;

    let mut vector_potentials = vec![0.0; dof_count];
    for &(index, value) in &prescribed {
        vector_potentials[index] = value;
    }
    for (index, &dof) in free.iter().enumerate() {
        vector_potentials[dof] = reduced_potentials[index];
    }

    let nodes = request
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| MagnetostaticPlaneNodeResult {
            index,
            id: node.id.clone(),
            x: node.x,
            y: node.y,
            vector_potential: vector_potentials[index],
            current_density: node.current_density,
        })
        .collect::<Vec<_>>();

    let elements = request
        .elements
        .iter()
        .zip(computed_elements.iter())
        .enumerate()
        .map(|(index, (element, computed))| {
            let element_potentials = [
                vector_potentials[element.node_i],
                vector_potentials[element.node_j],
                vector_potentials[element.node_k],
            ];
            let gradient = scalar_gradient(
                &computed.gradient_x,
                &computed.gradient_y,
                &element_potentials,
            );
            let magnetic_flux_density_x = gradient[1];
            let magnetic_flux_density_y = -gradient[0];
            let magnetic_field_strength_x = magnetic_flux_density_x / element.permeability;
            let magnetic_field_strength_y = magnetic_flux_density_y / element.permeability;
            let magnetic_field_strength_magnitude = (magnetic_field_strength_x
                * magnetic_field_strength_x
                + magnetic_field_strength_y * magnetic_field_strength_y)
                .sqrt();
            let magnetic_flux_density_magnitude = (magnetic_flux_density_x
                * magnetic_flux_density_x
                + magnetic_flux_density_y * magnetic_flux_density_y)
                .sqrt();
            let magnetic_energy_density =
                0.5 * magnetic_flux_density_magnitude * magnetic_field_strength_magnitude;
            let stored_energy = magnetic_energy_density * computed.area * element.thickness;

            MagnetostaticPlaneTriangleElementResult {
                index,
                id: element.id.clone(),
                node_i: element.node_i,
                node_j: element.node_j,
                node_k: element.node_k,
                area: computed.area,
                average_vector_potential: element_potentials.iter().sum::<f64>() / 3.0,
                vector_potential_gradient_x: gradient[0],
                vector_potential_gradient_y: gradient[1],
                magnetic_field_strength_x,
                magnetic_field_strength_y,
                magnetic_field_strength_magnitude,
                magnetic_flux_density_x,
                magnetic_flux_density_y,
                magnetic_flux_density_magnitude,
                magnetic_energy_density,
                stored_energy,
            }
        })
        .collect::<Vec<_>>();

    Ok(SolveMagnetostaticPlaneTriangle2dResult {
        input: request.clone(),
        max_vector_potential: nodes
            .iter()
            .map(|node| node.vector_potential.abs())
            .fold(0.0_f64, f64::max),
        max_magnetic_field_strength: elements
            .iter()
            .map(|element| element.magnetic_field_strength_magnitude)
            .fold(0.0_f64, f64::max),
        max_flux_density: elements
            .iter()
            .map(|element| element.magnetic_flux_density_magnitude)
            .fold(0.0_f64, f64::max),
        max_magnetic_energy_density: elements
            .iter()
            .map(|element| element.magnetic_energy_density)
            .fold(0.0_f64, f64::max),
        total_stored_energy: elements.iter().map(|element| element.stored_energy).sum(),
        nodes,
        elements,
    })
}

pub fn solve_magnetostatic_plane_quad_2d(
    request: &SolveMagnetostaticPlaneQuad2dRequest,
) -> Result<SolveMagnetostaticPlaneQuad2dResult, String> {
    validate_magnetostatic_plane_quad_request(request)?;

    let dof_count = request.nodes.len();
    let mut global_stiffness = SparseMatrix::new(dof_count);
    let mut source_vector = vec![0.0; dof_count];
    let computed_elements = request
        .elements
        .iter()
        .map(|element| precompute_quad_element(request, element))
        .collect::<Result<Vec<_>, String>>()?;

    for (index, node) in request.nodes.iter().enumerate() {
        source_vector[index] = node.current_density;
    }

    for (element, computed) in request.elements.iter().zip(computed_elements.iter()) {
        for (nodes, triangle) in [
            (
                [element.node_i, element.node_j, element.node_k],
                &computed.first,
            ),
            (
                [element.node_i, element.node_k, element.node_l],
                &computed.second,
            ),
        ] {
            for row in 0..3 {
                for column in 0..3 {
                    add_at(
                        &mut global_stiffness,
                        nodes[row],
                        nodes[column],
                        triangle.stiffness[row][column],
                    );
                }
            }
        }
    }

    let prescribed = request
        .nodes
        .iter()
        .enumerate()
        .filter_map(|(index, node)| {
            node.fix_vector_potential
                .then_some((index, node.vector_potential))
        })
        .collect::<Vec<_>>();
    let (reduced_stiffness, reduced_source, free) =
        reduce_sparse_system_with_prescribed(&global_stiffness, &source_vector, &prescribed);
    let reduced_potentials = solve_spd_system(&reduced_stiffness, &reduced_source)?;

    let mut vector_potentials = vec![0.0; dof_count];
    for &(index, value) in &prescribed {
        vector_potentials[index] = value;
    }
    for (index, &dof) in free.iter().enumerate() {
        vector_potentials[dof] = reduced_potentials[index];
    }

    let nodes = build_node_results(request.nodes.iter(), &vector_potentials);
    let elements = request
        .elements
        .iter()
        .zip(computed_elements.iter())
        .enumerate()
        .map(|(index, (element, computed))| {
            let first = [
                vector_potentials[element.node_i],
                vector_potentials[element.node_j],
                vector_potentials[element.node_k],
            ];
            let second = [
                vector_potentials[element.node_i],
                vector_potentials[element.node_k],
                vector_potentials[element.node_l],
            ];
            let first_gradient = scalar_gradient(
                &computed.first.gradient_x,
                &computed.first.gradient_y,
                &first,
            );
            let second_gradient = scalar_gradient(
                &computed.second.gradient_x,
                &computed.second.gradient_y,
                &second,
            );
            let total_area = computed.first.area + computed.second.area;
            let weighted = |left: f64, right: f64| {
                (left * computed.first.area + right * computed.second.area) / total_area
            };
            let gradient = [
                weighted(first_gradient[0], second_gradient[0]),
                weighted(first_gradient[1], second_gradient[1]),
            ];
            let magnetic_flux_density_x = gradient[1];
            let magnetic_flux_density_y = -gradient[0];
            let magnetic_field_strength_x = magnetic_flux_density_x / element.permeability;
            let magnetic_field_strength_y = magnetic_flux_density_y / element.permeability;
            let magnetic_field_strength_magnitude = (magnetic_field_strength_x
                * magnetic_field_strength_x
                + magnetic_field_strength_y * magnetic_field_strength_y)
                .sqrt();
            let magnetic_flux_density_magnitude = (magnetic_flux_density_x
                * magnetic_flux_density_x
                + magnetic_flux_density_y * magnetic_flux_density_y)
                .sqrt();
            let magnetic_energy_density =
                0.5 * magnetic_flux_density_magnitude * magnetic_field_strength_magnitude;
            let stored_energy = magnetic_energy_density * total_area * element.thickness;

            MagnetostaticPlaneQuadElementResult {
                index,
                id: element.id.clone(),
                node_i: element.node_i,
                node_j: element.node_j,
                node_k: element.node_k,
                node_l: element.node_l,
                area: total_area,
                average_vector_potential: (vector_potentials[element.node_i]
                    + vector_potentials[element.node_j]
                    + vector_potentials[element.node_k]
                    + vector_potentials[element.node_l])
                    / 4.0,
                vector_potential_gradient_x: gradient[0],
                vector_potential_gradient_y: gradient[1],
                magnetic_field_strength_x,
                magnetic_field_strength_y,
                magnetic_field_strength_magnitude,
                magnetic_flux_density_x,
                magnetic_flux_density_y,
                magnetic_flux_density_magnitude,
                magnetic_energy_density,
                stored_energy,
            }
        })
        .collect::<Vec<_>>();

    Ok(SolveMagnetostaticPlaneQuad2dResult {
        input: request.clone(),
        max_vector_potential: nodes
            .iter()
            .map(|node| node.vector_potential.abs())
            .fold(0.0_f64, f64::max),
        max_magnetic_field_strength: elements
            .iter()
            .map(|element| element.magnetic_field_strength_magnitude)
            .fold(0.0_f64, f64::max),
        max_flux_density: elements
            .iter()
            .map(|element| element.magnetic_flux_density_magnitude)
            .fold(0.0_f64, f64::max),
        max_magnetic_energy_density: elements
            .iter()
            .map(|element| element.magnetic_energy_density)
            .fold(0.0_f64, f64::max),
        total_stored_energy: elements.iter().map(|element| element.stored_energy).sum(),
        nodes,
        elements,
    })
}

fn build_node_results<'a>(
    nodes: impl Iterator<Item = &'a kyuubiki_protocol::MagnetostaticPlaneNodeInput>,
    vector_potentials: &[f64],
) -> Vec<MagnetostaticPlaneNodeResult> {
    nodes
        .enumerate()
        .map(|(index, node)| MagnetostaticPlaneNodeResult {
            index,
            id: node.id.clone(),
            x: node.x,
            y: node.y,
            vector_potential: vector_potentials[index],
            current_density: node.current_density,
        })
        .collect()
}
