use crate::{EngineSolveRequest, solve};
use kyuubiki_protocol::{
    AnalysisResult, HeatToThermoPlaneQuad2dWorkflowRequest, HeatToThermoPlaneQuad2dWorkflowResult,
    SolveElectrostaticPlaneQuad2dResult, SolveElectrostaticPlaneTriangle2dResult,
    SolveHeatPlaneQuad2dRequest, SolveHeatPlaneTriangle2dRequest, SolveHeatPlaneTriangle2dResult,
    SolveThermalPlaneQuad2dRequest, SolveThermalPlaneTriangle2dRequest,
};
use serde_json::Value;

pub fn bridge_heat_result_to_thermal_plane_quad_model(
    heat_result: &kyuubiki_protocol::SolveHeatPlaneQuad2dResult,
    thermo_seed_model: &SolveThermalPlaneQuad2dRequest,
) -> Result<SolveThermalPlaneQuad2dRequest, String> {
    if heat_result.nodes.len() != thermo_seed_model.nodes.len() {
        return Err("heat and thermo quad models must have the same node count".to_string());
    }
    if heat_result.input.elements.len() != thermo_seed_model.elements.len() {
        return Err("heat and thermo quad models must have the same element count".to_string());
    }

    let mut bridged = thermo_seed_model.clone();
    for (heat_node, thermo_node) in heat_result.nodes.iter().zip(bridged.nodes.iter_mut()) {
        if (heat_node.x - thermo_node.x).abs() > 1.0e-9
            || (heat_node.y - thermo_node.y).abs() > 1.0e-9
        {
            return Err(format!(
                "heat node {} does not align with thermo node {}",
                heat_node.id, thermo_node.id
            ));
        }
        thermo_node.temperature_delta = heat_node.temperature;
    }

    Ok(bridged)
}

pub fn bridge_heat_result_to_thermal_plane_triangle_model(
    heat_result: &SolveHeatPlaneTriangle2dResult,
    thermo_seed_model: &SolveThermalPlaneTriangle2dRequest,
) -> Result<SolveThermalPlaneTriangle2dRequest, String> {
    if heat_result.nodes.len() != thermo_seed_model.nodes.len() {
        return Err("heat and thermo triangle models must have the same node count".to_string());
    }
    if heat_result.input.elements.len() != thermo_seed_model.elements.len() {
        return Err("heat and thermo triangle models must have the same element count".to_string());
    }

    let mut bridged = thermo_seed_model.clone();
    for (heat_node, thermo_node) in heat_result.nodes.iter().zip(bridged.nodes.iter_mut()) {
        if (heat_node.x - thermo_node.x).abs() > 1.0e-9
            || (heat_node.y - thermo_node.y).abs() > 1.0e-9
        {
            return Err(format!(
                "heat node {} does not align with thermo node {}",
                heat_node.id, thermo_node.id
            ));
        }
        thermo_node.temperature_delta = heat_node.temperature;
    }

    Ok(bridged)
}

pub fn run_heat_to_thermo_plane_quad_2d_workflow(
    request: HeatToThermoPlaneQuad2dWorkflowRequest,
) -> Result<HeatToThermoPlaneQuad2dWorkflowResult, String> {
    let heat_result = match solve(EngineSolveRequest::HeatPlaneQuad2d(request.heat_model))? {
        AnalysisResult::HeatPlaneQuad2d(result) => result,
        _ => return Err("heat-plane-quad workflow produced an unexpected heat result".to_string()),
    };

    let bridged_model =
        bridge_heat_result_to_thermal_plane_quad_model(&heat_result, &request.thermo_seed_model)?;
    let thermo_result = match solve(EngineSolveRequest::ThermalPlaneQuad2d(
        bridged_model.clone(),
    ))? {
        AnalysisResult::ThermalPlaneQuad2d(result) => result,
        _ => {
            return Err("heat-to-thermo workflow produced an unexpected thermo result".to_string());
        }
    };

    Ok(HeatToThermoPlaneQuad2dWorkflowResult {
        workflow_id: "workflow.heat-to-thermo-quad-2d".to_string(),
        heat_result,
        bridged_model,
        thermo_result,
    })
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

    if distribution != "element_to_nodes" {
        return Err(format!(
            "unsupported electrostatic-to-heat bridge distribution: {distribution}"
        ));
    }
    if reduction != "mean" && reduction != "sum" && reduction != "area_weighted_mean" {
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
) -> Result<SolveHeatPlaneQuad2dRequest, String> {
    if electrostatic_result.nodes.len() != heat_seed_model.nodes.len() {
        return Err("electrostatic and heat quad models must have the same node count".to_string());
    }
    if electrostatic_result.input.elements.len() != heat_seed_model.elements.len() {
        return Err(
            "electrostatic and heat quad models must have the same element count".to_string(),
        );
    }
    if contract.distribution != "element_to_nodes" {
        return Err(format!(
            "unsupported electrostatic-to-heat bridge distribution: {}",
            contract.distribution
        ));
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

    let mut sums = vec![0.0; bridged.nodes.len()];
    let mut counts = vec![0usize; bridged.nodes.len()];
    let mut weighted_sums = vec![0.0; bridged.nodes.len()];
    let mut weight_totals = vec![0.0; bridged.nodes.len()];

    for element in &electrostatic_result.elements {
        let source_value =
            electrostatic_bridge_source_value(element, contract.source_field.as_str())?
                * contract.scale;
        let weight = element.area.abs();
        for field in &contract.node_index_fields {
            let index = electrostatic_bridge_node_index(element, field)?;
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
        }
    }

    for (index, heat_node) in bridged.nodes.iter_mut().enumerate() {
        let value = reduce_bridge_value(
            contract.reduction.as_str(),
            contract.default_value,
            counts[index],
            sums[index],
            weighted_sums[index],
            weight_totals[index],
        );
        match contract.target_field.as_str() {
            "heat_load" => heat_node.heat_load = value,
            "temperature" => heat_node.temperature = value,
            _ => unreachable!("bridge target field validated earlier"),
        }
    }

    Ok(bridged)
}

pub(crate) fn bridge_electrostatic_result_to_heat_plane_triangle_model(
    electrostatic_result: &SolveElectrostaticPlaneTriangle2dResult,
    heat_seed_model: &SolveHeatPlaneTriangle2dRequest,
    contract: &ElectrostaticToHeatBridgeContract,
) -> Result<SolveHeatPlaneTriangle2dRequest, String> {
    if electrostatic_result.nodes.len() != heat_seed_model.nodes.len() {
        return Err(
            "electrostatic and heat triangle models must have the same node count".to_string(),
        );
    }
    if electrostatic_result.input.elements.len() != heat_seed_model.elements.len() {
        return Err(
            "electrostatic and heat triangle models must have the same element count".to_string(),
        );
    }
    if contract.distribution != "element_to_nodes" {
        return Err(format!(
            "unsupported electrostatic-to-heat bridge distribution: {}",
            contract.distribution
        ));
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

    let mut sums = vec![0.0; bridged.nodes.len()];
    let mut counts = vec![0usize; bridged.nodes.len()];
    let mut weighted_sums = vec![0.0; bridged.nodes.len()];
    let mut weight_totals = vec![0.0; bridged.nodes.len()];

    for element in &electrostatic_result.elements {
        let source_value =
            electrostatic_triangle_bridge_source_value(element, contract.source_field.as_str())?
                * contract.scale;
        let weight = element.area.abs();
        for field in &contract.node_index_fields {
            let index = electrostatic_triangle_bridge_node_index(element, field)?;
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
        }
    }

    for (index, heat_node) in bridged.nodes.iter_mut().enumerate() {
        let value = reduce_bridge_value(
            contract.reduction.as_str(),
            contract.default_value,
            counts[index],
            sums[index],
            weighted_sums[index],
            weight_totals[index],
        );
        match contract.target_field.as_str() {
            "heat_load" => heat_node.heat_load = value,
            "temperature" => heat_node.temperature = value,
            _ => unreachable!("bridge target field validated earlier"),
        }
    }

    Ok(bridged)
}

fn reduce_bridge_value(
    reduction: &str,
    default_value: f64,
    count: usize,
    sum: f64,
    weighted_sum: f64,
    weight_total: f64,
) -> f64 {
    match reduction {
        "sum" if count > 0 => sum,
        "area_weighted_mean" if weight_total > 0.0 => weighted_sum / weight_total,
        _ if count > 0 => sum / count as f64,
        _ => default_value,
    }
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
