use crate::run_workflow_graph;
use kyuubiki_protocol::{
    WorkflowDefaults, WorkflowEdge, WorkflowGraph, WorkflowGraphRunRequest, WorkflowNode,
    WorkflowNodeKind, WorkflowNodePortRef, WorkflowNodeRunStatus, WorkflowPort,
};
use std::collections::BTreeMap;

#[test]
fn panic_boundary_converts_node_panic_to_error() {
    let error = crate::workflow::run_with_panic_boundary("panic_node", || -> Result<(), String> {
        panic!("synthetic panic")
    })
    .expect_err("panic should be converted into an error");

    assert!(error.contains("workflow node panic_node panicked"));
    assert!(error.contains("synthetic panic"));
}

#[test]
fn recoverable_node_failure_does_not_cascade_to_independent_branch() {
    let run = run_workflow_graph(WorkflowGraphRunRequest {
        graph: recovery_graph(true),
        input_artifacts: BTreeMap::from([
            ("main_input".to_string(), serde_json::json!({ "value": 7 })),
            ("bad_input".to_string(), serde_json::json!({ "value": 3 })),
        ]),
    })
    .expect("recoverable workflow should complete");

    assert_eq!(run.failed_nodes, vec!["recoverable_condition"]);
    assert_eq!(run.skipped_nodes, vec!["skipped_output"]);
    assert!(run.completed_nodes.contains(&"main_output".to_string()));
    assert_eq!(
        run.artifacts.get("main_output.result"),
        Some(&serde_json::json!({ "value": 7 }))
    );

    let failed_trace = run
        .node_runs
        .iter()
        .find(|trace| trace.node_id == "recoverable_condition")
        .expect("failed node trace");
    assert_eq!(failed_trace.status, WorkflowNodeRunStatus::Failed);
    assert!(
        failed_trace
            .error_message
            .as_deref()
            .is_some_and(|message| message.contains("unsupported condition operator"))
    );
}

#[test]
fn node_failure_without_recovery_policy_still_fails_fast() {
    let error = run_workflow_graph(WorkflowGraphRunRequest {
        graph: recovery_graph(false),
        input_artifacts: BTreeMap::from([
            ("main_input".to_string(), serde_json::json!({ "value": 7 })),
            ("bad_input".to_string(), serde_json::json!({ "value": 3 })),
        ]),
    })
    .expect_err("non-recoverable workflow should fail");

    assert!(error.contains("workflow node recoverable_condition failed"));
    assert!(error.contains("unsupported condition operator"));
}

#[test]
fn nonfinite_validation_error_preserves_independent_work_and_diagnostics() {
    for recover in [false, true] {
        let mut graph = recovery_graph(recover);
        let validation = graph
            .nodes
            .iter_mut()
            .find(|node| node.id == "recoverable_condition")
            .unwrap();
        validation.kind = WorkflowNodeKind::Transform;
        validation.operator_id = Some("transform.validate_summary_tolerance".to_string());
        validation.config = Some(if recover {
            serde_json::json!({"on_error": "skip"})
        } else {
            serde_json::json!({})
        });
        validation.inputs = vec![port("left"), port("right")];
        validation.outputs = vec![port("result")];
        graph.edges[1].to.port = "left".to_string();
        graph.edges[2].from.port = "result".to_string();
        graph.nodes.push(input_node("reference_input"));
        graph.entry_nodes.push("reference_input".to_string());
        graph.edges.push(edge(
            "reference-validation",
            "reference_input",
            "value",
            "recoverable_condition",
            "right",
        ));
        let result = run_workflow_graph(WorkflowGraphRunRequest {
            graph,
            input_artifacts: BTreeMap::from([
                ("main_input".to_string(), serde_json::json!({"value": 7})),
                (
                    "bad_input".to_string(),
                    serde_json::json!({"stress": -1.0e308}),
                ),
                (
                    "reference_input".to_string(),
                    serde_json::json!({"stress": 1.0e308}),
                ),
            ]),
        });
        if !recover {
            let error = result.expect_err("default validation errors fail the workflow");
            assert!(
                error.contains("stress") && error.contains("non-finite"),
                "{error}"
            );
            continue;
        }
        let run = result.expect("opt-in recovery must preserve the independent branch");
        assert_eq!(run.failed_nodes, vec!["recoverable_condition"]);
        assert_eq!(run.skipped_nodes, vec!["skipped_output"]);
        assert_eq!(
            run.artifacts["main_output.result"],
            serde_json::json!({"value": 7})
        );
        assert!(!run.artifacts.contains_key("recoverable_condition.result"));
        let trace = run
            .node_runs
            .iter()
            .find(|trace| trace.node_id == "recoverable_condition")
            .unwrap();
        assert_eq!(trace.status, WorkflowNodeRunStatus::Failed);
        let error = trace.error_message.as_deref().expect("failure diagnostic");
        assert!(
            error.contains("stress") && error.contains("non-finite"),
            "{error}"
        );
    }
}

fn recovery_graph(recover_condition: bool) -> WorkflowGraph {
    WorkflowGraph {
        schema_version: "kyuubiki.workflow-graph/v1".to_string(),
        id: "workflow.recovery".to_string(),
        name: "Recovery smoke graph".to_string(),
        version: "1.0.0".to_string(),
        description: None,
        dataset_contract: None,
        entry_nodes: vec!["main_input".to_string(), "bad_input".to_string()],
        output_nodes: vec!["main_output".to_string()],
        defaults: WorkflowDefaults::default(),
        nodes: vec![
            input_node("main_input"),
            input_node("bad_input"),
            condition_node(recover_condition),
            output_node("skipped_output", "value"),
            output_node("main_output", "result"),
        ],
        edges: vec![
            edge("main-main", "main_input", "value", "main_output", "result"),
            edge(
                "bad-condition",
                "bad_input",
                "value",
                "recoverable_condition",
                "value",
            ),
            edge(
                "condition-skipped",
                "recoverable_condition",
                "if_true",
                "skipped_output",
                "value",
            ),
        ],
    }
}

fn input_node(id: &str) -> WorkflowNode {
    WorkflowNode {
        id: id.to_string(),
        kind: WorkflowNodeKind::Input,
        operator_id: None,
        name: None,
        description: None,
        config: None,
        cache_policy: None,
        inputs: vec![],
        outputs: vec![port("value")],
    }
}

fn condition_node(recover: bool) -> WorkflowNode {
    let mut config = serde_json::json!({
        "predicate": { "operator": "unsupported_for_recovery_test" }
    });
    if recover {
        config["on_error"] = serde_json::json!("skip");
    }

    WorkflowNode {
        id: "recoverable_condition".to_string(),
        kind: WorkflowNodeKind::Condition,
        operator_id: None,
        name: None,
        description: None,
        config: Some(config),
        cache_policy: None,
        inputs: vec![port("value")],
        outputs: vec![port("if_true"), port("if_false")],
    }
}

fn output_node(id: &str, input_port: &str) -> WorkflowNode {
    WorkflowNode {
        id: id.to_string(),
        kind: WorkflowNodeKind::Output,
        operator_id: None,
        name: None,
        description: None,
        config: None,
        cache_policy: None,
        inputs: vec![port(input_port)],
        outputs: vec![],
    }
}

fn port(id: &str) -> WorkflowPort {
    WorkflowPort {
        id: id.to_string(),
        artifact_type: "generic/json".to_string(),
        name: None,
        required: None,
        cardinality: None,
        dataset_value: None,
    }
}

fn edge(id: &str, from_node: &str, from_port: &str, to_node: &str, to_port: &str) -> WorkflowEdge {
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
        artifact_type: "generic/json".to_string(),
        dataset_value: None,
    }
}
