use kyuubiki_engine::run_workflow_graph;
use kyuubiki_protocol::{
    WorkflowCachePolicy, WorkflowDefaults, WorkflowEdge, WorkflowGraph, WorkflowGraphRunRequest,
    WorkflowNode, WorkflowNodeKind, WorkflowNodePortRef, WorkflowPort,
};
use std::collections::BTreeMap;

#[test]
fn workflow_runs_solid_tetra_3d_solver() {
    let run = run_workflow_graph(WorkflowGraphRunRequest {
        graph: graph(),
        input_artifacts: BTreeMap::from([("solid_input".to_string(), solid_model())]),
    })
    .expect("workflow solid tetra solve should succeed");

    let result = run
        .artifacts
        .get("solid_output.result")
        .expect("workflow output should contain solid tetra result");
    assert_eq!(result["nodes"].as_array().unwrap().len(), 4);
    assert_eq!(result["elements"].as_array().unwrap().len(), 1);
    assert!(result["total_volume"].as_f64().unwrap() > 0.0);
    assert!(result["max_displacement"].as_f64().unwrap() > 0.0);
    assert!(result["max_von_mises_stress"].as_f64().unwrap() > 0.0);
    assert!(run.completed_nodes.contains(&"solve_solid".to_string()));
}

fn graph() -> WorkflowGraph {
    WorkflowGraph {
        schema_version: "kyuubiki.workflow-graph/v1".to_string(),
        id: "workflow.solid-tetra-3d".to_string(),
        name: "Solid tetra 3d".to_string(),
        version: "1.0.0".to_string(),
        description: None,
        dataset_contract: None,
        entry_nodes: vec!["solid_input".to_string()],
        output_nodes: vec!["solid_output".to_string()],
        defaults: WorkflowDefaults {
            cache_policy: Some(WorkflowCachePolicy::Cached),
            orchestrated: Some(true),
        },
        nodes: vec![input_node(), solve_node(), output_node()],
        edges: vec![
            edge(
                "input_to_solve",
                "solid_input",
                "model",
                "solve_solid",
                "model",
            ),
            edge(
                "solve_to_output",
                "solve_solid",
                "result",
                "solid_output",
                "result",
            ),
        ],
    }
}

fn input_node() -> WorkflowNode {
    WorkflowNode {
        id: "solid_input".to_string(),
        kind: WorkflowNodeKind::Input,
        operator_id: None,
        name: None,
        description: None,
        config: None,
        cache_policy: None,
        inputs: vec![],
        outputs: vec![port("model", "study_model/solid_tetra_3d")],
    }
}

fn solve_node() -> WorkflowNode {
    WorkflowNode {
        id: "solve_solid".to_string(),
        kind: WorkflowNodeKind::Solve,
        operator_id: Some("solve.solid_tetra_3d".to_string()),
        name: None,
        description: None,
        config: None,
        cache_policy: None,
        inputs: vec![port("model", "study_model/solid_tetra_3d")],
        outputs: vec![port("result", "result/solid_tetra_3d")],
    }
}

fn output_node() -> WorkflowNode {
    WorkflowNode {
        id: "solid_output".to_string(),
        kind: WorkflowNodeKind::Output,
        operator_id: None,
        name: None,
        description: None,
        config: None,
        cache_policy: None,
        inputs: vec![port("result", "result/solid_tetra_3d")],
        outputs: vec![],
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
        artifact_type: "result/solid_tetra_3d".to_string(),
        dataset_value: None,
    }
}

fn port(id: &str, artifact_type: &str) -> WorkflowPort {
    WorkflowPort {
        id: id.to_string(),
        artifact_type: artifact_type.to_string(),
        name: None,
        required: None,
        cardinality: None,
        dataset_value: None,
    }
}

fn solid_model() -> serde_json::Value {
    serde_json::json!({
        "nodes": [
            { "id": "n0", "x": 0.0, "y": 0.0, "z": 0.0, "fix_x": true, "fix_y": true, "fix_z": true, "load_x": 0.0, "load_y": 0.0, "load_z": 0.0 },
            { "id": "n1", "x": 1.0, "y": 0.0, "z": 0.0, "fix_x": true, "fix_y": true, "fix_z": true, "load_x": 0.0, "load_y": 0.0, "load_z": 0.0 },
            { "id": "n2", "x": 0.0, "y": 1.0, "z": 0.0, "fix_x": true, "fix_y": true, "fix_z": true, "load_x": 0.0, "load_y": 0.0, "load_z": 0.0 },
            { "id": "tip", "x": 0.0, "y": 0.0, "z": 1.0, "fix_x": false, "fix_y": false, "fix_z": false, "load_x": 0.0, "load_y": 0.0, "load_z": -1000.0 }
        ],
        "elements": [
            { "id": "tet0", "node_a": 0, "node_b": 1, "node_c": 2, "node_d": 3, "youngs_modulus": 70000000000.0, "poisson_ratio": 0.33 }
        ]
    })
}
