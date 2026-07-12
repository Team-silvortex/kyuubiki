use kyuubiki_protocol::{
    SolveStokesFlowPlaneQuad2dRequest, SolveStokesFlowPlaneTriangle2dRequest,
    StokesFlowPlaneNodeInput,
};

pub(crate) fn validate_quad_request(
    request: &SolveStokesFlowPlaneQuad2dRequest,
) -> Result<(), String> {
    if request.nodes.len() < 4 {
        return Err("stokes flow quad model requires at least four nodes".into());
    }
    if request.elements.is_empty() {
        return Err("stokes flow quad model requires at least one element".into());
    }

    validate_nodes(&request.nodes)?;

    for (index, element) in request.elements.iter().enumerate() {
        validate_node_index(request.nodes.len(), element.node_i, index, "node_i")?;
        validate_node_index(request.nodes.len(), element.node_j, index, "node_j")?;
        validate_node_index(request.nodes.len(), element.node_k, index, "node_k")?;
        validate_node_index(request.nodes.len(), element.node_l, index, "node_l")?;
        if has_duplicate(&[
            element.node_i,
            element.node_j,
            element.node_k,
            element.node_l,
        ]) {
            return Err(format!(
                "stokes flow quad element {index} must reference four distinct nodes"
            ));
        }
        validate_stokes_material(index, element.viscosity, element.density, element.thickness)?;
    }

    Ok(())
}

pub(crate) fn validate_triangle_request(
    request: &SolveStokesFlowPlaneTriangle2dRequest,
) -> Result<(), String> {
    if request.nodes.len() < 3 {
        return Err("stokes flow triangle model requires at least three nodes".into());
    }
    if request.elements.is_empty() {
        return Err("stokes flow triangle model requires at least one element".into());
    }
    validate_nodes(&request.nodes)?;
    for (index, element) in request.elements.iter().enumerate() {
        validate_node_index(request.nodes.len(), element.node_i, index, "node_i")?;
        validate_node_index(request.nodes.len(), element.node_j, index, "node_j")?;
        validate_node_index(request.nodes.len(), element.node_k, index, "node_k")?;
        if has_duplicate(&[element.node_i, element.node_j, element.node_k]) {
            return Err(format!(
                "stokes flow triangle element {index} must reference three distinct nodes"
            ));
        }
        validate_stokes_material(index, element.viscosity, element.density, element.thickness)?;
    }
    Ok(())
}

fn validate_nodes(nodes: &[StokesFlowPlaneNodeInput]) -> Result<(), String> {
    for (index, node) in nodes.iter().enumerate() {
        if !(node.x.is_finite() && node.y.is_finite()) {
            return Err(format!(
                "stokes flow node {index} coordinates must be finite"
            ));
        }
        if !(node.velocity_x.is_finite() && node.velocity_y.is_finite()) {
            return Err(format!("stokes flow node {index} velocity must be finite"));
        }
        if !node.pressure.is_finite() {
            return Err(format!("stokes flow node {index} pressure must be finite"));
        }
        if !(node.body_force_x.is_finite() && node.body_force_y.is_finite()) {
            return Err(format!(
                "stokes flow node {index} body force must be finite"
            ));
        }
    }
    Ok(())
}

fn validate_stokes_material(
    index: usize,
    viscosity: f64,
    density: f64,
    thickness: f64,
) -> Result<(), String> {
    if !viscosity.is_finite() {
        return Err(format!(
            "stokes flow element {index} viscosity must be finite"
        ));
    }
    if viscosity <= 0.0 {
        return Err(format!(
            "stokes flow element {index} has non-positive viscosity"
        ));
    }
    if !density.is_finite() {
        return Err(format!(
            "stokes flow element {index} density must be finite"
        ));
    }
    if density < 0.0 {
        return Err(format!("stokes flow element {index} has negative density"));
    }
    if !thickness.is_finite() {
        return Err(format!(
            "stokes flow element {index} thickness must be finite"
        ));
    }
    if thickness <= 0.0 {
        return Err(format!(
            "stokes flow element {index} has non-positive thickness"
        ));
    }
    Ok(())
}

fn validate_node_index(
    node_count: usize,
    node_index: usize,
    element_index: usize,
    field: &str,
) -> Result<(), String> {
    if node_index >= node_count {
        Err(format!(
            "stokes flow element {element_index} {field} references missing node {node_index}"
        ))
    } else {
        Ok(())
    }
}

fn has_duplicate(indices: &[usize]) -> bool {
    indices
        .iter()
        .enumerate()
        .any(|(index, value)| indices[index + 1..].contains(value))
}
