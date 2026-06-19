use crate::run_workflow_graph;
use kyuubiki_protocol::{
    WorkflowDefaults, WorkflowEdge, WorkflowGraph, WorkflowGraphRunRequest, WorkflowNode,
    WorkflowNodeKind, WorkflowNodePortRef, WorkflowPort,
};
use std::collections::BTreeMap;

fn port(id: &str) -> WorkflowPort {
    WorkflowPort {
        id: id.to_string(),
        artifact_type: "artifact/json".to_string(),
        name: None,
        required: None,
        cardinality: None,
        dataset_value: None,
    }
}

#[test]
fn runs_condition_branch_and_skips_inactive_path() {
    let graph = WorkflowGraph {
        schema_version: "kyuubiki.workflow-graph/v1".to_string(),
        id: "workflow.condition-branch".to_string(),
        name: "Condition branch".to_string(),
        version: "1.0.0".to_string(),
        description: Some("Route summary payload by threshold".to_string()),
        dataset_contract: None,
        entry_nodes: vec!["summary_input".to_string()],
        output_nodes: vec!["true_output".to_string(), "false_output".to_string()],
        defaults: WorkflowDefaults::default(),
        nodes: vec![
            WorkflowNode {
                id: "summary_input".to_string(),
                kind: WorkflowNodeKind::Input,
                operator_id: None,
                name: None,
                description: None,
                config: None,
                cache_policy: None,
                inputs: vec![],
                outputs: vec![port("value")],
            },
            WorkflowNode {
                id: "gate".to_string(),
                kind: WorkflowNodeKind::Condition,
                operator_id: None,
                name: None,
                description: None,
                config: Some(serde_json::json!({
                    "predicate": {
                        "path": "summary.max_displacement",
                        "operator": "gt",
                        "value": 1.0
                    }
                })),
                cache_policy: None,
                inputs: vec![port("value")],
                outputs: vec![port("if_true"), port("if_false")],
            },
            WorkflowNode {
                id: "true_output".to_string(),
                kind: WorkflowNodeKind::Output,
                operator_id: None,
                name: None,
                description: None,
                config: None,
                cache_policy: None,
                inputs: vec![port("result")],
                outputs: vec![],
            },
            WorkflowNode {
                id: "false_output".to_string(),
                kind: WorkflowNodeKind::Output,
                operator_id: None,
                name: None,
                description: None,
                config: None,
                cache_policy: None,
                inputs: vec![port("result")],
                outputs: vec![],
            },
        ],
        edges: vec![
            WorkflowEdge {
                id: "input-to-gate".to_string(),
                from: WorkflowNodePortRef {
                    node: "summary_input".to_string(),
                    port: "value".to_string(),
                },
                to: WorkflowNodePortRef {
                    node: "gate".to_string(),
                    port: "value".to_string(),
                },
                artifact_type: "artifact/json".to_string(),
                dataset_value: None,
            },
            WorkflowEdge {
                id: "gate-to-true".to_string(),
                from: WorkflowNodePortRef {
                    node: "gate".to_string(),
                    port: "if_true".to_string(),
                },
                to: WorkflowNodePortRef {
                    node: "true_output".to_string(),
                    port: "result".to_string(),
                },
                artifact_type: "artifact/json".to_string(),
                dataset_value: None,
            },
            WorkflowEdge {
                id: "gate-to-false".to_string(),
                from: WorkflowNodePortRef {
                    node: "gate".to_string(),
                    port: "if_false".to_string(),
                },
                to: WorkflowNodePortRef {
                    node: "false_output".to_string(),
                    port: "result".to_string(),
                },
                artifact_type: "artifact/json".to_string(),
                dataset_value: None,
            },
        ],
    };

    let run = run_workflow_graph(WorkflowGraphRunRequest {
        graph,
        input_artifacts: BTreeMap::from([(
            "summary_input".to_string(),
            serde_json::json!({
                "summary": { "max_displacement": 2.5, "max_stress": 14.0 }
            }),
        )]),
    })
    .expect("condition workflow should run");

    assert_eq!(
        run.completed_nodes,
        vec![
            "summary_input".to_string(),
            "gate".to_string(),
            "true_output".to_string()
        ]
    );
    assert_eq!(run.skipped_nodes, vec!["false_output".to_string()]);
    assert_eq!(run.branch_decisions.len(), 1);
    assert_eq!(run.branch_decisions[0].node_id, "gate");
    assert_eq!(run.branch_decisions[0].chosen_output, "if_true");
    assert!(run.branch_decisions[0].predicate_result);
    assert_eq!(run.node_runs.len(), 4);
    assert_eq!(run.node_runs[0].node_id, "summary_input");
    assert_eq!(
        run.node_runs[0].produced_artifacts,
        vec!["summary_input.value".to_string()]
    );
    assert_eq!(run.node_runs[1].node_id, "gate");
    assert_eq!(
        run.node_runs[1].consumed_artifacts,
        vec!["summary_input.value".to_string()]
    );
    assert_eq!(
        run.node_runs[1].produced_artifacts,
        vec!["gate.if_true".to_string()]
    );
    assert_eq!(run.node_runs[3].node_id, "false_output");
    assert!(run.artifacts.contains_key("gate.if_true"));
    assert!(run.artifacts.contains_key("true_output.result"));
    assert!(!run.artifacts.contains_key("gate.if_false"));
    assert!(!run.artifacts.contains_key("false_output.result"));
    assert_eq!(run.artifact_lineage.len(), 3);
    assert_eq!(run.artifact_lineage[1].artifact_key, "gate.if_true");
    assert_eq!(
        run.artifact_lineage[1].source_artifacts,
        vec!["summary_input.value".to_string()]
    );
}

#[test]
fn merges_active_condition_branch_back_into_single_lane() {
    let graph = WorkflowGraph {
        schema_version: "kyuubiki.workflow-graph/v1".to_string(),
        id: "workflow.condition-merge".to_string(),
        name: "Condition merge".to_string(),
        version: "1.0.0".to_string(),
        description: Some("Merge active branch after condition".to_string()),
        dataset_contract: None,
        entry_nodes: vec!["summary_input".to_string()],
        output_nodes: vec!["merged_output".to_string()],
        defaults: WorkflowDefaults::default(),
        nodes: vec![
            WorkflowNode {
                id: "summary_input".to_string(),
                kind: WorkflowNodeKind::Input,
                operator_id: None,
                name: None,
                description: None,
                config: None,
                cache_policy: None,
                inputs: vec![],
                outputs: vec![port("value")],
            },
            WorkflowNode {
                id: "gate".to_string(),
                kind: WorkflowNodeKind::Condition,
                operator_id: None,
                name: None,
                description: None,
                config: Some(serde_json::json!({
                    "predicate": {
                        "path": "summary.max_stress",
                        "operator": "gt",
                        "value": 10.0
                    }
                })),
                cache_policy: None,
                inputs: vec![port("value")],
                outputs: vec![port("if_true"), port("if_false")],
            },
            WorkflowNode {
                id: "join".to_string(),
                kind: WorkflowNodeKind::Transform,
                operator_id: Some("transform.first_available".to_string()),
                name: None,
                description: None,
                config: None,
                cache_policy: None,
                inputs: vec![port("left"), port("right")],
                outputs: vec![port("merged")],
            },
            WorkflowNode {
                id: "merged_output".to_string(),
                kind: WorkflowNodeKind::Output,
                operator_id: None,
                name: None,
                description: None,
                config: None,
                cache_policy: None,
                inputs: vec![port("result")],
                outputs: vec![],
            },
        ],
        edges: vec![
            WorkflowEdge {
                id: "input-to-gate".to_string(),
                from: WorkflowNodePortRef {
                    node: "summary_input".to_string(),
                    port: "value".to_string(),
                },
                to: WorkflowNodePortRef {
                    node: "gate".to_string(),
                    port: "value".to_string(),
                },
                artifact_type: "artifact/json".to_string(),
                dataset_value: None,
            },
            WorkflowEdge {
                id: "gate-true-to-join".to_string(),
                from: WorkflowNodePortRef {
                    node: "gate".to_string(),
                    port: "if_true".to_string(),
                },
                to: WorkflowNodePortRef {
                    node: "join".to_string(),
                    port: "left".to_string(),
                },
                artifact_type: "artifact/json".to_string(),
                dataset_value: None,
            },
            WorkflowEdge {
                id: "gate-false-to-join".to_string(),
                from: WorkflowNodePortRef {
                    node: "gate".to_string(),
                    port: "if_false".to_string(),
                },
                to: WorkflowNodePortRef {
                    node: "join".to_string(),
                    port: "right".to_string(),
                },
                artifact_type: "artifact/json".to_string(),
                dataset_value: None,
            },
            WorkflowEdge {
                id: "join-to-output".to_string(),
                from: WorkflowNodePortRef {
                    node: "join".to_string(),
                    port: "merged".to_string(),
                },
                to: WorkflowNodePortRef {
                    node: "merged_output".to_string(),
                    port: "result".to_string(),
                },
                artifact_type: "artifact/json".to_string(),
                dataset_value: None,
            },
        ],
    };

    let run = run_workflow_graph(WorkflowGraphRunRequest {
        graph,
        input_artifacts: BTreeMap::from([(
            "summary_input".to_string(),
            serde_json::json!({
                "summary": { "max_displacement": 0.4, "max_stress": 12.0 }
            }),
        )]),
    })
    .expect("condition merge workflow should run");

    assert!(run.artifacts.contains_key("join.merged"));
    assert!(run.artifacts.contains_key("merged_output.result"));
    assert_eq!(run.skipped_nodes.len(), 0);
    assert_eq!(run.branch_decisions.len(), 1);
    assert_eq!(run.branch_decisions[0].chosen_output, "if_true");
    assert_eq!(run.node_runs.len(), 4);
    assert!(run
        .artifact_lineage
        .iter()
        .any(|entry| entry.artifact_key == "join.merged"
            && entry.source_artifacts == vec!["gate.if_true".to_string()]));
    assert_eq!(
        run.artifacts.get("join.merged"),
        run.artifacts.get("gate.if_true")
    );
}
