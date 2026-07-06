use kyuubiki_engine::run_workflow_graph;
use kyuubiki_protocol::{
    WorkflowCachePolicy, WorkflowDefaults, WorkflowEdge, WorkflowGraph, WorkflowGraphRunRequest,
    WorkflowNode, WorkflowNodeKind, WorkflowNodePortRef, WorkflowPort,
};
use std::collections::BTreeMap;

#[test]
fn workflow_runs_contact_gap_1d_solver() {
    let run = run_workflow_graph(WorkflowGraphRunRequest {
        graph: WorkflowGraph {
            schema_version: "kyuubiki.workflow-graph/v1".to_string(),
            id: "workflow.contact-gap-1d".to_string(),
            name: "Contact gap 1d".to_string(),
            version: "1.0.0".to_string(),
            description: None,
            dataset_contract: None,
            entry_nodes: vec!["contact_input".to_string()],
            output_nodes: vec!["contact_output".to_string()],
            defaults: WorkflowDefaults {
                cache_policy: Some(WorkflowCachePolicy::Cached),
                orchestrated: Some(true),
            },
            nodes: vec![input_node(), solve_node(), output_node()],
            edges: vec![
                edge(
                    "input_to_solve",
                    "contact_input",
                    "model",
                    "solve_contact",
                    "model",
                    "study_model/contact_gap_1d",
                ),
                edge(
                    "solve_to_output",
                    "solve_contact",
                    "result",
                    "contact_output",
                    "result",
                    "result/contact_gap_1d",
                ),
            ],
        },
        input_artifacts: BTreeMap::from([("contact_input".to_string(), model())]),
    })
    .expect("workflow contact solve should succeed");

    let result = run
        .artifacts
        .get("contact_output.result")
        .expect("workflow output should contain contact result");
    assert_eq!(result["converged"], true);
    assert_eq!(result["active_contact_count"], 1);
    assert!(result["max_contact_force"].as_f64().unwrap() > 0.0);
}

fn input_node() -> WorkflowNode {
    WorkflowNode {
        id: "contact_input".to_string(),
        kind: WorkflowNodeKind::Input,
        operator_id: None,
        name: None,
        description: None,
        config: None,
        cache_policy: None,
        inputs: vec![],
        outputs: vec![port("model", "study_model/contact_gap_1d")],
    }
}

fn solve_node() -> WorkflowNode {
    WorkflowNode {
        id: "solve_contact".to_string(),
        kind: WorkflowNodeKind::Solve,
        operator_id: Some("solve.contact_gap_1d".to_string()),
        name: None,
        description: None,
        config: None,
        cache_policy: None,
        inputs: vec![port("model", "study_model/contact_gap_1d")],
        outputs: vec![port("result", "result/contact_gap_1d")],
    }
}

fn output_node() -> WorkflowNode {
    WorkflowNode {
        id: "contact_output".to_string(),
        kind: WorkflowNodeKind::Output,
        operator_id: None,
        name: None,
        description: None,
        config: None,
        cache_policy: None,
        inputs: vec![port("result", "result/contact_gap_1d")],
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

fn model() -> serde_json::Value {
    serde_json::json!({
        "nodes": [
            { "id": "fixed", "x": 0.0, "fix_x": true, "load_x": 0.0 },
            { "id": "tip", "x": 1.0, "fix_x": false, "load_x": 100.0 }
        ],
        "elements": [
            { "id": "spring", "node_i": 0, "node_j": 1, "stiffness": 1000.0, "cubic_stiffness": 0.0 }
        ],
        "contacts": [
            { "id": "stop", "node": 1, "gap": 0.05, "normal_stiffness": 10000.0 }
        ],
        "load_steps": 6,
        "max_iterations": 32,
        "tolerance": 1.0e-9
    })
}
