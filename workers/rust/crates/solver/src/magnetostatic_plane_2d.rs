use crate::linear_algebra::{
    SparseMatrix, add_at, reduce_sparse_system_with_prescribed, solve_spd_system,
};
use kyuubiki_protocol::{
    MagnetostaticPlaneNodeResult, MagnetostaticPlaneQuadElementInput,
    MagnetostaticPlaneQuadElementResult, MagnetostaticPlaneTriangleElementInput,
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
            let stored_energy = 0.5
                * magnetic_flux_density_magnitude
                * magnetic_field_strength_magnitude
                * computed.area
                * element.thickness;

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
            let stored_energy = 0.5
                * magnetic_flux_density_magnitude
                * magnetic_field_strength_magnitude
                * total_area
                * element.thickness;

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
        total_stored_energy: elements.iter().map(|element| element.stored_energy).sum(),
        nodes,
        elements,
    })
}

#[derive(Debug, Clone)]
struct TriangleComputed {
    stiffness: [[f64; 3]; 3],
    area: f64,
    gradient_x: [f64; 3],
    gradient_y: [f64; 3],
}

#[derive(Debug, Clone)]
struct QuadComputed {
    first: TriangleComputed,
    second: TriangleComputed,
}

fn precompute_triangle_element(
    request: &SolveMagnetostaticPlaneTriangle2dRequest,
    element: &MagnetostaticPlaneTriangleElementInput,
) -> Result<TriangleComputed, String> {
    let node_i = &request.nodes[element.node_i];
    let node_j = &request.nodes[element.node_j];
    let node_k = &request.nodes[element.node_k];
    let signed_area =
        signed_triangle_area(node_i.x, node_i.y, node_j.x, node_j.y, node_k.x, node_k.y);
    let area = signed_area.abs();
    if area <= 1.0e-12 {
        return Err("magnetostatic plane triangle element area must be positive".to_string());
    }

    let twice_area = signed_area * 2.0;
    let gradient_x = [
        (node_j.y - node_k.y) / twice_area,
        (node_k.y - node_i.y) / twice_area,
        (node_i.y - node_j.y) / twice_area,
    ];
    let gradient_y = [
        (node_k.x - node_j.x) / twice_area,
        (node_i.x - node_k.x) / twice_area,
        (node_j.x - node_i.x) / twice_area,
    ];

    let reluctivity = 1.0 / element.permeability;
    let scale = reluctivity * element.thickness * area;
    let mut stiffness = [[0.0; 3]; 3];
    for row in 0..3 {
        for column in 0..3 {
            stiffness[row][column] = scale
                * ((gradient_x[row] * gradient_x[column]) + (gradient_y[row] * gradient_y[column]));
        }
    }

    Ok(TriangleComputed {
        stiffness,
        area,
        gradient_x,
        gradient_y,
    })
}

fn precompute_quad_element(
    request: &SolveMagnetostaticPlaneQuad2dRequest,
    element: &MagnetostaticPlaneQuadElementInput,
) -> Result<QuadComputed, String> {
    let triangle_request = SolveMagnetostaticPlaneTriangle2dRequest {
        nodes: request.nodes.clone(),
        elements: vec![],
    };
    let first = MagnetostaticPlaneTriangleElementInput {
        id: format!("{}#0", element.id),
        node_i: element.node_i,
        node_j: element.node_j,
        node_k: element.node_k,
        thickness: element.thickness,
        permeability: element.permeability,
    };
    let second = MagnetostaticPlaneTriangleElementInput {
        id: format!("{}#1", element.id),
        node_i: element.node_i,
        node_j: element.node_k,
        node_k: element.node_l,
        thickness: element.thickness,
        permeability: element.permeability,
    };

    Ok(QuadComputed {
        first: precompute_triangle_element(&triangle_request, &first)?,
        second: precompute_triangle_element(&triangle_request, &second)?,
    })
}

fn validate_magnetostatic_plane_triangle_request(
    request: &SolveMagnetostaticPlaneTriangle2dRequest,
) -> Result<(), String> {
    if request.nodes.len() < 3 {
        return Err(
            "magnetostatic plane triangle model must define at least three nodes".to_string(),
        );
    }
    if request.elements.is_empty() {
        return Err(
            "magnetostatic plane triangle model must define at least one element".to_string(),
        );
    }
    if !request.nodes.iter().any(|node| node.fix_vector_potential) {
        return Err(
            "magnetostatic plane triangle model must include at least one vector potential support"
                .to_string(),
        );
    }
    for node in &request.nodes {
        if !(node.x.is_finite() && node.y.is_finite()) {
            return Err("magnetostatic plane triangle node coordinates must be finite".to_string());
        }
        if !node.vector_potential.is_finite() {
            return Err(
                "magnetostatic plane triangle node vector_potential must be finite".to_string(),
            );
        }
        if !node.current_density.is_finite() {
            return Err(
                "magnetostatic plane triangle node current_density must be finite".to_string(),
            );
        }
    }
    for element in &request.elements {
        validate_triangle_element(request, element)?;
    }
    Ok(())
}

fn validate_magnetostatic_plane_quad_request(
    request: &SolveMagnetostaticPlaneQuad2dRequest,
) -> Result<(), String> {
    if request.nodes.len() < 4 {
        return Err("magnetostatic plane quad model must define at least four nodes".to_string());
    }
    if request.elements.is_empty() {
        return Err("magnetostatic plane quad model must define at least one element".to_string());
    }
    if !request.nodes.iter().any(|node| node.fix_vector_potential) {
        return Err(
            "magnetostatic plane quad model must include at least one vector potential support"
                .to_string(),
        );
    }
    for node in &request.nodes {
        if !(node.x.is_finite() && node.y.is_finite()) {
            return Err("magnetostatic plane quad node coordinates must be finite".to_string());
        }
        if !node.vector_potential.is_finite() {
            return Err(
                "magnetostatic plane quad node vector_potential must be finite".to_string(),
            );
        }
        if !node.current_density.is_finite() {
            return Err("magnetostatic plane quad node current_density must be finite".to_string());
        }
    }
    for element in &request.elements {
        validate_quad_element(request, element)?;
    }
    Ok(())
}

fn validate_triangle_element(
    request: &SolveMagnetostaticPlaneTriangle2dRequest,
    element: &MagnetostaticPlaneTriangleElementInput,
) -> Result<(), String> {
    if element.node_i >= request.nodes.len()
        || element.node_j >= request.nodes.len()
        || element.node_k >= request.nodes.len()
    {
        return Err(
            "magnetostatic plane triangle element references an out-of-range node".to_string(),
        );
    }
    if !element.thickness.is_finite() || element.thickness <= 0.0 {
        return Err("magnetostatic plane triangle thickness must be positive".to_string());
    }
    if !element.permeability.is_finite() || element.permeability <= 0.0 {
        return Err("magnetostatic plane triangle permeability must be positive".to_string());
    }
    let ni = &request.nodes[element.node_i];
    let nj = &request.nodes[element.node_j];
    let nk = &request.nodes[element.node_k];
    if signed_triangle_area(ni.x, ni.y, nj.x, nj.y, nk.x, nk.y).abs() <= 1.0e-12 {
        return Err("magnetostatic plane triangle element area must be positive".to_string());
    }
    Ok(())
}

fn validate_quad_element(
    request: &SolveMagnetostaticPlaneQuad2dRequest,
    element: &MagnetostaticPlaneQuadElementInput,
) -> Result<(), String> {
    if element.node_i >= request.nodes.len()
        || element.node_j >= request.nodes.len()
        || element.node_k >= request.nodes.len()
        || element.node_l >= request.nodes.len()
    {
        return Err("magnetostatic plane quad element references an out-of-range node".to_string());
    }
    if !element.thickness.is_finite() || element.thickness <= 0.0 {
        return Err("magnetostatic plane quad thickness must be positive".to_string());
    }
    if !element.permeability.is_finite() || element.permeability <= 0.0 {
        return Err("magnetostatic plane quad permeability must be positive".to_string());
    }
    let n = |index: usize| &request.nodes[index];
    let first_area = signed_triangle_area(
        n(element.node_i).x,
        n(element.node_i).y,
        n(element.node_j).x,
        n(element.node_j).y,
        n(element.node_k).x,
        n(element.node_k).y,
    )
    .abs();
    let second_area = signed_triangle_area(
        n(element.node_i).x,
        n(element.node_i).y,
        n(element.node_k).x,
        n(element.node_k).y,
        n(element.node_l).x,
        n(element.node_l).y,
    )
    .abs();
    if first_area <= 1.0e-12 || second_area <= 1.0e-12 {
        return Err("magnetostatic plane quad triangles must have positive area".to_string());
    }
    Ok(())
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

fn signed_triangle_area(ix: f64, iy: f64, jx: f64, jy: f64, kx: f64, ky: f64) -> f64 {
    0.5 * ((jx - ix) * (ky - iy) - (kx - ix) * (jy - iy))
}

fn scalar_gradient(
    gradient_x: &[f64; 3],
    gradient_y: &[f64; 3],
    nodal_values: &[f64; 3],
) -> [f64; 2] {
    [
        (0..3)
            .map(|index| gradient_x[index] * nodal_values[index])
            .sum(),
        (0..3)
            .map(|index| gradient_y[index] * nodal_values[index])
            .sum(),
    ]
}
