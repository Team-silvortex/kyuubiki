use crate::{
    run_workflow_graph, workflow_executor::run_transform_operator,
    workflow_quality_objective::compose_quality_objective,
};
use kyuubiki_protocol::{
    WorkflowDefaults, WorkflowEdge, WorkflowGraph, WorkflowGraphRunRequest, WorkflowNode,
    WorkflowNodeKind, WorkflowNodePortRef, WorkflowPort,
};
use std::collections::BTreeMap;

fn approx_eq(left: Option<f64>, right: f64) {
    let value = left.expect("expected numeric objective value");
    assert!(
        (value - right).abs() < 1.0e-9,
        "left={value}, right={right}"
    );
}

#[test]
fn composes_weighted_multiphysics_quality_objective() {
    let objective = compose_quality_objective(
        serde_json::json!({
            "qualities": {
                "thermal": {
                    "thermal_quality_contract": "kyuubiki.thermal_quality_score/v1",
                    "thermal_quality_score": 2.5,
                    "thermal_quality_grade": "excellent",
                    "thermal_quality_ready": true,
                    "thermal_quality_missing_metric_count": 0,
                    "thermal_quality_watch_count": 1,
                    "thermal_quality_dominant_term": {
                        "field": "thermal_temperature_max",
                        "status": "watch",
                        "penalty": 2.5
                    },
                    "thermal_quality_blocking_terms": []
                },
                "transport": {
                    "transport_quality_contract": "kyuubiki.transport_quality_score/v1",
                    "transport_quality_score": 4.0,
                    "transport_quality_grade": "good",
                    "transport_quality_ready": true,
                    "transport_quality_missing_metric_count": 0
                }
            }
        }),
        serde_json::json!({
            "weights": {
                "thermal": 2.0,
                "transport": 0.5
            },
            "max_ready_score": 12.0
        }),
    )
    .expect("quality objective should compose");

    assert_eq!(
        objective["composite_quality_contract"].as_str(),
        Some("kyuubiki.composite_quality_objective/v1")
    );
    assert_eq!(objective["composite_quality_ready"].as_bool(), Some(true));
    assert_eq!(objective["composite_quality_grade"].as_str(), Some("good"));
    assert_eq!(objective["composite_quality_term_count"].as_u64(), Some(2));
    assert_eq!(objective["composite_quality_watch_count"].as_u64(), Some(1));
    assert_eq!(
        objective["composite_quality_dominant_term"]["domain"].as_str(),
        Some("thermal")
    );
    assert_eq!(
        objective["composite_quality_dominant_term"]["dominant_term"]["field"].as_str(),
        Some("thermal_temperature_max")
    );
    approx_eq(objective["composite_quality_score"].as_f64(), 7.0);
}

#[test]
fn blocks_composite_objective_when_a_domain_quality_is_not_ready() {
    let objective = compose_quality_objective(
        serde_json::json!({
            "thermal": {
                "thermal_quality_score": 1.0,
                "thermal_quality_ready": true,
                "thermal_quality_missing_metric_count": 0
            },
            "magnetostatic": {
                "magnetostatic_quality_score": 3.0,
                "magnetostatic_quality_ready": false,
                "magnetostatic_quality_grade": "block",
                "magnetostatic_quality_missing_metric_count": 2,
                "magnetostatic_quality_blocking_terms": [{
                    "field": "magnetostatic_flux_peak_magnitude",
                    "status": "missing"
                }]
            }
        }),
        serde_json::json!({
            "missing_metric_penalty": 4.0,
            "not_ready_penalty": 20.0,
            "max_ready_score": 10.0
        }),
    )
    .expect("quality objective should expose blocked term");

    assert_eq!(objective["composite_quality_ready"].as_bool(), Some(false));
    assert_eq!(objective["composite_quality_grade"].as_str(), Some("block"));
    assert_eq!(
        objective["composite_quality_missing_metric_count"].as_u64(),
        Some(2)
    );
    assert_eq!(
        objective["composite_quality_blocked_term_count"].as_u64(),
        Some(1)
    );
    assert_eq!(
        objective["composite_quality_blocking_terms"]
            .as_array()
            .map(Vec::len),
        Some(1)
    );
    assert_eq!(
        objective["composite_quality_blocking_terms"][0]["source_blocking_terms"][0]["field"]
            .as_str(),
        Some("magnetostatic_flux_peak_magnitude")
    );
    approx_eq(objective["composite_quality_score"].as_f64(), 32.0);
}

#[test]
fn runs_quality_objective_through_transform_executor() {
    let objective = run_transform_operator(
        "transform.compose_quality_objective",
        serde_json::json!({
            "thermal": {
                "thermal_quality_score": 1.5,
                "thermal_quality_ready": true,
                "thermal_quality_missing_metric_count": 0
            },
            "cfd": {
                "cfd_quality_score": 2.0,
                "cfd_quality_ready": true,
                "cfd_quality_missing_metric_count": 0
            }
        }),
        serde_json::json!({
            "weights": {
                "thermal": 1.0,
                "cfd": 2.0
            },
            "max_ready_score": 8.0
        }),
    )
    .expect("quality objective should run through executor");

    assert_eq!(objective["composite_quality_ready"].as_bool(), Some(true));
    assert_eq!(objective["composite_quality_grade"].as_str(), Some("good"));
    assert_eq!(objective["composite_quality_term_count"].as_u64(), Some(2));
    approx_eq(objective["composite_quality_score"].as_f64(), 5.5);
}

#[test]
fn composes_solver_backed_physics_quality_families() {
    let objective = compose_quality_objective(
        serde_json::json!({
            "qualities": {
                "structural": {
                    "structural_quality_contract": "kyuubiki.structural_quality_score/v1",
                    "structural_quality_score": 1.0,
                    "structural_quality_grade": "excellent",
                    "structural_quality_ready": true,
                    "structural_quality_missing_metric_count": 0
                },
                "modal": {
                    "modal_quality_contract": "kyuubiki.modal_quality_score/v1",
                    "modal_quality_score": 1.5,
                    "modal_quality_grade": "excellent",
                    "modal_quality_ready": true,
                    "modal_quality_missing_metric_count": 0
                },
                "dynamic": {
                    "dynamic_quality_contract": "kyuubiki.dynamic_quality_score/v1",
                    "dynamic_quality_score": 2.0,
                    "dynamic_quality_grade": "good",
                    "dynamic_quality_ready": true,
                    "dynamic_quality_missing_metric_count": 0
                },
                "acoustic": {
                    "acoustic_quality_contract": "kyuubiki.acoustic_quality_score/v1",
                    "acoustic_quality_score": 2.5,
                    "acoustic_quality_grade": "good",
                    "acoustic_quality_ready": true,
                    "acoustic_quality_missing_metric_count": 0
                },
                "electrostatic": {
                    "electrostatic_quality_contract": "kyuubiki.electrostatic_quality_score/v1",
                    "electrostatic_quality_score": 3.0,
                    "electrostatic_quality_grade": "good",
                    "electrostatic_quality_ready": true,
                    "electrostatic_quality_missing_metric_count": 0
                },
                "magnetostatic": {
                    "magnetostatic_quality_contract": "kyuubiki.magnetostatic_quality_score/v1",
                    "magnetostatic_quality_score": 3.5,
                    "magnetostatic_quality_grade": "review",
                    "magnetostatic_quality_ready": true,
                    "magnetostatic_quality_missing_metric_count": 0
                }
            }
        }),
        serde_json::json!({
            "weights": {
                "structural": 2.0,
                "modal": 1.0,
                "dynamic": 1.0,
                "acoustic": 0.5,
                "electrostatic": 0.5,
                "magnetostatic": 0.5
            },
            "max_ready_score": 12.0
        }),
    )
    .expect("solver-backed quality families should compose");

    assert_eq!(objective["composite_quality_ready"].as_bool(), Some(true));
    assert_eq!(
        objective["composite_quality_grade"].as_str(),
        Some("review")
    );
    assert_eq!(objective["composite_quality_term_count"].as_u64(), Some(6));
    approx_eq(objective["composite_quality_score"].as_f64(), 10.0);
    let domains = objective["composite_quality_terms"]
        .as_array()
        .expect("terms should be an array")
        .iter()
        .filter_map(|term| term["domain"].as_str())
        .collect::<Vec<_>>();
    assert!(domains.contains(&"structural"));
    assert!(domains.contains(&"modal"));
    assert!(domains.contains(&"dynamic"));
    assert!(domains.contains(&"acoustic"));
    assert!(domains.contains(&"electrostatic"));
    assert!(domains.contains(&"magnetostatic"));
}

#[test]
fn runs_quality_objective_inside_workflow_graph() {
    let graph = WorkflowGraph {
        schema_version: "kyuubiki.workflow-graph/v1".to_string(),
        id: "workflow.multiphysics-quality-objective".to_string(),
        name: "Multiphysics quality objective".to_string(),
        version: "1.0.0".to_string(),
        description: Some("Compose domain quality summaries into one objective.".to_string()),
        dataset_contract: None,
        entry_nodes: vec![
            "thermal_quality".to_string(),
            "transport_quality".to_string(),
        ],
        output_nodes: vec!["json_output".to_string()],
        defaults: WorkflowDefaults::default(),
        nodes: vec![
            input_node("thermal_quality"),
            input_node("transport_quality"),
            WorkflowNode {
                id: "compose_objective".to_string(),
                kind: WorkflowNodeKind::Transform,
                operator_id: Some("transform.compose_quality_objective".to_string()),
                name: Some("Compose quality objective".to_string()),
                description: None,
                config: Some(serde_json::json!({
                    "weights": {
                        "thermal": 2.0,
                        "transport": 1.0
                    },
                    "max_ready_score": 12.0
                })),
                cache_policy: None,
                inputs: vec![
                    port("thermal", "artifact/result_summary"),
                    port("transport", "artifact/result_summary"),
                ],
                outputs: vec![port("objective", "artifact/result_summary")],
            },
            WorkflowNode {
                id: "export_json".to_string(),
                kind: WorkflowNodeKind::Export,
                operator_id: Some("export.summary_json".to_string()),
                name: Some("Export objective JSON".to_string()),
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
                "edge-thermal",
                "thermal_quality",
                "summary",
                "compose_objective",
                "thermal",
                "artifact/result_summary",
            ),
            edge(
                "edge-transport",
                "transport_quality",
                "summary",
                "compose_objective",
                "transport",
                "artifact/result_summary",
            ),
            edge(
                "edge-objective-export",
                "compose_objective",
                "objective",
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
        input_artifacts: BTreeMap::from([
            (
                "thermal_quality".to_string(),
                serde_json::json!({
                    "thermal_quality_score": 2.0,
                    "thermal_quality_ready": true,
                    "thermal_quality_missing_metric_count": 0
                }),
            ),
            (
                "transport_quality".to_string(),
                serde_json::json!({
                    "transport_quality_score": 3.0,
                    "transport_quality_ready": true,
                    "transport_quality_missing_metric_count": 0
                }),
            ),
        ]),
    })
    .expect("quality objective workflow should run");

    let objective = run
        .artifacts
        .get("compose_objective.objective")
        .expect("composite objective artifact should exist");
    assert_eq!(
        objective["composite_quality_contract"].as_str(),
        Some("kyuubiki.composite_quality_objective/v1")
    );
    assert_eq!(objective["composite_quality_ready"].as_bool(), Some(true));
    approx_eq(objective["composite_quality_score"].as_f64(), 7.0);

    let exported = run
        .artifacts
        .get("json_output.json")
        .expect("json export artifact should exist");
    let content = exported["content"]
        .as_str()
        .expect("json content should be a string");
    assert!(content.contains("composite_quality_score"));
    assert!(content.contains("composite_quality_terms"));
}

fn input_node(id: &str) -> WorkflowNode {
    WorkflowNode {
        id: id.to_string(),
        kind: WorkflowNodeKind::Input,
        operator_id: None,
        name: Some("Quality summary input".to_string()),
        description: None,
        config: None,
        cache_policy: None,
        inputs: vec![],
        outputs: vec![port("summary", "artifact/result_summary")],
    }
}

fn port(id: &str, artifact_type: &str) -> WorkflowPort {
    WorkflowPort {
        id: id.to_string(),
        artifact_type: artifact_type.to_string(),
        name: None,
        required: None,
        cardinality: None,
        dataset_value: Some("result_summary".to_string()),
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
        dataset_value: Some("result_summary".to_string()),
    }
}
