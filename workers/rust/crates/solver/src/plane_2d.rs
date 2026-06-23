use crate::linear_algebra::{SparseMatrix, add_at, reduce_sparse_system, solve_spd_system};
use crate::plane_2d_math::{
    PlaneTriangleComputed, plane_triangle_state, precompute_plane_triangle_element,
    signed_triangle_area,
};
use kyuubiki_protocol::{
    PlaneNodeResult, PlaneQuadElementInput, PlaneQuadElementResult, PlaneTriangleElementInput,
    PlaneTriangleElementResult, SolvePlaneQuad2dRequest, SolvePlaneQuad2dResult,
    SolvePlaneTriangle2dRequest, SolvePlaneTriangle2dResult,
};

#[derive(Debug, Clone)]
struct PlaneQuadComputed {
    first: PlaneTriangleComputed,
    second: PlaneTriangleComputed,
}

pub fn solve_plane_triangle_2d(
    request: &SolvePlaneTriangle2dRequest,
) -> Result<SolvePlaneTriangle2dResult, String> {
    validate_plane_request(request)?;

    let dof_count = request.nodes.len() * 2;
    let mut global_stiffness = SparseMatrix::new(dof_count);
    let mut force_vector = vec![0.0; dof_count];
    let computed_elements = request
        .elements
        .iter()
        .map(|element| precompute_plane_triangle_element(request, element))
        .collect::<Result<Vec<_>, String>>()?;

    for (index, node) in request.nodes.iter().enumerate() {
        force_vector[index * 2] = node.load_x;
        force_vector[index * 2 + 1] = node.load_y;
    }

    for (element, computed) in request.elements.iter().zip(computed_elements.iter()) {
        let map = triangle_dof_map(element.node_i, element.node_j, element.node_k);
        for row in 0..6 {
            for column in 0..6 {
                add_at(
                    &mut global_stiffness,
                    map[row],
                    map[column],
                    computed.stiffness[row][column],
                );
            }
        }
    }

    let displacements = solve_plane_displacements(request, &global_stiffness, &force_vector)?;
    let nodes = build_plane_nodes(request, &displacements);
    let elements = request
        .elements
        .iter()
        .zip(computed_elements.iter())
        .enumerate()
        .map(|(index, (element, computed))| {
            let element_displacements = triangle_displacements(
                &displacements,
                element.node_i,
                element.node_j,
                element.node_k,
            );
            let state = plane_triangle_state(computed, &element_displacements);

            PlaneTriangleElementResult {
                index,
                id: element.id.clone(),
                node_i: element.node_i,
                node_j: element.node_j,
                node_k: element.node_k,
                area: computed.area,
                strain_x: state.strain[0],
                strain_y: state.strain[1],
                gamma_xy: state.strain[2],
                stress_x: state.stress[0],
                stress_y: state.stress[1],
                tau_xy: state.stress[2],
                principal_stress_1: state.principal_stress_1,
                principal_stress_2: state.principal_stress_2,
                max_in_plane_shear: state.max_in_plane_shear,
                von_mises: state.von_mises,
            }
        })
        .collect::<Vec<_>>();

    Ok(SolvePlaneTriangle2dResult {
        input: request.clone(),
        max_displacement: max_plane_displacement(&nodes),
        max_stress: elements
            .iter()
            .map(|element| element.von_mises.abs())
            .fold(0.0_f64, f64::max),
        nodes,
        elements,
    })
}

pub fn solve_plane_quad_2d(
    request: &SolvePlaneQuad2dRequest,
) -> Result<SolvePlaneQuad2dResult, String> {
    validate_plane_quad_request(request)?;

    let dof_count = request.nodes.len() * 2;
    let mut global_stiffness = SparseMatrix::new(dof_count);
    let mut force_vector = vec![0.0; dof_count];
    let computed_elements = request
        .elements
        .iter()
        .map(|element| precompute_plane_quad_element(request, element))
        .collect::<Result<Vec<_>, String>>()?;

    for (index, node) in request.nodes.iter().enumerate() {
        force_vector[index * 2] = node.load_x;
        force_vector[index * 2 + 1] = node.load_y;
    }

    for (element, computed) in request.elements.iter().zip(computed_elements.iter()) {
        let triangles = [
            (
                [element.node_i, element.node_j, element.node_k],
                &computed.first,
            ),
            (
                [element.node_i, element.node_k, element.node_l],
                &computed.second,
            ),
        ];
        for (nodes, triangle) in triangles {
            let map = triangle_dof_map(nodes[0], nodes[1], nodes[2]);
            for row in 0..6 {
                for column in 0..6 {
                    add_at(
                        &mut global_stiffness,
                        map[row],
                        map[column],
                        triangle.stiffness[row][column],
                    );
                }
            }
        }
    }

    let triangle_request = to_triangle_request(request);
    let displacements =
        solve_plane_displacements(&triangle_request, &global_stiffness, &force_vector)?;
    let nodes = build_plane_nodes(&triangle_request, &displacements);
    let elements = build_plane_quad_elements(request, &computed_elements, &displacements);

    Ok(SolvePlaneQuad2dResult {
        input: request.clone(),
        max_displacement: max_plane_displacement(&nodes),
        max_stress: elements
            .iter()
            .map(|element| element.von_mises.abs())
            .fold(0.0_f64, f64::max),
        nodes,
        elements,
    })
}

fn build_plane_quad_elements(
    request: &SolvePlaneQuad2dRequest,
    computed_elements: &[PlaneQuadComputed],
    displacements: &[f64],
) -> Vec<PlaneQuadElementResult> {
    request
        .elements
        .iter()
        .zip(computed_elements.iter())
        .enumerate()
        .map(|(index, (element, computed))| {
            let first_state = plane_triangle_state(
                &computed.first,
                &triangle_displacements(
                    displacements,
                    element.node_i,
                    element.node_j,
                    element.node_k,
                ),
            );
            let second_state = plane_triangle_state(
                &computed.second,
                &triangle_displacements(
                    displacements,
                    element.node_i,
                    element.node_k,
                    element.node_l,
                ),
            );
            let total_area = computed.first.area + computed.second.area;
            let weighted = |left: f64, right: f64| -> f64 {
                ((left * computed.first.area) + (right * computed.second.area)) / total_area
            };

            PlaneQuadElementResult {
                index,
                id: element.id.clone(),
                node_i: element.node_i,
                node_j: element.node_j,
                node_k: element.node_k,
                node_l: element.node_l,
                area: total_area,
                strain_x: weighted(first_state.strain[0], second_state.strain[0]),
                strain_y: weighted(first_state.strain[1], second_state.strain[1]),
                gamma_xy: weighted(first_state.strain[2], second_state.strain[2]),
                stress_x: weighted(first_state.stress[0], second_state.stress[0]),
                stress_y: weighted(first_state.stress[1], second_state.stress[1]),
                tau_xy: weighted(first_state.stress[2], second_state.stress[2]),
                principal_stress_1: weighted(
                    first_state.principal_stress_1,
                    second_state.principal_stress_1,
                ),
                principal_stress_2: weighted(
                    first_state.principal_stress_2,
                    second_state.principal_stress_2,
                ),
                max_in_plane_shear: weighted(
                    first_state.max_in_plane_shear,
                    second_state.max_in_plane_shear,
                ),
                von_mises: weighted(first_state.von_mises, second_state.von_mises),
            }
        })
        .collect()
}

fn validate_plane_request(request: &SolvePlaneTriangle2dRequest) -> Result<(), String> {
    if request.nodes.len() < 3 {
        return Err("plane model must define at least three nodes".to_string());
    }
    if request.elements.is_empty() {
        return Err("plane model must define at least one element".to_string());
    }
    if !request.nodes.iter().any(|node| node.fix_x || node.fix_y) {
        return Err("plane model must include at least one support".to_string());
    }

    for element in &request.elements {
        validate_plane_triangle_element(request, element)?;
    }
    Ok(())
}

fn validate_plane_quad_request(request: &SolvePlaneQuad2dRequest) -> Result<(), String> {
    if request.nodes.len() < 4 {
        return Err("plane quad model must define at least four nodes".to_string());
    }
    if request.elements.is_empty() {
        return Err("plane quad model must define at least one element".to_string());
    }
    if !request.nodes.iter().any(|node| node.fix_x || node.fix_y) {
        return Err("plane quad model must include at least one support".to_string());
    }

    for element in &request.elements {
        let indices = [
            element.node_i,
            element.node_j,
            element.node_k,
            element.node_l,
        ];
        if indices.iter().any(|&index| index >= request.nodes.len()) {
            return Err("plane quad element references an out-of-range node".to_string());
        }
        let unique_count = indices
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>()
            .len();
        if unique_count < 4 {
            return Err("plane quad element must reference four distinct nodes".to_string());
        }
        validate_plane_material(
            element.thickness,
            element.youngs_modulus,
            element.poisson_ratio,
        )?;
        let triangle_request = to_triangle_request(request);
        validate_positive_triangle_area(
            &triangle_request,
            element.node_i,
            element.node_j,
            element.node_k,
            "plane quad element must decompose into positive-area triangles",
        )?;
        validate_positive_triangle_area(
            &triangle_request,
            element.node_i,
            element.node_k,
            element.node_l,
            "plane quad element must decompose into positive-area triangles",
        )?;
    }
    Ok(())
}

fn validate_plane_triangle_element(
    request: &SolvePlaneTriangle2dRequest,
    element: &PlaneTriangleElementInput,
) -> Result<(), String> {
    if element.node_i >= request.nodes.len()
        || element.node_j >= request.nodes.len()
        || element.node_k >= request.nodes.len()
    {
        return Err("plane element references an out-of-range node".to_string());
    }
    validate_plane_material(
        element.thickness,
        element.youngs_modulus,
        element.poisson_ratio,
    )?;
    validate_positive_triangle_area(
        request,
        element.node_i,
        element.node_j,
        element.node_k,
        "plane element area must be positive",
    )
}

fn validate_plane_material(
    thickness: f64,
    youngs_modulus: f64,
    poisson_ratio: f64,
) -> Result<(), String> {
    if !(thickness.is_finite() && thickness > 0.0) {
        return Err("plane element thickness must be positive".to_string());
    }
    if !(youngs_modulus.is_finite() && youngs_modulus > 0.0) {
        return Err("plane element youngs_modulus must be positive".to_string());
    }
    if !(poisson_ratio.is_finite() && poisson_ratio > -1.0 && poisson_ratio < 0.5) {
        return Err("plane element poisson_ratio must be between -1.0 and 0.5".to_string());
    }
    Ok(())
}

fn precompute_plane_quad_element(
    request: &SolvePlaneQuad2dRequest,
    element: &PlaneQuadElementInput,
) -> Result<PlaneQuadComputed, String> {
    let first = PlaneTriangleElementInput {
        id: format!("{}#0", element.id),
        node_i: element.node_i,
        node_j: element.node_j,
        node_k: element.node_k,
        thickness: element.thickness,
        youngs_modulus: element.youngs_modulus,
        poisson_ratio: element.poisson_ratio,
    };
    let second = PlaneTriangleElementInput {
        id: format!("{}#1", element.id),
        node_i: element.node_i,
        node_j: element.node_k,
        node_k: element.node_l,
        thickness: element.thickness,
        youngs_modulus: element.youngs_modulus,
        poisson_ratio: element.poisson_ratio,
    };
    let triangle_request = to_triangle_request(request);

    Ok(PlaneQuadComputed {
        first: precompute_plane_triangle_element(&triangle_request, &first)?,
        second: precompute_plane_triangle_element(&triangle_request, &second)?,
    })
}

fn solve_plane_displacements(
    request: &SolvePlaneTriangle2dRequest,
    global_stiffness: &SparseMatrix,
    force_vector: &[f64],
) -> Result<Vec<f64>, String> {
    let constrained = request
        .nodes
        .iter()
        .enumerate()
        .flat_map(|(index, node)| {
            [
                node.fix_x.then_some(index * 2),
                node.fix_y.then_some(index * 2 + 1),
            ]
        })
        .flatten()
        .collect::<Vec<_>>();
    let (reduced_stiffness, reduced_force, free) =
        reduce_sparse_system(global_stiffness, force_vector, &constrained);
    let reduced_displacements = solve_spd_system(&reduced_stiffness, &reduced_force)?;
    let mut displacements = vec![0.0; request.nodes.len() * 2];
    for (index, &dof) in free.iter().enumerate() {
        displacements[dof] = reduced_displacements[index];
    }
    Ok(displacements)
}

fn build_plane_nodes(
    request: &SolvePlaneTriangle2dRequest,
    displacements: &[f64],
) -> Vec<PlaneNodeResult> {
    request
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| {
            let ux = displacements[index * 2];
            let uy = displacements[index * 2 + 1];
            PlaneNodeResult {
                index,
                id: node.id.clone(),
                x: node.x,
                y: node.y,
                ux,
                uy,
                displacement_magnitude: (ux * ux + uy * uy).sqrt(),
            }
        })
        .collect()
}

fn to_triangle_request(request: &SolvePlaneQuad2dRequest) -> SolvePlaneTriangle2dRequest {
    SolvePlaneTriangle2dRequest {
        nodes: request.nodes.clone(),
        elements: vec![],
    }
}

fn triangle_dof_map(node_i: usize, node_j: usize, node_k: usize) -> [usize; 6] {
    [
        node_i * 2,
        node_i * 2 + 1,
        node_j * 2,
        node_j * 2 + 1,
        node_k * 2,
        node_k * 2 + 1,
    ]
}

fn triangle_displacements(
    displacements: &[f64],
    node_i: usize,
    node_j: usize,
    node_k: usize,
) -> [f64; 6] {
    let map = triangle_dof_map(node_i, node_j, node_k);
    std::array::from_fn(|index| displacements[map[index]])
}

fn max_plane_displacement(nodes: &[PlaneNodeResult]) -> f64 {
    nodes
        .iter()
        .map(|node| node.displacement_magnitude)
        .fold(0.0_f64, f64::max)
}

fn validate_positive_triangle_area(
    request: &SolvePlaneTriangle2dRequest,
    node_i: usize,
    node_j: usize,
    node_k: usize,
    message: &str,
) -> Result<(), String> {
    let area = signed_triangle_area(
        &request.nodes[node_i],
        &request.nodes[node_j],
        &request.nodes[node_k],
    )
    .abs();
    if area <= 1.0e-12 {
        return Err(message.to_string());
    }
    Ok(())
}
