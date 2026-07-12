use kyuubiki_protocol::{AcousticBar1dElementInput, SolveAcousticBar1dRequest};

pub(crate) fn validate_request(request: &SolveAcousticBar1dRequest) -> Result<(), String> {
    if request.frequency_hz <= 0.0 || !request.frequency_hz.is_finite() {
        return Err("frequency_hz must be positive".to_string());
    }
    if request.nodes.len() < 2 {
        return Err("at least two acoustic nodes are required".to_string());
    }
    if request.elements.is_empty() {
        return Err("at least one acoustic element is required".to_string());
    }
    if !request.nodes.iter().any(|node| node.fix_pressure) {
        return Err("at least one pressure boundary condition is required".to_string());
    }

    for (index, node) in request.nodes.iter().enumerate() {
        if !node.x.is_finite() {
            return Err(format!("node {index} x must be finite"));
        }
        if !node.pressure.is_finite() {
            return Err(format!("node {index} pressure must be finite"));
        }
        if !node.volume_velocity_source.is_finite() {
            return Err(format!(
                "node {index} volume_velocity_source must be finite"
            ));
        }
    }
    for (index, element) in request.elements.iter().enumerate() {
        validate_element(request, element, index)?;
    }
    Ok(())
}

fn validate_element(
    request: &SolveAcousticBar1dRequest,
    element: &AcousticBar1dElementInput,
    index: usize,
) -> Result<(), String> {
    let count = request.nodes.len();
    if element.node_i >= count || element.node_j >= count || element.node_i == element.node_j {
        return Err(format!("element {index} node indices are invalid"));
    }
    if element.area <= 0.0 || !element.area.is_finite() {
        return Err(format!("element {index} area must be positive"));
    }
    if element.density <= 0.0 || !element.density.is_finite() {
        return Err(format!("element {index} density must be positive"));
    }
    if element.bulk_modulus <= 0.0 || !element.bulk_modulus.is_finite() {
        return Err(format!("element {index} bulk_modulus must be positive"));
    }
    if element.damping_ratio < 0.0 || !element.damping_ratio.is_finite() {
        return Err(format!(
            "element {index} damping_ratio must be non-negative"
        ));
    }

    let xi = request.nodes[element.node_i].x;
    let xj = request.nodes[element.node_j].x;
    let length = (xj - xi).abs();
    if !(length.is_finite() && length > 1.0e-12) {
        return Err(format!("element {index} length must be positive"));
    }
    Ok(())
}
