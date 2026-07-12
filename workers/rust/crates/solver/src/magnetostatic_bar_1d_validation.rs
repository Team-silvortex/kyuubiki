use kyuubiki_protocol::{MagnetostaticBar1dElementInput, SolveMagnetostaticBar1dRequest};

pub(crate) fn validate_request(request: &SolveMagnetostaticBar1dRequest) -> Result<(), String> {
    if request.nodes.len() < 2 {
        return Err("1d magnetostatic bar model must define at least two nodes".to_string());
    }
    if request.elements.is_empty() {
        return Err("1d magnetostatic bar model must define at least one element".to_string());
    }
    if !request.nodes.iter().any(|node| node.fix_magnetic_potential) {
        return Err(
            "1d magnetostatic bar model must include at least one magnetic potential support"
                .to_string(),
        );
    }

    for node in &request.nodes {
        if !node.x.is_finite() {
            return Err("1d magnetostatic bar node x must be finite".to_string());
        }
        if !node.magnetic_potential.is_finite() {
            return Err("1d magnetostatic bar node magnetic_potential must be finite".to_string());
        }
        if !node.magnetomotive_source.is_finite() {
            return Err(
                "1d magnetostatic bar node magnetomotive_source must be finite".to_string(),
            );
        }
    }

    for element in &request.elements {
        validate_element(request, element)?;
    }
    Ok(())
}

fn validate_element(
    request: &SolveMagnetostaticBar1dRequest,
    element: &MagnetostaticBar1dElementInput,
) -> Result<(), String> {
    if element.node_i >= request.nodes.len() || element.node_j >= request.nodes.len() {
        return Err("1d magnetostatic bar element references an out-of-range node".to_string());
    }
    if element.node_i == element.node_j {
        return Err("1d magnetostatic bar element must connect two distinct nodes".to_string());
    }
    if !element.area.is_finite() || element.area <= 0.0 {
        return Err("1d magnetostatic bar element area must be positive".to_string());
    }
    if !element.permeability.is_finite() || element.permeability <= 0.0 {
        return Err("1d magnetostatic bar element permeability must be positive".to_string());
    }

    let node_i = &request.nodes[element.node_i];
    let node_j = &request.nodes[element.node_j];
    let length = (node_j.x - node_i.x).abs();
    if !(length.is_finite() && length > 1.0e-12) {
        return Err("1d magnetostatic bar element length must be positive".to_string());
    }
    Ok(())
}
