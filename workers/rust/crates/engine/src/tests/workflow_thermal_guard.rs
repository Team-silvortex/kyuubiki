use crate::{
    run_workflow_graph, thermal_quality::score_thermal_quality,
    workflow_executor::run_transform_operator, workflow_guard_transforms::evaluate_thermal_guard,
};
use kyuubiki_protocol::{
    WorkflowDefaults, WorkflowEdge, WorkflowGraph, WorkflowGraphRunRequest, WorkflowNode,
    WorkflowNodeKind, WorkflowNodePortRef, WorkflowPort,
};
use std::collections::BTreeMap;

fn approx_eq(left: Option<f64>, right: f64) {
    let value = left.expect("expected numeric quality value");
    assert!(
        (value - right).abs() < 1.0e-9,
        "left={value}, right={right}"
    );
}

#[test]
fn evaluates_thermal_guard_as_pass() {
    let guard = evaluate_thermal_guard(
        serde_json::json!({
            "thermal_temperature_max": 80.0,
            "thermal_flux_peak_magnitude": 12.0
        }),
        serde_json::json!({
            "rules": [
                { "field": "thermal_temperature_max", "comparison": "gt", "threshold": 100.0, "severity": "warn", "label": "peak_temp" },
                { "field": "thermal_flux_peak_magnitude", "comparison": "gt", "threshold": 20.0, "severity": "block", "label": "peak_flux" }
            ]
        }),
    )
    .expect("guard should evaluate");

    assert_eq!(guard["guard_status"].as_str(), Some("pass"));
    assert_eq!(guard["guard_passed"].as_bool(), Some(true));
    assert_eq!(guard["guard_trigger_count"].as_u64(), Some(0));
    assert_eq!(
        guard["guard_summary"].as_str(),
        Some("All thermal guard rules passed.")
    );
}

#[test]
fn evaluates_thermal_guard_as_warn_and_block() {
    let guard = evaluate_thermal_guard(
        serde_json::json!({
            "thermo_temperature_delta_max": 135.0,
            "thermo_stress_peak": 260.0
        }),
        serde_json::json!({
            "rules": [
                { "field": "thermo_temperature_delta_max", "comparison": "gte", "threshold": 120.0, "severity": "warn", "label": "delta_max" },
                { "field": "thermo_stress_peak", "comparison": "gt", "value": 250.0, "severity": "block", "label": "stress_peak" }
            ]
        }),
    )
    .expect("guard should evaluate");

    assert_eq!(guard["guard_status"].as_str(), Some("block"));
    assert_eq!(guard["guard_passed"].as_bool(), Some(false));
    assert_eq!(guard["guard_warn_count"].as_u64(), Some(1));
    assert_eq!(guard["guard_block_count"].as_u64(), Some(1));
    assert_eq!(guard["guard_trigger_count"].as_u64(), Some(2));
    assert_eq!(
        guard["guard_recommendation"].as_str(),
        Some("hold_and_review")
    );
    let triggers = guard["guard_triggers"]
        .as_array()
        .expect("guard triggers should be an array");
    assert_eq!(triggers.len(), 2);
    assert_eq!(triggers[0]["label"].as_str(), Some("delta_max"));
    assert_eq!(triggers[1]["severity"].as_str(), Some("block"));
}

#[test]
fn runs_thermal_guard_through_transform_executor() {
    let guard = run_transform_operator(
        "transform.evaluate_thermal_guard",
        serde_json::json!({
            "thermal_temperature_max": 140.0,
            "thermal_flux_peak_magnitude": 9.0
        }),
        serde_json::json!({
            "rules": [
                { "field": "thermal_temperature_max", "comparison": "gte", "threshold": 120.0, "severity": "warn", "label": "temperature_limit" },
                { "field": "thermal_flux_peak_magnitude", "comparison": "gt", "threshold": 20.0, "severity": "block", "label": "flux_limit" }
            ]
        }),
    )
    .expect("workflow transform should evaluate");

    assert_eq!(guard["guard_status"].as_str(), Some("warn"));
    assert_eq!(guard["guard_warn_count"].as_u64(), Some(1));
    assert_eq!(guard["guard_block_count"].as_u64(), Some(0));
    assert_eq!(
        guard["guard_recommendation"].as_str(),
        Some("review_before_continue")
    );
}

#[test]
fn scores_thermal_quality_with_temperature_flux_and_stress_terms() {
    let quality = score_thermal_quality(
        serde_json::json!({
            "thermal_temperature_max": 80.0,
            "thermo_temperature_delta_max": 60.0,
            "thermal_flux_peak_magnitude": 10.0,
            "thermo_stress_peak": 150.0
        }),
        serde_json::json!({
            "targets": {
                "thermal_temperature_max": 120.0,
                "thermo_temperature_delta_max": 80.0,
                "thermal_flux_peak_magnitude": 20.0,
                "thermo_stress_peak": 250.0
            },
            "max_ready_score": 8.0
        }),
    )
    .expect("thermal quality should score");

    assert_eq!(
        quality["thermal_quality_contract"].as_str(),
        Some("kyuubiki.thermal_quality_score/v1")
    );
    assert_eq!(quality["thermal_quality_ready"].as_bool(), Some(true));
    assert_eq!(quality["thermal_quality_grade"].as_str(), Some("good"));
    assert_eq!(
        quality["thermal_quality_missing_metric_count"].as_u64(),
        Some(0)
    );
    assert_eq!(quality["thermal_quality_watch_count"].as_u64(), Some(0));
    assert_eq!(
        quality["thermal_quality_dominant_term"]["field"].as_str(),
        Some("thermal_temperature_max")
    );
    assert_eq!(
        quality["thermal_quality_blocking_terms"]
            .as_array()
            .map(Vec::len),
        Some(0)
    );
    approx_eq(quality["thermal_quality_score"].as_f64(), 5.1);
}

#[test]
fn scores_thermal_quality_from_solver_result_aliases() {
    let quality = score_thermal_quality(
        serde_json::json!({
            "max_temperature": 95.0,
            "max_heat_flux": 16.0,
            "total_thermal_energy": 1250.0
        }),
        serde_json::json!({
            "enabled_terms": [
                "thermal_temperature_max",
                "thermal_flux_peak_magnitude",
                "thermal_total_energy"
            ],
            "targets": {
                "thermal_temperature_max": 120.0,
                "thermal_flux_peak_magnitude": 20.0,
                "thermal_total_energy": 2000.0
            },
            "max_ready_score": 8.0
        }),
    )
    .expect("thermal solver aliases should score");

    assert_eq!(quality["thermal_quality_ready"].as_bool(), Some(true));
    assert_eq!(quality["thermal_quality_term_count"].as_u64(), Some(3));
    approx_eq(quality["thermal_quality_max_temperature"].as_f64(), 95.0);
    approx_eq(
        quality["thermal_quality_peak_flux_magnitude"].as_f64(),
        16.0,
    );
    approx_eq(quality["thermal_quality_total_energy"].as_f64(), 1250.0);
}

#[test]
fn blocks_thermal_quality_when_required_metrics_are_missing() {
    let quality = score_thermal_quality(
        serde_json::json!({
            "thermal_temperature_max": 80.0
        }),
        serde_json::json!({}),
    )
    .expect("thermal quality should report missing metrics");

    assert_eq!(quality["thermal_quality_ready"].as_bool(), Some(false));
    assert_eq!(quality["thermal_quality_grade"].as_str(), Some("block"));
    assert_eq!(
        quality["thermal_quality_missing_metric_count"].as_u64(),
        Some(3)
    );
    assert_eq!(quality["thermal_quality_watch_count"].as_u64(), Some(0));
    assert_eq!(
        quality["thermal_quality_blocking_terms"]
            .as_array()
            .map(Vec::len),
        Some(3)
    );
}

#[test]
fn runs_thermal_quality_through_transform_executor() {
    let quality = run_transform_operator(
        "transform.score_thermal_quality",
        serde_json::json!({
            "thermal_temperature_max": 30.0,
            "thermo_temperature_delta_max": 10.0,
            "thermal_flux_peak_magnitude": 2.0,
            "thermo_stress_peak": 25.0
        }),
        serde_json::json!({
            "max_ready_score": 8.0
        }),
    )
    .expect("thermal quality should run through executor");

    assert_eq!(quality["thermal_quality_ready"].as_bool(), Some(true));
    assert_eq!(quality["thermal_quality_grade"].as_str(), Some("excellent"));
    assert_eq!(quality["thermal_quality_term_count"].as_u64(), Some(4));
    assert!(
        quality["thermal_quality_summary"]
            .as_str()
            .is_some_and(|summary| summary.contains("watch=0"))
    );
}

#[test]
fn runs_transient_heat_quality_workflow_graph() {
    let graph = WorkflowGraph {
        schema_version: "kyuubiki.workflow-graph/v1".to_string(),
        id: "workflow.transient-heat-quality".to_string(),
        name: "Transient heat quality".to_string(),
        version: "1.0.0".to_string(),
        description: Some("Solve transient heat conduction and score thermal quality.".to_string()),
        dataset_contract: None,
        entry_nodes: vec!["model_input".to_string()],
        output_nodes: vec!["json_output".to_string()],
        defaults: WorkflowDefaults::default(),
        nodes: vec![
            input_node("model_input", "study_model/transient_heat_bar_1d"),
            WorkflowNode {
                id: "solve_heat".to_string(),
                kind: WorkflowNodeKind::Solve,
                operator_id: Some("solve.transient_heat_bar_1d".to_string()),
                name: Some("Solve transient heat".to_string()),
                description: None,
                config: None,
                cache_policy: None,
                inputs: vec![port("model", "study_model/transient_heat_bar_1d")],
                outputs: vec![port("result", "result/transient_heat_bar_1d")],
            },
            WorkflowNode {
                id: "score_thermal".to_string(),
                kind: WorkflowNodeKind::Transform,
                operator_id: Some("transform.score_thermal_quality".to_string()),
                name: Some("Score thermal quality".to_string()),
                description: None,
                config: Some(serde_json::json!({
                    "enabled_terms": [
                        "thermal_temperature_max",
                        "thermal_flux_peak_magnitude",
                        "thermal_total_energy"
                    ],
                    "targets": {
                        "thermal_temperature_max": 150.0,
                        "thermal_flux_peak_magnitude": 5000.0,
                        "thermal_total_energy": 100000.0
                    },
                    "max_ready_score": 8.0
                })),
                cache_policy: None,
                inputs: vec![port("payload", "result/transient_heat_bar_1d")],
                outputs: vec![port("summary", "artifact/result_summary")],
            },
            WorkflowNode {
                id: "export_json".to_string(),
                kind: WorkflowNodeKind::Export,
                operator_id: Some("export.summary_json".to_string()),
                name: Some("Export thermal quality".to_string()),
                description: None,
                config: None,
                cache_policy: None,
                inputs: vec![port("summary", "artifact/result_summary")],
                outputs: vec![port("json", "artifact/json")],
            },
            WorkflowNode {
                id: "json_output".to_string(),
                kind: WorkflowNodeKind::Output,
                operator_id: None,
                name: Some("JSON output".to_string()),
                description: None,
                config: None,
                cache_policy: None,
                inputs: vec![port("json", "artifact/json")],
                outputs: vec![],
            },
        ],
        edges: vec![
            edge(
                "edge-input-solve",
                "model_input",
                "model",
                "solve_heat",
                "model",
                "study_model/transient_heat_bar_1d",
            ),
            edge(
                "edge-solve-score",
                "solve_heat",
                "result",
                "score_thermal",
                "payload",
                "result/transient_heat_bar_1d",
            ),
            edge(
                "edge-score-export",
                "score_thermal",
                "summary",
                "export_json",
                "summary",
                "artifact/result_summary",
            ),
            edge(
                "edge-export-output",
                "export_json",
                "json",
                "json_output",
                "json",
                "artifact/json",
            ),
        ],
    };

    let run = run_workflow_graph(WorkflowGraphRunRequest {
        graph,
        input_artifacts: BTreeMap::from([(
            "model_input".to_string(),
            serde_json::json!({
                "nodes": [
                    { "id": "hot", "x": 0.0, "fix_temperature": true, "temperature": 100.0, "heat_load": 0.0 },
                    { "id": "mid", "x": 0.5, "fix_temperature": false, "temperature": 20.0, "heat_load": 5.0 },
                    { "id": "cold", "x": 1.0, "fix_temperature": true, "temperature": 20.0, "heat_load": 0.0 }
                ],
                "elements": [
                    { "id": "h0", "node_i": 0, "node_j": 1, "area": 1.0, "conductivity": 15.0, "density": 1.0, "specific_heat": 20.0 },
                    { "id": "h1", "node_i": 1, "node_j": 2, "area": 1.0, "conductivity": 15.0, "density": 1.0, "specific_heat": 20.0 }
                ],
                "time_step": 0.05,
                "steps": 4
            }),
        )]),
    })
    .expect("transient heat quality workflow should run");

    let quality = run
        .artifacts
        .get("score_thermal.summary")
        .expect("thermal quality summary should exist");
    assert_eq!(
        quality["thermal_quality_contract"].as_str(),
        Some("kyuubiki.thermal_quality_score/v1")
    );
    assert_eq!(quality["thermal_quality_ready"].as_bool(), Some(true));
    assert_eq!(quality["thermal_quality_term_count"].as_u64(), Some(3));
    assert!(
        quality["thermal_quality_total_energy"]
            .as_f64()
            .unwrap_or_default()
            > 0.0
    );

    let exported = run
        .artifacts
        .get("json_output.json")
        .expect("json export artifact should exist");
    let content = exported["content"]
        .as_str()
        .expect("json content should be a string");
    assert!(content.contains("thermal_quality_total_energy"));
}

fn input_node(id: &str, artifact_type: &str) -> WorkflowNode {
    WorkflowNode {
        id: id.to_string(),
        kind: WorkflowNodeKind::Input,
        operator_id: None,
        name: Some("Model input".to_string()),
        description: None,
        config: None,
        cache_policy: None,
        inputs: vec![],
        outputs: vec![port("model", artifact_type)],
    }
}

fn port(id: &str, artifact_type: &str) -> WorkflowPort {
    WorkflowPort {
        id: id.to_string(),
        artifact_type: artifact_type.to_string(),
        name: None,
        required: None,
        cardinality: None,
        dataset_value: Some(artifact_type.replace('/', "_")),
    }
}

fn edge(
    id: &str,
    from_node: &str,
    from_port: &str,
    to_node: &str,
    to_port: &str,
    artifact_type: &str,
) -> WorkflowEdge {
    WorkflowEdge {
        id: id.to_string(),
        from: WorkflowNodePortRef {
            node: from_node.to_string(),
            port: from_port.to_string(),
        },
        to: WorkflowNodePortRef {
            node: to_node.to_string(),
            port: to_port.to_string(),
        },
        artifact_type: artifact_type.to_string(),
        dataset_value: None,
    }
}
