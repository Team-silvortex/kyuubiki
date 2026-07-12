use kyuubiki_protocol::{SolveTorsion1dRequest, Torsion1dElementInput};

pub(crate) fn validate_request(request: &SolveTorsion1dRequest) -> Result<(), String> {
    if request.nodes.len() < 2 {
        return Err("1d torsion model must define at least two nodes".to_string());
    }
    if request.elements.is_empty() {
        return Err("1d torsion model must define at least one element".to_string());
    }
    if !request.nodes.iter().any(|node| node.fix_rz) {
        return Err("1d torsion model must include at least one rotational support".to_string());
    }

    for (index, node) in request.nodes.iter().enumerate() {
        if !node.x.is_finite() {
            return Err(format!("1d torsion node {index} has invalid x"));
        }
        if !node.torque_z.is_finite() {
            return Err(format!("1d torsion node {index} has invalid torque"));
        }
    }

    for element in &request.elements {
        validate_element(request, element)?;
    }
    Ok(())
}

fn validate_element(
    request: &SolveTorsion1dRequest,
    element: &Torsion1dElementInput,
) -> Result<(), String> {
    if element.node_i >= request.nodes.len() || element.node_j >= request.nodes.len() {
        return Err("1d torsion element references an out-of-range node".to_string());
    }
    if element.node_i == element.node_j {
        return Err("1d torsion element must connect two distinct nodes".to_string());
    }
    if !(element.shear_modulus.is_finite() && element.shear_modulus > 0.0) {
        return Err("1d torsion element shear_modulus must be positive".to_string());
    }
    if !(element.polar_moment.is_finite() && element.polar_moment > 0.0) {
        return Err("1d torsion element polar_moment must be positive".to_string());
    }
    if !(element.section_modulus.is_finite() && element.section_modulus > 0.0) {
        return Err("1d torsion element section_modulus must be positive".to_string());
    }

    let node_i = &request.nodes[element.node_i];
    let node_j = &request.nodes[element.node_j];
    let length = (node_j.x - node_i.x).abs();
    if !(length.is_finite() && length > 1.0e-12) {
        return Err("1d torsion element length must be positive".to_string());
    }
    Ok(())
}
