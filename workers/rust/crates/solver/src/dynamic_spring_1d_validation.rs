use kyuubiki_protocol::{
    SolveHarmonicSpring1dRequest, SolveTransientSpring1dRequest, TransientSpring1dElementInput,
    TransientSpring1dNodeInput,
};

pub(crate) fn validate_transient_request(
    request: &SolveTransientSpring1dRequest,
) -> Result<(), String> {
    validate_model_shape(&request.nodes, &request.elements, "transient spring 1d")?;
    if request.time_step <= 0.0 || !request.time_step.is_finite() || request.steps == 0 {
        return Err("transient spring 1d requires positive time_step and steps".to_string());
    }
    validate_nodes(&request.nodes, "transient spring")?;
    validate_elements(&request.nodes, &request.elements, "transient spring")?;
    Ok(())
}

pub(crate) fn validate_harmonic_request(
    request: &SolveHarmonicSpring1dRequest,
) -> Result<(), String> {
    validate_model_shape(&request.nodes, &request.elements, "harmonic spring 1d")?;
    if request.frequencies_hz.is_empty() {
        return Err("harmonic spring 1d must include at least one frequency".to_string());
    }
    if request.nodes.iter().all(|node| node.fix_x) {
        return Err("harmonic spring 1d must leave at least one free node".to_string());
    }
    validate_nodes(&request.nodes, "harmonic spring")?;
    for (index, frequency) in request.frequencies_hz.iter().enumerate() {
        if !frequency.is_finite() || *frequency < 0.0 {
            return Err(format!(
                "harmonic spring frequency {index} must be non-negative and finite"
            ));
        }
    }
    validate_elements(&request.nodes, &request.elements, "harmonic spring")?;
    Ok(())
}

fn validate_model_shape(
    nodes: &[TransientSpring1dNodeInput],
    elements: &[TransientSpring1dElementInput],
    label: &str,
) -> Result<(), String> {
    if nodes.len() < 2 {
        return Err(format!("{label} must define at least two nodes"));
    }
    if elements.is_empty() {
        return Err(format!("{label} must define at least one element"));
    }
    Ok(())
}

fn validate_nodes(nodes: &[TransientSpring1dNodeInput], label: &str) -> Result<(), String> {
    for node in nodes {
        if !node.x.is_finite()
            || !node.load_x.is_finite()
            || !node.mass.is_finite()
            || !node.initial_displacement.is_finite()
            || !node.initial_velocity.is_finite()
            || node.mass <= 0.0
        {
            return Err(format!(
                "{label} node {} must have finite coordinates, load, initial state, and positive mass",
                node.id
            ));
        }
    }
    Ok(())
}

fn validate_elements(
    nodes: &[TransientSpring1dNodeInput],
    elements: &[TransientSpring1dElementInput],
    label: &str,
) -> Result<(), String> {
    for element in elements {
        validate_element(nodes, element, label)?;
    }
    Ok(())
}

fn validate_element(
    nodes: &[TransientSpring1dNodeInput],
    element: &TransientSpring1dElementInput,
    label: &str,
) -> Result<(), String> {
    for index in [element.node_i, element.node_j] {
        if index >= nodes.len() {
            return Err(format!(
                "{label} element {} references missing node {}",
                element.id, index
            ));
        }
    }
    if element.node_i == element.node_j
        || !element.stiffness.is_finite()
        || element.stiffness <= 0.0
        || !element.damping.is_finite()
        || element.damping < 0.0
    {
        return Err(format!(
            "{label} element {} must have valid connectivity, stiffness, and damping",
            element.id
        ));
    }

    let length = (nodes[element.node_j].x - nodes[element.node_i].x).abs();
    if !(length.is_finite() && length > 1.0e-12) {
        return Err(format!(
            "{label} element {} length must be positive",
            element.id
        ));
    }
    Ok(())
}
