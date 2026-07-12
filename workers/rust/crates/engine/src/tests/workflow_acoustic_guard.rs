use crate::{
    acoustic_quality::score_acoustic_quality,
    run_workflow_graph,
    workflow_executor::run_transform_operator,
    workflow_guard_transforms::{benchmark_acoustic_pair, evaluate_acoustic_guard},
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
fn evaluates_acoustic_guard_with_spl_and_intensity_rules() {
    let guard = evaluate_acoustic_guard(
        serde_json::json!({
            "max_sound_pressure_level_db": 94.0,
            "max_acoustic_intensity": 0.36,
            "total_damping_loss": 0.08
        }),
        serde_json::json!({
            "rules": [
                { "field": "max_sound_pressure_level_db", "comparison": "gt", "threshold": 90.0, "severity": "warn", "label": "spl_limit" },
                { "field": "max_acoustic_intensity", "comparison": "gt", "threshold": 0.5, "severity": "block", "label": "intensity_limit" }
            ]
        }),
    )
    .expect("acoustic guard should evaluate");

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
fn benchmarks_acoustic_pair_by_spl_intensity_and_damping() {
    let benchmark = benchmark_acoustic_pair(
        serde_json::json!({
            "left": {
                "max_sound_pressure_level_db": 88.0,
                "max_acoustic_intensity": 0.26,
                "total_damping_loss": 0.12
            },
            "right": {
                "max_sound_pressure_level_db": 92.0,
                "max_acoustic_intensity": 0.21,
                "total_damping_loss": 0.18
            }
        }),
        serde_json::json!({
            "left_label": "quiet_candidate",
            "right_label": "damped_candidate",
            "criteria": [
                { "field": "max_sound_pressure_level_db", "goal": "min", "weight": 2.0 },
                { "field": "max_acoustic_intensity", "goal": "min", "weight": 1.0 },
                { "field": "total_damping_loss", "goal": "max", "weight": 1.0 }
            ]
        }),
    )
    .expect("acoustic benchmark should succeed");

    approx_eq(benchmark["quiet_candidate_score"].as_f64(), 2.0);
    approx_eq(benchmark["damped_candidate_score"].as_f64(), 2.0);
    assert_eq!(benchmark["benchmark_winner"].as_str(), Some("tie"));
    assert_eq!(benchmark["benchmark_criteria_count"].as_u64(), Some(3));
    assert_eq!(benchmark["benchmark_left_win_count"].as_u64(), Some(1));
    assert_eq!(benchmark["benchmark_right_win_count"].as_u64(), Some(2));
}

#[test]
fn runs_acoustic_guard_and_benchmark_through_transform_executor() {
    let guard = run_transform_operator(
        "transform.evaluate_acoustic_guard",
        serde_json::json!({
            "max_sound_pressure_level_db": 78.0
        }),
        serde_json::json!({
            "rules": [
                { "field": "max_sound_pressure_level_db", "comparison": "gt", "threshold": 90.0, "severity": "warn" }
            ]
        }),
    )
    .expect("acoustic guard should run through executor");

    assert_eq!(guard["guard_status"].as_str(), Some("pass"));
    assert_eq!(guard["guard_passed"].as_bool(), Some(true));

    let benchmark = run_transform_operator(
        "transform.benchmark_acoustic_pair",
        serde_json::json!({
            "left": { "max_sound_pressure_level_db": 82.0 },
            "right": { "max_sound_pressure_level_db": 86.0 }
        }),
        serde_json::json!({
            "criteria": [
                { "field": "max_sound_pressure_level_db", "goal": "min", "weight": 2.0 }
            ]
        }),
    )
    .expect("acoustic benchmark should run through executor");

    approx_eq(benchmark["left_score"].as_f64(), 2.0);
    approx_eq(benchmark["right_score"].as_f64(), 0.0);
    assert_eq!(benchmark["benchmark_winner"].as_str(), Some("left"));
}

#[test]
fn scores_acoustic_quality_with_spl_intensity_and_damping_terms() {
    let quality = score_acoustic_quality(
        serde_json::json!({
            "max_sound_pressure_level_db": 80.0,
            "max_acoustic_intensity": 0.2,
            "max_pressure_amplitude": 0.5,
            "total_damping_loss": 0.2
        }),
        serde_json::json!({
            "targets": {
                "max_sound_pressure_level_db": 85.0,
                "max_acoustic_intensity": 0.25,
                "max_pressure_amplitude": 1.0,
                "total_damping_loss": 0.1
            },
            "max_ready_score": 7.0
        }),
    )
    .expect("acoustic quality should score");

    assert_eq!(
        quality["acoustic_quality_contract"].as_str(),
        Some("kyuubiki.acoustic_quality_score/v1")
    );
    assert_eq!(quality["acoustic_quality_ready"].as_bool(), Some(true));
    assert_eq!(quality["acoustic_quality_grade"].as_str(), Some("review"));
    assert_eq!(
        quality["acoustic_quality_missing_metric_count"].as_u64(),
        Some(0)
    );
    assert_eq!(quality["acoustic_quality_watch_count"].as_u64(), Some(0));
    assert_eq!(
        quality["acoustic_quality_dominant_term"]["field"].as_str(),
        Some("max_sound_pressure_level_db")
    );
    approx_eq(
        quality["acoustic_quality_score"].as_f64(),
        5.423529411764706,
    );
}

#[test]
fn scores_acoustic_quality_from_solver_result_aliases() {
    let quality = score_acoustic_quality(
        serde_json::json!({
            "peak_spl_db": 80.0,
            "peak_acoustic_intensity": 0.2,
            "pressure_amplitude_peak": 0.5,
            "damping_loss_total": 0.2
        }),
        serde_json::json!({
            "enabled_terms": [
                "max_sound_pressure_level_db",
                "max_acoustic_intensity",
                "max_pressure_amplitude",
                "total_damping_loss"
            ],
            "targets": {
                "max_sound_pressure_level_db": 85.0,
                "max_acoustic_intensity": 0.25,
                "max_pressure_amplitude": 1.0,
                "total_damping_loss": 0.1
            },
            "max_ready_score": 7.0
        }),
    )
    .expect("acoustic solver aliases should score");

    assert_eq!(quality["acoustic_quality_ready"].as_bool(), Some(true));
    assert_eq!(quality["acoustic_quality_term_count"].as_u64(), Some(4));
    approx_eq(quality["acoustic_quality_max_spl_db"].as_f64(), 80.0);
    approx_eq(quality["acoustic_quality_max_intensity"].as_f64(), 0.2);
    approx_eq(quality["acoustic_quality_max_pressure"].as_f64(), 0.5);
    approx_eq(quality["acoustic_quality_total_damping_loss"].as_f64(), 0.2);
}

#[test]
fn blocks_acoustic_quality_when_required_metrics_are_missing() {
    let quality = score_acoustic_quality(
        serde_json::json!({
            "max_sound_pressure_level_db": 80.0
        }),
        serde_json::json!({}),
    )
    .expect("acoustic quality should report missing metrics");

    assert_eq!(quality["acoustic_quality_ready"].as_bool(), Some(false));
    assert_eq!(quality["acoustic_quality_grade"].as_str(), Some("block"));
    assert_eq!(
        quality["acoustic_quality_missing_metric_count"].as_u64(),
        Some(3)
    );
    assert_eq!(
        quality["acoustic_quality_blocking_terms"]
            .as_array()
            .expect("blocking terms")
            .len(),
        3
    );
    assert_eq!(
        quality["acoustic_quality_blocking_terms"][0]["status"].as_str(),
        Some("missing")
    );
}

#[test]
fn runs_acoustic_quality_through_transform_executor() {
    let quality = run_transform_operator(
        "transform.score_acoustic_quality",
        serde_json::json!({
            "max_sound_pressure_level_db": 70.0,
            "max_acoustic_intensity": 0.1,
            "max_pressure_amplitude": 0.25,
            "total_damping_loss": 0.4
        }),
        serde_json::json!({
            "max_ready_score": 7.0
        }),
    )
    .expect("acoustic quality should run through executor");

    assert_eq!(quality["acoustic_quality_ready"].as_bool(), Some(true));
    assert_eq!(quality["acoustic_quality_grade"].as_str(), Some("good"));
    assert_eq!(quality["acoustic_quality_term_count"].as_u64(), Some(4));
}

#[test]
fn runs_acoustic_bar_quality_workflow_graph() {
    let run = run_workflow_graph(WorkflowGraphRunRequest {
        graph: quality_graph(),
        input_artifacts: BTreeMap::from([(
            "model_input".to_string(),
            serde_json::json!({
                "frequency_hz": 100.0,
                "nodes": [
                    { "id": "a0", "x": 0.0, "fix_pressure": true, "pressure": 1.0, "volume_velocity_source": 0.0 },
                    { "id": "a1", "x": 1.0, "fix_pressure": false, "pressure": 0.0, "volume_velocity_source": 0.01 }
                ],
                "elements": [
                    { "id": "ae0", "node_i": 0, "node_j": 1, "area": 0.1, "density": 1.2, "bulk_modulus": 142000.0, "damping_ratio": 0.02 }
                ]
            }),
        )]),
    })
    .expect("acoustic quality workflow should run");

    let quality = run
        .artifacts
        .get("score_quality.summary")
        .expect("quality summary should exist");
    assert_eq!(
        quality["acoustic_quality_contract"].as_str(),
        Some("kyuubiki.acoustic_quality_score/v1")
    );
    assert_eq!(quality["acoustic_quality_ready"].as_bool(), Some(true));
    assert!(
        quality["acoustic_quality_max_pressure"]
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
            .contains("acoustic_quality_max_pressure")
    );
}

fn quality_graph() -> WorkflowGraph {
    WorkflowGraph {
        schema_version: "kyuubiki.workflow-graph/v1".to_string(),
        id: "workflow.acoustic-bar-quality".to_string(),
        name: "Acoustic bar quality".to_string(),
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
                vec![port("model", "study_model/acoustic_bar_1d")],
                None,
            ),
            node(
                "solve",
                WorkflowNodeKind::Solve,
                Some("solve.acoustic_bar_1d"),
                vec![port("model", "study_model/acoustic_bar_1d")],
                vec![port("result", "result/acoustic_bar_1d")],
                None,
            ),
            node(
                "score_quality",
                WorkflowNodeKind::Transform,
                Some("transform.score_acoustic_quality"),
                vec![port("payload", "result/acoustic_bar_1d")],
                vec![port("summary", "artifact/result_summary")],
                Some(serde_json::json!({
                    "enabled_terms": [
                        "max_sound_pressure_level_db",
                        "max_acoustic_intensity",
                        "max_pressure_amplitude",
                        "total_damping_loss"
                    ],
                    "targets": {
                        "max_sound_pressure_level_db": 160.0,
                        "max_acoustic_intensity": 1000.0,
                        "max_pressure_amplitude": 100.0,
                        "total_damping_loss": 0.00001
                    },
                    "max_ready_score": 12.0
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
                "study_model/acoustic_bar_1d",
            ),
            edge(
                "edge-solve-score",
                "solve",
                "result",
                "score_quality",
                "payload",
                "result/acoustic_bar_1d",
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
