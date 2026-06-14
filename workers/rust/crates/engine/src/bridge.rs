use kyuubiki_protocol::{
    SolveElectrostaticPlaneQuad2dResult, SolveElectrostaticPlaneTriangle2dResult,
    SolveHeatPlaneQuad2dRequest, SolveHeatPlaneTriangle2dRequest,
};
use serde::Serialize;
use serde_json::Value;

#[derive(Clone, Serialize)]
pub struct BridgeDiagnostics {
    pub(crate) bridge_kind: String,
    pub(crate) mapped_count: usize,
    pub(crate) defaulted_count: usize,
    pub(crate) source_field: String,
    pub(crate) target_field: String,
    pub(crate) source_value_min: Option<f64>,
    pub(crate) source_value_max: Option<f64>,
    pub(crate) reduction: Option<String>,
    pub(crate) scale: Option<f64>,
}

pub(crate) fn attach_bridge_diagnostics<T: Serialize>(
    model: &T,
    diagnostics: &BridgeDiagnostics,
) -> Result<Value, String> {
    let mut value = serde_json::to_value(model).map_err(|err| err.to_string())?;
    let object = value
        .as_object_mut()
        .ok_or_else(|| "bridge output must serialize to an object payload".to_string())?;
    object.insert(
        "__bridge_diagnostics".to_string(),
        serde_json::to_value(diagnostics).map_err(|err| err.to_string())?,
    );
    Ok(value)
}

#[derive(Debug, Clone)]
pub(crate) struct ElectrostaticToHeatBridgeContract {
    source_field: String,
    distribution: String,
    node_index_fields: Vec<String>,
    scale: f64,
    reduction: String,
    default_value: f64,
    target_field: String,
}

pub(crate) fn resolve_electrostatic_to_heat_bridge_contract(
    config: &Value,
) -> Result<ElectrostaticToHeatBridgeContract, String> {
    let contract = config.get("contract").unwrap_or(config);
    let source = contract.get("source").and_then(Value::as_object);
    let transform = contract.get("transform").and_then(Value::as_object);
    let target = contract.get("target").and_then(Value::as_object);

    let source_field = source
        .and_then(|value| value.get("field"))
        .and_then(Value::as_str)
        .unwrap_or("electric_field_magnitude")
        .to_string();
    let distribution = source
        .and_then(|value| value.get("distribution"))
        .and_then(Value::as_str)
        .unwrap_or("element_to_nodes")
        .to_string();
    let node_index_fields = source
        .and_then(|value| value.get("node_index_fields"))
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        })
        .filter(|items| !items.is_empty())
        .unwrap_or_else(|| {
            vec![
                "node_i".to_string(),
                "node_j".to_string(),
                "node_k".to_string(),
                "node_l".to_string(),
            ]
        });
    let scale = transform
        .and_then(|value| value.get("scale"))
        .and_then(Value::as_f64)
        .or_else(|| config.get("field_to_heat_scale").and_then(Value::as_f64))
        .unwrap_or(1.0);
    let reduction = transform
        .and_then(|value| value.get("reduction"))
        .and_then(Value::as_str)
        .unwrap_or("mean")
        .to_string();
    let default_value = transform
        .and_then(|value| value.get("default_value"))
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let target_field = target
        .and_then(|value| value.get("field"))
        .and_then(Value::as_str)
        .unwrap_or("heat_load")
        .to_string();

    if distribution != "element_to_nodes" && distribution != "node_to_node" {
        return Err(format!(
            "unsupported electrostatic-to-heat bridge distribution: {distribution}"
        ));
    }
    if reduction != "mean"
        && reduction != "sum"
        && reduction != "area_weighted_mean"
        && reduction != "max"
        && reduction != "min"
    {
        return Err(format!(
            "unsupported electrostatic-to-heat bridge reduction: {reduction}"
        ));
    }
    if target_field != "heat_load" && target_field != "temperature" {
        return Err(format!(
            "unsupported electrostatic-to-heat bridge target field: {target_field}"
        ));
    }

    Ok(ElectrostaticToHeatBridgeContract {
        source_field,
        distribution,
        node_index_fields,
        scale,
        reduction,
        default_value,
        target_field,
    })
}

pub(crate) fn bridge_electrostatic_result_to_heat_plane_quad_model(
    electrostatic_result: &SolveElectrostaticPlaneQuad2dResult,
    heat_seed_model: &SolveHeatPlaneQuad2dRequest,
    contract: &ElectrostaticToHeatBridgeContract,
) -> Result<(SolveHeatPlaneQuad2dRequest, BridgeDiagnostics), String> {
    if electrostatic_result.nodes.len() != heat_seed_model.nodes.len() {
        return Err("electrostatic and heat quad models must have the same node count".to_string());
    }
    if contract.distribution == "element_to_nodes"
        && electrostatic_result.input.elements.len() != heat_seed_model.elements.len()
    {
        return Err(
            "electrostatic and heat quad models must have the same element count".to_string(),
        );
    }

    let mut bridged = heat_seed_model.clone();
    for (electrostatic_node, heat_node) in electrostatic_result
        .nodes
        .iter()
        .zip(bridged.nodes.iter_mut())
    {
        if (electrostatic_node.x - heat_node.x).abs() > 1.0e-9
            || (electrostatic_node.y - heat_node.y).abs() > 1.0e-9
        {
            return Err(format!(
                "electrostatic node {} does not align with heat node {}",
                electrostatic_node.id, heat_node.id
            ));
        }
    }

    let (source_values, nodal_values) = if contract.distribution == "node_to_node" {
        derive_direct_nodal_target_field(&electrostatic_result.nodes, contract)?
    } else {
        derive_element_nodal_target_field(
            &electrostatic_result.elements,
            bridged.nodes.len(),
            contract,
            electrostatic_bridge_source_value,
            electrostatic_bridge_node_index,
            |element| element.area.abs(),
        )?
    };

    for (index, heat_node) in bridged.nodes.iter_mut().enumerate() {
        let value = nodal_values[index];
        match contract.target_field.as_str() {
            "heat_load" => heat_node.heat_load = value,
            "temperature" => heat_node.temperature = value,
            _ => unreachable!("bridge target field validated earlier"),
        }
    }

    Ok((
        bridged,
        BridgeDiagnostics {
            bridge_kind: "electrostatic_to_heat_quad_2d".to_string(),
            mapped_count: nodal_values.len(),
            defaulted_count: 0,
            source_field: contract.source_field.clone(),
            target_field: contract.target_field.clone(),
            source_value_min: source_values.iter().copied().reduce(f64::min),
            source_value_max: source_values.iter().copied().reduce(f64::max),
            reduction: Some(contract.reduction.clone()),
            scale: Some(contract.scale),
        },
    ))
}

pub(crate) fn bridge_electrostatic_result_to_heat_plane_triangle_model(
    electrostatic_result: &SolveElectrostaticPlaneTriangle2dResult,
    heat_seed_model: &SolveHeatPlaneTriangle2dRequest,
    contract: &ElectrostaticToHeatBridgeContract,
) -> Result<(SolveHeatPlaneTriangle2dRequest, BridgeDiagnostics), String> {
    if electrostatic_result.nodes.len() != heat_seed_model.nodes.len() {
        return Err(
            "electrostatic and heat triangle models must have the same node count".to_string(),
        );
    }
    if contract.distribution == "element_to_nodes"
        && electrostatic_result.input.elements.len() != heat_seed_model.elements.len()
    {
        return Err(
            "electrostatic and heat triangle models must have the same element count".to_string(),
        );
    }

    let mut bridged = heat_seed_model.clone();
    for (electrostatic_node, heat_node) in electrostatic_result
        .nodes
        .iter()
        .zip(bridged.nodes.iter_mut())
    {
        if (electrostatic_node.x - heat_node.x).abs() > 1.0e-9
            || (electrostatic_node.y - heat_node.y).abs() > 1.0e-9
        {
            return Err(format!(
                "electrostatic node {} does not align with heat node {}",
                electrostatic_node.id, heat_node.id
            ));
        }
    }

    let (source_values, nodal_values) = if contract.distribution == "node_to_node" {
        derive_direct_nodal_target_field(&electrostatic_result.nodes, contract)?
    } else {
        derive_element_nodal_target_field(
            &electrostatic_result.elements,
            bridged.nodes.len(),
            contract,
            electrostatic_triangle_bridge_source_value,
            electrostatic_triangle_bridge_node_index,
            |element| element.area.abs(),
        )?
    };

    for (index, heat_node) in bridged.nodes.iter_mut().enumerate() {
        let value = nodal_values[index];
        match contract.target_field.as_str() {
            "heat_load" => heat_node.heat_load = value,
            "temperature" => heat_node.temperature = value,
            _ => unreachable!("bridge target field validated earlier"),
        }
    }

    Ok((
        bridged,
        BridgeDiagnostics {
            bridge_kind: "electrostatic_to_heat_triangle_2d".to_string(),
            mapped_count: nodal_values.len(),
            defaulted_count: 0,
            source_field: contract.source_field.clone(),
            target_field: contract.target_field.clone(),
            source_value_min: source_values.iter().copied().reduce(f64::min),
            source_value_max: source_values.iter().copied().reduce(f64::max),
            reduction: Some(contract.reduction.clone()),
            scale: Some(contract.scale),
        },
    ))
}

fn reduce_bridge_value(
    reduction: &str,
    default_value: f64,
    count: usize,
    sum: f64,
    weighted_sum: f64,
    weight_total: f64,
    minimum: f64,
    maximum: f64,
) -> f64 {
    match reduction {
        "sum" if count > 0 => sum,
        "max" if count > 0 => maximum,
        "min" if count > 0 => minimum,
        "area_weighted_mean" if weight_total > 0.0 => weighted_sum / weight_total,
        _ if count > 0 => sum / count as f64,
        _ => default_value,
    }
}

fn derive_direct_nodal_target_field(
    nodes: &[kyuubiki_protocol::ElectrostaticPlaneNodeResult],
    contract: &ElectrostaticToHeatBridgeContract,
) -> Result<(Vec<f64>, Vec<f64>), String> {
    let mut source_values = Vec::with_capacity(nodes.len());
    let nodal_values = nodes
        .iter()
        .map(|node| {
            let source_value =
                electrostatic_node_source_value(node, contract.source_field.as_str())?
                    * contract.scale;
            source_values.push(source_value);
            Ok(source_value)
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok((source_values, nodal_values))
}

fn derive_element_nodal_target_field<TElement>(
    elements: &[TElement],
    node_count: usize,
    contract: &ElectrostaticToHeatBridgeContract,
    resolve_source: fn(&TElement, &str) -> Result<f64, String>,
    resolve_node_index: fn(&TElement, &str) -> Result<usize, String>,
    resolve_weight: fn(&TElement) -> f64,
) -> Result<(Vec<f64>, Vec<f64>), String> {
    let mut sums = vec![0.0; node_count];
    let mut counts = vec![0usize; node_count];
    let mut weighted_sums = vec![0.0; node_count];
    let mut weight_totals = vec![0.0; node_count];
    let mut minima = vec![f64::INFINITY; node_count];
    let mut maxima = vec![f64::NEG_INFINITY; node_count];
    let mut source_values = Vec::new();

    for element in elements {
        let source_value =
            resolve_source(element, contract.source_field.as_str())? * contract.scale;
        source_values.push(source_value);
        let weight = resolve_weight(element);
        for field in &contract.node_index_fields {
            let index = resolve_node_index(element, field)?;
            let Some(sum) = sums.get_mut(index) else {
                return Err(format!(
                    "bridge node index {} from field {} is out of bounds",
                    index, field
                ));
            };
            *sum += source_value;
            counts[index] += 1;
            weighted_sums[index] += source_value * weight;
            weight_totals[index] += weight;
            minima[index] = minima[index].min(source_value);
            maxima[index] = maxima[index].max(source_value);
        }
    }

    let nodal_values = (0..node_count)
        .map(|index| {
            reduce_bridge_value(
                contract.reduction.as_str(),
                contract.default_value,
                counts[index],
                sums[index],
                weighted_sums[index],
                weight_totals[index],
                minima[index],
                maxima[index],
            )
        })
        .collect::<Vec<_>>();

    Ok((source_values, nodal_values))
}

fn electrostatic_bridge_source_value(
    element: &kyuubiki_protocol::ElectrostaticPlaneQuadElementResult,
    field: &str,
) -> Result<f64, String> {
    match field {
        "electric_field_magnitude" => Ok(element.electric_field_magnitude),
        "electric_field_x" => Ok(element.electric_field_x),
        "electric_field_y" => Ok(element.electric_field_y),
        "flux_magnitude" | "electric_flux_density_magnitude" => {
            Ok(element.electric_flux_density_magnitude)
        }
        "electric_flux_density_x" => Ok(element.electric_flux_density_x),
        "electric_flux_density_y" => Ok(element.electric_flux_density_y),
        "average_potential" => Ok(element.average_potential),
        other => Err(format!(
            "unsupported electrostatic-to-heat bridge source field: {other}"
        )),
    }
}

fn electrostatic_triangle_bridge_source_value(
    element: &kyuubiki_protocol::ElectrostaticPlaneTriangleElementResult,
    field: &str,
) -> Result<f64, String> {
    match field {
        "electric_field_magnitude" => Ok(element.electric_field_magnitude),
        "electric_field_x" => Ok(element.electric_field_x),
        "electric_field_y" => Ok(element.electric_field_y),
        "average_potential" => Ok(element.average_potential),
        "flux_magnitude" | "electric_flux_density_magnitude" => {
            Ok(element.electric_flux_density_magnitude)
        }
        "electric_flux_density_x" => Ok(element.electric_flux_density_x),
        "electric_flux_density_y" => Ok(element.electric_flux_density_y),
        _ => Err(format!(
            "unsupported electrostatic triangle bridge source field: {field}"
        )),
    }
}

fn electrostatic_bridge_node_index(
    element: &kyuubiki_protocol::ElectrostaticPlaneQuadElementResult,
    field: &str,
) -> Result<usize, String> {
    match field {
        "node_i" => Ok(element.node_i),
        "node_j" => Ok(element.node_j),
        "node_k" => Ok(element.node_k),
        "node_l" => Ok(element.node_l),
        other => Err(format!(
            "unsupported electrostatic-to-heat bridge node index field: {other}"
        )),
    }
}

fn electrostatic_triangle_bridge_node_index(
    element: &kyuubiki_protocol::ElectrostaticPlaneTriangleElementResult,
    field: &str,
) -> Result<usize, String> {
    match field {
        "node_i" => Ok(element.node_i),
        "node_j" => Ok(element.node_j),
        "node_k" => Ok(element.node_k),
        _ => Err(format!(
            "unsupported electrostatic triangle bridge node index field: {field}"
        )),
    }
}

fn electrostatic_node_source_value(
    node: &kyuubiki_protocol::ElectrostaticPlaneNodeResult,
    field: &str,
) -> Result<f64, String> {
    match field {
        "potential" => Ok(node.potential),
        "charge_density" => Ok(node.charge_density),
        other => Err(format!(
            "unsupported electrostatic node bridge source field: {other}"
        )),
    }
}
