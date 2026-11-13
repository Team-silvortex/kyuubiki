use kyuubiki_protocol::{SolveHeatPlaneQuad2dResult, SolveThermalQ9RectRequest};
use std::collections::{BTreeMap, BTreeSet};

/// Validate correspondence, not the provenance or physical accuracy of an arbitrary result.
pub(super) fn validate(
    heat: &SolveHeatPlaneQuad2dResult,
    model: &SolveThermalQ9RectRequest,
) -> Result<Vec<f64>, String> {
    let input = &heat.input;
    let count = model.nodes.len();
    if input.nodes.len() != count
        || heat.nodes.len() != count
        || input.elements.len() != 4 * model.elements.len()
        || heat.elements.len() != input.elements.len()
    {
        return Err(
            "Q9 bridge requires identical node sets and four Q4 heat cells per Q9 cell".into(),
        );
    }
    if !input.contact_interfaces.is_empty() || !heat.contact_interfaces.is_empty() {
        return Err(
            "Q9 bridge does not support thermal contact interfaces or temperature jumps".into(),
        );
    }
    if [
        heat.max_temperature,
        heat.max_heat_flux,
        heat.total_abs_heat_flow_rate,
    ]
    .iter()
    .any(|v| !v.is_finite() || *v < 0.0)
    {
        return Err("Q9 bridge heat summary is not finite and nonnegative".into());
    }
    let mut ids = BTreeMap::new();
    for (i, n) in input.nodes.iter().enumerate() {
        if n.id.is_empty()
            || n.id.len() > 128
            || ids.insert(n.id.as_str(), i).is_some()
            || [n.x, n.y, n.heat_load, n.temperature]
                .iter()
                .any(|v| !v.is_finite())
            || (n.fix_temperature && n.temperature < 0.0)
        {
            return Err(
                "Q9 bridge heat input has invalid identity, coordinates, load or temperature"
                    .into(),
            );
        }
    }
    let mut temperatures = vec![None; count];
    for n in &heat.nodes {
        let source = input
            .nodes
            .get(n.index)
            .ok_or("Q9 bridge heat result index is out of range")?;
        if temperatures[n.index].is_some()
            || n.id != source.id
            || n.x != source.x
            || n.y != source.y
            || n.heat_load != source.heat_load
            || !n.temperature.is_finite()
            || n.temperature < 0.0
            || (source.fix_temperature && n.temperature != source.temperature)
        {
            return Err("Q9 bridge heat result has duplicate or inconsistent node identity, coordinates or temperature".into());
        }
        temperatures[n.index] = Some(n.temperature);
    }
    let mut mapping = Vec::with_capacity(count);
    let mut output = Vec::with_capacity(count);
    for node in &model.nodes {
        let &i = ids
            .get(node.id.as_str())
            .ok_or("Q9 bridge target node ID is absent from heat result")?;
        let source = &input.nodes[i];
        if (source.x, source.y) != (node.x, node.y) {
            return Err(
                "Q9 bridge target coordinates must exactly match heat node identity".into(),
            );
        }
        mapping.push(i);
        output.push(temperatures[i].ok_or("Q9 bridge heat node has no result")?);
    }
    // Requiring the four actual subcells prevents a same-node but different-domain transfer.
    let mut expected = BTreeMap::new();
    for element in &model.elements {
        for local in [[0, 1, 4, 3], [1, 2, 5, 4], [3, 4, 7, 6], [4, 5, 8, 7]] {
            let nodes = local.map(|i| mapping[element.nodes[i]]);
            expected.insert(canonical(nodes), element.thickness);
        }
    }
    let mut names = BTreeSet::new();
    for e in &input.elements {
        let nodes = [e.node_i, e.node_j, e.node_k, e.node_l];
        let thickness = expected
            .remove(&canonical(nodes))
            .ok_or("Q9 bridge heat cell is not a unique Q9 subcell")?;
        if e.id.is_empty()
            || e.id.len() > 128
            || !names.insert(&e.id)
            || !e.conductivity.is_finite()
            || e.conductivity <= 0.0
            || e.thickness != thickness
        {
            return Err(
                "Q9 bridge heat cell identity, conductivity or thickness is inconsistent".into(),
            );
        }
    }
    let mut seen = BTreeSet::new();
    for e in &heat.elements {
        let source = input
            .elements
            .get(e.index)
            .ok_or("Q9 bridge heat element result index is out of range")?;
        if !seen.insert(e.index)
            || e.id != source.id
            || [e.node_i, e.node_j, e.node_k, e.node_l]
                != [source.node_i, source.node_j, source.node_k, source.node_l]
            || [
                e.area,
                e.average_temperature,
                e.temperature_gradient_x,
                e.temperature_gradient_y,
                e.heat_flux_x,
                e.heat_flux_y,
                e.heat_flux_magnitude,
                e.heat_flow_rate,
            ]
            .iter()
            .any(|v| !v.is_finite())
            || e.area <= 0.0
            || e.average_temperature < 0.0
            || e.heat_flux_magnitude < 0.0
        {
            return Err(
                "Q9 bridge heat element result is nonfinite or inconsistent with its source".into(),
            );
        }
    }
    Ok(output)
}

// Preserve orientation while allowing a cyclic change of the first Q4 corner.
fn canonical(mut nodes: [usize; 4]) -> [usize; 4] {
    let start = (0..4).min_by_key(|&i| nodes[i]).unwrap();
    nodes.rotate_left(start);
    nodes
}
