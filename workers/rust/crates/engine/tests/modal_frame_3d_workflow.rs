use kyuubiki_engine::run_workflow_graph;
use kyuubiki_protocol::{
    WorkflowCachePolicy, WorkflowDefaults, WorkflowEdge, WorkflowGraph, WorkflowGraphRunRequest,
    WorkflowNode, WorkflowNodeKind, WorkflowNodePortRef, WorkflowPort,
};
use std::collections::BTreeMap;

#[test]
fn workflow_runs_modal_frame_3d_solver() {
    let run = run_workflow_graph(WorkflowGraphRunRequest {
        graph: graph(),
        input_artifacts: BTreeMap::from([("modal_input".to_string(), modal_model())]),
    })
    .expect("workflow 3d modal solve should succeed");

    let result = run
        .artifacts
        .get("modal_output.result")
        .expect("workflow output should contain modal result");
    assert_eq!(result["modes"].as_array().unwrap().len(), 3);
    assert!(result["min_frequency_hz"].as_f64().unwrap() > 0.0);
    assert_eq!(result["free_dofs"].as_array().unwrap().len(), 6);
    assert!(run.completed_nodes.contains(&"solve_modal".to_string()));
}

fn graph() -> WorkflowGraph {
    WorkflowGraph {
        schema_version: "kyuubiki.workflow-graph/v1".to_string(),
        id: "workflow.modal-frame-3d".to_string(),
        name: "Modal frame 3d".to_string(),
        version: "1.0.0".to_string(),
        description: None,
        dataset_contract: None,
        entry_nodes: vec!["modal_input".to_string()],
        output_nodes: vec!["modal_output".to_string()],
        defaults: WorkflowDefaults {
            cache_policy: Some(WorkflowCachePolicy::Cached),
            orchestrated: Some(true),
        },
        nodes: vec![input_node(), solve_node(), output_node()],
        edges: vec![
            edge(
                "input_to_solve",
                "modal_input",
                "model",
                "solve_modal",
                "model",
                "study_model/modal_frame_3d",
            ),
            edge(
                "solve_to_output",
                "solve_modal",
                "result",
                "modal_output",
                "result",
                "result/modal_frame_3d",
            ),
        ],
    }
}

fn input_node() -> WorkflowNode {
    WorkflowNode {
        id: "modal_input".to_string(),
        kind: WorkflowNodeKind::Input,
        operator_id: None,
        name: None,
        description: None,
        config: None,
        cache_policy: None,
        inputs: vec![],
        outputs: vec![port("model", "study_model/modal_frame_3d")],
    }
}

fn solve_node() -> WorkflowNode {
    WorkflowNode {
        id: "solve_modal".to_string(),
        kind: WorkflowNodeKind::Solve,
        operator_id: Some("solve.modal_frame_3d".to_string()),
        name: None,
        description: None,
        config: None,
        cache_policy: None,
        inputs: vec![port("model", "study_model/modal_frame_3d")],
        outputs: vec![port("result", "result/modal_frame_3d")],
    }
}

fn output_node() -> WorkflowNode {
    WorkflowNode {
        id: "modal_output".to_string(),
        kind: WorkflowNodeKind::Output,
        operator_id: None,
        name: None,
        description: None,
        config: None,
        cache_policy: None,
        inputs: vec![port("result", "result/modal_frame_3d")],
        outputs: vec![],
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

fn modal_model() -> serde_json::Value {
    serde_json::json!({
        "nodes": [
            { "id": "n0", "x": 0.0, "y": 0.0, "z": 0.0, "fix_x": true, "fix_y": true, "fix_z": true, "fix_rx": true, "fix_ry": true, "fix_rz": true, "load_x": 0.0, "load_y": 0.0, "load_z": 0.0, "moment_x": 0.0, "moment_y": 0.0, "moment_z": 0.0 },
            { "id": "n1", "x": 2.0, "y": 0.0, "z": 0.0, "fix_x": false, "fix_y": false, "fix_z": false, "fix_rx": false, "fix_ry": false, "fix_rz": false, "load_x": 0.0, "load_y": 0.0, "load_z": 0.0, "moment_x": 0.0, "moment_y": 0.0, "moment_z": 0.0 }
        ],
        "elements": [
            { "id": "e0", "node_i": 0, "node_j": 1, "area": 0.01, "youngs_modulus": 210000000000.0, "shear_modulus": 80000000000.0, "torsion_constant": 0.00001, "moment_of_inertia_y": 0.000008333, "moment_of_inertia_z": 0.000008333, "density": 7850.0 }
        ],
        "mode_count": 3
    })
}
