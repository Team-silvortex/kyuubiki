use crate::truss_numerics::validate_small_displacement;
use kyuubiki_protocol::{SolveThermalTruss2dRequest, SolveThermalTruss3dRequest};

pub(crate) fn validate_thermal_truss_2d_request(
    request: &SolveThermalTruss2dRequest,
) -> Result<(), String> {
    if request.nodes.len() < 2 {
        return Err("thermal truss must define at least two nodes".to_string());
    }

    if request.elements.is_empty() {
        return Err("thermal truss must define at least one element".to_string());
    }

    if !request.nodes.iter().any(|node| node.fix_x || node.fix_y) {
        return Err("thermal truss must include at least one support".to_string());
    }

    for (index, node) in request.nodes.iter().enumerate() {
        if !(node.x.is_finite() && node.y.is_finite()) {
            return Err(format!(
                "thermal truss node {index} coordinates must be finite"
            ));
        }
        if !(node.load_x.is_finite() && node.load_y.is_finite()) {
            return Err(format!("thermal truss node {index} load must be finite"));
        }
        if !node.temperature_delta.is_finite() {
            return Err(format!(
                "thermal truss node {index} temperature_delta must be finite"
            ));
        }
    }

    for element in &request.elements {
        if element.node_i >= request.nodes.len() || element.node_j >= request.nodes.len() {
            return Err("thermal truss element references an out-of-range node".to_string());
        }
        if element.node_i == element.node_j {
            return Err("thermal truss element must connect two distinct nodes".to_string());
        }

        if !(element.area.is_finite() && element.area > 0.0) {
            return Err("thermal truss element area must be positive".to_string());
        }

        if !(element.youngs_modulus.is_finite() && element.youngs_modulus > 0.0) {
            return Err("thermal truss element youngs_modulus must be positive".to_string());
        }

        if !(element.thermal_expansion.is_finite() && element.thermal_expansion >= 0.0) {
            return Err("thermal truss element thermal_expansion must be non-negative".to_string());
        }

        let node_i = &request.nodes[element.node_i];
        let node_j = &request.nodes[element.node_j];
        let dx = node_j.x - node_i.x;
        let dy = node_j.y - node_i.y;
        let length = dx.hypot(dy);
        if !(length.is_finite() && length > 1.0e-12) {
            return Err("thermal truss element length must be positive".to_string());
        }
    }

    Ok(())
}

pub(crate) fn validate_thermal_truss_3d_request(
    request: &SolveThermalTruss3dRequest,
) -> Result<(), String> {
    if request.nodes.len() < 3 {
        return Err("3d thermal truss must define at least three nodes".to_string());
    }

    if request.elements.is_empty() {
        return Err("3d thermal truss must define at least one element".to_string());
    }

    let constrained_dofs = request.nodes.iter().fold(0, |sum, node| {
        sum + usize::from(node.fix_x) + usize::from(node.fix_y) + usize::from(node.fix_z)
    });
    if constrained_dofs < 6 {
        return Err("3d thermal truss must restrain at least six degrees of freedom".to_string());
    }

    for (index, node) in request.nodes.iter().enumerate() {
        if !(node.x.is_finite() && node.y.is_finite() && node.z.is_finite()) {
            return Err(format!(
                "3d thermal truss node {index} coordinates must be finite"
            ));
        }
        if !(node.load_x.is_finite() && node.load_y.is_finite() && node.load_z.is_finite()) {
            return Err(format!("3d thermal truss node {index} load must be finite"));
        }
        if !node.temperature_delta.is_finite() {
            return Err(format!(
                "3d thermal truss node {index} temperature_delta must be finite"
            ));
        }
    }

    for element in &request.elements {
        if element.node_i >= request.nodes.len() || element.node_j >= request.nodes.len() {
            return Err("3d thermal truss element references an out-of-range node".to_string());
        }
        if element.node_i == element.node_j {
            return Err("3d thermal truss element must connect two distinct nodes".to_string());
        }
        if !(element.area.is_finite() && element.area > 0.0) {
            return Err("3d thermal truss element area must be positive".to_string());
        }
        if !(element.youngs_modulus.is_finite() && element.youngs_modulus > 0.0) {
            return Err("3d thermal truss element youngs_modulus must be positive".to_string());
        }
        if !(element.thermal_expansion.is_finite() && element.thermal_expansion >= 0.0) {
            return Err(
                "3d thermal truss element thermal_expansion must be non-negative".to_string(),
            );
        }
        let node_i = &request.nodes[element.node_i];
        let node_j = &request.nodes[element.node_j];
        let length = (node_j.x - node_i.x)
            .hypot(node_j.y - node_i.y)
            .hypot(node_j.z - node_i.z);
        if !(length.is_finite() && length > 1.0e-12) {
            return Err("3d thermal truss element length must be positive".to_string());
        }
    }

    Ok(())
}

pub(crate) fn validate_small_displacement_thermal_truss_2d(
    request: &SolveThermalTruss2dRequest,
    max_displacement: f64,
) -> Result<(), String> {
    validate_small_displacement(
        "thermal truss",
        request.nodes.iter().map(|node| [node.x, node.y]),
        max_displacement,
    )
}

pub(crate) fn validate_small_displacement_thermal_truss_3d(
    request: &SolveThermalTruss3dRequest,
    max_displacement: f64,
) -> Result<(), String> {
    validate_small_displacement(
        "thermal truss",
        request.nodes.iter().map(|node| [node.x, node.y, node.z]),
        max_displacement,
    )
}
