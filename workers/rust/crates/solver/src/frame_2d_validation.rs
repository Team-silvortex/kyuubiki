use kyuubiki_protocol::{
    Frame2dNodeInput, SolveFrame2dRequest, SolveThermalFrame2dRequest, ThermalFrame2dNodeInput,
};

pub(crate) fn validate_frame_2d_request(request: &SolveFrame2dRequest) -> Result<(), String> {
    if request.nodes.len() < 2 {
        return Err("2d frame must define at least two nodes".to_string());
    }
    if request.elements.is_empty() {
        return Err("2d frame must define at least one element".to_string());
    }
    if !request
        .nodes
        .iter()
        .any(|node| node.fix_x || node.fix_y || node.fix_rz)
    {
        return Err("2d frame must include at least one support".to_string());
    }
    if frame_2d_constrained_dofs(&request.nodes) < 3 {
        return Err("2d frame must restrain at least three degrees of freedom".to_string());
    }
    for (index, node) in request.nodes.iter().enumerate() {
        validate_frame_2d_node(index, node)?;
    }
    for element in &request.elements {
        validate_frame_2d_element(request, element.node_i, element.node_j)?;
        if !(element.area.is_finite() && element.area > 0.0) {
            return Err("2d frame element area must be positive".to_string());
        }
        if !(element.youngs_modulus.is_finite() && element.youngs_modulus > 0.0) {
            return Err("2d frame element youngs_modulus must be positive".to_string());
        }
        if !(element.moment_of_inertia.is_finite() && element.moment_of_inertia > 0.0) {
            return Err("2d frame element moment_of_inertia must be positive".to_string());
        }
        if !(element.section_modulus.is_finite() && element.section_modulus > 0.0) {
            return Err("2d frame element section_modulus must be positive".to_string());
        }
    }
    Ok(())
}

pub(crate) fn validate_thermal_frame_2d_request(
    request: &SolveThermalFrame2dRequest,
) -> Result<(), String> {
    if request.nodes.len() < 2 {
        return Err("thermal frame must define at least two nodes".to_string());
    }
    if request.elements.is_empty() {
        return Err("thermal frame must define at least one element".to_string());
    }
    if !request
        .nodes
        .iter()
        .any(|node| node.fix_x || node.fix_y || node.fix_rz)
    {
        return Err("thermal frame must include at least one support".to_string());
    }
    for (index, node) in request.nodes.iter().enumerate() {
        validate_thermal_frame_2d_node(index, node)?;
    }
    for element in &request.elements {
        validate_thermal_frame_2d_element(request, element.node_i, element.node_j)?;
        if !(element.area.is_finite() && element.area > 0.0) {
            return Err("thermal frame element area must be positive".to_string());
        }
        if !(element.youngs_modulus.is_finite() && element.youngs_modulus > 0.0) {
            return Err("thermal frame element youngs_modulus must be positive".to_string());
        }
        if !(element.moment_of_inertia.is_finite() && element.moment_of_inertia > 0.0) {
            return Err("thermal frame element moment_of_inertia must be positive".to_string());
        }
        if !(element.section_modulus.is_finite() && element.section_modulus > 0.0) {
            return Err("thermal frame element section_modulus must be positive".to_string());
        }
        if !(element.thermal_expansion.is_finite() && element.thermal_expansion >= 0.0) {
            return Err("thermal frame element thermal_expansion must be non-negative".to_string());
        }
        if !(element.section_depth.is_finite() && element.section_depth > 0.0) {
            return Err("thermal frame element section_depth must be positive".to_string());
        }
        if !element.temperature_gradient_y.is_finite() {
            return Err("thermal frame element temperature_gradient_y must be finite".to_string());
        }
    }
    Ok(())
}

fn validate_frame_2d_node(index: usize, node: &Frame2dNodeInput) -> Result<(), String> {
    if !node.x.is_finite() || !node.y.is_finite() {
        return Err(format!("2d frame node {index} has invalid coordinates"));
    }
    if !node.load_x.is_finite() || !node.load_y.is_finite() || !node.moment_z.is_finite() {
        return Err(format!("2d frame node {index} has invalid load"));
    }
    Ok(())
}

fn validate_thermal_frame_2d_node(
    index: usize,
    node: &ThermalFrame2dNodeInput,
) -> Result<(), String> {
    if !node.x.is_finite() || !node.y.is_finite() {
        return Err(format!(
            "thermal frame node {index} has invalid coordinates"
        ));
    }
    if !node.load_x.is_finite() || !node.load_y.is_finite() || !node.moment_z.is_finite() {
        return Err(format!("thermal frame node {index} has invalid load"));
    }
    if !node.temperature_delta.is_finite() {
        return Err("thermal frame node temperature_delta must be finite".to_string());
    }
    Ok(())
}

fn validate_frame_2d_element(
    request: &SolveFrame2dRequest,
    node_i: usize,
    node_j: usize,
) -> Result<(), String> {
    if node_i >= request.nodes.len() || node_j >= request.nodes.len() {
        return Err("2d frame element references an out-of-range node".to_string());
    }
    if node_i == node_j {
        return Err("2d frame element must connect two distinct nodes".to_string());
    }
    validate_length(
        request.nodes[node_j].x - request.nodes[node_i].x,
        request.nodes[node_j].y - request.nodes[node_i].y,
        "2d frame element length must be positive",
    )
}

fn validate_thermal_frame_2d_element(
    request: &SolveThermalFrame2dRequest,
    node_i: usize,
    node_j: usize,
) -> Result<(), String> {
    if node_i >= request.nodes.len() || node_j >= request.nodes.len() {
        return Err("thermal frame element references an out-of-range node".to_string());
    }
    if node_i == node_j {
        return Err("thermal frame element cannot connect a node to itself".to_string());
    }
    validate_length(
        request.nodes[node_j].x - request.nodes[node_i].x,
        request.nodes[node_j].y - request.nodes[node_i].y,
        "thermal frame element length must be positive",
    )
}

fn validate_length(dx: f64, dy: f64, message: &str) -> Result<(), String> {
    let length = (dx * dx + dy * dy).sqrt();
    if !(length.is_finite() && length > 0.0) {
        return Err(message.to_string());
    }
    Ok(())
}

fn frame_2d_constrained_dofs(nodes: &[Frame2dNodeInput]) -> usize {
    nodes.iter().fold(0, |sum, node| {
        sum + usize::from(node.fix_x) + usize::from(node.fix_y) + usize::from(node.fix_rz)
    })
}
