use crate::run_workflow_graph;
use kyuubiki_protocol::{
    WorkflowCachePolicy, WorkflowDefaults, WorkflowEdge, WorkflowGraph, WorkflowGraphRunRequest,
    WorkflowNode, WorkflowNodeKind, WorkflowNodePortRef, WorkflowPort,
};
use std::collections::BTreeMap;

#[test]
fn first_available_waits_for_all_predecessors_then_uses_edge_priority() {
    let graph = WorkflowGraph {
        schema_version: "kyuubiki.workflow-graph/v1".to_string(),
        id: "workflow.execution-plan.edge-priority".to_string(),
        name: "Execution plan edge priority".to_string(),
        version: "1.0.0".to_string(),
        description: None,
        dataset_contract: None,
        entry_nodes: vec!["input_a".to_string(), "input_b".to_string()],
        output_nodes: vec!["output".to_string()],
        defaults: WorkflowDefaults::default(),
        nodes: vec![
            input_node("input_a"),
            first_available_node(),
            input_node("input_b"),
            output_node(),
        ],
        edges: vec![
            edge("b-first", "input_b", "join", "left"),
            edge("a-second", "input_a", "join", "right"),
            edge("join-output", "join", "output", "result"),
        ],
    };
    let run = run_workflow_graph(WorkflowGraphRunRequest {
        graph,
        input_artifacts: BTreeMap::from([
            ("input_a".to_string(), serde_json::json!({"source": "a"})),
            ("input_b".to_string(), serde_json::json!({"source": "b"})),
        ]),
    })
    .expect("compiled execution plan should run unordered declarations");

    assert_eq!(
        run.completed_nodes,
        ["input_a", "input_b", "join", "output"]
    );
    assert_eq!(run.artifacts["join.merged"]["source"], "b");
    assert_eq!(run.artifacts["output.result"]["source"], "b");
}

#[test]
fn first_available_validates_direct_input_before_reusing_it() {
    let graph = WorkflowGraph {
        schema_version: "kyuubiki.workflow-graph/v1".to_string(),
        id: "workflow.execution-plan.identity-budget".to_string(),
        name: "Execution plan identity budget".to_string(),
        version: "1.0.0".to_string(),
        description: None,
        dataset_contract: None,
        entry_nodes: vec!["input_a".to_string()],
        output_nodes: vec!["output".to_string()],
        defaults: WorkflowDefaults::default(),
        nodes: vec![input_node("input_a"), first_available_node(), output_node()],
        edges: vec![
            edge("input-join", "input_a", "join", "left"),
            edge("join-output", "join", "output", "result"),
        ],
    };
    let error = run_workflow_graph(WorkflowGraphRunRequest {
        graph,
        input_artifacts: BTreeMap::from([(
            "input_a".to_string(),
            serde_json::json!({"value": "x".repeat(500_001)}),
        )]),
    })
    .expect_err("identity transform must apply the stricter output artifact budget");

    assert!(error.contains("workflow node join output"));
    assert!(error.contains("string exceeds length security budget"));
}

#[test]
fn ephemeral_identity_artifact_is_released_after_its_last_consumer() {
    let mut join = first_available_node();
    join.cache_policy = Some(WorkflowCachePolicy::Ephemeral);
    let graph = WorkflowGraph {
        schema_version: "kyuubiki.workflow-graph/v1".to_string(),
        id: "workflow.execution-plan.ephemeral-retention".to_string(),
        name: "Execution plan ephemeral retention".to_string(),
        version: "1.0.0".to_string(),
        description: None,
        dataset_contract: None,
        entry_nodes: vec!["input_a".to_string()],
        output_nodes: vec!["output".to_string()],
        defaults: WorkflowDefaults::default(),
        nodes: vec![input_node("input_a"), join, output_node()],
        edges: vec![
            edge("input-join", "input_a", "join", "left"),
            edge("join-output", "join", "output", "result"),
        ],
    };
    let run = run_workflow_graph(WorkflowGraphRunRequest {
        graph,
        input_artifacts: BTreeMap::from([(
            "input_a".to_string(),
            serde_json::json!({"source": "input"}),
        )]),
    })
    .expect("ephemeral identity chain should run");

    assert!(!run.artifacts.contains_key("join.merged"));
    assert_eq!(run.artifacts["output.result"]["source"], "input");
    assert!(
        run.artifact_lineage
            .iter()
            .any(|entry| entry.artifact_key == "join.merged")
    );
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

fn first_available_node() -> WorkflowNode {
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
    }
}

fn output_node() -> WorkflowNode {
    WorkflowNode {
        id: "output".to_string(),
        kind: WorkflowNodeKind::Output,
        operator_id: None,
        name: None,
        description: None,
        config: None,
        cache_policy: None,
        inputs: vec![port("result")],
        outputs: vec![],
    }
}

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

fn edge(id: &str, source: &str, target: &str, target_port: &str) -> WorkflowEdge {
    WorkflowEdge {
        id: id.to_string(),
        from: WorkflowNodePortRef {
            node: source.to_string(),
            port: if source == "join" { "merged" } else { "value" }.to_string(),
        },
        to: WorkflowNodePortRef {
            node: target.to_string(),
            port: target_port.to_string(),
        },
        artifact_type: "artifact/json".to_string(),
        dataset_value: None,
    }
}
