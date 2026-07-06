use kyuubiki_engine::run_workflow_graph;
use kyuubiki_protocol::{
    WorkflowCachePolicy, WorkflowDefaults, WorkflowEdge, WorkflowGraph, WorkflowGraphRunRequest,
    WorkflowNode, WorkflowNodeKind, WorkflowNodePortRef, WorkflowPort,
};
use std::collections::BTreeMap;

#[test]
fn workflow_runs_nonlinear_spring_1d_solver() {
    let run = run_workflow_graph(WorkflowGraphRunRequest {
        graph: WorkflowGraph {
            schema_version: "kyuubiki.workflow-graph/v1".to_string(),
            id: "workflow.nonlinear-spring-1d".to_string(),
            name: "Nonlinear spring 1d".to_string(),
            version: "1.0.0".to_string(),
            description: None,
            dataset_contract: None,
            entry_nodes: vec!["spring_input".to_string()],
            output_nodes: vec!["spring_output".to_string()],
            defaults: WorkflowDefaults {
                cache_policy: Some(WorkflowCachePolicy::Cached),
                orchestrated: Some(true),
            },
            nodes: vec![input_node(), solve_node(), output_node()],
            edges: vec![
                edge(
                    "input_to_solve",
                    "spring_input",
                    "model",
                    "solve_spring",
                    "model",
                    "study_model/nonlinear_spring_1d",
                ),
                edge(
                    "solve_to_output",
                    "solve_spring",
                    "result",
                    "spring_output",
                    "result",
                    "result/nonlinear_spring_1d",
                ),
            ],
        },
        input_artifacts: BTreeMap::from([("spring_input".to_string(), model())]),
    })
    .expect("workflow nonlinear spring solve should succeed");

    let result = run
        .artifacts
        .get("spring_output.result")
        .expect("workflow output should contain nonlinear spring result");
    assert_eq!(result["converged"], true);
    assert!(result["steps"].as_array().unwrap().len() >= 2);
    assert!(result["max_force"].as_f64().unwrap() > 0.0);
}

fn input_node() -> WorkflowNode {
    WorkflowNode {
        id: "spring_input".to_string(),
        kind: WorkflowNodeKind::Input,
        operator_id: None,
        name: None,
        description: None,
        config: None,
        cache_policy: None,
        inputs: vec![],
        outputs: vec![port("model", "study_model/nonlinear_spring_1d")],
    }
}

fn solve_node() -> WorkflowNode {
    WorkflowNode {
        id: "solve_spring".to_string(),
        kind: WorkflowNodeKind::Solve,
        operator_id: Some("solve.nonlinear_spring_1d".to_string()),
        name: None,
        description: None,
        config: None,
        cache_policy: None,
        inputs: vec![port("model", "study_model/nonlinear_spring_1d")],
        outputs: vec![port("result", "result/nonlinear_spring_1d")],
    }
}

fn output_node() -> WorkflowNode {
    WorkflowNode {
        id: "spring_output".to_string(),
        kind: WorkflowNodeKind::Output,
        operator_id: None,
        name: None,
        description: None,
        config: None,
        cache_policy: None,
        inputs: vec![port("result", "result/nonlinear_spring_1d")],
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
            { "id": "nl0", "node_i": 0, "node_j": 1, "stiffness": 1000.0, "cubic_stiffness": 50000.0 }
        ],
        "load_steps": 6,
        "max_iterations": 32,
        "tolerance": 1.0e-9
    })
}
