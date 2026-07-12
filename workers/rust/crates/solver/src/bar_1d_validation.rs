use kyuubiki_protocol::{
    SolveBarRequest, SolveElectrostaticBar1dRequest, SolveHeatBar1dRequest,
    SolveThermalBar1dRequest,
};

pub(crate) fn validate_request(request: &SolveBarRequest) -> Result<(), String> {
    if !(request.length.is_finite() && request.length > 0.0) {
        return Err("length must be a positive finite number".to_string());
    }
    if !(request.area.is_finite() && request.area > 0.0) {
        return Err("area must be a positive finite number".to_string());
    }
    if !(request.youngs_modulus.is_finite() && request.youngs_modulus > 0.0) {
        return Err("youngs_modulus must be a positive finite number".to_string());
    }
    if request.elements == 0 {
        return Err("elements must be a positive integer".to_string());
    }
    if !request.tip_force.is_finite() {
        return Err("tip_force must be a finite number".to_string());
    }
    Ok(())
}

pub(crate) fn validate_thermal_bar_1d_request(
    request: &SolveThermalBar1dRequest,
) -> Result<(), String> {
    if request.nodes.len() < 2 {
        return Err("1d thermal bar model must define at least two nodes".to_string());
    }
    if request.elements.is_empty() {
        return Err("1d thermal bar model must define at least one element".to_string());
    }
    if !request.nodes.iter().any(|node| node.fix_x) {
        return Err("1d thermal bar model must include at least one axial support".to_string());
    }

    for (index, node) in request.nodes.iter().enumerate() {
        if !node.x.is_finite() {
            return Err(format!("1d thermal bar node {index} x must be finite"));
        }
        if !node.load_x.is_finite() {
            return Err(format!("1d thermal bar node {index} load_x must be finite"));
        }
        if !node.temperature_delta.is_finite() {
            return Err(format!(
                "1d thermal bar node {index} temperature_delta must be finite"
            ));
        }
    }

    for element in &request.elements {
        if element.node_i >= request.nodes.len() || element.node_j >= request.nodes.len() {
            return Err("1d thermal bar element references an out-of-range node".to_string());
        }
        if element.node_i == element.node_j {
            return Err("1d thermal bar element must connect two distinct nodes".to_string());
        }
        if !(element.area.is_finite() && element.area > 0.0) {
            return Err("1d thermal bar element area must be positive".to_string());
        }
        if !(element.youngs_modulus.is_finite() && element.youngs_modulus > 0.0) {
            return Err("1d thermal bar element youngs_modulus must be positive".to_string());
        }
        if !(element.thermal_expansion.is_finite() && element.thermal_expansion >= 0.0) {
            return Err(
                "1d thermal bar element thermal_expansion must be non-negative".to_string(),
            );
        }

        let node_i = &request.nodes[element.node_i];
        let node_j = &request.nodes[element.node_j];
        let length = (node_j.x - node_i.x).abs();
        if !(length.is_finite() && length > 1.0e-12) {
            return Err("1d thermal bar element length must be positive".to_string());
        }
    }

    Ok(())
}

pub(crate) fn validate_heat_bar_1d_request(request: &SolveHeatBar1dRequest) -> Result<(), String> {
    if request.nodes.len() < 2 {
        return Err("1d heat bar model must define at least two nodes".to_string());
    }
    if request.elements.is_empty() {
        return Err("1d heat bar model must define at least one element".to_string());
    }
    if !request.nodes.iter().any(|node| node.fix_temperature) {
        return Err("1d heat bar model must include at least one temperature support".to_string());
    }

    for node in &request.nodes {
        if !node.x.is_finite() {
            return Err("1d heat bar node x must be finite".to_string());
        }
        if !node.temperature.is_finite() {
            return Err("1d heat bar node temperature must be finite".to_string());
        }
        if !node.heat_load.is_finite() {
            return Err("1d heat bar node heat_load must be finite".to_string());
        }
    }

    for element in &request.elements {
        if element.node_i >= request.nodes.len() || element.node_j >= request.nodes.len() {
            return Err("1d heat bar element references an out-of-range node".to_string());
        }
        if element.node_i == element.node_j {
            return Err("1d heat bar element must connect two distinct nodes".to_string());
        }
        if !element.area.is_finite() || element.area <= 0.0 {
            return Err("1d heat bar element area must be positive".to_string());
        }
        if !element.conductivity.is_finite() || element.conductivity <= 0.0 {
            return Err("1d heat bar element conductivity must be positive".to_string());
        }

        let node_i = &request.nodes[element.node_i];
        let node_j = &request.nodes[element.node_j];
        let length = (node_j.x - node_i.x).abs();
        if !length.is_finite() || length <= 0.0 {
            return Err("1d heat bar element length must be positive".to_string());
        }
    }

    Ok(())
}

pub(crate) fn validate_electrostatic_bar_1d_request(
    request: &SolveElectrostaticBar1dRequest,
) -> Result<(), String> {
    if request.nodes.len() < 2 {
        return Err("1d electrostatic bar model must define at least two nodes".to_string());
    }
    if request.elements.is_empty() {
        return Err("1d electrostatic bar model must define at least one element".to_string());
    }
    if !request.nodes.iter().any(|node| node.fix_potential) {
        return Err(
            "1d electrostatic bar model must include at least one potential support".to_string(),
        );
    }

    for node in &request.nodes {
        if !node.x.is_finite() {
            return Err("1d electrostatic bar node x must be finite".to_string());
        }
        if !node.potential.is_finite() {
            return Err("1d electrostatic bar node potential must be finite".to_string());
        }
        if !node.charge_density.is_finite() {
            return Err("1d electrostatic bar node charge_density must be finite".to_string());
        }
    }

    for element in &request.elements {
        if element.node_i >= request.nodes.len() || element.node_j >= request.nodes.len() {
            return Err("1d electrostatic bar element references an out-of-range node".to_string());
        }
        if element.node_i == element.node_j {
            return Err("1d electrostatic bar element must connect two distinct nodes".to_string());
        }
        if !element.area.is_finite() || element.area <= 0.0 {
            return Err("1d electrostatic bar element area must be positive".to_string());
        }
        if !element.permittivity.is_finite() || element.permittivity <= 0.0 {
            return Err("1d electrostatic bar element permittivity must be positive".to_string());
        }

        let node_i = &request.nodes[element.node_i];
        let node_j = &request.nodes[element.node_j];
        let length = (node_j.x - node_i.x).abs();
        if !length.is_finite() || length <= 0.0 {
            return Err("1d electrostatic bar element length must be positive".to_string());
        }
    }

    Ok(())
}
