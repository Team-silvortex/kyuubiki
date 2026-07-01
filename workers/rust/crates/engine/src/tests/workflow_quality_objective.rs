use crate::{
    run_workflow_graph,
    workflow_executor::run_transform_operator,
    workflow_quality_objective::{
        compose_quality_objective, prepare_quality_next_round_request, rank_quality_candidates,
    },
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
                    "thermal_quality_missing_metric_count": 0
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
                "magnetostatic_quality_missing_metric_count": 2
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
fn ranks_quality_candidates_by_readiness_and_score() {
    let ranking = rank_quality_candidates(
        serde_json::json!({
            "candidates": {
                "candidate_a": {
                    "qualities": {
                        "thermal": {
                            "thermal_quality_score": 2.0,
                            "thermal_quality_ready": true,
                            "thermal_quality_missing_metric_count": 0
                        },
                        "cfd": {
                            "cfd_quality_score": 5.0,
                            "cfd_quality_ready": true,
                            "cfd_quality_missing_metric_count": 0
                        }
                    }
                },
                "candidate_b": {
                    "qualities": {
                        "thermal": {
                            "thermal_quality_score": 1.0,
                            "thermal_quality_ready": true,
                            "thermal_quality_missing_metric_count": 0
                        },
                        "cfd": {
                            "cfd_quality_score": 1.5,
                            "cfd_quality_ready": true,
                            "cfd_quality_missing_metric_count": 0
                        }
                    }
                },
                "candidate_blocked": {
                    "qualities": {
                        "thermal": {
                            "thermal_quality_score": 0.5,
                            "thermal_quality_ready": false,
                            "thermal_quality_missing_metric_count": 1
                        }
                    }
                }
            }
        }),
        serde_json::json!({
            "objective": {
                "weights": {"cfd": 2.0},
                "not_ready_penalty": 20.0
            }
        }),
    )
    .expect("quality candidates should rank");

    assert_eq!(
        ranking["quality_candidate_ranking_contract"].as_str(),
        Some("kyuubiki.quality_candidate_ranking/v1")
    );
    assert_eq!(ranking["candidate_count"].as_u64(), Some(3));
    assert_eq!(ranking["ready_candidate_count"].as_u64(), Some(2));
    assert_eq!(ranking["best_candidate_id"].as_str(), Some("candidate_b"));
    assert_eq!(ranking["best_candidate_ready"].as_bool(), Some(true));
    assert_eq!(ranking["ranking"][0]["rank"].as_u64(), Some(1));
}

#[test]
fn runs_quality_candidate_ranking_through_transform_executor() {
    let ranking = run_transform_operator(
        "transform.rank_quality_candidates",
        serde_json::json!({
            "candidate_a": {
                "thermal": {
                    "thermal_quality_score": 4.0,
                    "thermal_quality_ready": true,
                    "thermal_quality_missing_metric_count": 0
                }
            },
            "candidate_b": {
                "thermal": {
                    "thermal_quality_score": 2.0,
                    "thermal_quality_ready": true,
                    "thermal_quality_missing_metric_count": 0
                }
            }
        }),
        serde_json::json!({}),
    )
    .expect("quality candidate ranking should run through executor");

    assert_eq!(ranking["best_candidate_id"].as_str(), Some("candidate_b"));
    approx_eq(ranking["best_candidate_score"].as_f64(), 2.0);
}

#[test]
fn prepares_quality_next_round_request_from_ranking() {
    let request = prepare_quality_next_round_request(
        serde_json::json!({
            "quality_candidate_ranking_contract": "kyuubiki.quality_candidate_ranking/v1",
            "ranking": [{
                "rank": 1,
                "candidate_id": "candidate_b",
                "score": 2.5,
                "ready": true,
                "objective": {"composite_quality_score": 2.5}
            }]
        }),
        serde_json::json!({
            "target_score": 2.0,
            "max_candidates": 12,
            "search_space": {"thickness_mm": [1.0, 4.0]}
        }),
    )
    .expect("quality next round request should build");

    assert_eq!(
        request["quality_next_round_contract"].as_str(),
        Some("kyuubiki.quality_next_round_request/v1")
    );
    assert_eq!(request["action"].as_str(), Some("continue"));
    assert_eq!(
        request["selected_candidate_id"].as_str(),
        Some("candidate_b")
    );
    approx_eq(request["request_payload"]["max_candidates"].as_f64(), 12.0);
}

#[test]
fn runs_quality_next_round_request_through_transform_executor() {
    let request = run_transform_operator(
        "transform.prepare_quality_next_round_request",
        serde_json::json!({
            "ranking": [{
                "rank": 1,
                "candidate_id": "candidate_ready",
                "score": 1.5,
                "ready": true
            }]
        }),
        serde_json::json!({"target_score": 2.0}),
    )
    .expect("quality next round request should run through executor");

    assert_eq!(request["action"].as_str(), Some("stop"));
    assert_eq!(
        request["selected_candidate_id"].as_str(),
        Some("candidate_ready")
    );
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
