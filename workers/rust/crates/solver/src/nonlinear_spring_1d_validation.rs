use kyuubiki_protocol::{SolveContactGap1dRequest, SolveNonlinearSpring1dRequest};

pub(crate) fn validate_request(request: &SolveNonlinearSpring1dRequest) -> Result<(), String> {
    if request.nodes.len() < 2 {
        return Err("nonlinear spring model must define at least two nodes".to_string());
    }
    if request.elements.is_empty() {
        return Err("nonlinear spring model must define at least one element".to_string());
    }
    if !request.nodes.iter().any(|node| node.fix_x) {
        return Err(
            "nonlinear spring model must restrain at least one degree of freedom".to_string(),
        );
    }
    if request
        .tolerance
        .is_some_and(|value| !(value.is_finite() && value > 0.0))
    {
        return Err("nonlinear spring tolerance must be positive".to_string());
    }

    for (index, node) in request.nodes.iter().enumerate() {
        if !(node.x.is_finite() && node.load_x.is_finite()) {
            return Err(format!(
                "nonlinear spring node {index} coordinates and loads must be finite"
            ));
        }
    }

    for element in &request.elements {
        if element.node_i >= request.nodes.len() || element.node_j >= request.nodes.len() {
            return Err("nonlinear spring element references an out-of-range node".to_string());
        }
        if element.node_i == element.node_j {
            return Err("nonlinear spring element must connect two distinct nodes".to_string());
        }
        if !(element.stiffness.is_finite() && element.stiffness > 0.0) {
            return Err("nonlinear spring element stiffness must be positive".to_string());
        }
        if !element.cubic_stiffness.is_finite() {
            return Err("nonlinear spring cubic stiffness must be finite".to_string());
        }
        if element.stiffness + element.cubic_stiffness < 1.0e-12 {
            return Err(
                "nonlinear spring tangent stiffness must stay positive near unit extension"
                    .to_string(),
            );
        }
        let length = (request.nodes[element.node_j].x - request.nodes[element.node_i].x).abs();
        if !(length.is_finite() && length > 1.0e-12) {
            return Err("nonlinear spring element length must be positive".to_string());
        }
    }

    Ok(())
}

pub(crate) fn validate_contact_request(request: &SolveContactGap1dRequest) -> Result<(), String> {
    let spring_request = SolveNonlinearSpring1dRequest {
        nodes: request.nodes.clone(),
        elements: request.elements.clone(),
        load_steps: request.load_steps,
        max_iterations: request.max_iterations,
        tolerance: request.tolerance,
    };
    validate_request(&spring_request)?;

    if request.contacts.is_empty() {
        return Err("contact gap model must define at least one contact".to_string());
    }

    for contact in &request.contacts {
        if contact.node >= request.nodes.len() {
            return Err("contact gap references an out-of-range node".to_string());
        }
        if !(contact.gap.is_finite() && contact.gap >= 0.0) {
            return Err("contact gap must be finite and non-negative".to_string());
        }
        if !(contact.normal_stiffness.is_finite() && contact.normal_stiffness > 0.0) {
            return Err("contact normal stiffness must be positive".to_string());
        }
    }

    Ok(())
}
