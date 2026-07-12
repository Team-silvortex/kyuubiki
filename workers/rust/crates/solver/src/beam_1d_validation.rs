use kyuubiki_protocol::{SolveBeam1dRequest, SolveThermalBeam1dRequest};

pub(crate) fn validate_beam_1d_request(request: &SolveBeam1dRequest) -> Result<(), String> {
    if request.nodes.len() < 2 {
        return Err("1d beam must define at least two nodes".to_string());
    }

    if request.elements.is_empty() {
        return Err("1d beam must define at least one element".to_string());
    }

    let constrained_dofs = request.nodes.iter().fold(0, |sum, node| {
        sum + usize::from(node.fix_y) + usize::from(node.fix_rz)
    });
    if constrained_dofs < 2 {
        return Err("1d beam must restrain at least two degrees of freedom".to_string());
    }

    for (index, node) in request.nodes.iter().enumerate() {
        if !node.x.is_finite() {
            return Err(format!("1d beam node {index} has invalid x"));
        }
        if !node.load_y.is_finite() || !node.moment_z.is_finite() {
            return Err(format!("1d beam node {index} has invalid load"));
        }
    }

    for element in &request.elements {
        if element.node_i >= request.nodes.len() || element.node_j >= request.nodes.len() {
            return Err("1d beam element references an out-of-range node".to_string());
        }
        if element.node_i == element.node_j {
            return Err("1d beam element must connect two distinct nodes".to_string());
        }
        if !(element.youngs_modulus.is_finite() && element.youngs_modulus > 0.0) {
            return Err("1d beam element youngs_modulus must be positive".to_string());
        }
        if !(element.moment_of_inertia.is_finite() && element.moment_of_inertia > 0.0) {
            return Err("1d beam element moment_of_inertia must be positive".to_string());
        }
        if !(element.section_modulus.is_finite() && element.section_modulus > 0.0) {
            return Err("1d beam element section_modulus must be positive".to_string());
        }
        if !element.distributed_load_y.is_finite() {
            return Err("1d beam element distributed_load_y must be finite".to_string());
        }

        let node_i = &request.nodes[element.node_i];
        let node_j = &request.nodes[element.node_j];
        let length = (node_j.x - node_i.x).abs();
        if !length.is_finite() || length <= 1.0e-12 {
            return Err("1d beam element length must be positive".to_string());
        }
    }

    Ok(())
}

pub(crate) fn validate_thermal_beam_1d_request(
    request: &SolveThermalBeam1dRequest,
) -> Result<(), String> {
    if request.nodes.len() < 2 {
        return Err("thermal beam requires at least two nodes".to_string());
    }

    if request.elements.is_empty() {
        return Err("thermal beam requires at least one element".to_string());
    }

    for (index, node) in request.nodes.iter().enumerate() {
        if !node.x.is_finite() {
            return Err(format!("thermal beam node {index} has invalid x"));
        }
        if !node.load_y.is_finite() || !node.moment_z.is_finite() {
            return Err(format!("thermal beam node {index} has invalid load"));
        }
    }

    for (index, element) in request.elements.iter().enumerate() {
        if element.node_i >= request.nodes.len() || element.node_j >= request.nodes.len() {
            return Err(format!(
                "thermal beam element {index} references an unknown node"
            ));
        }

        if element.node_i == element.node_j {
            return Err(format!(
                "thermal beam element {index} must connect two distinct nodes"
            ));
        }

        let node_i = &request.nodes[element.node_i];
        let node_j = &request.nodes[element.node_j];
        let length = (node_j.x - node_i.x).abs();
        if !(length.is_finite() && length > f64::EPSILON) {
            return Err(format!("thermal beam element {index} has zero length"));
        }

        if !(element.youngs_modulus.is_finite() && element.youngs_modulus > 0.0)
            || !(element.moment_of_inertia.is_finite() && element.moment_of_inertia > 0.0)
            || !(element.section_modulus.is_finite() && element.section_modulus > 0.0)
            || !(element.section_depth.is_finite() && element.section_depth > 0.0)
        {
            return Err(format!(
                "thermal beam element {index} must have positive stiffness and section properties"
            ));
        }

        if !element.thermal_expansion.is_finite()
            || !element.distributed_load_y.is_finite()
            || !element.temperature_gradient_y.is_finite()
        {
            return Err(format!(
                "thermal beam element {index} has invalid thermal load data"
            ));
        }
    }

    Ok(())
}
