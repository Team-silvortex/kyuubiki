use crate::bridge::BridgeDiagnostics;
use kyuubiki_protocol::{
    HeatPlaneNodeResult, SolveHeatPlaneQuad2dResult, SolveHeatPlaneTriangle2dResult,
    SolveThermalPlaneQuad2dRequest, SolveThermalPlaneTriangle2dRequest,
};
use serde_json::Value;

#[derive(Debug, Clone)]
pub(crate) struct HeatToThermoBridgeContract {
    source_field: String,
    distribution: String,
    node_index_fields: Vec<String>,
    reduction: String,
    target_field: String,
    scale: f64,
    default_value: f64,
}

pub fn bridge_heat_result_to_thermal_plane_quad_model(
    heat_result: &SolveHeatPlaneQuad2dResult,
    thermo_seed_model: &SolveThermalPlaneQuad2dRequest,
) -> Result<(SolveThermalPlaneQuad2dRequest, BridgeDiagnostics), String> {
    let contract = default_heat_to_thermo_bridge_contract();
    bridge_heat_result_to_thermal_plane_quad_model_with_contract(
        heat_result,
        thermo_seed_model,
        &contract,
    )
}

pub(crate) fn bridge_heat_result_to_thermal_plane_quad_model_with_contract(
    heat_result: &SolveHeatPlaneQuad2dResult,
    thermo_seed_model: &SolveThermalPlaneQuad2dRequest,
    contract: &HeatToThermoBridgeContract,
) -> Result<(SolveThermalPlaneQuad2dRequest, BridgeDiagnostics), String> {
    if heat_result.nodes.len() != thermo_seed_model.nodes.len() {
        return Err("heat and thermo quad models must have the same node count".to_string());
    }
    if contract.distribution == "element_to_nodes"
        && heat_result.input.elements.len() != thermo_seed_model.elements.len()
    {
        return Err("heat and thermo quad models must have the same element count".to_string());
    }

    let mut bridged = thermo_seed_model.clone();
    for (heat_node, thermo_node) in heat_result.nodes.iter().zip(bridged.nodes.iter()) {
        if (heat_node.x - thermo_node.x).abs() > 1.0e-9
            || (heat_node.y - thermo_node.y).abs() > 1.0e-9
        {
            return Err(format!(
                "heat node {} does not align with thermo node {}",
                heat_node.id, thermo_node.id
            ));
        }
    }
    let (source_values, nodal_values) = if contract.distribution == "node_to_node" {
        derive_direct_heat_nodal_target_field(&heat_result.nodes, contract)?
    } else {
        derive_element_heat_nodal_target_field(
            &heat_result.elements,
            bridged.nodes.len(),
            contract,
            heat_quad_bridge_source_value,
            heat_quad_bridge_node_index,
            |element| element.area.abs(),
        )?
    };
    for (index, thermo_node) in bridged.nodes.iter_mut().enumerate() {
        thermo_node.temperature_delta = nodal_values[index];
    }

    Ok((
        bridged,
        BridgeDiagnostics {
            bridge_kind: "heat_to_thermo_quad_2d".to_string(),
            mapped_count: heat_result.nodes.len(),
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

pub(crate) fn bridge_heat_result_to_thermal_plane_triangle_model_with_contract(
    heat_result: &SolveHeatPlaneTriangle2dResult,
    thermo_seed_model: &SolveThermalPlaneTriangle2dRequest,
    contract: &HeatToThermoBridgeContract,
) -> Result<(SolveThermalPlaneTriangle2dRequest, BridgeDiagnostics), String> {
    if heat_result.nodes.len() != thermo_seed_model.nodes.len() {
        return Err("heat and thermo triangle models must have the same node count".to_string());
    }
    if contract.distribution == "element_to_nodes"
        && heat_result.input.elements.len() != thermo_seed_model.elements.len()
    {
        return Err("heat and thermo triangle models must have the same element count".to_string());
    }

    let mut bridged = thermo_seed_model.clone();
    for (heat_node, thermo_node) in heat_result.nodes.iter().zip(bridged.nodes.iter()) {
        if (heat_node.x - thermo_node.x).abs() > 1.0e-9
            || (heat_node.y - thermo_node.y).abs() > 1.0e-9
        {
            return Err(format!(
                "heat node {} does not align with thermo node {}",
                heat_node.id, thermo_node.id
            ));
        }
    }
    let (source_values, nodal_values) = if contract.distribution == "node_to_node" {
        derive_direct_heat_nodal_target_field(&heat_result.nodes, contract)?
    } else {
        derive_element_heat_nodal_target_field(
            &heat_result.elements,
            bridged.nodes.len(),
            contract,
            heat_triangle_bridge_source_value,
            heat_triangle_bridge_node_index,
            |element| element.area.abs(),
        )?
    };
    for (index, thermo_node) in bridged.nodes.iter_mut().enumerate() {
        thermo_node.temperature_delta = nodal_values[index];
    }

    Ok((
        bridged,
        BridgeDiagnostics {
            bridge_kind: "heat_to_thermo_triangle_2d".to_string(),
            mapped_count: heat_result.nodes.len(),
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

pub fn bridge_heat_result_to_thermal_plane_triangle_model(
    heat_result: &SolveHeatPlaneTriangle2dResult,
    thermo_seed_model: &SolveThermalPlaneTriangle2dRequest,
) -> Result<(SolveThermalPlaneTriangle2dRequest, BridgeDiagnostics), String> {
    let contract = default_heat_to_thermo_bridge_contract();
    bridge_heat_result_to_thermal_plane_triangle_model_with_contract(
        heat_result,
        thermo_seed_model,
        &contract,
    )
}

fn default_heat_to_thermo_bridge_contract() -> HeatToThermoBridgeContract {
    HeatToThermoBridgeContract {
        source_field: "temperature".to_string(),
        distribution: "node_to_node".to_string(),
        node_index_fields: vec![],
        reduction: "copy".to_string(),
        target_field: "temperature_delta".to_string(),
        scale: 1.0,
        default_value: 0.0,
    }
}

pub(crate) fn resolve_heat_to_thermo_bridge_contract(
    config: &Value,
) -> Result<HeatToThermoBridgeContract, String> {
    let contract = config.get("contract").unwrap_or(config);
    let source = contract.get("source").and_then(Value::as_object);
    let transform = contract.get("transform").and_then(Value::as_object);
    let target = contract.get("target").and_then(Value::as_object);
    let source_field = source
        .and_then(|value| value.get("field"))
        .and_then(Value::as_str)
        .unwrap_or("temperature")
        .to_string();
    let distribution = source
        .and_then(|value| value.get("distribution"))
        .and_then(Value::as_str)
        .unwrap_or("node_to_node")
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
        .unwrap_or_else(|| {
            vec![
                "node_i".to_string(),
                "node_j".to_string(),
                "node_k".to_string(),
                "node_l".to_string(),
            ]
        });
    let reduction = transform
        .and_then(|value| value.get("reduction"))
        .and_then(Value::as_str)
        .unwrap_or(if distribution == "node_to_node" {
            "copy"
        } else {
            "mean"
        })
        .to_string();
    let target_field = target
        .and_then(|value| value.get("field"))
        .and_then(Value::as_str)
        .unwrap_or("temperature_delta")
        .to_string();
    let scale = transform
        .and_then(|value| value.get("scale"))
        .and_then(Value::as_f64)
        .unwrap_or(1.0);
    let default_value = transform
        .and_then(|value| value.get("default_value"))
        .and_then(Value::as_f64)
        .unwrap_or(0.0);

    if source_field != "temperature" && source_field != "heat_load" {
        let supports_element_field = distribution == "element_to_nodes"
            && matches!(
                source_field.as_str(),
                "average_temperature" | "heat_flux_x" | "heat_flux_y" | "heat_flux_magnitude"
            );
        if !supports_element_field {
            return Err(format!(
                "unsupported heat-to-thermo bridge source field: {source_field}"
            ));
        }
    }
    if distribution != "node_to_node" && distribution != "element_to_nodes" {
        return Err(format!(
            "unsupported heat-to-thermo bridge distribution: {distribution}"
        ));
    }
    if reduction != "copy"
        && reduction != "mean"
        && reduction != "sum"
        && reduction != "area_weighted_mean"
        && reduction != "max"
        && reduction != "min"
    {
        return Err(format!(
            "unsupported heat-to-thermo bridge reduction: {reduction}"
        ));
    }
    if target_field != "temperature_delta" {
        return Err(format!(
            "unsupported heat-to-thermo bridge target field: {target_field}"
        ));
    }

    Ok(HeatToThermoBridgeContract {
        source_field,
        distribution,
        node_index_fields,
        reduction,
        target_field,
        scale,
        default_value,
    })
}

fn derive_direct_heat_nodal_target_field(
    nodes: &[HeatPlaneNodeResult],
    contract: &HeatToThermoBridgeContract,
) -> Result<(Vec<f64>, Vec<f64>), String> {
    let mut source_values = Vec::with_capacity(nodes.len());
    let nodal_values = nodes
        .iter()
        .map(|node| {
            let source_value = resolve_heat_bridge_node_value(node, contract)? * contract.scale;
            source_values.push(source_value);
            Ok(source_value)
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok((source_values, nodal_values))
}

fn derive_element_heat_nodal_target_field<TElement>(
    elements: &[TElement],
    node_count: usize,
    contract: &HeatToThermoBridgeContract,
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
                    "heat bridge node index {} from field {} is out of bounds",
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
            reduce_heat_bridge_value(
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

fn reduce_heat_bridge_value(
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
        "copy" | "mean" if count > 0 => sum / count as f64,
        "sum" if count > 0 => sum,
        "max" if count > 0 => maximum,
        "min" if count > 0 => minimum,
        "area_weighted_mean" if weight_total > 0.0 => weighted_sum / weight_total,
        _ => default_value,
    }
}

fn resolve_heat_bridge_node_value(
    node: &HeatPlaneNodeResult,
    contract: &HeatToThermoBridgeContract,
) -> Result<f64, String> {
    let source_value = match contract.source_field.as_str() {
        "temperature" => node.temperature,
        "heat_load" => node.heat_load,
        other => {
            return Err(format!(
                "unsupported heat-to-thermo bridge source field: {other}"
            ));
        }
    };
    Ok(if source_value.is_finite() {
        source_value
    } else {
        contract.default_value
    })
}

fn heat_quad_bridge_source_value(
    element: &kyuubiki_protocol::HeatPlaneQuadElementResult,
    field: &str,
) -> Result<f64, String> {
    match field {
        "average_temperature" => Ok(element.average_temperature),
        "heat_flux_x" => Ok(element.heat_flux_x),
        "heat_flux_y" => Ok(element.heat_flux_y),
        "heat_flux_magnitude" | "heat_flux" => Ok(element.heat_flux_magnitude),
        other => Err(format!(
            "unsupported heat quad bridge source field: {other}"
        )),
    }
}

fn heat_triangle_bridge_source_value(
    element: &kyuubiki_protocol::HeatPlaneTriangleElementResult,
    field: &str,
) -> Result<f64, String> {
    match field {
        "average_temperature" => Ok(element.average_temperature),
        "heat_flux_x" => Ok(element.heat_flux_x),
        "heat_flux_y" => Ok(element.heat_flux_y),
        "heat_flux_magnitude" | "heat_flux" => Ok(element.heat_flux_magnitude),
        other => Err(format!(
            "unsupported heat triangle bridge source field: {other}"
        )),
    }
}

fn heat_quad_bridge_node_index(
    element: &kyuubiki_protocol::HeatPlaneQuadElementResult,
    field: &str,
) -> Result<usize, String> {
    match field {
        "node_i" => Ok(element.node_i),
        "node_j" => Ok(element.node_j),
        "node_k" => Ok(element.node_k),
        "node_l" => Ok(element.node_l),
        other => Err(format!(
            "unsupported heat quad bridge node index field: {other}"
        )),
    }
}

fn heat_triangle_bridge_node_index(
    element: &kyuubiki_protocol::HeatPlaneTriangleElementResult,
    field: &str,
) -> Result<usize, String> {
    match field {
        "node_i" => Ok(element.node_i),
        "node_j" => Ok(element.node_j),
        "node_k" => Ok(element.node_k),
        other => Err(format!(
            "unsupported heat triangle bridge node index field: {other}"
        )),
    }
}
