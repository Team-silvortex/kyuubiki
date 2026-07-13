use crate::plane_2d_math::signed_triangle_area;
use kyuubiki_protocol::{
    PlaneNodeInput, PlaneQuadElementInput, PlaneTriangleElementInput, SolvePlaneQuad2dRequest,
    SolvePlaneTriangle2dRequest,
};

pub(crate) fn validate_plane_request(request: &SolvePlaneTriangle2dRequest) -> Result<(), String> {
    if request.nodes.len() < 3 {
        return Err("plane model must define at least three nodes".to_string());
    }
    if request.elements.is_empty() {
        return Err("plane model must define at least one element".to_string());
    }
    if !request.nodes.iter().any(|node| node.fix_x || node.fix_y) {
        return Err("plane model must include at least one support".to_string());
    }
    validate_plane_nodes(&request.nodes, "plane")?;
    for element in &request.elements {
        validate_plane_triangle_element(request, element)?;
    }
    Ok(())
}

pub(crate) fn validate_plane_quad_request(request: &SolvePlaneQuad2dRequest) -> Result<(), String> {
    if request.nodes.len() < 4 {
        return Err("plane quad model must define at least four nodes".to_string());
    }
    if request.elements.is_empty() {
        return Err("plane quad model must define at least one element".to_string());
    }
    if !request.nodes.iter().any(|node| node.fix_x || node.fix_y) {
        return Err("plane quad model must include at least one support".to_string());
    }
    validate_plane_nodes(&request.nodes, "plane quad")?;
    for element in &request.elements {
        validate_plane_quad_element(request, element)?;
    }
    Ok(())
}

fn validate_plane_nodes(nodes: &[PlaneNodeInput], label: &str) -> Result<(), String> {
    for (index, node) in nodes.iter().enumerate() {
        if !node.x.is_finite() || !node.y.is_finite() {
            return Err(format!("{label} node {index} has invalid coordinates"));
        }
        if !node.load_x.is_finite() || !node.load_y.is_finite() {
            return Err(format!("{label} node {index} has invalid load"));
        }
    }
    Ok(())
}

fn validate_plane_triangle_element(
    request: &SolvePlaneTriangle2dRequest,
    element: &PlaneTriangleElementInput,
) -> Result<(), String> {
    let indices = [element.node_i, element.node_j, element.node_k];
    if indices.iter().any(|&index| index >= request.nodes.len()) {
        return Err("plane element references an out-of-range node".to_string());
    }
    if has_duplicate(&indices) {
        return Err("plane element must reference three distinct nodes".to_string());
    }
    validate_plane_material(
        element.thickness,
        element.youngs_modulus,
        element.poisson_ratio,
    )?;
    validate_positive_triangle_area(
        &request.nodes,
        element.node_i,
        element.node_j,
        element.node_k,
        "plane element area must be positive",
    )
}

fn validate_plane_quad_element(
    request: &SolvePlaneQuad2dRequest,
    element: &PlaneQuadElementInput,
) -> Result<(), String> {
    let indices = [
        element.node_i,
        element.node_j,
        element.node_k,
        element.node_l,
    ];
    if indices.iter().any(|&index| index >= request.nodes.len()) {
        return Err("plane quad element references an out-of-range node".to_string());
    }
    if has_duplicate(&indices) {
        return Err("plane quad element must reference four distinct nodes".to_string());
    }
    validate_plane_material(
        element.thickness,
        element.youngs_modulus,
        element.poisson_ratio,
    )?;
    validate_positive_triangle_area(
        &request.nodes,
        element.node_i,
        element.node_j,
        element.node_k,
        "plane quad element must decompose into positive-area triangles",
    )?;
    validate_positive_triangle_area(
        &request.nodes,
        element.node_i,
        element.node_k,
        element.node_l,
        "plane quad element must decompose into positive-area triangles",
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

fn has_duplicate(indices: &[usize]) -> bool {
    indices
        .iter()
        .enumerate()
        .any(|(left, value)| indices[(left + 1)..].contains(value))
}
