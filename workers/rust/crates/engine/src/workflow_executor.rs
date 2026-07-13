use crate::{
    operator_sdk_runtime::{
        run_registered_export_operator, run_registered_extract_operator,
        run_registered_transform_operator,
    },
    workflow_solve_executor::SUPPORTED_SOLVE_OPERATORS,
};
use serde_json::Value;
use std::collections::BTreeMap;

pub use crate::workflow_solve_executor::{run_solve_operator, solve_operator_runtime_manifest};

const SUPPORTED_TRANSFORM_OPERATORS: &[&str] = &[
    "bridge.temperature_field_to_thermo_quad_2d",
    "bridge.temperature_field_to_thermo_triangle_2d",
    "bridge.electrostatic_field_to_heat_quad_2d",
    "bridge.electrostatic_field_to_heat_triangle_2d",
    "bridge.magnetostatic_field_to_heat_quad_2d",
    "transform.first_available",
    "transform.merge_summary_pair",
    "transform.compare_summary_pair",
    "transform.validate_summary_tolerance",
    "transform.aggregate_summary_collection",
    "transform.normalize_summary_fields",
    "transform.select_best_summary",
    "transform.compose_quality_objective",
    "transform.rank_quality_candidates",
    "transform.prepare_quality_next_round_request",
    "transform.compose_quality_lineage_report",
    "transform.build_quality_parameter_sweep_plan",
    "transform.materialize_quality_sweep_expansion",
    "transform.expand_parameter_sweep",
    "transform.summarize_parameter_sweep",
    "transform.join_parameter_sweep_results",
    "transform.score_parameter_sweep",
    "transform.map_parameter_sweep_scores_to_quality_candidates",
    "transform.compose_material_study_envelope",
    "transform.evaluate_material_margins",
    "transform.rank_material_candidates",
    "transform.extract_material_pareto_frontier",
    "transform.evaluate_acoustic_guard",
    "transform.benchmark_acoustic_pair",
    "transform.score_acoustic_quality",
    "transform.evaluate_modal_guard",
    "transform.benchmark_modal_pair",
    "transform.score_modal_quality",
    "transform.score_dynamic_quality",
    "transform.evaluate_structural_guard",
    "transform.benchmark_structural_pair",
    "transform.score_structural_quality",
    "transform.evaluate_thermal_guard",
    "transform.benchmark_coupled_heat_pair",
    "transform.score_thermal_quality",
    "transform.evaluate_electrostatic_guard",
    "transform.benchmark_electrostatic_pair",
    "transform.score_electrostatic_quality",
    "transform.evaluate_magnetostatic_guard",
    "transform.benchmark_magnetostatic_pair",
    "transform.score_magnetostatic_quality",
    "transform.evaluate_cfd_guard",
    "transform.benchmark_cfd_pair",
    "transform.score_cfd_quality",
    "transform.evaluate_transport_guard",
    "transform.benchmark_transport_pair",
    "transform.score_transport_quality",
    "transform.evaluate_coupled_readiness",
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
    "extract.magnetostatic_result_diagnostics",
    "extract.magnetostatic_peak_field",
    "extract.transport_result_diagnostics",
    "extract.stokes_flow_result_diagnostics",
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
        || operator_id == "transform.validate_summary_tolerance"
        || operator_id == "transform.aggregate_summary_collection"
        || operator_id == "transform.select_best_summary"
        || operator_id == "transform.compose_quality_objective"
        || operator_id == "transform.compose_quality_lineage_report"
        || operator_id == "transform.join_parameter_sweep_results"
        || operator_id == "transform.evaluate_coupled_readiness"
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

pub fn run_transform_operator(
    operator_id: &str,
    payload: Value,
    config: Value,
) -> Result<Value, String> {
    if operator_id == "transform.first_available" {
        return Ok(payload);
    }

    match operator_id {
        "bridge.temperature_field_to_thermo_quad_2d"
        | "bridge.temperature_field_to_thermo_triangle_2d"
        | "bridge.electrostatic_field_to_heat_quad_2d"
        | "bridge.electrostatic_field_to_heat_triangle_2d"
        | "bridge.magnetostatic_field_to_heat_quad_2d"
        | "transform.merge_summary_pair"
        | "transform.compare_summary_pair"
        | "transform.validate_summary_tolerance"
        | "transform.aggregate_summary_collection"
        | "transform.normalize_summary_fields"
        | "transform.select_best_summary"
        | "transform.compose_quality_objective"
        | "transform.rank_quality_candidates"
        | "transform.prepare_quality_next_round_request"
        | "transform.compose_quality_lineage_report"
        | "transform.build_quality_parameter_sweep_plan"
        | "transform.materialize_quality_sweep_expansion"
        | "transform.expand_parameter_sweep"
        | "transform.summarize_parameter_sweep"
        | "transform.join_parameter_sweep_results"
        | "transform.score_parameter_sweep"
        | "transform.map_parameter_sweep_scores_to_quality_candidates"
        | "transform.compose_material_study_envelope"
        | "transform.evaluate_material_margins"
        | "transform.rank_material_candidates"
        | "transform.extract_material_pareto_frontier"
        | "transform.evaluate_acoustic_guard"
        | "transform.benchmark_acoustic_pair"
        | "transform.score_acoustic_quality"
        | "transform.evaluate_modal_guard"
        | "transform.benchmark_modal_pair"
        | "transform.score_modal_quality"
        | "transform.score_dynamic_quality"
        | "transform.evaluate_structural_guard"
        | "transform.benchmark_structural_pair"
        | "transform.score_structural_quality"
        | "transform.evaluate_thermal_guard"
        | "transform.benchmark_coupled_heat_pair"
        | "transform.score_thermal_quality"
        | "transform.evaluate_electrostatic_guard"
        | "transform.benchmark_electrostatic_pair"
        | "transform.score_electrostatic_quality"
        | "transform.evaluate_magnetostatic_guard"
        | "transform.benchmark_magnetostatic_pair"
        | "transform.score_magnetostatic_quality"
        | "transform.evaluate_cfd_guard"
        | "transform.benchmark_cfd_pair"
        | "transform.score_cfd_quality"
        | "transform.evaluate_transport_guard"
        | "transform.benchmark_transport_pair"
        | "transform.score_transport_quality"
        | "transform.evaluate_coupled_readiness"
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
