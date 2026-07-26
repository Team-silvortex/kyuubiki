use std::collections::BTreeMap;

use kyuubiki_engine::{EngineSolveRequest, chunk_result, run_workflow_graph, solve};
use kyuubiki_protocol::{
    AnalysisResult, ResultChunkKind, ResultChunkRequest, SolveCohesiveInterfaceMesh2dRequest,
    WorkflowCachePolicy, WorkflowDefaults, WorkflowEdge, WorkflowGraph, WorkflowGraphRunRequest,
    WorkflowNode, WorkflowNodeKind, WorkflowNodePortRef, WorkflowPort,
};

#[test]
fn workflow_runs_incremental_cohesive_interface_mesh_2d() {
    let run = run_workflow_graph(WorkflowGraphRunRequest {
        graph: graph(),
        input_artifacts: BTreeMap::from([("mesh_input".to_string(), model())]),
    })
    .expect("cohesive interface mesh workflow should succeed");

    let result = run
        .artifacts
        .get("mesh_output.result")
        .expect("workflow output should contain cohesive mesh result");
    assert_eq!(result["converged"], true);
    assert_eq!(result["nodes"].as_array().unwrap().len(), 4);
    assert_eq!(result["elements"].as_array().unwrap().len(), 1);
    assert_eq!(result["steps"].as_array().unwrap().len(), 2);
    assert!((result["nodes"][2]["displacement"][1].as_f64().unwrap() - 0.005).abs() < 1.0e-10);
    assert_eq!(
        result["_solver_provenance"]["operator_id"],
        "solve.cohesive_interface_mesh_2d"
    );
}

#[test]
fn cohesive_mesh_results_support_node_and_element_chunks() {
    let request: SolveCohesiveInterfaceMesh2dRequest =
        serde_json::from_value(model()).expect("mesh fixture should decode");
    let result = solve(EngineSolveRequest::CohesiveInterfaceMesh2d(request))
        .expect("engine cohesive mesh should solve");
    assert!(matches!(result, AnalysisResult::CohesiveInterfaceMesh2d(_)));

    let nodes = chunk_result(
        &result,
        &ResultChunkRequest {
            kind: ResultChunkKind::Nodes,
            offset: 1,
            limit: 2,
        },
    )
    .expect("node chunk should encode");
    let elements = chunk_result(
        &result,
        &ResultChunkRequest {
            kind: ResultChunkKind::Elements,
            offset: 0,
            limit: 1,
        },
    )
    .expect("element chunk should encode");
    assert_eq!(nodes.items.len(), 2);
    assert_eq!(nodes.total, 4);
    assert_eq!(elements.items.len(), 1);
    assert_eq!(elements.total, 1);
}

fn graph() -> WorkflowGraph {
    WorkflowGraph {
        schema_version: "kyuubiki.workflow-graph/v1".to_string(),
        id: "workflow.cohesive-interface-mesh-2d".to_string(),
        name: "Cohesive interface mesh 2d".to_string(),
        version: "1.0.0".to_string(),
        description: None,
        dataset_contract: None,
        entry_nodes: vec!["mesh_input".to_string()],
        output_nodes: vec!["mesh_output".to_string()],
        defaults: WorkflowDefaults {
            cache_policy: Some(WorkflowCachePolicy::Cached),
            orchestrated: Some(true),
        },
        nodes: vec![
            node(
                "mesh_input",
                WorkflowNodeKind::Input,
                None,
                vec![],
                vec![port("model", "study_model/cohesive_interface_mesh_2d")],
            ),
            node(
                "solve_mesh",
                WorkflowNodeKind::Solve,
                Some("solve.cohesive_interface_mesh_2d"),
                vec![port("model", "study_model/cohesive_interface_mesh_2d")],
                vec![port("result", "result/cohesive_interface_mesh_2d")],
            ),
            node(
                "mesh_output",
                WorkflowNodeKind::Output,
                None,
                vec![port("result", "result/cohesive_interface_mesh_2d")],
                vec![],
            ),
        ],
        edges: vec![
            edge(
                "input_to_solve",
                "mesh_input",
                "solve_mesh",
                "study_model/cohesive_interface_mesh_2d",
            ),
            edge(
                "solve_to_output",
                "solve_mesh",
                "mesh_output",
                "result/cohesive_interface_mesh_2d",
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

fn edge(id: &str, from_node: &str, to_node: &str, artifact_type: &str) -> WorkflowEdge {
    WorkflowEdge {
        id: id.to_string(),
        from: WorkflowNodePortRef {
            node: from_node.to_string(),
            port: if from_node == "mesh_input" {
                "model"
            } else {
                "result"
            }
            .to_string(),
        },
        to: WorkflowNodePortRef {
            node: to_node.to_string(),
            port: if to_node == "solve_mesh" {
                "model"
            } else {
                "result"
            }
            .to_string(),
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
        "id": "mesh.workflow",
        "nodes": [
            {"id": "lower-i", "x": 0.0, "y": 0.0, "fixed": [true, true], "load": [0.0, 0.0]},
            {"id": "lower-j", "x": 1.0, "y": 0.0, "fixed": [true, true], "load": [0.0, 0.0]},
            {"id": "upper-i", "x": 0.0, "y": 0.0, "fixed": [true, false], "load": [0.0, 2.5]},
            {"id": "upper-j", "x": 1.0, "y": 0.0, "fixed": [true, false], "load": [0.0, 2.5]}
        ],
        "materials": [{
            "id": "adhesive",
            "properties": {
                "normal_initial_stiffness": 1000.0,
                "normal_compression_stiffness": 2000.0,
                "normal_peak_traction": 10.0,
                "normal_failure_separation": 0.05,
                "shear_initial_stiffness": 500.0,
                "shear_peak_traction": 5.0,
                "shear_failure_separation": 0.05
            }
        }],
        "elements": [{
            "id": "interface-0",
            "lower_i": 0,
            "lower_j": 1,
            "upper_i": 2,
            "upper_j": 3,
            "thickness": 1.0,
            "material_id": "adhesive"
        }],
        "load_steps": 2,
        "max_iterations": 12,
        "tolerance": 1.0e-11
    })
}
