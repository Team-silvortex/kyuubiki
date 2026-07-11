use crate::{
    run_workflow_graph,
    structural_quality::score_structural_quality,
    workflow_executor::run_transform_operator,
    workflow_guard_transforms::{benchmark_structural_pair, evaluate_structural_guard},
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
fn evaluates_structural_guard_with_contact_and_stress_rules() {
    let guard = evaluate_structural_guard(
        serde_json::json!({
            "max_displacement": 0.018,
            "max_stress": 265.0,
            "max_contact_force": 42.0,
            "active_contact_count": 1
        }),
        serde_json::json!({
            "rules": [
                { "field": "max_displacement", "comparison": "gt", "threshold": 0.02, "severity": "warn", "label": "serviceability" },
                { "field": "max_stress", "comparison": "gt", "threshold": 250.0, "severity": "block", "label": "stress_limit" },
                { "field": "max_contact_force", "comparison": "gt", "threshold": 50.0, "severity": "warn", "label": "contact_force" }
            ]
        }),
    )
    .expect("structural guard should evaluate");

    assert_eq!(guard["guard_status"].as_str(), Some("block"));
    assert_eq!(guard["guard_passed"].as_bool(), Some(false));
    assert_eq!(guard["guard_warn_count"].as_u64(), Some(0));
    assert_eq!(guard["guard_block_count"].as_u64(), Some(1));
    assert_eq!(guard["guard_trigger_count"].as_u64(), Some(1));
    assert_eq!(
        guard["guard_recommendation"].as_str(),
        Some("hold_and_review")
    );
}

#[test]
fn benchmarks_structural_pair_across_serviceability_and_mass() {
    let benchmark = benchmark_structural_pair(
        serde_json::json!({
            "left": {
                "max_displacement": 0.012,
                "max_stress": 180.0,
                "mass": 14.0,
                "stiffness_margin": 1.4
            },
            "right": {
                "max_displacement": 0.016,
                "max_stress": 165.0,
                "mass": 11.0,
                "stiffness_margin": 1.1
            }
        }),
        serde_json::json!({
            "left_label": "stiff_candidate",
            "right_label": "light_candidate",
            "criteria": [
                { "field": "max_displacement", "goal": "min", "weight": 2.0 },
                { "field": "max_stress", "goal": "min", "weight": 1.0 },
                { "field": "mass", "goal": "min", "weight": 2.0 },
                { "field": "stiffness_margin", "goal": "max", "weight": 1.0 }
            ]
        }),
    )
    .expect("structural benchmark should succeed");

    approx_eq(benchmark["stiff_candidate_score"].as_f64(), 3.0);
    approx_eq(benchmark["light_candidate_score"].as_f64(), 3.0);
    assert_eq!(benchmark["benchmark_winner"].as_str(), Some("tie"));
    assert_eq!(benchmark["benchmark_criteria_count"].as_u64(), Some(4));
    assert_eq!(benchmark["benchmark_left_win_count"].as_u64(), Some(2));
    assert_eq!(benchmark["benchmark_right_win_count"].as_u64(), Some(2));
}

#[test]
fn runs_structural_guard_and_benchmark_through_transform_executor() {
    let guard = run_transform_operator(
        "transform.evaluate_structural_guard",
        serde_json::json!({
            "max_displacement": 0.009,
            "max_stress": 120.0
        }),
        serde_json::json!({
            "rules": [
                { "field": "max_displacement", "comparison": "gt", "threshold": 0.02, "severity": "warn" },
                { "field": "max_stress", "comparison": "gt", "threshold": 250.0, "severity": "block" }
            ]
        }),
    )
    .expect("structural guard should run through executor");

    assert_eq!(guard["guard_status"].as_str(), Some("pass"));
    assert_eq!(guard["guard_passed"].as_bool(), Some(true));

    let benchmark = run_transform_operator(
        "transform.benchmark_structural_pair",
        serde_json::json!({
            "left": { "max_displacement": 0.008 },
            "right": { "max_displacement": 0.011 }
        }),
        serde_json::json!({
            "criteria": [
                { "field": "max_displacement", "goal": "min", "weight": 2.0 }
            ]
        }),
    )
    .expect("structural benchmark should run through executor");

    approx_eq(benchmark["left_score"].as_f64(), 2.0);
    approx_eq(benchmark["right_score"].as_f64(), 0.0);
    assert_eq!(benchmark["benchmark_winner"].as_str(), Some("left"));
}

#[test]
fn scores_structural_quality_with_serviceability_stress_and_mass_terms() {
    let quality = score_structural_quality(
        serde_json::json!({
            "max_displacement": 0.012,
            "max_stress": 180.0,
            "mass": 12.0,
            "stiffness_margin": 1.5
        }),
        serde_json::json!({
            "targets": {
                "max_displacement": 0.02,
                "max_stress": 250.0,
                "mass": 15.0,
                "stiffness_margin": 1.2
            },
            "max_ready_score": 8.0
        }),
    )
    .expect("structural quality should score");

    assert_eq!(
        quality["structural_quality_contract"].as_str(),
        Some("kyuubiki.structural_quality_score/v1")
    );
    assert_eq!(quality["structural_quality_ready"].as_bool(), Some(true));
    assert_eq!(quality["structural_quality_grade"].as_str(), Some("good"));
    assert_eq!(
        quality["structural_quality_missing_metric_count"].as_u64(),
        Some(0)
    );
    assert_eq!(quality["structural_quality_watch_count"].as_u64(), Some(0));
    assert_eq!(
        quality["structural_quality_dominant_term"]["field"].as_str(),
        Some("max_stress")
    );
    assert_eq!(
        quality["structural_quality_blocking_terms"]
            .as_array()
            .map(Vec::len),
        Some(0)
    );
    approx_eq(quality["structural_quality_score"].as_f64(), 5.56);
}

#[test]
fn scores_structural_quality_with_enabled_solver_terms() {
    let quality = score_structural_quality(
        serde_json::json!({
            "max_displacement": 0.01,
            "max_stress": 120.0
        }),
        serde_json::json!({
            "enabled_terms": ["max_displacement", "max_stress"],
            "targets": {
                "max_displacement": 0.02,
                "max_stress": 250.0
            },
            "max_ready_score": 8.0
        }),
    )
    .expect("structural solver terms should score");

    assert_eq!(quality["structural_quality_ready"].as_bool(), Some(true));
    assert_eq!(quality["structural_quality_term_count"].as_u64(), Some(2));
    approx_eq(
        quality["structural_quality_max_displacement"].as_f64(),
        0.01,
    );
    approx_eq(quality["structural_quality_max_stress"].as_f64(), 120.0);
}

#[test]
fn blocks_structural_quality_when_required_metrics_are_missing() {
    let quality = score_structural_quality(
        serde_json::json!({
            "max_displacement": 0.01
        }),
        serde_json::json!({}),
    )
    .expect("structural quality should report missing metrics");

    assert_eq!(quality["structural_quality_ready"].as_bool(), Some(false));
    assert_eq!(quality["structural_quality_grade"].as_str(), Some("block"));
    assert_eq!(
        quality["structural_quality_missing_metric_count"].as_u64(),
        Some(3)
    );
    assert_eq!(
        quality["structural_quality_blocking_terms"]
            .as_array()
            .map(Vec::len),
        Some(3)
    );
}

#[test]
fn runs_structural_quality_through_transform_executor() {
    let quality = run_transform_operator(
        "transform.score_structural_quality",
        serde_json::json!({
            "max_displacement": 0.003,
            "max_stress": 50.0,
            "mass": 5.0,
            "stiffness_margin": 4.0
        }),
        serde_json::json!({
            "max_ready_score": 8.0
        }),
    )
    .expect("structural quality should run through executor");

    assert_eq!(quality["structural_quality_ready"].as_bool(), Some(true));
    assert_eq!(
        quality["structural_quality_grade"].as_str(),
        Some("excellent")
    );
    assert_eq!(quality["structural_quality_term_count"].as_u64(), Some(4));
    assert!(
        quality["structural_quality_summary"]
            .as_str()
            .is_some_and(|summary| summary.contains("watch=0"))
    );
}

#[test]
fn runs_bar_structural_quality_workflow_graph() {
    let run = run_workflow_graph(WorkflowGraphRunRequest {
        graph: quality_graph(),
        input_artifacts: BTreeMap::from([(
            "model_input".to_string(),
            serde_json::json!({
                "length": 1.0,
                "area": 1.0,
                "youngs_modulus": 200.0,
                "elements": 2,
                "tip_force": 10.0
            }),
        )]),
    })
    .expect("bar structural quality workflow should run");

    let quality = run
        .artifacts
        .get("score_quality.summary")
        .expect("quality summary should exist");
    assert_eq!(
        quality["structural_quality_contract"].as_str(),
        Some("kyuubiki.structural_quality_score/v1")
    );
    assert_eq!(quality["structural_quality_ready"].as_bool(), Some(true));
    assert_eq!(quality["structural_quality_term_count"].as_u64(), Some(2));

    let exported = run
        .artifacts
        .get("json_output.json")
        .expect("json export artifact should exist");
    assert!(
        exported["content"]
            .as_str()
            .unwrap_or_default()
            .contains("structural_quality_max_stress")
    );
}

fn quality_graph() -> WorkflowGraph {
    WorkflowGraph {
        schema_version: "kyuubiki.workflow-graph/v1".to_string(),
        id: "workflow.bar-structural-quality".to_string(),
        name: "Bar structural quality".to_string(),
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
                vec![port("model", "study_model/bar_1d")],
                None,
            ),
            node(
                "solve",
                WorkflowNodeKind::Solve,
                Some("solve.bar_1d"),
                vec![port("model", "study_model/bar_1d")],
                vec![port("result", "result/bar_1d")],
                None,
            ),
            node(
                "score_quality",
                WorkflowNodeKind::Transform,
                Some("transform.score_structural_quality"),
                vec![port("payload", "result/bar_1d")],
                vec![port("summary", "artifact/result_summary")],
                Some(serde_json::json!({
                    "enabled_terms": ["max_displacement", "max_stress"],
                    "targets": {
                        "max_displacement": 0.1,
                        "max_stress": 20.0
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
                "study_model/bar_1d",
            ),
            edge(
                "edge-solve-score",
                "solve",
                "result",
                "score_quality",
                "payload",
                "result/bar_1d",
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
