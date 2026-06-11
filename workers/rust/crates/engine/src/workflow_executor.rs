use crate::bridge::{
    bridge_electrostatic_result_to_heat_plane_quad_model,
    bridge_heat_result_to_thermal_plane_quad_model, resolve_electrostatic_to_heat_bridge_contract,
};
use crate::{EngineSolveRequest, solve};
use kyuubiki_protocol::{
    AnalysisResult, SolveBeam1dRequest, SolveElectrostaticBar1dRequest,
    SolveElectrostaticPlaneQuad2dRequest, SolveElectrostaticPlaneTriangle2dRequest,
    SolveFrame2dRequest, SolveFrame3dRequest, SolveHeatPlaneQuad2dRequest, SolveSpring1dRequest,
    SolveSpring2dRequest, SolveSpring3dRequest, SolveThermalBeam1dRequest,
    SolveThermalFrame2dRequest, SolveThermalFrame3dRequest, SolveThermalPlaneQuad2dRequest,
    SolveThermalTruss3dRequest, SolveTruss2dRequest, SolveTruss3dRequest,
};
use serde_json::Value;
use std::collections::BTreeMap;

const SUPPORTED_SOLVE_OPERATORS: &[&str] = &[
    "solve.electrostatic_bar_1d",
    "solve.electrostatic_plane_triangle_2d",
    "solve.electrostatic_plane_quad_2d",
    "solve.heat_plane_quad_2d",
    "solve.frame_3d",
    "solve.thermal_frame_3d",
    "solve.thermal_plane_quad_2d",
    "solve.thermal_truss_3d",
    "solve.spring_1d",
    "solve.spring_2d",
    "solve.spring_3d",
    "solve.truss_2d",
    "solve.truss_3d",
    "solve.frame_2d",
    "solve.beam_1d",
    "solve.thermal_beam_1d",
    "solve.thermal_frame_2d",
];

const SUPPORTED_TRANSFORM_OPERATORS: &[&str] = &[
    "bridge.temperature_field_to_thermo_quad_2d",
    "bridge.electrostatic_field_to_heat_quad_2d",
];

const SUPPORTED_EXTRACT_OPERATORS: &[&str] = &["extract.result_summary"];

const SUPPORTED_EXPORT_OPERATORS: &[&str] = &["export.summary_json", "export.summary_csv"];

pub fn artifact_key(node_id: &str, port_id: &str) -> String {
    format!("{node_id}.{port_id}")
}

pub fn supported_workflow_operator_ids() -> Vec<&'static str> {
    SUPPORTED_SOLVE_OPERATORS
        .iter()
        .chain(SUPPORTED_TRANSFORM_OPERATORS.iter())
        .chain(SUPPORTED_EXTRACT_OPERATORS.iter())
        .chain(SUPPORTED_EXPORT_OPERATORS.iter())
        .copied()
        .collect()
}

pub fn is_supported_workflow_operator(operator_id: &str) -> bool {
    supported_workflow_operator_ids().contains(&operator_id)
}

pub fn resolve_single_input_payload(
    node: &kyuubiki_protocol::WorkflowNode,
    incoming: &[&kyuubiki_protocol::WorkflowEdge],
    artifacts: &BTreeMap<String, Value>,
) -> Result<Value, String> {
    let first = incoming.first().ok_or_else(|| {
        format!(
            "workflow node {} requires at least one input artifact in the first executor",
            node.id
        )
    })?;
    artifacts
        .get(&artifact_key(&first.from.node, &first.from.port))
        .cloned()
        .ok_or_else(|| {
            format!(
                "workflow node {} could not resolve input from {}.{}",
                node.id, first.from.node, first.from.port
            )
        })
}

pub fn run_solve_operator(operator_id: &str, payload: Value) -> Result<Value, String> {
    match operator_id {
        "solve.electrostatic_bar_1d" => {
            let request: SolveElectrostaticBar1dRequest =
                serde_json::from_value(payload).map_err(|err| err.to_string())?;
            let result = match solve(EngineSolveRequest::ElectrostaticBar1d(request))? {
                AnalysisResult::ElectrostaticBar1d(result) => result,
                _ => unreachable!("solve.electrostatic_bar_1d returned unexpected result"),
            };
            serde_json::to_value(result).map_err(|err| err.to_string())
        }
        "solve.electrostatic_plane_triangle_2d" => {
            let request: SolveElectrostaticPlaneTriangle2dRequest =
                serde_json::from_value(payload).map_err(|err| err.to_string())?;
            let result = match solve(EngineSolveRequest::ElectrostaticPlaneTriangle2d(request))? {
                AnalysisResult::ElectrostaticPlaneTriangle2d(result) => result,
                _ => {
                    unreachable!("solve.electrostatic_plane_triangle_2d returned unexpected result")
                }
            };
            serde_json::to_value(result).map_err(|err| err.to_string())
        }
        "solve.electrostatic_plane_quad_2d" => {
            let request: SolveElectrostaticPlaneQuad2dRequest =
                serde_json::from_value(payload).map_err(|err| err.to_string())?;
            let result = match solve(EngineSolveRequest::ElectrostaticPlaneQuad2d(request))? {
                AnalysisResult::ElectrostaticPlaneQuad2d(result) => result,
                _ => unreachable!("solve.electrostatic_plane_quad_2d returned unexpected result"),
            };
            serde_json::to_value(result).map_err(|err| err.to_string())
        }
        "solve.heat_plane_quad_2d" => {
            let request: SolveHeatPlaneQuad2dRequest =
                serde_json::from_value(payload).map_err(|err| err.to_string())?;
            let result = match solve(EngineSolveRequest::HeatPlaneQuad2d(request))? {
                AnalysisResult::HeatPlaneQuad2d(result) => result,
                _ => unreachable!("solve.heat_plane_quad_2d returned unexpected result"),
            };
            serde_json::to_value(result).map_err(|err| err.to_string())
        }
        "solve.frame_3d" => {
            let request: SolveFrame3dRequest =
                serde_json::from_value(payload).map_err(|err| err.to_string())?;
            let result = match solve(EngineSolveRequest::Frame3d(request))? {
                AnalysisResult::Frame3d(result) => result,
                _ => unreachable!("solve.frame_3d returned unexpected result"),
            };
            serde_json::to_value(result).map_err(|err| err.to_string())
        }
        "solve.thermal_frame_3d" => {
            let request: SolveThermalFrame3dRequest =
                serde_json::from_value(payload).map_err(|err| err.to_string())?;
            let result = match solve(EngineSolveRequest::ThermalFrame3d(request))? {
                AnalysisResult::ThermalFrame3d(result) => result,
                _ => unreachable!("solve.thermal_frame_3d returned unexpected result"),
            };
            serde_json::to_value(result).map_err(|err| err.to_string())
        }
        "solve.thermal_plane_quad_2d" => {
            let request: SolveThermalPlaneQuad2dRequest =
                serde_json::from_value(payload).map_err(|err| err.to_string())?;
            let result = match solve(EngineSolveRequest::ThermalPlaneQuad2d(request))? {
                AnalysisResult::ThermalPlaneQuad2d(result) => result,
                _ => unreachable!("solve.thermal_plane_quad_2d returned unexpected result"),
            };
            serde_json::to_value(result).map_err(|err| err.to_string())
        }
        "solve.thermal_truss_3d" => {
            let request: SolveThermalTruss3dRequest =
                serde_json::from_value(payload).map_err(|err| err.to_string())?;
            let result = match solve(EngineSolveRequest::ThermalTruss3d(request))? {
                AnalysisResult::ThermalTruss3d(result) => result,
                _ => unreachable!("solve.thermal_truss_3d returned unexpected result"),
            };
            serde_json::to_value(result).map_err(|err| err.to_string())
        }
        "solve.spring_1d" => {
            let request: SolveSpring1dRequest =
                serde_json::from_value(payload).map_err(|err| err.to_string())?;
            let result = match solve(EngineSolveRequest::Spring1d(request))? {
                AnalysisResult::Spring1d(result) => result,
                _ => unreachable!("solve.spring_1d returned unexpected result"),
            };
            serde_json::to_value(result).map_err(|err| err.to_string())
        }
        "solve.spring_2d" => {
            let request: SolveSpring2dRequest =
                serde_json::from_value(payload).map_err(|err| err.to_string())?;
            let result = match solve(EngineSolveRequest::Spring2d(request))? {
                AnalysisResult::Spring2d(result) => result,
                _ => unreachable!("solve.spring_2d returned unexpected result"),
            };
            serde_json::to_value(result).map_err(|err| err.to_string())
        }
        "solve.spring_3d" => {
            let request: SolveSpring3dRequest =
                serde_json::from_value(payload).map_err(|err| err.to_string())?;
            let result = match solve(EngineSolveRequest::Spring3d(request))? {
                AnalysisResult::Spring3d(result) => result,
                _ => unreachable!("solve.spring_3d returned unexpected result"),
            };
            serde_json::to_value(result).map_err(|err| err.to_string())
        }
        "solve.truss_2d" => {
            let request: SolveTruss2dRequest =
                serde_json::from_value(payload).map_err(|err| err.to_string())?;
            let result = match solve(EngineSolveRequest::Truss2d(request))? {
                AnalysisResult::Truss2d(result) => result,
                _ => unreachable!("solve.truss_2d returned unexpected result"),
            };
            serde_json::to_value(result).map_err(|err| err.to_string())
        }
        "solve.truss_3d" => {
            let request: SolveTruss3dRequest =
                serde_json::from_value(payload).map_err(|err| err.to_string())?;
            let result = match solve(EngineSolveRequest::Truss3d(request))? {
                AnalysisResult::Truss3d(result) => result,
                _ => unreachable!("solve.truss_3d returned unexpected result"),
            };
            serde_json::to_value(result).map_err(|err| err.to_string())
        }
        "solve.frame_2d" => {
            let request: SolveFrame2dRequest =
                serde_json::from_value(payload).map_err(|err| err.to_string())?;
            let result = match solve(EngineSolveRequest::Frame2d(request))? {
                AnalysisResult::Frame2d(result) => result,
                _ => unreachable!("solve.frame_2d returned unexpected result"),
            };
            serde_json::to_value(result).map_err(|err| err.to_string())
        }
        "solve.beam_1d" => {
            let request: SolveBeam1dRequest =
                serde_json::from_value(payload).map_err(|err| err.to_string())?;
            let result = match solve(EngineSolveRequest::Beam1d(request))? {
                AnalysisResult::Beam1d(result) => result,
                _ => unreachable!("solve.beam_1d returned unexpected result"),
            };
            serde_json::to_value(result).map_err(|err| err.to_string())
        }
        "solve.thermal_beam_1d" => {
            let request: SolveThermalBeam1dRequest =
                serde_json::from_value(payload).map_err(|err| err.to_string())?;
            let result = match solve(EngineSolveRequest::ThermalBeam1d(request))? {
                AnalysisResult::ThermalBeam1d(result) => result,
                _ => unreachable!("solve.thermal_beam_1d returned unexpected result"),
            };
            serde_json::to_value(result).map_err(|err| err.to_string())
        }
        "solve.thermal_frame_2d" => {
            let request: SolveThermalFrame2dRequest =
                serde_json::from_value(payload).map_err(|err| err.to_string())?;
            let result = match solve(EngineSolveRequest::ThermalFrame2d(request))? {
                AnalysisResult::ThermalFrame2d(result) => result,
                _ => unreachable!("solve.thermal_frame_2d returned unexpected result"),
            };
            serde_json::to_value(result).map_err(|err| err.to_string())
        }
        _ => Err(format!(
            "unsupported solve operator in first executor: {operator_id}"
        )),
    }
}

pub fn run_transform_operator(
    operator_id: &str,
    payload: Value,
    config: Value,
) -> Result<Value, String> {
    match operator_id {
        "bridge.temperature_field_to_thermo_quad_2d" => {
            let heat_result = serde_json::from_value(payload).map_err(|err| err.to_string())?;
            let thermo_seed_model: SolveThermalPlaneQuad2dRequest =
                serde_json::from_value(config).map_err(|err| err.to_string())?;
            let bridged =
                bridge_heat_result_to_thermal_plane_quad_model(&heat_result, &thermo_seed_model)?;
            serde_json::to_value(bridged).map_err(|err| err.to_string())
        }
        "bridge.electrostatic_field_to_heat_quad_2d" => {
            let electrostatic_result =
                serde_json::from_value(payload).map_err(|err| err.to_string())?;
            let seed_model_value = config.get("seed_model").cloned().ok_or_else(|| {
                "bridge.electrostatic_field_to_heat_quad_2d requires config.seed_model".to_string()
            })?;
            let heat_seed_model: SolveHeatPlaneQuad2dRequest =
                serde_json::from_value(seed_model_value).map_err(|err| err.to_string())?;
            let contract = resolve_electrostatic_to_heat_bridge_contract(&config)?;
            let bridged = bridge_electrostatic_result_to_heat_plane_quad_model(
                &electrostatic_result,
                &heat_seed_model,
                &contract,
            )?;
            serde_json::to_value(bridged).map_err(|err| err.to_string())
        }
        _ => Err(format!(
            "unsupported transform operator in first executor: {operator_id}"
        )),
    }
}

pub fn run_extract_operator(
    operator_id: &str,
    payload: Value,
    config: Value,
) -> Result<Value, String> {
    match operator_id {
        "extract.result_summary" => extract_result_summary(payload, config),
        _ => Err(format!(
            "unsupported extract operator in first executor: {operator_id}"
        )),
    }
}

pub fn run_export_operator(
    operator_id: &str,
    payload: Value,
    config: Value,
) -> Result<Value, String> {
    match operator_id {
        "export.summary_json" => export_summary_json(payload),
        "export.summary_csv" => export_summary_csv(payload, config),
        _ => Err(format!(
            "unsupported export operator in first executor: {operator_id}"
        )),
    }
}

fn extract_result_summary(payload: Value, config: Value) -> Result<Value, String> {
    let object = payload
        .as_object()
        .ok_or_else(|| "extract.result_summary expects an object payload".to_string())?;

    let requested_fields = config
        .get("fields")
        .and_then(Value::as_array)
        .map(|fields| {
            fields
                .iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        });

    let mut summary = serde_json::Map::new();
    if let Some(fields) = requested_fields {
        for field in fields {
            if let Some(value) = object.get(&field) {
                summary.insert(field, value.clone());
            }
        }
    } else {
        for (key, value) in object {
            if key.starts_with("max_") {
                summary.insert(key.clone(), value.clone());
            }
        }
    }

    if summary.is_empty() {
        return Err("extract.result_summary did not find any summary fields".to_string());
    }

    Ok(Value::Object(summary))
}

fn export_summary_json(payload: Value) -> Result<Value, String> {
    if !payload.is_object() {
        return Err("export.summary_json expects an object payload".to_string());
    }
    let content = serde_json::to_string_pretty(&payload).map_err(|err| err.to_string())?;
    Ok(serde_json::json!({
        "format": "json",
        "content_type": "application/json",
        "content": content
    }))
}

fn export_summary_csv(payload: Value, config: Value) -> Result<Value, String> {
    let object = payload
        .as_object()
        .ok_or_else(|| "export.summary_csv expects an object payload".to_string())?;

    let requested_fields = config
        .get("fields")
        .and_then(Value::as_array)
        .map(|fields| {
            fields
                .iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        });

    let mut rows = vec!["key,value".to_string()];
    if let Some(fields) = requested_fields {
        for field in fields {
            if let Some(value) = object.get(&field) {
                rows.push(format!("{},{}", field, csv_cell(value)));
            }
        }
    } else {
        for (key, value) in object {
            rows.push(format!("{},{}", key, csv_cell(value)));
        }
    }

    if rows.len() == 1 {
        return Err("export.summary_csv did not find any exportable fields".to_string());
    }

    Ok(serde_json::json!({
        "format": "csv",
        "content_type": "text/csv",
        "content": rows.join("\n")
    }))
}

fn csv_cell(value: &Value) -> String {
    match value {
        Value::Null => "".to_string(),
        Value::Bool(boolean) => boolean.to_string(),
        Value::Number(number) => number.to_string(),
        Value::String(string) => {
            if string.contains([',', '"', '\n']) {
                format!("\"{}\"", string.replace('"', "\"\""))
            } else {
                string.clone()
            }
        }
        other => serde_json::to_string(other).unwrap_or_else(|_| "\"<invalid>\"".to_string()),
    }
}
