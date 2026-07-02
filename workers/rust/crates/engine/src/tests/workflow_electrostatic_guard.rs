use crate::{
    electrostatic_quality::score_electrostatic_quality,
    run_workflow_graph,
    workflow_executor::run_transform_operator,
    workflow_guard_transforms::{benchmark_electrostatic_pair, evaluate_electrostatic_guard},
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
fn evaluates_electrostatic_guard_as_warn() {
    let guard = evaluate_electrostatic_guard(
        serde_json::json!({
            "electrostatic_field_peak_magnitude": 12.5,
            "electrostatic_peak_energy_density": 0.42
        }),
        serde_json::json!({
            "rules": [
                { "field": "electrostatic_field_peak_magnitude", "comparison": "gt", "threshold": 10.0, "severity": "warn", "label": "field_limit" },
                { "field": "electrostatic_peak_energy_density", "comparison": "gt", "threshold": 0.8, "severity": "block", "label": "energy_limit" }
            ]
        }),
    )
    .expect("electrostatic guard should evaluate");

    assert_eq!(guard["guard_status"].as_str(), Some("warn"));
    assert_eq!(guard["guard_passed"].as_bool(), Some(false));
    assert_eq!(guard["guard_warn_count"].as_u64(), Some(1));
    assert_eq!(guard["guard_block_count"].as_u64(), Some(0));
    assert_eq!(
        guard["guard_recommendation"].as_str(),
        Some("review_before_continue")
    );
}

#[test]
fn benchmarks_electrostatic_pair_by_field_and_energy() {
    let benchmark = benchmark_electrostatic_pair(
        serde_json::json!({
            "left": {
                "electrostatic_field_peak_magnitude": 8.0,
                "electrostatic_peak_energy_density": 0.31,
                "electrostatic_potential_span": 4.4
            },
            "right": {
                "electrostatic_field_peak_magnitude": 9.5,
                "electrostatic_peak_energy_density": 0.27,
                "electrostatic_potential_span": 5.1
            }
        }),
        serde_json::json!({
            "left_label": "insulated_candidate",
            "right_label": "high_gradient_candidate",
            "criteria": [
                { "field": "electrostatic_field_peak_magnitude", "goal": "min", "weight": 2.0 },
                { "field": "electrostatic_peak_energy_density", "goal": "min", "weight": 1.0 },
                { "field": "electrostatic_potential_span", "goal": "max", "weight": 1.0 }
            ]
        }),
    )
    .expect("electrostatic benchmark should succeed");

    approx_eq(benchmark["insulated_candidate_score"].as_f64(), 2.0);
    approx_eq(benchmark["high_gradient_candidate_score"].as_f64(), 2.0);
    assert_eq!(benchmark["benchmark_winner"].as_str(), Some("tie"));
    assert_eq!(benchmark["benchmark_left_win_count"].as_u64(), Some(1));
    assert_eq!(benchmark["benchmark_right_win_count"].as_u64(), Some(2));
}

#[test]
fn runs_electrostatic_guard_and_benchmark_through_transform_executor() {
    let guard = run_transform_operator(
        "transform.evaluate_electrostatic_guard",
        serde_json::json!({
            "electrostatic_field_peak_magnitude": 7.2
        }),
        serde_json::json!({
            "rules": [
                { "field": "electrostatic_field_peak_magnitude", "comparison": "gt", "threshold": 10.0, "severity": "warn" }
            ]
        }),
    )
    .expect("electrostatic guard should run through executor");

    assert_eq!(guard["guard_status"].as_str(), Some("pass"));
    assert_eq!(guard["guard_passed"].as_bool(), Some(true));

    let benchmark = run_transform_operator(
        "transform.benchmark_electrostatic_pair",
        serde_json::json!({
            "left": { "electrostatic_field_peak_magnitude": 8.0 },
            "right": { "electrostatic_field_peak_magnitude": 11.0 }
        }),
        serde_json::json!({
            "criteria": [
                { "field": "electrostatic_field_peak_magnitude", "goal": "min", "weight": 2.0 }
            ]
        }),
    )
    .expect("electrostatic benchmark should run through executor");

    approx_eq(benchmark["left_score"].as_f64(), 2.0);
    approx_eq(benchmark["right_score"].as_f64(), 0.0);
    assert_eq!(benchmark["benchmark_winner"].as_str(), Some("left"));
}

#[test]
fn scores_electrostatic_quality_with_field_energy_and_potential_terms() {
    let quality = score_electrostatic_quality(
        serde_json::json!({
            "electrostatic_field_peak_magnitude": 8.0,
            "electrostatic_peak_energy_density": 0.4,
            "electrostatic_potential_span": 5.0
        }),
        serde_json::json!({
            "targets": {
                "electrostatic_field_peak_magnitude": 10.0,
                "electrostatic_peak_energy_density": 0.8,
                "electrostatic_potential_span": 4.0
            },
            "max_ready_score": 8.0
        }),
    )
    .expect("electrostatic quality should score");

    assert_eq!(
        quality["electrostatic_quality_contract"].as_str(),
        Some("kyuubiki.electrostatic_quality_score/v1")
    );
    assert_eq!(quality["electrostatic_quality_ready"].as_bool(), Some(true));
    assert_eq!(
        quality["electrostatic_quality_grade"].as_str(),
        Some("good")
    );
    assert_eq!(
        quality["electrostatic_quality_missing_metric_count"].as_u64(),
        Some(0)
    );
    approx_eq(quality["electrostatic_quality_score"].as_f64(), 5.0);
}

#[test]
fn scores_electrostatic_quality_from_solver_result_aliases() {
    let quality = score_electrostatic_quality(
        serde_json::json!({
            "max_electric_field": 8.0,
            "max_potential": 10.0,
            "total_stored_energy": 2.5
        }),
        serde_json::json!({
            "enabled_terms": [
                "electrostatic_field_peak_magnitude",
                "electrostatic_potential_span",
                "electrostatic_total_stored_energy"
            ],
            "targets": {
                "electrostatic_field_peak_magnitude": 10.0,
                "electrostatic_potential_span": 4.0,
                "electrostatic_total_stored_energy": 5.0
            },
            "max_ready_score": 8.0
        }),
    )
    .expect("electrostatic solver aliases should score");

    assert_eq!(quality["electrostatic_quality_ready"].as_bool(), Some(true));
    assert_eq!(
        quality["electrostatic_quality_term_count"].as_u64(),
        Some(3)
    );
    approx_eq(quality["electrostatic_quality_peak_field"].as_f64(), 8.0);
    approx_eq(quality["electrostatic_quality_total_energy"].as_f64(), 2.5);
}

#[test]
fn blocks_electrostatic_quality_when_required_metrics_are_missing() {
    let quality = score_electrostatic_quality(
        serde_json::json!({
            "electrostatic_field_peak_magnitude": 8.0
        }),
        serde_json::json!({}),
    )
    .expect("electrostatic quality should report missing metrics");

    assert_eq!(
        quality["electrostatic_quality_ready"].as_bool(),
        Some(false)
    );
    assert_eq!(
        quality["electrostatic_quality_grade"].as_str(),
        Some("block")
    );
    assert_eq!(
        quality["electrostatic_quality_missing_metric_count"].as_u64(),
        Some(2)
    );
}

#[test]
fn runs_electrostatic_quality_through_transform_executor() {
    let quality = run_transform_operator(
        "transform.score_electrostatic_quality",
        serde_json::json!({
            "electrostatic_field_peak_magnitude": 2.0,
            "electrostatic_peak_energy_density": 0.08,
            "electrostatic_potential_span": 10.0
        }),
        serde_json::json!({
            "max_ready_score": 8.0
        }),
    )
    .expect("electrostatic quality should run through executor");

    assert_eq!(quality["electrostatic_quality_ready"].as_bool(), Some(true));
    assert_eq!(
        quality["electrostatic_quality_grade"].as_str(),
        Some("excellent")
    );
    assert_eq!(
        quality["electrostatic_quality_term_count"].as_u64(),
        Some(3)
    );
}

#[test]
fn runs_electrostatic_bar_quality_workflow_graph() {
    let run = run_workflow_graph(WorkflowGraphRunRequest {
        graph: quality_graph(
            "workflow.electrostatic-bar-quality",
            "study_model/electrostatic_bar_1d",
            "solve.electrostatic_bar_1d",
            "result/electrostatic_bar_1d",
            "transform.score_electrostatic_quality",
            serde_json::json!({
                "enabled_terms": [
                    "electrostatic_field_peak_magnitude",
                    "electrostatic_potential_span",
                    "electrostatic_total_stored_energy"
                ],
                "targets": {
                    "electrostatic_field_peak_magnitude": 20.0,
                    "electrostatic_potential_span": 5.0,
                    "electrostatic_total_stored_energy": 10.0
                },
                "max_ready_score": 8.0
            }),
        ),
        input_artifacts: BTreeMap::from([(
            "model_input".to_string(),
            serde_json::json!({
                "nodes": [
                    { "id": "ground", "x": 0.0, "fix_potential": true, "potential": 0.0, "charge_density": 0.0 },
                    { "id": "plate", "x": 1.0, "fix_potential": true, "potential": 10.0, "charge_density": 0.0 }
                ],
                "elements": [
                    { "id": "e0", "node_i": 0, "node_j": 1, "area": 1.0, "permittivity": 1.0 }
                ]
            }),
        )]),
    })
    .expect("electrostatic quality workflow should run");

    let quality = run
        .artifacts
        .get("score_quality.summary")
        .expect("quality summary should exist");
    assert_eq!(
        quality["electrostatic_quality_contract"].as_str(),
        Some("kyuubiki.electrostatic_quality_score/v1")
    );
    assert_eq!(quality["electrostatic_quality_ready"].as_bool(), Some(true));
    assert!(
        quality["electrostatic_quality_total_energy"]
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
            .contains("electrostatic_quality_total_energy")
    );
}

fn quality_graph(
    id: &str,
    model_type: &str,
    solve_operator: &str,
    result_type: &str,
    quality_operator: &str,
    quality_config: serde_json::Value,
) -> WorkflowGraph {
    WorkflowGraph {
        schema_version: "kyuubiki.workflow-graph/v1".to_string(),
        id: id.to_string(),
        name: "Field quality".to_string(),
        version: "1.0.0".to_string(),
        description: None,
        dataset_contract: None,
        entry_nodes: vec!["model_input".to_string()],
        output_nodes: vec!["json_output".to_string()],
        defaults: WorkflowDefaults::default(),
        nodes: vec![
            input_node("model_input", model_type),
            solve_node(solve_operator, model_type, result_type),
            quality_node(quality_operator, result_type, quality_config),
            export_node(),
            output_node(),
        ],
        edges: vec![
            edge(
                "edge-input-solve",
                "model_input",
                "model",
                "solve",
                "model",
                model_type,
            ),
            edge(
                "edge-solve-score",
                "solve",
                "result",
                "score_quality",
                "payload",
                result_type,
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

fn input_node(id: &str, artifact_type: &str) -> WorkflowNode {
    WorkflowNode {
        id: id.to_string(),
        kind: WorkflowNodeKind::Input,
        operator_id: None,
        name: None,
        description: None,
        config: None,
        cache_policy: None,
        inputs: vec![],
        outputs: vec![port("model", artifact_type)],
    }
}

fn solve_node(operator_id: &str, model_type: &str, result_type: &str) -> WorkflowNode {
    WorkflowNode {
        id: "solve".to_string(),
        kind: WorkflowNodeKind::Solve,
        operator_id: Some(operator_id.to_string()),
        name: None,
        description: None,
        config: None,
        cache_policy: None,
        inputs: vec![port("model", model_type)],
        outputs: vec![port("result", result_type)],
    }
}

fn quality_node(operator_id: &str, result_type: &str, config: serde_json::Value) -> WorkflowNode {
    WorkflowNode {
        id: "score_quality".to_string(),
        kind: WorkflowNodeKind::Transform,
        operator_id: Some(operator_id.to_string()),
        name: None,
        description: None,
        config: Some(config),
        cache_policy: None,
        inputs: vec![port("payload", result_type)],
        outputs: vec![port("summary", "artifact/result_summary")],
    }
}

fn export_node() -> WorkflowNode {
    WorkflowNode {
        id: "export_json".to_string(),
        kind: WorkflowNodeKind::Export,
        operator_id: Some("export.summary_json".to_string()),
        name: None,
        description: None,
        config: None,
        cache_policy: None,
        inputs: vec![port("summary", "artifact/result_summary")],
        outputs: vec![port("json", "artifact/json")],
    }
}

fn output_node() -> WorkflowNode {
    WorkflowNode {
        id: "json_output".to_string(),
        kind: WorkflowNodeKind::Output,
        operator_id: None,
        name: None,
        description: None,
        config: None,
        cache_policy: None,
        inputs: vec![port("json", "artifact/json")],
        outputs: vec![],
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
