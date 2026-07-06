use crate::{
    modal_quality::score_modal_quality,
    run_workflow_graph,
    workflow_executor::run_transform_operator,
    workflow_guard_transforms::{benchmark_modal_pair, evaluate_modal_guard},
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
fn evaluates_modal_guard_with_frequency_band_rules() {
    let guard = evaluate_modal_guard(
        serde_json::json!({
            "min_frequency_hz": 18.0,
            "max_frequency_hz": 220.0,
            "total_mass": 42.0,
            "mode_1_participation_norm": 1.7
        }),
        serde_json::json!({
            "rules": [
                { "field": "min_frequency_hz", "comparison": "lt", "threshold": 20.0, "severity": "block", "label": "first_mode_floor" },
                { "field": "max_frequency_hz", "comparison": "gt", "threshold": 260.0, "severity": "warn", "label": "upper_band" }
            ]
        }),
    )
    .expect("modal guard should evaluate");

    assert_eq!(guard["guard_status"].as_str(), Some("block"));
    assert_eq!(guard["guard_passed"].as_bool(), Some(false));
    assert_eq!(guard["guard_block_count"].as_u64(), Some(1));
    assert_eq!(guard["guard_warn_count"].as_u64(), Some(0));
    assert_eq!(
        guard["guard_recommendation"].as_str(),
        Some("hold_and_review")
    );
}

#[test]
fn benchmarks_modal_pair_by_frequency_mass_and_participation() {
    let benchmark = benchmark_modal_pair(
        serde_json::json!({
            "left": {
                "min_frequency_hz": 28.0,
                "total_mass": 15.0,
                "mode_1_participation_norm": 1.2
            },
            "right": {
                "min_frequency_hz": 24.0,
                "total_mass": 11.0,
                "mode_1_participation_norm": 1.6
            }
        }),
        serde_json::json!({
            "left_label": "stiff_modal_candidate",
            "right_label": "light_modal_candidate",
            "criteria": [
                { "field": "min_frequency_hz", "goal": "max", "weight": 2.0 },
                { "field": "total_mass", "goal": "min", "weight": 2.0 },
                { "field": "mode_1_participation_norm", "goal": "min", "weight": 1.0 }
            ]
        }),
    )
    .expect("modal benchmark should succeed");

    approx_eq(benchmark["stiff_modal_candidate_score"].as_f64(), 3.0);
    approx_eq(benchmark["light_modal_candidate_score"].as_f64(), 2.0);
    assert_eq!(
        benchmark["benchmark_winner"].as_str(),
        Some("stiff_modal_candidate")
    );
    assert_eq!(benchmark["benchmark_criteria_count"].as_u64(), Some(3));
    assert_eq!(benchmark["benchmark_left_win_count"].as_u64(), Some(2));
    assert_eq!(benchmark["benchmark_right_win_count"].as_u64(), Some(1));
}

#[test]
fn runs_modal_guard_and_benchmark_through_transform_executor() {
    let guard = run_transform_operator(
        "transform.evaluate_modal_guard",
        serde_json::json!({
            "min_frequency_hz": 35.0
        }),
        serde_json::json!({
            "rules": [
                { "field": "min_frequency_hz", "comparison": "lt", "threshold": 20.0, "severity": "block" }
            ]
        }),
    )
    .expect("modal guard should run through executor");

    assert_eq!(guard["guard_status"].as_str(), Some("pass"));
    assert_eq!(guard["guard_passed"].as_bool(), Some(true));

    let benchmark = run_transform_operator(
        "transform.benchmark_modal_pair",
        serde_json::json!({
            "left": { "min_frequency_hz": 32.0 },
            "right": { "min_frequency_hz": 26.0 }
        }),
        serde_json::json!({
            "criteria": [
                { "field": "min_frequency_hz", "goal": "max", "weight": 2.0 }
            ]
        }),
    )
    .expect("modal benchmark should run through executor");

    approx_eq(benchmark["left_score"].as_f64(), 2.0);
    approx_eq(benchmark["right_score"].as_f64(), 0.0);
    assert_eq!(benchmark["benchmark_winner"].as_str(), Some("left"));
}

#[test]
fn scores_modal_quality_with_frequency_and_mass_penalties() {
    let quality = score_modal_quality(
        serde_json::json!({
            "min_frequency_hz": 32.0,
            "max_frequency_hz": 180.0,
            "total_mass": 18.0,
            "mode_1_participation_norm": 1.3
        }),
        serde_json::json!({
            "targets": {
                "min_frequency_hz": 20.0,
                "total_mass": 25.0,
                "mode_1_participation_norm": 2.0,
                "frequency_span_hz": 250.0
            },
            "max_ready_score": 8.0
        }),
    )
    .expect("modal quality should score");

    assert_eq!(quality["modal_quality_ready"].as_bool(), Some(true));
    assert_eq!(
        quality["modal_quality_missing_metric_count"].as_u64(),
        Some(0)
    );
    assert_eq!(quality["modal_quality_term_count"].as_u64(), Some(4));
    assert_eq!(quality["modal_quality_grade"].as_str(), Some("good"));
    let terms = quality["modal_quality_terms"]
        .as_array()
        .expect("quality terms should be an array");
    assert_eq!(terms[0]["goal"].as_str(), Some("max"));
    assert_eq!(terms[3]["field"].as_str(), Some("frequency_span_hz"));
    approx_eq(terms[3]["value"].as_f64(), 148.0);
}

#[test]
fn scores_modal_quality_from_solver_modes() {
    let quality = score_modal_quality(
        serde_json::json!({
            "min_frequency_hz": 32.0,
            "max_frequency_hz": 120.0,
            "total_mass": 16.0,
            "modes": [
                { "index": 0, "participation_norm": 1.1 },
                { "index": 1, "participation_norm": 0.6 }
            ]
        }),
        serde_json::json!({
            "enabled_terms": [
                "min_frequency_hz",
                "total_mass",
                "mode_1_participation_norm",
                "frequency_span_hz"
            ],
            "targets": {
                "min_frequency_hz": 20.0,
                "total_mass": 25.0,
                "mode_1_participation_norm": 2.0,
                "frequency_span_hz": 250.0
            },
            "max_ready_score": 8.0
        }),
    )
    .expect("modal solver fields should score");

    assert_eq!(quality["modal_quality_ready"].as_bool(), Some(true));
    assert_eq!(quality["modal_quality_term_count"].as_u64(), Some(4));
    approx_eq(quality["modal_quality_frequency_span_hz"].as_f64(), 88.0);
    approx_eq(
        quality["modal_quality_mode_1_participation_norm"].as_f64(),
        1.1,
    );
}

#[test]
fn blocks_modal_quality_when_required_metrics_are_missing() {
    let quality = score_modal_quality(
        serde_json::json!({
            "min_frequency_hz": 12.0
        }),
        serde_json::json!({}),
    )
    .expect("modal quality should still return missing terms");

    assert_eq!(quality["modal_quality_ready"].as_bool(), Some(false));
    assert_eq!(
        quality["modal_quality_missing_metric_count"].as_u64(),
        Some(3)
    );
    assert_eq!(quality["modal_quality_grade"].as_str(), Some("block"));
}

#[test]
fn runs_modal_quality_through_transform_executor() {
    let quality = run_transform_operator(
        "transform.score_modal_quality",
        serde_json::json!({
            "min_frequency_hz": 28.0,
            "max_frequency_hz": 120.0,
            "total_mass": 16.0,
            "mode_1_participation_norm": 1.1
        }),
        serde_json::json!({
            "max_ready_score": 8.0
        }),
    )
    .expect("modal quality should run through executor");

    assert_eq!(
        quality["modal_quality_contract"].as_str(),
        Some("kyuubiki.modal_quality_score/v1")
    );
    assert_eq!(quality["modal_quality_ready"].as_bool(), Some(true));
}

#[test]
fn runs_modal_frame_quality_workflow_graph() {
    let run = run_workflow_graph(WorkflowGraphRunRequest {
        graph: quality_graph(),
        input_artifacts: BTreeMap::from([("model_input".to_string(), modal_model())]),
    })
    .expect("modal quality workflow should run");

    let quality = run
        .artifacts
        .get("score_quality.summary")
        .expect("quality summary should exist");
    assert_eq!(
        quality["modal_quality_contract"].as_str(),
        Some("kyuubiki.modal_quality_score/v1")
    );
    assert_eq!(quality["modal_quality_ready"].as_bool(), Some(true));
    assert_eq!(quality["modal_quality_term_count"].as_u64(), Some(4));
    assert!(
        quality["modal_quality_min_frequency_hz"]
            .as_f64()
            .unwrap_or_default()
            > 0.0
    );

    let exported = run
        .artifacts
        .get("json_output.json")
        .expect("json export artifact should exist");
    assert!(exported["content"]
        .as_str()
        .unwrap_or_default()
        .contains("modal_quality_frequency_span_hz"));
}

fn quality_graph() -> WorkflowGraph {
    WorkflowGraph {
        schema_version: "kyuubiki.workflow-graph/v1".to_string(),
        id: "workflow.modal-frame-quality".to_string(),
        name: "Modal frame quality".to_string(),
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
                vec![port("model", "study_model/modal_frame_2d")],
                None,
            ),
            node(
                "solve",
                WorkflowNodeKind::Solve,
                Some("solve.modal_frame_2d"),
                vec![port("model", "study_model/modal_frame_2d")],
                vec![port("result", "result/modal_frame_2d")],
                None,
            ),
            node(
                "score_quality",
                WorkflowNodeKind::Transform,
                Some("transform.score_modal_quality"),
                vec![port("payload", "result/modal_frame_2d")],
                vec![port("summary", "artifact/result_summary")],
                Some(serde_json::json!({
                    "enabled_terms": [
                        "min_frequency_hz",
                        "total_mass",
                        "mode_1_participation_norm",
                        "frequency_span_hz"
                    ],
                    "targets": {
                        "min_frequency_hz": 0.1,
                        "total_mass": 1000.0,
                        "mode_1_participation_norm": 100.0,
                        "frequency_span_hz": 100000.0
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
                "study_model/modal_frame_2d",
            ),
            edge(
                "edge-solve-score",
                "solve",
                "result",
                "score_quality",
                "payload",
                "result/modal_frame_2d",
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

fn modal_model() -> serde_json::Value {
    serde_json::json!({
        "nodes": [
            { "id": "n0", "x": 0.0, "y": 0.0, "fix_x": true, "fix_y": true, "fix_rz": true, "load_x": 0.0, "load_y": 0.0, "moment_z": 0.0 },
            { "id": "n1", "x": 2.0, "y": 0.0, "fix_x": false, "fix_y": false, "fix_rz": false, "load_x": 0.0, "load_y": 0.0, "moment_z": 0.0 }
        ],
        "elements": [
            { "id": "e0", "node_i": 0, "node_j": 1, "area": 0.01, "youngs_modulus": 210000000000.0, "moment_of_inertia": 0.000008333, "section_modulus": 0.0001667, "density": 7850.0 }
        ],
        "mode_count": 2
    })
}
