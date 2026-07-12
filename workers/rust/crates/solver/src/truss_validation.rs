use kyuubiki_protocol::{SolveTruss2dRequest, SolveTruss3dRequest};

pub(crate) fn validate_truss_request(request: &SolveTruss2dRequest) -> Result<(), String> {
    if request.nodes.len() < 2 {
        return Err("truss must define at least two nodes".to_string());
    }

    if request.elements.is_empty() {
        return Err("truss must define at least one element".to_string());
    }

    if !request.nodes.iter().any(|node| node.fix_x || node.fix_y) {
        return Err("truss must include at least one support".to_string());
    }

    for (index, node) in request.nodes.iter().enumerate() {
        if !node.x.is_finite() || !node.y.is_finite() {
            return Err(format!("truss node {index} has invalid coordinates"));
        }
        if !node.load_x.is_finite() || !node.load_y.is_finite() {
            return Err(format!("truss node {index} has invalid load"));
        }
    }

    for element in &request.elements {
        if element.node_i >= request.nodes.len() || element.node_j >= request.nodes.len() {
            return Err("truss element references an out-of-range node".to_string());
        }
        if element.node_i == element.node_j {
            return Err("truss element must connect two distinct nodes".to_string());
        }

        if !(element.area.is_finite() && element.area > 0.0) {
            return Err("truss element area must be positive".to_string());
        }

        if !(element.youngs_modulus.is_finite() && element.youngs_modulus > 0.0) {
            return Err("truss element youngs_modulus must be positive".to_string());
        }

        let node_i = &request.nodes[element.node_i];
        let node_j = &request.nodes[element.node_j];
        let length = ((node_j.x - node_i.x).powi(2) + (node_j.y - node_i.y).powi(2)).sqrt();
        if !length.is_finite() || length <= 1.0e-12 {
            return Err("truss element length must be positive".to_string());
        }
    }

    Ok(())
}

pub(crate) fn validate_truss_3d_request(request: &SolveTruss3dRequest) -> Result<(), String> {
    if request.nodes.len() < 3 {
        return Err("3d truss must define at least three nodes".to_string());
    }

    if request.elements.is_empty() {
        return Err("3d truss must define at least one element".to_string());
    }

    let constrained_dofs = request.nodes.iter().fold(0, |sum, node| {
        sum + usize::from(node.fix_x) + usize::from(node.fix_y) + usize::from(node.fix_z)
    });
    if constrained_dofs < 6 {
        return Err("3d truss must restrain at least six degrees of freedom".to_string());
    }

    for (index, node) in request.nodes.iter().enumerate() {
        if !node.x.is_finite() || !node.y.is_finite() || !node.z.is_finite() {
            return Err(format!("3d truss node {index} has invalid coordinates"));
        }
        if !node.load_x.is_finite() || !node.load_y.is_finite() || !node.load_z.is_finite() {
            return Err(format!("3d truss node {index} has invalid load"));
        }
    }

    for element in &request.elements {
        if element.node_i >= request.nodes.len() || element.node_j >= request.nodes.len() {
            return Err("3d truss element references an out-of-range node".to_string());
        }
        if element.node_i == element.node_j {
            return Err("3d truss element must connect two distinct nodes".to_string());
        }
        if !(element.area.is_finite() && element.area > 0.0) {
            return Err("3d truss element area must be positive".to_string());
        }
        if !(element.youngs_modulus.is_finite() && element.youngs_modulus > 0.0) {
            return Err("3d truss element youngs_modulus must be positive".to_string());
        }
        let node_i = &request.nodes[element.node_i];
        let node_j = &request.nodes[element.node_j];
        let length = ((node_j.x - node_i.x).powi(2)
            + (node_j.y - node_i.y).powi(2)
            + (node_j.z - node_i.z).powi(2))
        .sqrt();
        if !length.is_finite() || length <= 1.0e-12 {
            return Err("3d truss element length must be positive".to_string());
        }
    }

    Ok(())
}
