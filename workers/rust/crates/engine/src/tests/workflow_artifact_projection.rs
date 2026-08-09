use crate::{run_workflow_graph, run_workflow_graph_with_options};
use kyuubiki_protocol::{
    WorkflowArtifactProjection, WorkflowDefaults, WorkflowEdge, WorkflowGraph,
    WorkflowGraphRunOptions, WorkflowGraphRunRequest, WorkflowNode, WorkflowNodeKind,
    WorkflowNodePortRef, WorkflowPort,
};
use std::collections::BTreeMap;

#[test]
fn projects_workflow_artifacts_without_changing_execution_evidence() {
    let full = run_workflow_graph(request()).expect("full workflow should run");
    let outputs =
        run_workflow_graph_with_options(request(), options(WorkflowArtifactProjection::Outputs))
            .expect("output-projected workflow should run");
    let none =
        run_workflow_graph_with_options(request(), options(WorkflowArtifactProjection::None))
            .expect("artifact-free workflow should run");

    assert_eq!(full.artifacts.len(), 3);
    assert_eq!(full.artifacts["output.result"]["value"], 42);
    assert_eq!(outputs.artifacts.len(), 1);
    assert_eq!(outputs.artifacts["output.result"]["value"], 42);
    assert!(none.artifacts.is_empty());
    assert_eq!(outputs.completed_nodes, full.completed_nodes);
    assert_eq!(none.completed_nodes, full.completed_nodes);
    assert_eq!(outputs.node_runs, full.node_runs);
    assert_eq!(none.node_runs, full.node_runs);
    assert_eq!(outputs.artifact_lineage, full.artifact_lineage);
    assert_eq!(none.artifact_lineage, full.artifact_lineage);
}

fn options(artifact_projection: WorkflowArtifactProjection) -> WorkflowGraphRunOptions {
    WorkflowGraphRunOptions {
        artifact_projection,
    }
}

fn request() -> WorkflowGraphRunRequest {
    WorkflowGraphRunRequest {
        graph: WorkflowGraph {
            schema_version: "kyuubiki.workflow-graph/v1".to_string(),
            id: "workflow.artifact-projection".to_string(),
            name: "Artifact projection".to_string(),
            version: "1.0.0".to_string(),
            description: None,
            dataset_contract: None,
            entry_nodes: vec!["input".to_string()],
            output_nodes: vec!["output".to_string()],
            defaults: WorkflowDefaults::default(),
            nodes: vec![input_node(), pass_node(), output_node()],
            edges: vec![
                edge("input.value", "input", "value", "pass", "input"),
                edge("pass.result", "pass", "result", "output", "result"),
            ],
        },
        input_artifacts: BTreeMap::from([("input".to_string(), serde_json::json!({"value": 42}))]),
    }
}

fn input_node() -> WorkflowNode {
    WorkflowNode {
        id: "input".to_string(),
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

fn pass_node() -> WorkflowNode {
    WorkflowNode {
        id: "pass".to_string(),
        kind: WorkflowNodeKind::Transform,
        operator_id: Some("transform.first_available".to_string()),
        name: None,
        description: None,
        config: None,
        cache_policy: None,
        inputs: vec![port("input")],
        outputs: vec![port("result")],
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
        artifact_type: "artifact/json".to_string(),
        dataset_value: None,
    }
}
