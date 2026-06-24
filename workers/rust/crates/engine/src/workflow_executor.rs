use crate::{
    EngineSolveRequest,
    operator_sdk_runtime::{
        run_registered_export_operator, run_registered_extract_operator,
        run_registered_transform_operator,
    },
    solve,
};
use kyuubiki_protocol::{
    AnalysisResult, SolveBarRequest, SolveBeam1dRequest, SolveElectrostaticBar1dRequest,
    SolveElectrostaticPlaneQuad2dRequest, SolveElectrostaticPlaneTriangle2dRequest,
    SolveFrame2dRequest, SolveFrame3dRequest, SolveHeatBar1dRequest, SolveHeatPlaneQuad2dRequest,
    SolveHeatPlaneTriangle2dRequest, SolveMagnetostaticBar1dRequest, SolvePlaneQuad2dRequest,
    SolvePlaneTriangle2dRequest, SolveSpring1dRequest, SolveSpring2dRequest, SolveSpring3dRequest,
    SolveThermalBar1dRequest, SolveThermalBeam1dRequest, SolveThermalFrame2dRequest,
    SolveThermalFrame3dRequest, SolveThermalPlaneQuad2dRequest, SolveThermalPlaneTriangle2dRequest,
    SolveThermalTruss2dRequest, SolveThermalTruss3dRequest, SolveTorsion1dRequest,
    SolveTruss2dRequest, SolveTruss3dRequest,
};
use serde_json::Value;
use std::collections::BTreeMap;

const SUPPORTED_SOLVE_OPERATORS: &[&str] = &[
    "solve.bar_1d",
    "solve.thermal_bar_1d",
    "solve.heat_bar_1d",
    "solve.electrostatic_bar_1d",
    "solve.magnetostatic_bar_1d",
    "solve.electrostatic_plane_triangle_2d",
    "solve.electrostatic_plane_quad_2d",
    "solve.heat_plane_triangle_2d",
    "solve.heat_plane_quad_2d",
    "solve.thermal_truss_2d",
    "solve.frame_3d",
    "solve.plane_triangle_2d",
    "solve.thermal_plane_triangle_2d",
    "solve.plane_quad_2d",
    "solve.thermal_frame_3d",
    "solve.thermal_plane_quad_2d",
    "solve.thermal_truss_3d",
    "solve.torsion_1d",
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
    "bridge.temperature_field_to_thermo_triangle_2d",
    "bridge.electrostatic_field_to_heat_quad_2d",
    "bridge.electrostatic_field_to_heat_triangle_2d",
    "transform.first_available",
    "transform.merge_summary_pair",
    "transform.compare_summary_pair",
    "transform.aggregate_summary_collection",
    "transform.normalize_summary_fields",
    "transform.select_best_summary",
    "transform.evaluate_thermal_guard",
    "transform.benchmark_coupled_heat_pair",
    "transform.compose_diagnostics_bundle",
    "transform.evaluate_diagnostics_bundle_guard",
    "transform.compose_diagnostics_report_payload",
    "transform.select_focus_payload",
    "transform.compose_focus_chain_input",
    "transform.compose_focus_bridge_request",
    "transform.resolve_focus_bridge_execution",
    "transform.execute_focus_bridge_execution",
];

const SUPPORTED_EXTRACT_OPERATORS: &[&str] = &[
    "extract.result_summary",
    "extract.field_statistics",
    "extract.field_hotspots",
    "extract.electrostatic_result_diagnostics",
    "extract.electrostatic_peak_field",
    "extract.thermal_result_diagnostics",
    "extract.heat_peak_flux",
    "extract.thermo_result_diagnostics",
    "extract.thermo_peak_response",
];

const SUPPORTED_EXPORT_OPERATORS: &[&str] = &[
    "export.summary_json",
    "export.summary_csv",
    "export.alert_markdown",
    "export.diagnostics_bundle_markdown",
];

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

pub fn resolve_first_available_input_payload(
    node: &kyuubiki_protocol::WorkflowNode,
    incoming: &[&kyuubiki_protocol::WorkflowEdge],
    artifacts: &BTreeMap<String, Value>,
) -> Result<Value, String> {
    incoming
        .iter()
        .find_map(|edge| artifacts.get(&artifact_key(&edge.from.node, &edge.from.port)))
        .cloned()
        .ok_or_else(|| {
            format!(
                "workflow node {} requires at least one resolved input artifact",
                node.id
            )
        })
}

pub fn transform_operator_accepts_partial_inputs(operator_id: &str) -> bool {
    operator_id == "transform.first_available"
}

pub fn transform_operator_requires_port_map(operator_id: &str) -> bool {
    operator_id == "transform.merge_summary_pair"
        || operator_id == "transform.compare_summary_pair"
        || operator_id == "transform.aggregate_summary_collection"
        || operator_id == "transform.select_best_summary"
        || operator_id == "transform.compose_diagnostics_bundle"
        || operator_id == "transform.compose_diagnostics_report_payload"
        || operator_id == "transform.resolve_focus_bridge_execution"
}

pub fn resolve_named_input_payloads(
    node: &kyuubiki_protocol::WorkflowNode,
    incoming: &[&kyuubiki_protocol::WorkflowEdge],
    artifacts: &BTreeMap<String, Value>,
) -> Result<Value, String> {
    let mut payload = serde_json::Map::new();
    for edge in incoming {
        let artifact = artifacts
            .get(&artifact_key(&edge.from.node, &edge.from.port))
            .cloned()
            .ok_or_else(|| {
                format!(
                    "workflow node {} could not resolve input from {}.{}",
                    node.id, edge.from.node, edge.from.port
                )
            })?;
        payload.insert(edge.to.port.clone(), artifact);
    }
    if payload.is_empty() {
        return Err(format!(
            "workflow node {} requires at least one resolved named input artifact",
            node.id
        ));
    }
    Ok(Value::Object(payload))
}

pub fn evaluate_condition_operator(payload: &Value, config: &Value) -> Result<bool, String> {
    let predicate = config
        .get("predicate")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let operator = predicate
        .get("operator")
        .and_then(Value::as_str)
        .unwrap_or("gt");
    let target = predicate
        .get("path")
        .and_then(Value::as_str)
        .map(|path| resolve_condition_target(payload, path))
        .unwrap_or(payload);

    match operator {
        "truthy" => Ok(is_truthy(target)),
        "falsy" => Ok(!is_truthy(target)),
        "eq" => Ok(target == predicate.get("value").unwrap_or(&Value::Null)),
        "neq" => Ok(target != predicate.get("value").unwrap_or(&Value::Null)),
        "gt" | "gte" | "lt" | "lte" => {
            let left = target
                .as_f64()
                .ok_or_else(|| format!("condition operator {operator} expects numeric input"))?;
            let right = predicate
                .get("value")
                .and_then(Value::as_f64)
                .ok_or_else(|| {
                    format!("condition operator {operator} expects numeric config.value")
                })?;
            Ok(match operator {
                "gt" => left > right,
                "gte" => left >= right,
                "lt" => left < right,
                "lte" => left <= right,
                _ => unreachable!(),
            })
        }
        "contains" => {
            let right = predicate.get("value").unwrap_or(&Value::Null);
            match target {
                Value::String(text) => {
                    Ok(right.as_str().is_some_and(|needle| text.contains(needle)))
                }
                Value::Array(items) => Ok(items.iter().any(|item| item == right)),
                _ => Err("condition operator contains expects string or array input".to_string()),
            }
        }
        _ => Err(format!(
            "unsupported condition operator in first executor: {operator}"
        )),
    }
}

fn resolve_condition_target<'a>(payload: &'a Value, path: &str) -> &'a Value {
    let mut current = payload;
    for segment in path.split('.').filter(|segment| !segment.is_empty()) {
        current = match current {
            Value::Object(map) => map.get(segment).unwrap_or(&Value::Null),
            Value::Array(items) => segment
                .parse::<usize>()
                .ok()
                .and_then(|index| items.get(index))
                .unwrap_or(&Value::Null),
            _ => &Value::Null,
        };
    }
    current
}

fn is_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(flag) => *flag,
        Value::Number(number) => number.as_f64().is_some_and(|entry| entry != 0.0),
        Value::String(text) => !text.is_empty(),
        Value::Array(items) => !items.is_empty(),
        Value::Object(map) => !map.is_empty(),
    }
}

pub fn run_solve_operator(operator_id: &str, payload: Value) -> Result<Value, String> {
    match operator_id {
        "solve.bar_1d" => {
            let request: SolveBarRequest =
                serde_json::from_value(payload).map_err(|err| err.to_string())?;
            let result = match solve(EngineSolveRequest::Bar1d(request))? {
                AnalysisResult::Bar1d(result) => result,
                _ => unreachable!("solve.bar_1d returned unexpected result"),
            };
            serde_json::to_value(result).map_err(|err| err.to_string())
        }
        "solve.thermal_bar_1d" => {
            let request: SolveThermalBar1dRequest =
                serde_json::from_value(payload).map_err(|err| err.to_string())?;
            let result = match solve(EngineSolveRequest::ThermalBar1d(request))? {
                AnalysisResult::ThermalBar1d(result) => result,
                _ => unreachable!("solve.thermal_bar_1d returned unexpected result"),
            };
            serde_json::to_value(result).map_err(|err| err.to_string())
        }
        "solve.heat_bar_1d" => {
            let request: SolveHeatBar1dRequest =
                serde_json::from_value(payload).map_err(|err| err.to_string())?;
            let result = match solve(EngineSolveRequest::HeatBar1d(request))? {
                AnalysisResult::HeatBar1d(result) => result,
                _ => unreachable!("solve.heat_bar_1d returned unexpected result"),
            };
            serde_json::to_value(result).map_err(|err| err.to_string())
        }
        "solve.electrostatic_bar_1d" => {
            let request: SolveElectrostaticBar1dRequest =
                serde_json::from_value(payload).map_err(|err| err.to_string())?;
            let result = match solve(EngineSolveRequest::ElectrostaticBar1d(request))? {
                AnalysisResult::ElectrostaticBar1d(result) => result,
                _ => unreachable!("solve.electrostatic_bar_1d returned unexpected result"),
            };
            serde_json::to_value(result).map_err(|err| err.to_string())
        }
        "solve.magnetostatic_bar_1d" => {
            let request: SolveMagnetostaticBar1dRequest =
                serde_json::from_value(payload).map_err(|err| err.to_string())?;
            let result = match solve(EngineSolveRequest::MagnetostaticBar1d(request))? {
                AnalysisResult::MagnetostaticBar1d(result) => result,
                _ => unreachable!("solve.magnetostatic_bar_1d returned unexpected result"),
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
        "solve.heat_plane_triangle_2d" => {
            let request: SolveHeatPlaneTriangle2dRequest =
                serde_json::from_value(payload).map_err(|err| err.to_string())?;
            let result = match solve(EngineSolveRequest::HeatPlaneTriangle2d(request))? {
                AnalysisResult::HeatPlaneTriangle2d(result) => result,
                _ => unreachable!("solve.heat_plane_triangle_2d returned unexpected result"),
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
        "solve.thermal_truss_2d" => {
            let request: SolveThermalTruss2dRequest =
                serde_json::from_value(payload).map_err(|err| err.to_string())?;
            let result = match solve(EngineSolveRequest::ThermalTruss2d(request))? {
                AnalysisResult::ThermalTruss2d(result) => result,
                _ => unreachable!("solve.thermal_truss_2d returned unexpected result"),
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
        "solve.plane_triangle_2d" => {
            let request: SolvePlaneTriangle2dRequest =
                serde_json::from_value(payload).map_err(|err| err.to_string())?;
            let result = match solve(EngineSolveRequest::PlaneTriangle2d(request))? {
                AnalysisResult::PlaneTriangle2d(result) => result,
                _ => unreachable!("solve.plane_triangle_2d returned unexpected result"),
            };
            serde_json::to_value(result).map_err(|err| err.to_string())
        }
        "solve.thermal_plane_triangle_2d" => {
            let request: SolveThermalPlaneTriangle2dRequest =
                serde_json::from_value(payload).map_err(|err| err.to_string())?;
            let result = match solve(EngineSolveRequest::ThermalPlaneTriangle2d(request))? {
                AnalysisResult::ThermalPlaneTriangle2d(result) => result,
                _ => unreachable!("solve.thermal_plane_triangle_2d returned unexpected result"),
            };
            serde_json::to_value(result).map_err(|err| err.to_string())
        }
        "solve.plane_quad_2d" => {
            let request: SolvePlaneQuad2dRequest =
                serde_json::from_value(payload).map_err(|err| err.to_string())?;
            let result = match solve(EngineSolveRequest::PlaneQuad2d(request))? {
                AnalysisResult::PlaneQuad2d(result) => result,
                _ => unreachable!("solve.plane_quad_2d returned unexpected result"),
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
        "solve.torsion_1d" => {
            let request: SolveTorsion1dRequest =
                serde_json::from_value(payload).map_err(|err| err.to_string())?;
            let result = match solve(EngineSolveRequest::Torsion1d(request))? {
                AnalysisResult::Torsion1d(result) => result,
                _ => unreachable!("solve.torsion_1d returned unexpected result"),
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
        "bridge.temperature_field_to_thermo_quad_2d"
        | "bridge.temperature_field_to_thermo_triangle_2d"
        | "bridge.electrostatic_field_to_heat_quad_2d"
        | "bridge.electrostatic_field_to_heat_triangle_2d"
        | "transform.first_available"
        | "transform.merge_summary_pair"
        | "transform.compare_summary_pair"
        | "transform.aggregate_summary_collection"
        | "transform.normalize_summary_fields"
        | "transform.select_best_summary"
        | "transform.evaluate_thermal_guard"
        | "transform.benchmark_coupled_heat_pair"
        | "transform.compose_diagnostics_bundle"
        | "transform.evaluate_diagnostics_bundle_guard"
        | "transform.select_focus_payload"
        | "transform.compose_focus_chain_input"
        | "transform.compose_focus_bridge_request"
        | "transform.resolve_focus_bridge_execution"
        | "transform.execute_focus_bridge_execution" => {
            run_registered_transform_operator(operator_id, payload, config)
        }
        "transform.compose_diagnostics_report_payload" => {
            run_registered_transform_operator(operator_id, payload, config)
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
    run_registered_extract_operator(operator_id, payload, config)
}

pub fn run_export_operator(
    operator_id: &str,
    payload: Value,
    config: Value,
) -> Result<Value, String> {
    run_registered_export_operator(operator_id, payload, config)
}
