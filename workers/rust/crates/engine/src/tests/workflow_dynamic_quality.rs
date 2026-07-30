use crate::{
    dynamic_quality::score_dynamic_quality,
    run_workflow_graph,
    workflow_executor::run_transform_operator,
    workflow_guard_transforms::{benchmark_dynamic_pair, evaluate_dynamic_guard},
};
use kyuubiki_protocol::{
    WorkflowDefaults, WorkflowEdge, WorkflowGraph, WorkflowGraphRunRequest, WorkflowNode,
    WorkflowNodeKind, WorkflowNodePortRef, WorkflowPort,
};
use std::collections::BTreeMap;

fn approx_eq(left: Option<f64>, right: f64) {
    let value = left.expect("expected numeric dynamic quality value");
    assert!(
        (value - right).abs() < 1.0e-9,
        "left={value}, right={right}"
    );
}

#[test]
fn evaluates_dynamic_guard_with_response_threshold_rules() {
    let guard = evaluate_dynamic_guard(
        serde_json::json!({
            "peak_frequency_hz": 18.0,
            "max_displacement": 0.035,
            "max_acceleration": 190.0,
            "max_force": 1800.0
        }),
        serde_json::json!({
            "rules": [
                { "field": "peak_frequency_hz", "comparison": "lt", "threshold": 20.0, "severity": "warn", "label": "frequency_floor" },
                { "field": "max_displacement", "comparison": "gt", "threshold": 0.02, "severity": "block", "label": "displacement_limit" }
            ]
        }),
    )
    .expect("dynamic guard should evaluate");

    assert_eq!(guard["guard_status"].as_str(), Some("block"));
    assert_eq!(guard["guard_passed"].as_bool(), Some(false));
    assert_eq!(guard["guard_warn_count"].as_u64(), Some(1));
    assert_eq!(guard["guard_block_count"].as_u64(), Some(1));
}

#[test]
fn benchmarks_dynamic_pair_by_frequency_displacement_and_force() {
    let benchmark = benchmark_dynamic_pair(
        serde_json::json!({
            "left": {
                "peak_frequency_hz": 42.0,
                "max_displacement": 0.014,
                "max_force": 2400.0
            },
            "right": {
                "peak_frequency_hz": 35.0,
                "max_displacement": 0.010,
                "max_force": 2600.0
            }
        }),
        serde_json::json!({
            "left_label": "stiff_dynamic_candidate",
            "right_label": "light_dynamic_candidate",
            "criteria": [
                { "field": "peak_frequency_hz", "goal": "max", "weight": 2.0 },
                { "field": "max_displacement", "goal": "min", "weight": 1.0 },
                { "field": "max_force", "goal": "min", "weight": 1.0 }
            ]
        }),
    )
    .expect("dynamic benchmark should succeed");

    approx_eq(benchmark["stiff_dynamic_candidate_score"].as_f64(), 3.0);
    approx_eq(benchmark["light_dynamic_candidate_score"].as_f64(), 1.0);
    assert_eq!(
        benchmark["benchmark_winner"].as_str(),
        Some("stiff_dynamic_candidate")
    );
    assert_eq!(benchmark["benchmark_criteria_count"].as_u64(), Some(3));
}

#[test]
fn derives_dynamic_guard_and_benchmark_metrics_from_alias_inputs() {
    let guard = evaluate_dynamic_guard(
        serde_json::json!({
            "frequencies": [
                { "freq_hz": 10.0, "displacement_amplitude": 0.01 },
                { "freq_hz": 18.0, "displacement_amplitude": 0.04 }
            ]
        }),
        serde_json::json!({
            "rules": [
                { "field": "max_displacement", "comparison": "gt", "threshold": 0.02, "severity": "block" },
                { "field": "peak_frequency_hz", "comparison": "lt", "threshold": 20.0, "severity": "warn" }
            ]
        }),
    )
    .expect("dynamic guard should derive metrics from frequency responses");

    assert_eq!(guard["guard_status"].as_str(), Some("block"));
    assert_eq!(guard["guard_trigger_count"].as_u64(), Some(2));
    approx_eq(guard["guard_triggers"][0]["value"].as_f64(), 0.04);

    let benchmark = benchmark_dynamic_pair(
        serde_json::json!({
            "left": {
                "frequencies": [
                    { "freq_hz": 24.0, "displacement_amplitude": 0.02 },
                    { "freq_hz": 42.0, "displacement_amplitude": 0.03 }
                ]
            },
            "right": { "response_peak_frequency_hz": 35.0 }
        }),
        serde_json::json!({
            "criteria": [
                { "field": "peak_frequency_hz", "goal": "max", "weight": 2.0 }
            ]
        }),
    )
    .expect("dynamic benchmark should derive comparable metrics");

    approx_eq(benchmark["left_score"].as_f64(), 2.0);
    approx_eq(benchmark["right_score"].as_f64(), 0.0);
    approx_eq(
        benchmark["benchmark_breakdown"][0]["left_value"].as_f64(),
        42.0,
    );
}

#[test]
fn scores_dynamic_quality_from_summary_fields() {
    let quality = score_dynamic_quality(
        serde_json::json!({
            "peak_frequency_hz": 32.0,
            "max_displacement": 0.012,
            "max_acceleration": 180.0,
            "max_force": 3200.0
        }),
        serde_json::json!({
            "targets": {
                "peak_frequency_hz": 25.0,
                "max_displacement": 0.02,
                "max_acceleration": 250.0,
                "max_force": 5000.0
            },
            "max_ready_score": 8.0
        }),
    )
    .expect("dynamic quality should score");

    assert_eq!(
        quality["dynamic_quality_contract"].as_str(),
        Some("kyuubiki.dynamic_quality_score/v1")
    );
    assert_eq!(quality["dynamic_quality_ready"].as_bool(), Some(true));
    assert_eq!(
        quality["dynamic_quality_missing_metric_count"].as_u64(),
        Some(0)
    );
    assert_eq!(quality["dynamic_quality_watch_count"].as_u64(), Some(0));
    assert_eq!(
        quality["dynamic_quality_dominant_term"]["field"].as_str(),
        Some("peak_frequency_hz")
    );
    assert_eq!(quality["dynamic_quality_term_count"].as_u64(), Some(4));
    approx_eq(quality["dynamic_quality_peak_frequency_hz"].as_f64(), 32.0);
}

#[test]
fn derives_dynamic_quality_from_harmonic_frequency_results() {
    let quality = score_dynamic_quality(
        serde_json::json!({
            "frequencies": [
                { "freq_hz": 5.0, "displacement_amplitude": 0.01, "acceleration_amplitude": 25.0, "force_amplitude": 100.0 },
                { "freq_hz": 12.0, "displacement_amplitude": 0.03, "acceleration_amplitude": 180.0, "force_amplitude": 450.0 },
                { "freq_hz": 30.0, "displacement_amplitude": 0.02, "acceleration_amplitude": 220.0, "force_amplitude": 390.0 }
            ]
        }),
        serde_json::json!({
            "targets": {
                "peak_frequency_hz": 10.0,
                "max_displacement": 0.05,
                "max_acceleration": 300.0,
                "max_force": 1000.0
            }
        }),
    )
    .expect("harmonic result should derive dynamic metrics");

    approx_eq(quality["dynamic_quality_peak_frequency_hz"].as_f64(), 12.0);
    approx_eq(quality["dynamic_quality_max_displacement"].as_f64(), 0.03);
    approx_eq(quality["dynamic_quality_max_force"].as_f64(), 450.0);
    assert_eq!(quality["dynamic_quality_ready"].as_bool(), Some(true));
}

#[test]
fn derives_dynamic_quality_from_transient_spring_results() {
    let quality = score_dynamic_quality(
        serde_json::json!({
            "nodes": [
                { "id": "fixed", "ux": 0.0, "vx": 0.0, "ax": 0.0 },
                { "id": "tip", "ux": -0.012, "vx": 0.8, "ax": -12.0 }
            ],
            "max_force": 150.0
        }),
        serde_json::json!({
            "enabled_terms": [
                "max_displacement",
                "max_velocity",
                "max_acceleration",
                "max_force"
            ],
            "targets": {
                "max_displacement": 0.02,
                "max_velocity": 1.0,
                "max_acceleration": 20.0,
                "max_force": 300.0
            }
        }),
    )
    .expect("transient result should derive dynamic metrics");

    assert_eq!(quality["dynamic_quality_ready"].as_bool(), Some(true));
    assert_eq!(quality["dynamic_quality_term_count"].as_u64(), Some(4));
    approx_eq(quality["dynamic_quality_max_velocity"].as_f64(), 0.8);
    approx_eq(quality["dynamic_quality_max_acceleration"].as_f64(), 12.0);
}

#[test]
fn blocks_dynamic_quality_when_required_metrics_are_missing() {
    let quality = score_dynamic_quality(
        serde_json::json!({ "max_force": 100.0 }),
        serde_json::json!({}),
    )
    .expect("dynamic quality should return missing terms");

    assert_eq!(quality["dynamic_quality_ready"].as_bool(), Some(false));
    assert_eq!(
        quality["dynamic_quality_missing_metric_count"].as_u64(),
        Some(3)
    );
    assert_eq!(
        quality["dynamic_quality_blocking_terms"]
            .as_array()
            .expect("blocking terms")
            .len(),
        3
    );
    assert_eq!(quality["dynamic_quality_grade"].as_str(), Some("block"));
}

#[test]
fn runs_dynamic_guard_and_benchmark_through_transform_executor() {
    let guard = run_transform_operator(
        "transform.evaluate_dynamic_guard",
        serde_json::json!({
            "max_displacement": 0.012
        }),
        serde_json::json!({
            "rules": [
                { "field": "max_displacement", "comparison": "gt", "threshold": 0.02, "severity": "block" }
            ]
        }),
    )
    .expect("dynamic guard should run through executor");

    assert_eq!(guard["guard_status"].as_str(), Some("pass"));
    assert_eq!(guard["guard_passed"].as_bool(), Some(true));

    let benchmark = run_transform_operator(
        "transform.benchmark_dynamic_pair",
        serde_json::json!({
            "left": { "peak_frequency_hz": 40.0 },
            "right": { "peak_frequency_hz": 32.0 }
        }),
        serde_json::json!({
            "criteria": [
                { "field": "peak_frequency_hz", "goal": "max", "weight": 2.0 }
            ]
        }),
    )
    .expect("dynamic benchmark should run through executor");

    approx_eq(benchmark["left_score"].as_f64(), 2.0);
    approx_eq(benchmark["right_score"].as_f64(), 0.0);
}

#[test]
fn runs_dynamic_quality_through_transform_executor() {
    let quality = run_transform_operator(
        "transform.score_dynamic_quality",
        serde_json::json!({
            "peak_frequency_hz": 40.0,
            "max_displacement": 0.01,
            "max_acceleration": 100.0,
            "max_force": 1000.0
        }),
        serde_json::json!({ "max_ready_score": 8.0 }),
    )
    .expect("dynamic quality should run through executor");

    assert_eq!(quality["dynamic_quality_ready"].as_bool(), Some(true));
    assert_eq!(quality["dynamic_quality_term_count"].as_u64(), Some(4));
}

#[test]
fn runs_harmonic_solve_dynamic_quality_workflow_graph() {
    let graph = WorkflowGraph {
        schema_version: "kyuubiki.workflow-graph/v1".to_string(),
        id: "workflow.harmonic-dynamic-quality".to_string(),
        name: "Harmonic dynamic quality".to_string(),
        version: "1.0.0".to_string(),
        description: Some("Solve a frequency response and score dynamic readiness.".to_string()),
        dataset_contract: None,
        entry_nodes: vec!["model_input".to_string()],
        output_nodes: vec!["json_output".to_string()],
        defaults: WorkflowDefaults::default(),
        nodes: vec![
            input_node("model_input", "study_model/harmonic_spring_1d"),
            WorkflowNode {
                id: "solve_harmonic".to_string(),
                kind: WorkflowNodeKind::Solve,
                operator_id: Some("solve.harmonic_spring_1d".to_string()),
                name: Some("Solve harmonic spring".to_string()),
                description: None,
                config: None,
                cache_policy: None,
                inputs: vec![port("model", "study_model/harmonic_spring_1d")],
                outputs: vec![port("result", "result/harmonic_spring_1d")],
            },
            WorkflowNode {
                id: "score_dynamic".to_string(),
                kind: WorkflowNodeKind::Transform,
                operator_id: Some("transform.score_dynamic_quality".to_string()),
                name: Some("Score dynamic quality".to_string()),
                description: None,
                config: Some(serde_json::json!({
                    "targets": {
                        "peak_frequency_hz": 0.5,
                        "max_displacement": 0.2,
                        "max_acceleration": 100.0,
                        "max_force": 100.0
                    },
                    "max_ready_score": 12.0
                })),
                cache_policy: None,
                inputs: vec![port("payload", "result/harmonic_spring_1d")],
                outputs: vec![port("summary", "artifact/result_summary")],
            },
            WorkflowNode {
                id: "export_json".to_string(),
                kind: WorkflowNodeKind::Export,
                operator_id: Some("export.summary_json".to_string()),
                name: Some("Export dynamic quality".to_string()),
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
                "solve_harmonic",
                "model",
                "study_model/harmonic_spring_1d",
            ),
            edge(
                "edge-solve-score",
                "solve_harmonic",
                "result",
                "score_dynamic",
                "payload",
                "result/harmonic_spring_1d",
            ),
            edge(
                "edge-score-export",
                "score_dynamic",
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
                    { "id": "fixed", "x": 0.0, "fix_x": true, "load_x": 0.0, "mass": 1.0 },
                    { "id": "tip", "x": 1.0, "fix_x": false, "load_x": 10.0, "mass": 2.0 }
                ],
                "elements": [
                    { "id": "s0", "node_i": 0, "node_j": 1, "stiffness": 100.0, "damping": 1.0 }
                ],
                "frequencies_hz": [0.0, 0.5, 1.0]
            }),
        )]),
    })
    .expect("dynamic quality workflow should run");

    let quality = run
        .artifacts
        .get("score_dynamic.summary")
        .expect("dynamic quality summary should exist");
    assert_eq!(
        quality["dynamic_quality_contract"].as_str(),
        Some("kyuubiki.dynamic_quality_score/v1")
    );
    assert_eq!(quality["dynamic_quality_term_count"].as_u64(), Some(4));

    let exported = run
        .artifacts
        .get("json_output.json")
        .expect("json export artifact should exist");
    let content = exported["content"]
        .as_str()
        .expect("json content should be a string");
    assert!(content.contains("dynamic_quality_score"));
}

#[test]
fn runs_transient_solve_dynamic_quality_workflow_graph() {
    let graph = WorkflowGraph {
        schema_version: "kyuubiki.workflow-graph/v1".to_string(),
        id: "workflow.transient-dynamic-quality".to_string(),
        name: "Transient dynamic quality".to_string(),
        version: "1.0.0".to_string(),
        description: Some("Solve a transient response and score dynamic readiness.".to_string()),
        dataset_contract: None,
        entry_nodes: vec!["model_input".to_string()],
        output_nodes: vec!["json_output".to_string()],
        defaults: WorkflowDefaults::default(),
        nodes: vec![
            input_node("model_input", "study_model/transient_spring_1d"),
            WorkflowNode {
                id: "solve_transient".to_string(),
                kind: WorkflowNodeKind::Solve,
                operator_id: Some("solve.transient_spring_1d".to_string()),
                name: Some("Solve transient spring".to_string()),
                description: None,
                config: None,
                cache_policy: None,
                inputs: vec![port("model", "study_model/transient_spring_1d")],
                outputs: vec![port("result", "result/transient_spring_1d")],
            },
            WorkflowNode {
                id: "score_dynamic".to_string(),
                kind: WorkflowNodeKind::Transform,
                operator_id: Some("transform.score_dynamic_quality".to_string()),
                name: Some("Score transient dynamic quality".to_string()),
                description: None,
                config: Some(serde_json::json!({
                    "enabled_terms": [
                        "max_displacement",
                        "max_velocity",
                        "max_acceleration",
                        "max_force"
                    ],
                    "targets": {
                        "max_displacement": 0.2,
                        "max_velocity": 10.0,
                        "max_acceleration": 100.0,
                        "max_force": 1000.0
                    },
                    "max_ready_score": 12.0
                })),
                cache_policy: None,
                inputs: vec![port("payload", "result/transient_spring_1d")],
                outputs: vec![port("summary", "artifact/result_summary")],
            },
            WorkflowNode {
                id: "export_json".to_string(),
                kind: WorkflowNodeKind::Export,
                operator_id: Some("export.summary_json".to_string()),
                name: Some("Export dynamic quality".to_string()),
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
                "solve_transient",
                "model",
                "study_model/transient_spring_1d",
            ),
            edge(
                "edge-solve-score",
                "solve_transient",
                "result",
                "score_dynamic",
                "payload",
                "result/transient_spring_1d",
            ),
            edge(
                "edge-score-export",
                "score_dynamic",
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
                    { "id": "fixed", "x": 0.0, "fix_x": true, "load_x": 0.0, "mass": 1.0 },
                    { "id": "tip", "x": 1.0, "fix_x": false, "load_x": 10.0, "mass": 2.0 }
                ],
                "elements": [
                    { "id": "s0", "node_i": 0, "node_j": 1, "stiffness": 100.0, "damping": 0.5 }
                ],
                "time_step": 0.01,
                "steps": 10
            }),
        )]),
    })
    .expect("transient dynamic quality workflow should run");

    let quality = run
        .artifacts
        .get("score_dynamic.summary")
        .expect("dynamic quality summary should exist");
    assert_eq!(
        quality["dynamic_quality_contract"].as_str(),
        Some("kyuubiki.dynamic_quality_score/v1")
    );
    assert_eq!(quality["dynamic_quality_term_count"].as_u64(), Some(4));
    assert_eq!(quality["dynamic_quality_ready"].as_bool(), Some(true));
    assert!(
        quality["dynamic_quality_max_acceleration"]
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
    assert!(content.contains("dynamic_quality_max_velocity"));
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
