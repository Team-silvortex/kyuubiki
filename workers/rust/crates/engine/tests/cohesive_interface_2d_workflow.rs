use kyuubiki_engine::run_workflow_graph;
use kyuubiki_protocol::{
    WorkflowCachePolicy, WorkflowDefaults, WorkflowEdge, WorkflowGraph, WorkflowGraphRunRequest,
    WorkflowNode, WorkflowNodeKind, WorkflowNodePortRef, WorkflowPort,
};
use std::collections::BTreeMap;

#[test]
fn workflow_runs_zero_thickness_cohesive_interface_2d() {
    let run = run_workflow_graph(WorkflowGraphRunRequest {
        graph: graph(),
        input_artifacts: BTreeMap::from([("interface_input".to_string(), model())]),
    })
    .expect("workflow cohesive interface 2d solve should succeed");

    let result = run
        .artifacts
        .get("interface_output.result")
        .expect("workflow output should contain cohesive interface result");
    assert_eq!(result["steps"].as_array().unwrap().len(), 1);
    assert_eq!(result["interface_length"], 1.0);
    assert!(result["max_normal_damage"].as_f64().unwrap() > 0.0);
    assert!(result["max_shear_damage"].as_f64().unwrap() > 0.0);
    assert_eq!(
        result["steps"][0]["element_nodal_internal_forces"]
            .as_array()
            .unwrap()
            .len(),
        4
    );
}

fn graph() -> WorkflowGraph {
    WorkflowGraph {
        schema_version: "kyuubiki.workflow-graph/v1".to_string(),
        id: "workflow.cohesive-interface-2d".to_string(),
        name: "Cohesive interface 2d".to_string(),
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
                vec![port("model", "study_model/cohesive_interface_2d")],
            ),
            node(
                "solve_interface",
                WorkflowNodeKind::Solve,
                Some("solve.cohesive_interface_2d"),
                vec![port("model", "study_model/cohesive_interface_2d")],
                vec![port("result", "result/cohesive_interface_2d")],
            ),
            node(
                "interface_output",
                WorkflowNodeKind::Output,
                None,
                vec![port("result", "result/cohesive_interface_2d")],
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
                "study_model/cohesive_interface_2d",
            ),
            edge(
                "solve_to_output",
                "solve_interface",
                "result",
                "interface_output",
                "result",
                "result/cohesive_interface_2d",
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
        "nodes": [
            {"id": "lower-i", "x": 0.0, "y": 0.0},
            {"id": "lower-j", "x": 1.0, "y": 0.0},
            {"id": "upper-i", "x": 0.0, "y": 0.0},
            {"id": "upper-j", "x": 1.0, "y": 0.0}
        ],
        "element": {
            "id": "interface-0",
            "lower_i": 0,
            "lower_j": 1,
            "upper_i": 2,
            "upper_j": 3,
            "thickness": 1.0
        },
        "material": {
            "normal_initial_stiffness": 1000.0,
            "normal_compression_stiffness": 2000.0,
            "normal_peak_traction": 10.0,
            "normal_failure_separation": 0.05,
            "shear_initial_stiffness": 500.0,
            "shear_peak_traction": 5.0,
            "shear_failure_separation": 0.05
        },
        "displacement_history": [{
            "nodal_displacements": [
                [0.0, 0.0],
                [0.0, 0.0],
                [0.03, 0.03],
                [0.03, 0.03]
            ]
        }]
    })
}
