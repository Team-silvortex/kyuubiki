use crate::bridge::{
    derive_element_nodal_target_field, resolve_electrostatic_to_heat_bridge_contract,
    BridgeDiagnostics, ElectrostaticToHeatBridgeContract,
};
use kyuubiki_protocol::{SolveHeatPlaneQuad2dRequest, SolveMagnetostaticPlaneQuad2dResult};
use serde_json::Value;

pub(crate) fn resolve_magnetostatic_to_heat_bridge_contract(
    config: &Value,
) -> Result<ElectrostaticToHeatBridgeContract, String> {
    let mut patched = config.clone();
    let object = patched
        .as_object_mut()
        .ok_or_else(|| "magnetostatic-to-heat bridge config must be an object".to_string())?;
    let contract = object
        .entry("contract")
        .or_insert_with(|| serde_json::json!({}));
    let contract_object = contract
        .as_object_mut()
        .ok_or_else(|| "magnetostatic-to-heat bridge contract must be an object".to_string())?;
    let source = contract_object
        .entry("source")
        .or_insert_with(|| serde_json::json!({}));
    let source_object = source
        .as_object_mut()
        .ok_or_else(|| "magnetostatic-to-heat bridge source must be an object".to_string())?;
    source_object
        .entry("field")
        .or_insert_with(|| Value::String("magnetic_flux_density_magnitude".to_string()));

    resolve_electrostatic_to_heat_bridge_contract(&patched)
}

pub(crate) fn bridge_magnetostatic_result_to_heat_plane_quad_model(
    magnetostatic_result: &SolveMagnetostaticPlaneQuad2dResult,
    heat_seed_model: &SolveHeatPlaneQuad2dRequest,
    contract: &ElectrostaticToHeatBridgeContract,
) -> Result<(SolveHeatPlaneQuad2dRequest, BridgeDiagnostics), String> {
    if magnetostatic_result.nodes.len() != heat_seed_model.nodes.len() {
        return Err("magnetostatic and heat quad models must have the same node count".to_string());
    }
    if contract.distribution == "element_to_nodes"
        && magnetostatic_result.input.elements.len() != heat_seed_model.elements.len()
    {
        return Err(
            "magnetostatic and heat quad models must have the same element count".to_string(),
        );
    }

    let mut bridged = heat_seed_model.clone();
    for (magnetic_node, heat_node) in magnetostatic_result.nodes.iter().zip(bridged.nodes.iter()) {
        if (magnetic_node.x - heat_node.x).abs() > 1.0e-9
            || (magnetic_node.y - heat_node.y).abs() > 1.0e-9
        {
            return Err(format!(
                "magnetostatic node {} does not align with heat node {}",
                magnetic_node.id, heat_node.id
            ));
        }
    }

    let (source_values, nodal_values) = if contract.distribution == "node_to_node" {
        derive_direct_nodal_target_field(&magnetostatic_result.nodes, contract)?
    } else {
        derive_element_nodal_target_field(
            &magnetostatic_result.elements,
            bridged.nodes.len(),
            contract,
            bridge_source_value,
            bridge_node_index,
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
            bridge_kind: "magnetostatic_to_heat_quad_2d".to_string(),
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

fn derive_direct_nodal_target_field(
    nodes: &[kyuubiki_protocol::MagnetostaticPlaneNodeResult],
    contract: &ElectrostaticToHeatBridgeContract,
) -> Result<(Vec<f64>, Vec<f64>), String> {
    let mut source_values = Vec::with_capacity(nodes.len());
    let nodal_values = nodes
        .iter()
        .map(|node| {
            let source_value =
                node_source_value(node, contract.source_field.as_str())? * contract.scale;
            source_values.push(source_value);
            Ok(source_value)
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok((source_values, nodal_values))
}

fn bridge_source_value(
    element: &kyuubiki_protocol::MagnetostaticPlaneQuadElementResult,
    field: &str,
) -> Result<f64, String> {
    match field {
        "magnetic_field_strength_magnitude" => Ok(element.magnetic_field_strength_magnitude),
        "magnetic_field_strength_x" => Ok(element.magnetic_field_strength_x),
        "magnetic_field_strength_y" => Ok(element.magnetic_field_strength_y),
        "flux_magnitude" | "magnetic_flux_density_magnitude" => {
            Ok(element.magnetic_flux_density_magnitude)
        }
        "magnetic_flux_density_x" => Ok(element.magnetic_flux_density_x),
        "magnetic_flux_density_y" => Ok(element.magnetic_flux_density_y),
        "average_vector_potential" => Ok(element.average_vector_potential),
        "stored_energy" | "energy" => Ok(element.stored_energy),
        "stored_energy_area_density" | "energy_area_density" => {
            Ok(element.stored_energy / element.area.max(f64::EPSILON))
        }
        other => Err(format!(
            "unsupported magnetostatic-to-heat bridge source field: {other}"
        )),
    }
}

fn bridge_node_index(
    element: &kyuubiki_protocol::MagnetostaticPlaneQuadElementResult,
    field: &str,
) -> Result<usize, String> {
    match field {
        "node_i" => Ok(element.node_i),
        "node_j" => Ok(element.node_j),
        "node_k" => Ok(element.node_k),
        "node_l" => Ok(element.node_l),
        other => Err(format!(
            "unsupported magnetostatic-to-heat bridge node index field: {other}"
        )),
    }
}

fn node_source_value(
    node: &kyuubiki_protocol::MagnetostaticPlaneNodeResult,
    field: &str,
) -> Result<f64, String> {
    match field {
        "vector_potential" => Ok(node.vector_potential),
        "current_density" => Ok(node.current_density),
        other => Err(format!(
            "unsupported magnetostatic node bridge source field: {other}"
        )),
    }
}
