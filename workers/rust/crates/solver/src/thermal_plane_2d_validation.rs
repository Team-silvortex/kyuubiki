use crate::plane_2d_math::signed_triangle_area;
use kyuubiki_protocol::{
    PlaneNodeInput, SolveThermalPlaneQuad2dRequest, SolveThermalPlaneTriangle2dRequest,
    ThermalPlaneQuadElementInput, ThermalPlaneTriangleElementInput,
};

pub(super) fn validate_thermal_plane_triangle_request(
    request: &SolveThermalPlaneTriangle2dRequest,
) -> Result<(), String> {
    if request.nodes.len() < 3 {
        return Err("thermal plane model must define at least three nodes".to_string());
    }
    if request.elements.is_empty() {
        return Err("thermal plane model must define at least one element".to_string());
    }
    if !request.nodes.iter().any(|node| node.fix_x || node.fix_y) {
        return Err("thermal plane model must include at least one support".to_string());
    }
    validate_thermal_plane_nodes(&request.nodes, "thermal plane")?;
    let plane_nodes = to_plane_nodes(request);
    for element in &request.elements {
        validate_thermal_triangle_element(request, &plane_nodes, element)?;
    }
    Ok(())
}

pub(super) fn validate_thermal_plane_quad_request(
    request: &SolveThermalPlaneQuad2dRequest,
) -> Result<(), String> {
    if request.nodes.len() < 4 {
        return Err("thermal plane quad model must define at least four nodes".to_string());
    }
    if request.elements.is_empty() {
        return Err("thermal plane quad model must define at least one element".to_string());
    }
    if !request.nodes.iter().any(|node| node.fix_x || node.fix_y) {
        return Err("thermal plane quad model must include at least one support".to_string());
    }
    validate_thermal_plane_nodes(&request.nodes, "thermal plane quad")?;
    let plane_nodes = to_quad_plane_nodes(request);
    for element in &request.elements {
        validate_thermal_quad_element(request, &plane_nodes, element)?;
    }
    Ok(())
}

fn validate_thermal_triangle_element(
    request: &SolveThermalPlaneTriangle2dRequest,
    plane_nodes: &[PlaneNodeInput],
    element: &ThermalPlaneTriangleElementInput,
) -> Result<(), String> {
    if element.node_i >= request.nodes.len()
        || element.node_j >= request.nodes.len()
        || element.node_k >= request.nodes.len()
    {
        return Err("thermal plane element references an out-of-range node".to_string());
    }
    if has_duplicate(&[element.node_i, element.node_j, element.node_k]) {
        return Err("thermal plane element must reference three distinct nodes".to_string());
    }
    validate_thermal_material(
        element.thickness,
        element.youngs_modulus,
        element.poisson_ratio,
        element.thermal_expansion,
        "thermal plane element",
    )?;
    validate_positive_triangle_area(
        plane_nodes,
        element.node_i,
        element.node_j,
        element.node_k,
        "thermal plane element area must be positive",
    )
}

fn validate_thermal_quad_element(
    request: &SolveThermalPlaneQuad2dRequest,
    plane_nodes: &[PlaneNodeInput],
    element: &ThermalPlaneQuadElementInput,
) -> Result<(), String> {
    let indices = [
        element.node_i,
        element.node_j,
        element.node_k,
        element.node_l,
    ];
    if indices.iter().any(|&index| index >= request.nodes.len()) {
        return Err("thermal plane quad element references an out-of-range node".to_string());
    }
    let unique_count = indices
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    if unique_count < 4 {
        return Err("thermal plane quad element must reference four distinct nodes".to_string());
    }
    validate_thermal_material(
        element.thickness,
        element.youngs_modulus,
        element.poisson_ratio,
        element.thermal_expansion,
        "thermal plane quad element",
    )?;

    validate_positive_triangle_area(
        plane_nodes,
        element.node_i,
        element.node_j,
        element.node_k,
        "thermal plane quad element must decompose into positive-area triangles",
    )?;
    validate_positive_triangle_area(
        plane_nodes,
        element.node_i,
        element.node_k,
        element.node_l,
        "thermal plane quad element must decompose into positive-area triangles",
    )
}

fn validate_thermal_material(
    thickness: f64,
    youngs_modulus: f64,
    poisson_ratio: f64,
    thermal_expansion: f64,
    label: &str,
) -> Result<(), String> {
    if !(thickness.is_finite() && thickness > 0.0) {
        return Err(format!("{label} thickness must be positive"));
    }
    if !(youngs_modulus.is_finite() && youngs_modulus > 0.0) {
        return Err(format!("{label} youngs_modulus must be positive"));
    }
    if !(poisson_ratio.is_finite() && poisson_ratio > -1.0 && poisson_ratio < 0.5) {
        return Err(format!(
            "{label} poisson_ratio must be between -1.0 and 0.5"
        ));
    }
    if !(thermal_expansion.is_finite() && thermal_expansion >= 0.0) {
        return Err(format!("{label} thermal_expansion must be non-negative"));
    }
    Ok(())
}

fn validate_positive_triangle_area(
    nodes: &[PlaneNodeInput],
    node_i: usize,
    node_j: usize,
    node_k: usize,
    message: &str,
) -> Result<(), String> {
    let area = signed_triangle_area(&nodes[node_i], &nodes[node_j], &nodes[node_k]).abs();
    if !(area.is_finite() && area > 1.0e-12) {
        return Err(message.to_string());
    }
    Ok(())
}

fn validate_thermal_plane_nodes(
    nodes: &[kyuubiki_protocol::ThermalPlaneNodeInput],
    label: &str,
) -> Result<(), String> {
    for (index, node) in nodes.iter().enumerate() {
        if !node.x.is_finite() || !node.y.is_finite() {
            return Err(format!("{label} node {index} coordinates must be finite"));
        }
        if !node.load_x.is_finite() || !node.load_y.is_finite() {
            return Err(format!("{label} node {index} loads must be finite"));
        }
        if !node.temperature_delta.is_finite() {
            return Err(format!(
                "{label} node {index} temperature_delta must be finite"
            ));
        }
    }
    Ok(())
}

fn has_duplicate(indices: &[usize]) -> bool {
    indices
        .iter()
        .enumerate()
        .any(|(left, value)| indices[(left + 1)..].contains(value))
}

fn to_plane_nodes(request: &SolveThermalPlaneTriangle2dRequest) -> Vec<PlaneNodeInput> {
    request
        .nodes
        .iter()
        .map(thermal_node_to_plane_node)
        .collect()
}

fn to_quad_plane_nodes(request: &SolveThermalPlaneQuad2dRequest) -> Vec<PlaneNodeInput> {
    request
        .nodes
        .iter()
        .map(thermal_node_to_plane_node)
        .collect()
}

fn thermal_node_to_plane_node(node: &kyuubiki_protocol::ThermalPlaneNodeInput) -> PlaneNodeInput {
    PlaneNodeInput {
        id: node.id.clone(),
        x: node.x,
        y: node.y,
        fix_x: node.fix_x,
        fix_y: node.fix_y,
        load_x: node.load_x,
        load_y: node.load_y,
    }
}
