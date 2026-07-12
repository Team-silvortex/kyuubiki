use kyuubiki_protocol::{
    ModalFrame2dElementInput, ModalFrame3dElementInput, SolveModalFrame2dRequest,
    SolveModalFrame3dRequest,
};

pub(crate) fn validate_modal_frame_2d_request(
    request: &SolveModalFrame2dRequest,
) -> Result<(), String> {
    if request.nodes.len() < 2 {
        return Err("modal frame 2d must define at least two nodes".to_string());
    }
    if request.elements.is_empty() {
        return Err("modal frame 2d must define at least one element".to_string());
    }
    if constrained_2d_count(request) < 3 {
        return Err("modal frame 2d must restrain at least three degrees of freedom".to_string());
    }
    for (index, node) in request.nodes.iter().enumerate() {
        if !node.x.is_finite() || !node.y.is_finite() {
            return Err(format!(
                "modal frame 2d node {index} has invalid coordinates"
            ));
        }
        if !node.load_x.is_finite() || !node.load_y.is_finite() || !node.moment_z.is_finite() {
            return Err(format!("modal frame 2d node {index} has invalid load"));
        }
    }
    for element in &request.elements {
        validate_modal_frame_2d_element(request, element)?;
    }
    Ok(())
}

pub(crate) fn validate_modal_frame_3d_request(
    request: &SolveModalFrame3dRequest,
) -> Result<(), String> {
    if request.nodes.len() < 2 {
        return Err("modal frame 3d must define at least two nodes".to_string());
    }
    if request.elements.is_empty() {
        return Err("modal frame 3d must define at least one element".to_string());
    }
    if constrained_3d_count(request) < 6 {
        return Err("modal frame 3d must restrain at least six degrees of freedom".to_string());
    }
    for (index, node) in request.nodes.iter().enumerate() {
        if !node.x.is_finite() || !node.y.is_finite() || !node.z.is_finite() {
            return Err(format!(
                "modal frame 3d node {index} has invalid coordinates"
            ));
        }
        if !node.load_x.is_finite() || !node.load_y.is_finite() || !node.load_z.is_finite() {
            return Err(format!("modal frame 3d node {index} has invalid load"));
        }
        if !node.moment_x.is_finite() || !node.moment_y.is_finite() || !node.moment_z.is_finite() {
            return Err(format!("modal frame 3d node {index} has invalid moment"));
        }
    }
    for element in &request.elements {
        validate_modal_frame_3d_element(request, element)?;
    }
    Ok(())
}

fn validate_modal_frame_2d_element(
    request: &SolveModalFrame2dRequest,
    element: &ModalFrame2dElementInput,
) -> Result<(), String> {
    if element.node_i >= request.nodes.len() || element.node_j >= request.nodes.len() {
        return Err("modal frame 2d element references an out-of-range node".to_string());
    }
    if element.node_i == element.node_j {
        return Err("modal frame 2d element must connect two distinct nodes".to_string());
    }
    for (label, value) in [
        ("area", element.area),
        ("youngs_modulus", element.youngs_modulus),
        ("moment_of_inertia", element.moment_of_inertia),
        ("section_modulus", element.section_modulus),
        ("density", element.density),
    ] {
        if !(value.is_finite() && value > 0.0) {
            return Err(format!("modal frame 2d element {label} must be positive"));
        }
    }
    let node_i = &request.nodes[element.node_i];
    let node_j = &request.nodes[element.node_j];
    let length = ((node_j.x - node_i.x).powi(2) + (node_j.y - node_i.y).powi(2)).sqrt();
    if !(length.is_finite() && length > 1.0e-12) {
        return Err("modal frame 2d element length must be positive".to_string());
    }
    Ok(())
}

fn validate_modal_frame_3d_element(
    request: &SolveModalFrame3dRequest,
    element: &ModalFrame3dElementInput,
) -> Result<(), String> {
    if element.node_i >= request.nodes.len() || element.node_j >= request.nodes.len() {
        return Err("modal frame 3d element references an out-of-range node".to_string());
    }
    if element.node_i == element.node_j {
        return Err("modal frame 3d element must connect two distinct nodes".to_string());
    }
    for (label, value) in [
        ("area", element.area),
        ("youngs_modulus", element.youngs_modulus),
        ("shear_modulus", element.shear_modulus),
        ("torsion_constant", element.torsion_constant),
        ("moment_of_inertia_y", element.moment_of_inertia_y),
        ("moment_of_inertia_z", element.moment_of_inertia_z),
        ("density", element.density),
    ] {
        if !(value.is_finite() && value > 0.0) {
            return Err(format!("modal frame 3d element {label} must be positive"));
        }
    }
    let node_i = &request.nodes[element.node_i];
    let node_j = &request.nodes[element.node_j];
    let dx = node_j.x - node_i.x;
    let dy = node_j.y - node_i.y;
    let dz = node_j.z - node_i.z;
    let length = (dx * dx + dy * dy + dz * dz).sqrt();
    if !(length.is_finite() && length > 1.0e-12) {
        return Err("modal frame 3d element length must be positive".to_string());
    }
    Ok(())
}

fn constrained_2d_count(request: &SolveModalFrame2dRequest) -> usize {
    request
        .nodes
        .iter()
        .map(|node| node.fix_x as usize + node.fix_y as usize + node.fix_rz as usize)
        .sum()
}

fn constrained_3d_count(request: &SolveModalFrame3dRequest) -> usize {
    request
        .nodes
        .iter()
        .map(|node| {
            node.fix_x as usize
                + node.fix_y as usize
                + node.fix_z as usize
                + node.fix_rx as usize
                + node.fix_ry as usize
                + node.fix_rz as usize
        })
        .sum()
}
