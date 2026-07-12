use kyuubiki_protocol::{SolveSpring1dRequest, SolveSpring2dRequest, SolveSpring3dRequest};

pub(crate) fn validate_spring_1d_request(request: &SolveSpring1dRequest) -> Result<(), String> {
    if request.nodes.len() < 2 {
        return Err("1d spring model must define at least two nodes".to_string());
    }

    if request.elements.is_empty() {
        return Err("1d spring model must define at least one element".to_string());
    }

    if !request.nodes.iter().any(|node| node.fix_x) {
        return Err("1d spring model must restrain at least one degree of freedom".to_string());
    }

    for (index, node) in request.nodes.iter().enumerate() {
        if !node.x.is_finite() || !node.load_x.is_finite() {
            return Err(format!(
                "1d spring node {index} coordinates and loads must be finite"
            ));
        }
    }

    for element in &request.elements {
        if element.node_i >= request.nodes.len() || element.node_j >= request.nodes.len() {
            return Err("1d spring element references an out-of-range node".to_string());
        }
        if element.node_i == element.node_j {
            return Err("1d spring element must connect two distinct nodes".to_string());
        }
        if !(element.stiffness.is_finite() && element.stiffness > 0.0) {
            return Err("1d spring element stiffness must be positive".to_string());
        }

        let node_i = &request.nodes[element.node_i];
        let node_j = &request.nodes[element.node_j];
        let length = (node_j.x - node_i.x).abs();
        if !(length.is_finite() && length > 1.0e-12) {
            return Err("1d spring element length must be positive".to_string());
        }
    }

    Ok(())
}

pub(crate) fn validate_spring_2d_request(request: &SolveSpring2dRequest) -> Result<(), String> {
    if request.nodes.len() < 2 {
        return Err("2d spring model must define at least two nodes".to_string());
    }

    if request.elements.is_empty() {
        return Err("2d spring model must define at least one element".to_string());
    }

    if !request.nodes.iter().any(|node| node.fix_x || node.fix_y) {
        return Err("2d spring model must restrain at least one degree of freedom".to_string());
    }

    for (index, node) in request.nodes.iter().enumerate() {
        if !node.x.is_finite()
            || !node.y.is_finite()
            || !node.load_x.is_finite()
            || !node.load_y.is_finite()
        {
            return Err(format!(
                "2d spring node {index} coordinates and loads must be finite"
            ));
        }
    }

    for element in &request.elements {
        if element.node_i >= request.nodes.len() || element.node_j >= request.nodes.len() {
            return Err("2d spring element references an out-of-range node".to_string());
        }
        if element.node_i == element.node_j {
            return Err("2d spring element must connect two distinct nodes".to_string());
        }
        if !(element.stiffness.is_finite() && element.stiffness > 0.0) {
            return Err("2d spring element stiffness must be positive".to_string());
        }

        let node_i = &request.nodes[element.node_i];
        let node_j = &request.nodes[element.node_j];
        let dx = node_j.x - node_i.x;
        let dy = node_j.y - node_i.y;
        let length = (dx * dx + dy * dy).sqrt();
        if !(length.is_finite() && length > 1.0e-12) {
            return Err("2d spring element length must be positive".to_string());
        }
    }

    Ok(())
}

pub(crate) fn validate_spring_3d_request(request: &SolveSpring3dRequest) -> Result<(), String> {
    if request.nodes.len() < 2 {
        return Err("3d spring model must define at least two nodes".to_string());
    }

    if request.elements.is_empty() {
        return Err("3d spring model must define at least one element".to_string());
    }

    if !request
        .nodes
        .iter()
        .any(|node| node.fix_x || node.fix_y || node.fix_z)
    {
        return Err("3d spring model must restrain at least one degree of freedom".to_string());
    }

    for (index, node) in request.nodes.iter().enumerate() {
        if !node.x.is_finite()
            || !node.y.is_finite()
            || !node.z.is_finite()
            || !node.load_x.is_finite()
            || !node.load_y.is_finite()
            || !node.load_z.is_finite()
        {
            return Err(format!(
                "3d spring node {index} coordinates and loads must be finite"
            ));
        }
    }

    for element in &request.elements {
        if element.node_i >= request.nodes.len() || element.node_j >= request.nodes.len() {
            return Err("3d spring element references an out-of-range node".to_string());
        }
        if element.node_i == element.node_j {
            return Err("3d spring element must connect two distinct nodes".to_string());
        }
        if !(element.stiffness.is_finite() && element.stiffness > 0.0) {
            return Err("3d spring element stiffness must be positive".to_string());
        }

        let node_i = &request.nodes[element.node_i];
        let node_j = &request.nodes[element.node_j];
        let dx = node_j.x - node_i.x;
        let dy = node_j.y - node_i.y;
        let dz = node_j.z - node_i.z;
        let length = (dx * dx + dy * dy + dz * dz).sqrt();
        if !(length.is_finite() && length > 1.0e-12) {
            return Err("3d spring element length must be positive".to_string());
        }
    }

    Ok(())
}
