use crate::{
    magnetostatic_quality::score_magnetostatic_quality,
    run_workflow_graph,
    workflow_executor::run_transform_operator,
    workflow_guard_transforms::{benchmark_magnetostatic_pair, evaluate_magnetostatic_guard},
};
use kyuubiki_protocol::{
    WorkflowDefaults, WorkflowEdge, WorkflowGraph, WorkflowGraphRunRequest, WorkflowNode,
    WorkflowNodeKind, WorkflowNodePortRef, WorkflowPort,
};
use std::collections::BTreeMap;

fn approx_eq(left: Option<f64>, right: f64) {
    let value = left.expect("expected numeric benchmark value");
    assert!(
        (value - right).abs() < 1.0e-9,
        "left={value}, right={right}"
    );
}

#[test]
fn evaluates_magnetostatic_guard_as_block() {
    let guard = evaluate_magnetostatic_guard(
        serde_json::json!({
            "magnetostatic_field_peak_magnitude": 13.0,
            "total_stored_energy": 9.5
        }),
        serde_json::json!({
            "rules": [
                { "field": "magnetostatic_field_peak_magnitude", "threshold": 12.0, "severity": "block", "label": "h_peak" },
                { "field": "total_stored_energy", "threshold": 20.0, "severity": "warn" }
            ]
        }),
    )
    .expect("magnetostatic guard should succeed");

    assert_eq!(guard["guard_status"].as_str(), Some("block"));
    assert_eq!(guard["guard_block_count"].as_u64(), Some(1));
    assert_eq!(guard["guard_trigger_count"].as_u64(), Some(1));
}

#[test]
fn benchmarks_magnetostatic_pair_by_field_and_energy() {
    let benchmark = benchmark_magnetostatic_pair(
        serde_json::json!({
            "left": {
                "magnetostatic_field_peak_magnitude": 11.0,
                "total_stored_energy": 7.0
            },
            "right": {
                "magnetostatic_field_peak_magnitude": 13.0,
                "total_stored_energy": 9.5
            }
        }),
        serde_json::json!({
            "left_label": "candidate_a",
            "right_label": "candidate_b",
            "criteria": [
                { "field": "magnetostatic_field_peak_magnitude", "goal": "min", "weight": 2.0 },
                { "field": "total_stored_energy", "goal": "min", "weight": 1.0 }
            ]
        }),
    )
    .expect("magnetostatic benchmark should succeed");

    assert_eq!(benchmark["benchmark_winner"].as_str(), Some("candidate_a"));
    approx_eq(benchmark["candidate_a_score"].as_f64(), 3.0);
    approx_eq(benchmark["candidate_b_score"].as_f64(), 0.0);
}

#[test]
fn scores_magnetostatic_quality_with_field_flux_energy_and_current_terms() {
    let quality = score_magnetostatic_quality(
        serde_json::json!({
            "magnetostatic_field_peak_magnitude": 9.0,
            "magnetostatic_flux_peak_magnitude": 12.0,
            "magnetostatic_energy_density_peak": 4.0,
            "magnetostatic_current_density_sum": 5.0
        }),
        serde_json::json!({
            "targets": {
                "magnetostatic_field_peak_magnitude": 12.0,
                "magnetostatic_flux_peak_magnitude": 16.0,
                "magnetostatic_energy_density_peak": 8.0,
                "magnetostatic_current_density_sum": 10.0
            },
            "max_ready_score": 8.0
        }),
    )
    .expect("magnetostatic quality should score");

    assert_eq!(
        quality["magnetostatic_quality_contract"].as_str(),
        Some("kyuubiki.magnetostatic_quality_score/v1")
    );
    assert_eq!(quality["magnetostatic_quality_ready"].as_bool(), Some(true));
    assert_eq!(
        quality["magnetostatic_quality_grade"].as_str(),
        Some("good")
    );
    assert_eq!(
        quality["magnetostatic_quality_missing_metric_count"].as_u64(),
        Some(0)
    );
    approx_eq(quality["magnetostatic_quality_score"].as_f64(), 5.25);
}

#[test]
fn scores_magnetostatic_quality_from_solver_result_aliases() {
    let quality = score_magnetostatic_quality(
        serde_json::json!({
            "max_magnetic_field_strength": 9.0,
            "max_flux_density": 12.0,
            "total_stored_energy": 2.5
        }),
        serde_json::json!({
            "enabled_terms": [
                "magnetostatic_field_peak_magnitude",
                "magnetostatic_flux_peak_magnitude",
                "magnetostatic_total_stored_energy"
            ],
            "targets": {
                "magnetostatic_field_peak_magnitude": 12.0,
                "magnetostatic_flux_peak_magnitude": 16.0,
                "magnetostatic_total_stored_energy": 5.0
            },
            "max_ready_score": 8.0
        }),
    )
    .expect("magnetostatic solver aliases should score");

    assert_eq!(quality["magnetostatic_quality_ready"].as_bool(), Some(true));
    assert_eq!(
        quality["magnetostatic_quality_term_count"].as_u64(),
        Some(3)
    );
    approx_eq(quality["magnetostatic_quality_peak_field"].as_f64(), 9.0);
    approx_eq(quality["magnetostatic_quality_peak_flux"].as_f64(), 12.0);
    approx_eq(quality["magnetostatic_quality_total_energy"].as_f64(), 2.5);
}

#[test]
fn blocks_magnetostatic_quality_when_required_metrics_are_missing() {
    let quality = score_magnetostatic_quality(
        serde_json::json!({
            "magnetostatic_field_peak_magnitude": 8.0
        }),
        serde_json::json!({}),
    )
    .expect("magnetostatic quality should report missing metrics");

    assert_eq!(
        quality["magnetostatic_quality_ready"].as_bool(),
        Some(false)
    );
    assert_eq!(
        quality["magnetostatic_quality_grade"].as_str(),
        Some("block")
    );
    assert_eq!(
        quality["magnetostatic_quality_missing_metric_count"].as_u64(),
        Some(3)
    );
}

#[test]
fn runs_magnetostatic_quality_through_transform_executor() {
    let quality = run_transform_operator(
        "transform.score_magnetostatic_quality",
        serde_json::json!({
            "magnetostatic_field_peak_magnitude": 2.0,
            "magnetostatic_flux_peak_magnitude": 3.0,
            "magnetostatic_energy_density_peak": 1.0,
            "magnetostatic_current_density_sum": 1.5
        }),
        serde_json::json!({
            "max_ready_score": 8.0
        }),
    )
    .expect("magnetostatic quality should run through executor");

    assert_eq!(quality["magnetostatic_quality_ready"].as_bool(), Some(true));
    assert_eq!(
        quality["magnetostatic_quality_grade"].as_str(),
        Some("excellent")
    );
    assert_eq!(
        quality["magnetostatic_quality_term_count"].as_u64(),
        Some(4)
    );
}

#[test]
fn runs_magnetostatic_guard_and_benchmark_through_transform_executor() {
    let guard = run_transform_operator(
        "transform.evaluate_magnetostatic_guard",
        serde_json::json!({
            "magnetostatic_field_peak_magnitude": 8.0
        }),
        serde_json::json!({
            "rules": [
                { "field": "magnetostatic_field_peak_magnitude", "threshold": 12.0, "severity": "warn" }
            ]
        }),
    )
    .expect("magnetostatic guard should run through executor");

    assert_eq!(guard["guard_status"].as_str(), Some("pass"));
    assert_eq!(guard["guard_passed"].as_bool(), Some(true));

    let benchmark = run_transform_operator(
        "transform.benchmark_magnetostatic_pair",
        serde_json::json!({
            "left": { "magnetostatic_field_peak_magnitude": 8.0 },
            "right": { "magnetostatic_field_peak_magnitude": 10.0 }
        }),
        serde_json::json!({
            "criteria": [
                { "field": "magnetostatic_field_peak_magnitude", "goal": "min", "weight": 2.0 }
            ]
        }),
    )
    .expect("magnetostatic benchmark should run through executor");

    approx_eq(benchmark["left_score"].as_f64(), 2.0);
    approx_eq(benchmark["right_score"].as_f64(), 0.0);
    assert_eq!(benchmark["benchmark_winner"].as_str(), Some("left"));
}

#[test]
fn runs_magnetostatic_bar_quality_workflow_graph() {
    let run = run_workflow_graph(WorkflowGraphRunRequest {
        graph: quality_graph(),
        input_artifacts: BTreeMap::from([(
            "model_input".to_string(),
            serde_json::json!({
                "nodes": [
                    { "id": "ground", "x": 0.0, "fix_magnetic_potential": true, "magnetic_potential": 0.0, "magnetomotive_source": 0.0 },
                    { "id": "pole", "x": 1.0, "fix_magnetic_potential": true, "magnetic_potential": 10.0, "magnetomotive_source": 0.0 }
                ],
                "elements": [
                    { "id": "m0", "node_i": 0, "node_j": 1, "area": 1.0, "permeability": 1.0 }
                ]
            }),
        )]),
    })
    .expect("magnetostatic quality workflow should run");

    let quality = run
        .artifacts
        .get("score_quality.summary")
        .expect("quality summary should exist");
    assert_eq!(
        quality["magnetostatic_quality_contract"].as_str(),
        Some("kyuubiki.magnetostatic_quality_score/v1")
    );
    assert_eq!(quality["magnetostatic_quality_ready"].as_bool(), Some(true));
    assert!(
        quality["magnetostatic_quality_total_energy"]
            .as_f64()
            .unwrap_or_default()
            > 0.0
    );

    let exported = run
        .artifacts
        .get("json_output.json")
        .expect("json export artifact should exist");
    assert!(
        exported["content"]
            .as_str()
            .unwrap_or_default()
            .contains("magnetostatic_quality_total_energy")
    );
}

fn quality_graph() -> WorkflowGraph {
    WorkflowGraph {
        schema_version: "kyuubiki.workflow-graph/v1".to_string(),
        id: "workflow.magnetostatic-bar-quality".to_string(),
        name: "Magnetostatic quality".to_string(),
        version: "1.0.0".to_string(),
        description: None,
        dataset_contract: None,
        entry_nodes: vec!["model_input".to_string()],
        output_nodes: vec!["json_output".to_string()],
        defaults: WorkflowDefaults::default(),
        nodes: vec![
            node(
                "model_input",
                WorkflowNodeKind::Input,
                None,
                vec![],
                vec![port("model", "study_model/magnetostatic_bar_1d")],
                None,
            ),
            node(
                "solve",
                WorkflowNodeKind::Solve,
                Some("solve.magnetostatic_bar_1d"),
                vec![port("model", "study_model/magnetostatic_bar_1d")],
                vec![port("result", "result/magnetostatic_bar_1d")],
                None,
            ),
            node(
                "score_quality",
                WorkflowNodeKind::Transform,
                Some("transform.score_magnetostatic_quality"),
                vec![port("payload", "result/magnetostatic_bar_1d")],
                vec![port("summary", "artifact/result_summary")],
                Some(serde_json::json!({
                    "enabled_terms": [
                        "magnetostatic_field_peak_magnitude",
                        "magnetostatic_flux_peak_magnitude",
                        "magnetostatic_total_stored_energy"
                    ],
                    "targets": {
                        "magnetostatic_field_peak_magnitude": 20.0,
                        "magnetostatic_flux_peak_magnitude": 20.0,
                        "magnetostatic_total_stored_energy": 10.0
                    },
                    "max_ready_score": 8.0
                })),
            ),
            node(
                "export_json",
                WorkflowNodeKind::Export,
                Some("export.summary_json"),
                vec![port("summary", "artifact/result_summary")],
                vec![port("json", "artifact/json")],
                None,
            ),
            node(
                "json_output",
                WorkflowNodeKind::Output,
                None,
                vec![port("json", "artifact/json")],
                vec![],
                None,
            ),
        ],
        edges: vec![
            edge(
                "edge-input-solve",
                "model_input",
                "model",
                "solve",
                "model",
                "study_model/magnetostatic_bar_1d",
            ),
            edge(
                "edge-solve-score",
                "solve",
                "result",
                "score_quality",
                "payload",
                "result/magnetostatic_bar_1d",
            ),
            edge(
                "edge-score-export",
                "score_quality",
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
    }
}

fn node(
    id: &str,
    kind: WorkflowNodeKind,
    operator_id: Option<&str>,
    inputs: Vec<WorkflowPort>,
    outputs: Vec<WorkflowPort>,
    config: Option<serde_json::Value>,
) -> WorkflowNode {
    WorkflowNode {
        id: id.to_string(),
        kind,
        operator_id: operator_id.map(str::to_string),
        name: None,
        description: None,
        config,
        cache_policy: None,
        inputs,
        outputs,
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
