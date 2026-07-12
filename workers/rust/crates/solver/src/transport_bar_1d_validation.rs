use kyuubiki_protocol::{AdvectionDiffusionBar1dElementInput, SolveAdvectionDiffusionBar1dRequest};

pub(crate) fn validate_request(
    request: &SolveAdvectionDiffusionBar1dRequest,
) -> Result<(), String> {
    if request.nodes.len() < 2 {
        return Err("1d advection-diffusion model must define at least two nodes".to_string());
    }
    if request.elements.is_empty() {
        return Err("1d advection-diffusion model must define at least one element".to_string());
    }
    if !request.nodes.iter().any(|node| node.fix_concentration) {
        return Err(
            "1d advection-diffusion model must include at least one concentration support"
                .to_string(),
        );
    }

    for node in &request.nodes {
        if !node.x.is_finite() || !node.concentration.is_finite() || !node.source.is_finite() {
            return Err("1d advection-diffusion node values must be finite".to_string());
        }
    }

    for element in &request.elements {
        validate_element(request, element)?;
    }
    Ok(())
}

fn validate_element(
    request: &SolveAdvectionDiffusionBar1dRequest,
    element: &AdvectionDiffusionBar1dElementInput,
) -> Result<(), String> {
    if element.node_i >= request.nodes.len() || element.node_j >= request.nodes.len() {
        return Err("1d advection-diffusion element references an out-of-range node".to_string());
    }
    if element.node_i == element.node_j {
        return Err("1d advection-diffusion element must connect two distinct nodes".to_string());
    }
    if !element.area.is_finite() || element.area <= 0.0 {
        return Err("1d advection-diffusion element area must be positive".to_string());
    }
    if !element.diffusivity.is_finite() || element.diffusivity <= 0.0 {
        return Err("1d advection-diffusion element diffusivity must be positive".to_string());
    }
    if !element.velocity.is_finite() {
        return Err("1d advection-diffusion element velocity must be finite".to_string());
    }

    let length = (request.nodes[element.node_j].x - request.nodes[element.node_i].x).abs();
    if !(length.is_finite() && length > 1.0e-12) {
        return Err("1d advection-diffusion element length must be positive".to_string());
    }
    Ok(())
}
