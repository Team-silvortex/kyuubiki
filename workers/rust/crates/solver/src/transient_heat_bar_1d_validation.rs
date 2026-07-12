use kyuubiki_protocol::{SolveTransientHeatBar1dRequest, TransientHeatBar1dElementInput};

pub(crate) fn validate_request(request: &SolveTransientHeatBar1dRequest) -> Result<(), String> {
    if request.nodes.len() < 2 {
        return Err("transient heat bar must define at least two nodes".to_string());
    }
    if request.elements.is_empty() {
        return Err("transient heat bar must define at least one element".to_string());
    }
    if request.time_step <= 0.0 || !request.time_step.is_finite() {
        return Err("transient heat bar time_step must be positive".to_string());
    }
    if request.steps == 0 {
        return Err("transient heat bar steps must be positive".to_string());
    }

    for (index, node) in request.nodes.iter().enumerate() {
        if !node.x.is_finite() {
            return Err(format!("transient heat bar node {index} x must be finite"));
        }
        if !node.temperature.is_finite() {
            return Err(format!(
                "transient heat bar node {index} temperature must be finite"
            ));
        }
        if !node.heat_load.is_finite() {
            return Err(format!(
                "transient heat bar node {index} heat_load must be finite"
            ));
        }
    }

    let mut capacity = vec![0.0; request.nodes.len()];
    for element in &request.elements {
        validate_element(request, element, &mut capacity)?;
    }
    if capacity.iter().any(|value| *value <= 0.0) {
        return Err(
            "transient heat bar every node must receive positive heat capacity".to_string(),
        );
    }
    Ok(())
}

fn validate_element(
    request: &SolveTransientHeatBar1dRequest,
    element: &TransientHeatBar1dElementInput,
    capacity: &mut [f64],
) -> Result<(), String> {
    for index in [element.node_i, element.node_j] {
        if index >= request.nodes.len() {
            return Err(format!(
                "transient heat bar element {} references missing node {}",
                element.id, index
            ));
        }
    }
    let length = (request.nodes[element.node_j].x - request.nodes[element.node_i].x).abs();
    if element.node_i == element.node_j || !(length.is_finite() && length > 1.0e-12) {
        return Err(format!(
            "transient heat bar element {} must have non-zero length",
            element.id
        ));
    }
    if element.area <= 0.0
        || element.conductivity <= 0.0
        || !element.area.is_finite()
        || !element.conductivity.is_finite()
    {
        return Err(format!(
            "transient heat bar element {} must have positive area and conductivity",
            element.id
        ));
    }
    if element.density <= 0.0
        || element.specific_heat <= 0.0
        || !element.density.is_finite()
        || !element.specific_heat.is_finite()
    {
        return Err(format!(
            "transient heat bar element {} must have positive density and specific_heat",
            element.id
        ));
    }

    let mass_heat_capacity = element.density * element.specific_heat * element.area * length;
    capacity[element.node_i] += 0.5 * mass_heat_capacity;
    capacity[element.node_j] += 0.5 * mass_heat_capacity;
    Ok(())
}
