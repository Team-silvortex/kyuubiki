use crate::plane_2d_math::signed_triangle_area;
use kyuubiki_protocol::{
    SolvePlaneTriangle2dRequest, SolveThermalPlaneQuad2dRequest,
    SolveThermalPlaneTriangle2dRequest, ThermalPlaneQuadElementInput,
    ThermalPlaneTriangleElementInput,
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
    for node in &request.nodes {
        if !node.temperature_delta.is_finite() {
            return Err("thermal plane node temperature_delta must be finite".to_string());
        }
    }
    for element in &request.elements {
        validate_thermal_triangle_element(request, element)?;
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
    for node in &request.nodes {
        if !node.temperature_delta.is_finite() {
            return Err("thermal plane quad node temperature_delta must be finite".to_string());
        }
    }
    for element in &request.elements {
        validate_thermal_quad_element(request, element)?;
    }
    Ok(())
}

fn validate_thermal_triangle_element(
    request: &SolveThermalPlaneTriangle2dRequest,
    element: &ThermalPlaneTriangleElementInput,
) -> Result<(), String> {
    if element.node_i >= request.nodes.len()
        || element.node_j >= request.nodes.len()
        || element.node_k >= request.nodes.len()
    {
        return Err("thermal plane element references an out-of-range node".to_string());
    }
    validate_thermal_material(
        element.thickness,
        element.youngs_modulus,
        element.poisson_ratio,
        element.thermal_expansion,
        "thermal plane element",
    )?;
    validate_positive_triangle_area(
        &to_plane_triangle_request(request),
        element.node_i,
        element.node_j,
        element.node_k,
        "thermal plane element area must be positive",
    )
}

fn validate_thermal_quad_element(
    request: &SolveThermalPlaneQuad2dRequest,
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

    let triangle_request = to_triangle_request(request);
    validate_positive_triangle_area(
        &triangle_request,
        element.node_i,
        element.node_j,
        element.node_k,
        "thermal plane quad element must decompose into positive-area triangles",
    )?;
    validate_positive_triangle_area(
        &triangle_request,
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

fn to_plane_triangle_request(
    request: &SolveThermalPlaneTriangle2dRequest,
) -> SolvePlaneTriangle2dRequest {
    SolvePlaneTriangle2dRequest {
        nodes: request
            .nodes
            .iter()
            .map(|node| kyuubiki_protocol::PlaneNodeInput {
                id: node.id.clone(),
                x: node.x,
                y: node.y,
                fix_x: node.fix_x,
                fix_y: node.fix_y,
                load_x: node.load_x,
                load_y: node.load_y,
            })
            .collect(),
        elements: vec![],
    }
}

fn to_triangle_request(request: &SolveThermalPlaneQuad2dRequest) -> SolvePlaneTriangle2dRequest {
    SolvePlaneTriangle2dRequest {
        nodes: request
            .nodes
            .iter()
            .map(|node| kyuubiki_protocol::PlaneNodeInput {
                id: node.id.clone(),
                x: node.x,
                y: node.y,
                fix_x: node.fix_x,
                fix_y: node.fix_y,
                load_x: node.load_x,
                load_y: node.load_y,
            })
            .collect(),
        elements: vec![],
    }
}
