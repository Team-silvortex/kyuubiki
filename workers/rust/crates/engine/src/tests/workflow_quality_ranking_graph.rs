use crate::run_workflow_graph;
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
fn runs_quality_ranking_to_parameter_sweep_workflow_graph() {
    let run = run_workflow_graph(WorkflowGraphRunRequest {
        graph: quality_ranking_to_sweep_graph(0.5),
        input_artifacts: BTreeMap::from([(
            "quality_candidates".to_string(),
            serde_json::json!({
                "candidates": {
                    "candidate_a": {
                        "qualities": {
                            "structural": {
                                "structural_quality_score": 4.0,
                                "structural_quality_ready": true,
                                "structural_quality_missing_metric_count": 0
                            }
                        }
                    },
                    "candidate_b": {
                        "qualities": {
                            "structural": {
                                "structural_quality_score": 1.0,
                                "structural_quality_ready": true,
                                "structural_quality_missing_metric_count": 0
                            }
                        }
                    }
                }
            }),
        )]),
    })
    .expect("quality ranking to sweep workflow should run");

    let ranking = run
        .artifacts
        .get("rank_candidates.ranking")
        .expect("ranking artifact should exist");
    assert_eq!(ranking["best_candidate_id"].as_str(), Some("candidate_b"));
    approx_eq(ranking["best_candidate_score"].as_f64(), 1.0);

    let request = run
        .artifacts
        .get("prepare_request.request")
        .expect("next round request should exist");
    assert_eq!(request["action"].as_str(), Some("continue"));
    assert_eq!(
        request["selected_candidate_id"].as_str(),
        Some("candidate_b")
    );

    let expanded = run
        .artifacts
        .get("expand_cases.cases")
        .expect("expanded sweep cases should exist");
    assert_eq!(expanded["case_count"].as_u64(), Some(4));
    assert_eq!(expanded["axis_count"].as_u64(), Some(2));
    assert_eq!(
        expanded["cases"][0]["id"].as_str(),
        Some("quality_candidate_0")
    );
    assert_eq!(
        expanded["cases"][3]["model"]["material"]["density"].as_f64(),
        Some(7800.0)
    );
}

#[test]
fn stops_quality_ranking_to_sweep_workflow_graph_when_target_is_met() {
    let run = run_workflow_graph(WorkflowGraphRunRequest {
        graph: quality_ranking_to_sweep_graph(2.0),
        input_artifacts: BTreeMap::from([(
            "quality_candidates".to_string(),
            serde_json::json!({
                "candidates": {
                    "candidate_ready": {
                        "qualities": {
                            "structural": {
                                "structural_quality_score": 1.0,
                                "structural_quality_ready": true,
                                "structural_quality_missing_metric_count": 0
                            }
                        }
                    }
                }
            }),
        )]),
    })
    .expect("quality ranking to sweep workflow should stop cleanly");

    let request = run
        .artifacts
        .get("prepare_request.request")
        .expect("next round request should exist");
    assert_eq!(request["action"].as_str(), Some("stop"));
    assert_eq!(
        request["selected_candidate_id"].as_str(),
        Some("candidate_ready")
    );

    let plan = run
        .artifacts
        .get("build_plan.plan")
        .expect("disabled sweep plan should exist");
    assert_eq!(plan["sweep_enabled"].as_bool(), Some(false));
    assert_eq!(plan["case_count_estimate"].as_u64(), Some(0));

    let expanded = run
        .artifacts
        .get("expand_cases.cases")
        .expect("empty sweep result should exist");
    assert_eq!(expanded["case_count"].as_u64(), Some(0));
    assert_eq!(expanded["sweep_enabled"].as_bool(), Some(false));
    assert_eq!(expanded["sweep_action"].as_str(), Some("stop"));
    assert_eq!(expanded["cases"].as_array().map(Vec::len), Some(0));
}

fn quality_ranking_to_sweep_graph(target_score: f64) -> WorkflowGraph {
    WorkflowGraph {
        schema_version: "kyuubiki.workflow-graph/v1".to_string(),
        id: "workflow.quality-ranking-to-sweep".to_string(),
        name: "Quality ranking to sweep".to_string(),
        version: "1.0.0".to_string(),
        description: None,
        dataset_contract: None,
        entry_nodes: vec!["quality_candidates".to_string()],
        output_nodes: vec!["sweep_output".to_string()],
        defaults: WorkflowDefaults::default(),
        nodes: vec![
            graph_node(
                "quality_candidates",
                WorkflowNodeKind::Input,
                None,
                vec![],
                vec![graph_port("payload")],
                None,
            ),
            graph_node(
                "rank_candidates",
                WorkflowNodeKind::Transform,
                Some("transform.rank_quality_candidates"),
                vec![graph_port("payload")],
                vec![graph_port("ranking")],
                Some(serde_json::json!({})),
            ),
            graph_node(
                "prepare_request",
                WorkflowNodeKind::Transform,
                Some("transform.prepare_quality_next_round_request"),
                vec![graph_port("ranking")],
                vec![graph_port("request")],
                Some(serde_json::json!({
                    "target_score": target_score,
                    "max_candidates": 4,
                    "search_space": {
                        "elements.0.thickness": [0.01, 0.02],
                        "material.density": [2700.0, 7800.0]
                    }
                })),
            ),
            graph_node(
                "build_plan",
                WorkflowNodeKind::Transform,
                Some("transform.build_quality_parameter_sweep_plan"),
                vec![graph_port("request")],
                vec![graph_port("plan")],
                Some(serde_json::json!({
                    "samples_per_axis": 2,
                    "id_prefix": "quality_candidate",
                    "base": {
                        "elements": [{"thickness": 0.01}],
                        "material": {"density": 2700.0}
                    }
                })),
            ),
            graph_node(
                "materialize",
                WorkflowNodeKind::Transform,
                Some("transform.materialize_quality_sweep_expansion"),
                vec![graph_port("plan")],
                vec![graph_port("expansion")],
                None,
            ),
            graph_node(
                "expand_cases",
                WorkflowNodeKind::Transform,
                Some("transform.expand_parameter_sweep"),
                vec![graph_port("expansion")],
                vec![graph_port("cases")],
                None,
            ),
            graph_node(
                "sweep_output",
                WorkflowNodeKind::Output,
                None,
                vec![graph_port("cases")],
                vec![],
                None,
            ),
        ],
        edges: vec![
            graph_edge(
                "edge-candidates-ranking",
                "quality_candidates",
                "payload",
                "rank_candidates",
                "payload",
            ),
            graph_edge(
                "edge-ranking-request",
                "rank_candidates",
                "ranking",
                "prepare_request",
                "ranking",
            ),
            graph_edge(
                "edge-request-plan",
                "prepare_request",
                "request",
                "build_plan",
                "request",
            ),
            graph_edge(
                "edge-plan-materialize",
                "build_plan",
                "plan",
                "materialize",
                "plan",
            ),
            graph_edge(
                "edge-materialize-expand",
                "materialize",
                "expansion",
                "expand_cases",
                "expansion",
            ),
            graph_edge(
                "edge-expand-output",
                "expand_cases",
                "cases",
                "sweep_output",
                "cases",
            ),
        ],
    }
}

fn graph_node(
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

fn graph_port(id: &str) -> WorkflowPort {
    WorkflowPort {
        id: id.to_string(),
        artifact_type: "artifact/result_summary".to_string(),
        name: None,
        required: None,
        cardinality: None,
        dataset_value: Some("quality_exploration".to_string()),
    }
}

fn graph_edge(
    id: &str,
    from_node: &str,
    from_port: &str,
    to_node: &str,
    to_port: &str,
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
        artifact_type: "artifact/result_summary".to_string(),
        dataset_value: Some("quality_exploration".to_string()),
    }
}
