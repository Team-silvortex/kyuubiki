use kyuubiki_engine::run_workflow_graph;
use kyuubiki_protocol::{
    WorkflowCachePolicy, WorkflowDefaults, WorkflowEdge, WorkflowGraph, WorkflowGraphRunRequest,
    WorkflowNode, WorkflowNodeKind, WorkflowNodePortRef, WorkflowPort,
};
use std::collections::BTreeMap;

#[test]
fn workflow_runs_history_dependent_cohesive_interface_solver() {
    let run = run_workflow_graph(WorkflowGraphRunRequest {
        graph: graph(),
        input_artifacts: BTreeMap::from([("interface_input".to_string(), model())]),
    })
    .expect("workflow cohesive solve should succeed");

    let result = run
        .artifacts
        .get("interface_output.result")
        .expect("workflow output should contain cohesive result");
    assert_eq!(result["steps"].as_array().unwrap().len(), 4);
    assert!(result["max_damage"].as_f64().unwrap() > 0.0);
    assert_eq!(result["fully_failed"], false);
    assert_eq!(result["steps"][3]["regime"], "unloading_reloading");
}

fn graph() -> WorkflowGraph {
    WorkflowGraph {
        schema_version: "kyuubiki.workflow-graph/v1".to_string(),
        id: "workflow.cohesive-interface-1d".to_string(),
        name: "Cohesive interface 1d".to_string(),
        version: "1.0.0".to_string(),
        description: None,
        dataset_contract: None,
        entry_nodes: vec!["interface_input".to_string()],
        output_nodes: vec!["interface_output".to_string()],
        defaults: WorkflowDefaults {
            cache_policy: Some(WorkflowCachePolicy::Cached),
            orchestrated: Some(true),
        },
        nodes: vec![
            node(
                "interface_input",
                WorkflowNodeKind::Input,
                None,
                vec![],
                vec![port("model", "study_model/cohesive_interface_1d")],
            ),
            node(
                "solve_interface",
                WorkflowNodeKind::Solve,
                Some("solve.cohesive_interface_1d"),
                vec![port("model", "study_model/cohesive_interface_1d")],
                vec![port("result", "result/cohesive_interface_1d")],
            ),
            node(
                "interface_output",
                WorkflowNodeKind::Output,
                None,
                vec![port("result", "result/cohesive_interface_1d")],
                vec![],
            ),
        ],
        edges: vec![
            edge(
                "input_to_solve",
                "interface_input",
                "model",
                "solve_interface",
                "model",
                "study_model/cohesive_interface_1d",
            ),
            edge(
                "solve_to_output",
                "solve_interface",
                "result",
                "interface_output",
                "result",
                "result/cohesive_interface_1d",
            ),
        ],
    }
}

fn node(
    id: &str,
    kind: WorkflowNodeKind,
    operator_id: Option<&str>,
    inputs: Vec<WorkflowPort>,
    outputs: Vec<WorkflowPort>,
) -> WorkflowNode {
    WorkflowNode {
        id: id.to_string(),
        kind,
        operator_id: operator_id.map(str::to_string),
        name: None,
        description: None,
        config: None,
        cache_policy: None,
        inputs,
        outputs,
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
        "id": "interface-0",
        "initial_stiffness": 1000.0,
        "compression_stiffness": 2000.0,
        "peak_traction": 10.0,
        "failure_separation": 0.05,
        "separation_history": [0.0, 0.01, 0.03, 0.015]
    })
}
