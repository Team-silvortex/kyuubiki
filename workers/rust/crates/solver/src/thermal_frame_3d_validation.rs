use kyuubiki_protocol::{SolveThermalFrame3dRequest, ThermalFrame3dElementInput};

pub(crate) fn validate_request(request: &SolveThermalFrame3dRequest) -> Result<(), String> {
    if request.nodes.len() < 2 {
        return Err("thermal 3d frame must define at least two nodes".to_string());
    }
    if request.elements.is_empty() {
        return Err("thermal 3d frame must define at least one element".to_string());
    }
    if constrained_dof_count(request) < 6 {
        return Err("thermal 3d frame must restrain at least six degrees of freedom".to_string());
    }

    for (index, node) in request.nodes.iter().enumerate() {
        if !node.x.is_finite() || !node.y.is_finite() || !node.z.is_finite() {
            return Err(format!(
                "thermal 3d frame node {index} has invalid coordinates"
            ));
        }
        if !node.load_x.is_finite() || !node.load_y.is_finite() || !node.load_z.is_finite() {
            return Err(format!("thermal 3d frame node {index} has invalid load"));
        }
        if !node.moment_x.is_finite() || !node.moment_y.is_finite() || !node.moment_z.is_finite() {
            return Err(format!("thermal 3d frame node {index} has invalid moment"));
        }
        if !node.temperature_delta.is_finite() {
            return Err("thermal 3d frame node temperature_delta must be finite".to_string());
        }
    }

    for element in &request.elements {
        validate_element(request, element)?;
    }
    Ok(())
}

fn validate_element(
    request: &SolveThermalFrame3dRequest,
    element: &ThermalFrame3dElementInput,
) -> Result<(), String> {
    if element.node_i >= request.nodes.len() || element.node_j >= request.nodes.len() {
        return Err("thermal 3d frame element references an out-of-range node".to_string());
    }
    if element.node_i == element.node_j {
        return Err("thermal 3d frame element cannot connect a node to itself".to_string());
    }
    validate_positive_frame_properties(element)?;
    if !(element.thermal_expansion.is_finite() && element.thermal_expansion >= 0.0) {
        return Err("thermal 3d frame element thermal_expansion must be non-negative".to_string());
    }
    if !(element.section_depth_y.is_finite() && element.section_depth_y > 0.0) {
        return Err("thermal 3d frame element section_depth_y must be positive".to_string());
    }
    if !(element.section_depth_z.is_finite() && element.section_depth_z > 0.0) {
        return Err("thermal 3d frame element section_depth_z must be positive".to_string());
    }
    if !element.temperature_gradient_y.is_finite() {
        return Err("thermal 3d frame element temperature_gradient_y must be finite".to_string());
    }
    if !element.temperature_gradient_z.is_finite() {
        return Err("thermal 3d frame element temperature_gradient_z must be finite".to_string());
    }

    let node_i = &request.nodes[element.node_i];
    let node_j = &request.nodes[element.node_j];
    let dx = node_j.x - node_i.x;
    let dy = node_j.y - node_i.y;
    let dz = node_j.z - node_i.z;
    let length = (dx * dx + dy * dy + dz * dz).sqrt();
    if !(length.is_finite() && length > 1.0e-12) {
        return Err("3d frame element length must be positive".to_string());
    }
    Ok(())
}

fn validate_positive_frame_properties(element: &ThermalFrame3dElementInput) -> Result<(), String> {
    for (label, value) in [
        ("area", element.area),
        ("youngs_modulus", element.youngs_modulus),
        ("shear_modulus", element.shear_modulus),
        ("torsion_constant", element.torsion_constant),
        ("moment_of_inertia_y", element.moment_of_inertia_y),
        ("moment_of_inertia_z", element.moment_of_inertia_z),
        ("section_modulus_y", element.section_modulus_y),
        ("section_modulus_z", element.section_modulus_z),
    ] {
        if !(value.is_finite() && value > 0.0) {
            return Err(format!("thermal 3d frame element {label} must be positive"));
        }
    }
    Ok(())
}

fn constrained_dof_count(request: &SolveThermalFrame3dRequest) -> usize {
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
