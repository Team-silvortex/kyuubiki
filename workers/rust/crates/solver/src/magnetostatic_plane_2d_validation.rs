use kyuubiki_protocol::{
    MagnetostaticPlaneNodeInput, MagnetostaticPlaneQuadElementInput,
    MagnetostaticPlaneTriangleElementInput, SolveMagnetostaticPlaneQuad2dRequest,
    SolveMagnetostaticPlaneTriangle2dRequest,
};

pub(crate) fn validate_magnetostatic_plane_triangle_request(
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
    validate_nodes(&request.nodes, "magnetostatic plane triangle")?;
    for element in &request.elements {
        validate_triangle_element(request, element)?;
    }
    Ok(())
}

pub(crate) fn validate_magnetostatic_plane_quad_request(
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
    validate_nodes(&request.nodes, "magnetostatic plane quad")?;
    for element in &request.elements {
        validate_quad_element(request, element)?;
    }
    Ok(())
}

fn validate_nodes(nodes: &[MagnetostaticPlaneNodeInput], label: &str) -> Result<(), String> {
    for node in nodes {
        if !(node.x.is_finite() && node.y.is_finite()) {
            return Err(format!("{label} node coordinates must be finite"));
        }
        if !node.vector_potential.is_finite() {
            return Err(format!("{label} node vector_potential must be finite"));
        }
        if !node.current_density.is_finite() {
            return Err(format!("{label} node current_density must be finite"));
        }
    }
    Ok(())
}

fn validate_triangle_element(
    request: &SolveMagnetostaticPlaneTriangle2dRequest,
    element: &MagnetostaticPlaneTriangleElementInput,
) -> Result<(), String> {
    let indices = [element.node_i, element.node_j, element.node_k];
    if indices.iter().any(|&index| index >= request.nodes.len()) {
        return Err(
            "magnetostatic plane triangle element references an out-of-range node".to_string(),
        );
    }
    if has_duplicate(&indices) {
        return Err(
            "magnetostatic plane triangle element must reference three distinct nodes".to_string(),
        );
    }
    validate_material(
        element.thickness,
        element.permeability,
        "magnetostatic plane triangle",
    )?;
    let ni = &request.nodes[element.node_i];
    let nj = &request.nodes[element.node_j];
    let nk = &request.nodes[element.node_k];
    let area = signed_triangle_area(ni.x, ni.y, nj.x, nj.y, nk.x, nk.y).abs();
    if !(area.is_finite() && area > 1.0e-12) {
        return Err("magnetostatic plane triangle element area must be positive".to_string());
    }
    Ok(())
}

fn validate_quad_element(
    request: &SolveMagnetostaticPlaneQuad2dRequest,
    element: &MagnetostaticPlaneQuadElementInput,
) -> Result<(), String> {
    let indices = [
        element.node_i,
        element.node_j,
        element.node_k,
        element.node_l,
    ];
    if indices.iter().any(|&index| index >= request.nodes.len()) {
        return Err("magnetostatic plane quad element references an out-of-range node".to_string());
    }
    if has_duplicate(&indices) {
        return Err(
            "magnetostatic plane quad element must reference four distinct nodes".to_string(),
        );
    }
    validate_material(
        element.thickness,
        element.permeability,
        "magnetostatic plane quad",
    )?;
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
    if !(first_area.is_finite() && first_area > 1.0e-12)
        || !(second_area.is_finite() && second_area > 1.0e-12)
    {
        return Err("magnetostatic plane quad triangles must have positive area".to_string());
    }
    Ok(())
}

fn validate_material(thickness: f64, permeability: f64, label: &str) -> Result<(), String> {
    if !thickness.is_finite() || thickness <= 0.0 {
        return Err(format!("{label} thickness must be positive"));
    }
    if !permeability.is_finite() || permeability <= 0.0 {
        return Err(format!("{label} permeability must be positive"));
    }
    Ok(())
}

fn signed_triangle_area(ix: f64, iy: f64, jx: f64, jy: f64, kx: f64, ky: f64) -> f64 {
    0.5 * ((jx - ix) * (ky - iy) - (kx - ix) * (jy - iy))
}

fn has_duplicate(indices: &[usize]) -> bool {
    indices
        .iter()
        .enumerate()
        .any(|(left, value)| indices[(left + 1)..].contains(value))
}
