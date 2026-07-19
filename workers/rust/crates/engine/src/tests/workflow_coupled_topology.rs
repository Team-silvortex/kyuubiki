use crate::{analyze_workflow_topology, run_workflow_graph};
use kyuubiki_protocol::{
    WorkflowDefaults, WorkflowEdge, WorkflowGraph, WorkflowGraphRunRequest, WorkflowNode,
    WorkflowNodeKind, WorkflowNodePortRef, WorkflowPort,
};
use std::collections::BTreeMap;

const LAYER_COUNT: usize = 20;
const ARTIFACT_TYPE: &str = "artifact/coupled_workflow_payload";

#[test]
fn runs_twenty_layer_fan_out_fan_in_coupled_topology() {
    let graph = WorkflowGraph {
        schema_version: "kyuubiki.workflow-graph/v1".to_string(),
        id: "workflow.coupled-topology-20-layer".to_string(),
        name: "Twenty-layer coupled topology baseline".to_string(),
        version: "1.0.0".to_string(),
        description: Some(
            "Twenty fan-out/fan-in stages establish the 3.0 composite workflow baseline."
                .to_string(),
        ),
        dataset_contract: None,
        entry_nodes: vec!["source".to_string()],
        output_nodes: vec!["output".to_string()],
        defaults: WorkflowDefaults::default(),
        nodes: build_nodes(),
        edges: build_edges(),
    };
    let topology = analyze_workflow_topology(&graph).expect("topology should be valid");
    assert_eq!(topology.node_count, 62);
    assert_eq!(topology.edge_count, 81);
    assert_eq!(topology.dependency_layers, 42);
    assert_eq!(topology.max_parallel_width, 2);

    let run = run_workflow_graph(WorkflowGraphRunRequest {
        graph,
        input_artifacts: BTreeMap::from([(
            "source".to_string(),
            serde_json::json!({ "case_id": "topology-baseline", "value": 42 }),
        )]),
    })
    .expect("twenty-layer coupled topology should run");

    assert_eq!(run.completed_nodes.len(), 2 + LAYER_COUNT * 3);
    assert!(run.failed_nodes.is_empty());
    assert!(run.skipped_nodes.is_empty());
    assert_eq!(
        run.artifacts.get("output.result"),
        Some(&serde_json::json!({ "case_id": "topology-baseline", "value": 42 }))
    );
}

fn build_nodes() -> Vec<WorkflowNode> {
    let mut nodes = vec![input_node("source"), output_node("output")];
    for layer in 0..LAYER_COUNT {
        nodes.push(pass_through_node(
            &branch_id(layer, "left"),
            vec![port("input")],
        ));
        nodes.push(pass_through_node(
            &branch_id(layer, "right"),
            vec![port("input")],
        ));
        nodes.push(pass_through_node(
            &join_id(layer),
            vec![port("left"), port("right")],
        ));
    }
    nodes
}

fn build_edges() -> Vec<WorkflowEdge> {
    let mut edges = Vec::with_capacity(LAYER_COUNT * 4 + 1);
    for layer in 0..LAYER_COUNT {
        let source = if layer == 0 {
            "source".to_string()
        } else {
            join_id(layer - 1)
        };
        for branch in ["left", "right"] {
            edges.push(edge(
                &format!("layer-{layer}-{branch}"),
                &source,
                &branch_id(layer, branch),
                "input",
            ));
        }
        for branch in ["left", "right"] {
            edges.push(edge(
                &format!("layer-{layer}-{branch}-join"),
                &branch_id(layer, branch),
                &join_id(layer),
                branch,
            ));
        }
    }
    edges.push(edge(
        "final-output",
        &join_id(LAYER_COUNT - 1),
        "output",
        "result",
    ));
    edges
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
        outputs: vec![port("result")],
    }
}

fn pass_through_node(id: &str, inputs: Vec<WorkflowPort>) -> WorkflowNode {
    WorkflowNode {
        id: id.to_string(),
        kind: WorkflowNodeKind::Transform,
        operator_id: Some("transform.first_available".to_string()),
        name: None,
        description: None,
        config: None,
        cache_policy: None,
        inputs,
        outputs: vec![port("result")],
    }
}

fn output_node(id: &str) -> WorkflowNode {
    WorkflowNode {
        id: id.to_string(),
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

fn edge(id: &str, from_node: &str, to_node: &str, to_port: &str) -> WorkflowEdge {
    WorkflowEdge {
        id: id.to_string(),
        from: WorkflowNodePortRef {
            node: from_node.to_string(),
            port: "result".to_string(),
        },
        to: WorkflowNodePortRef {
            node: to_node.to_string(),
            port: to_port.to_string(),
        },
        artifact_type: ARTIFACT_TYPE.to_string(),
        dataset_value: None,
    }
}

fn port(id: &str) -> WorkflowPort {
    WorkflowPort {
        id: id.to_string(),
        artifact_type: ARTIFACT_TYPE.to_string(),
        name: None,
        required: None,
        cardinality: None,
        dataset_value: None,
    }
}

fn branch_id(layer: usize, branch: &str) -> String {
    format!("layer_{layer:02}_{branch}")
}

fn join_id(layer: usize) -> String {
    format!("layer_{layer:02}_join")
}
