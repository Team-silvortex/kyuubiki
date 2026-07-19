use kyuubiki_engine::run_workflow_graph;
use kyuubiki_protocol::{
    WorkflowCachePolicy, WorkflowDefaults, WorkflowEdge, WorkflowGraph, WorkflowGraphRunRequest,
    WorkflowNode, WorkflowNodeKind, WorkflowNodePortRef, WorkflowPort,
};
use std::collections::BTreeMap;

#[test]
fn workflow_runs_magnetostatic_plane_triangle_solver() {
    let run = run_workflow_graph(WorkflowGraphRunRequest {
        graph: graph(),
        input_artifacts: BTreeMap::from([("magnetic_input".to_string(), magnetic_model())]),
    })
    .expect("workflow magnetostatic triangle solve should succeed");

    let result = run
        .artifacts
        .get("magnetic_output.result")
        .expect("workflow output should contain magnetostatic result");
    assert!(result["max_flux_density"].as_f64().unwrap() > 0.0);
    assert!(result["total_stored_energy"].as_f64().unwrap() > 0.0);
    assert_eq!(result["elements"].as_array().unwrap().len(), 1);
    assert!(run.completed_nodes.contains(&"solve_magnetic".to_string()));
}

#[test]
fn workflow_runs_magnetostatic_plane_quad_solver() {
    let run = run_workflow_graph(WorkflowGraphRunRequest {
        graph: graph_for(
            "workflow.magnetostatic-plane-quad-2d",
            "Magnetostatic plane quad 2d",
            "study_model/magnetostatic_plane_quad_2d",
            "solve.magnetostatic_plane_quad_2d",
            "result/magnetostatic_plane_quad_2d",
        ),
        input_artifacts: BTreeMap::from([("magnetic_input".to_string(), magnetic_quad_model())]),
    })
    .expect("workflow magnetostatic quad solve should succeed");

    let result = run
        .artifacts
        .get("magnetic_output.result")
        .expect("workflow output should contain magnetostatic quad result");
    assert!(result["max_flux_density"].as_f64().unwrap() > 0.0);
    assert_eq!(result["elements"].as_array().unwrap().len(), 1);
    assert!(run.completed_nodes.contains(&"solve_magnetic".to_string()));
}

#[test]
fn workflow_runs_magnetostatic_quad_to_heat_to_thermo_quad() {
    let graph = WorkflowGraph {
        schema_version: "kyuubiki.workflow-graph/v1".to_string(),
        id: "workflow.magnetostatic-heat-thermo-quad".to_string(),
        name: "Magnetostatic heat thermo quad".to_string(),
        version: "1.0.0".to_string(),
        description: Some("Magnetostatic field to heat to thermo-mechanical workflow.".to_string()),
        dataset_contract: None,
        entry_nodes: vec!["magnetic_input".to_string()],
        output_nodes: vec!["thermo_output".to_string()],
        defaults: WorkflowDefaults::default(),
        nodes: vec![
            input_node("study_model/magnetostatic_plane_quad_2d"),
            solve_node(
                "solve.magnetostatic_plane_quad_2d",
                "study_model/magnetostatic_plane_quad_2d",
                "result/magnetostatic_plane_quad_2d",
            ),
            WorkflowNode {
                id: "bridge_heat".to_string(),
                kind: WorkflowNodeKind::Transform,
                operator_id: Some("bridge.magnetostatic_field_to_heat_quad_2d".to_string()),
                name: None,
                description: None,
                config: Some(serde_json::json!({
                    "seed_model": {
                        "nodes": [
                            { "id": "h0", "x": 0.0, "y": 0.0, "fix_temperature": true, "temperature": 100.0, "heat_load": 0.0 },
                            { "id": "h1", "x": 1.0, "y": 0.0, "fix_temperature": false, "temperature": 0.0, "heat_load": 0.0 },
                            { "id": "h2", "x": 1.0, "y": 1.0, "fix_temperature": true, "temperature": 20.0, "heat_load": 0.0 },
                            { "id": "h3", "x": 0.0, "y": 1.0, "fix_temperature": true, "temperature": 20.0, "heat_load": 0.0 }
                        ],
                        "elements": [{ "id": "hq0", "node_i": 0, "node_j": 1, "node_k": 2, "node_l": 3, "thickness": 0.02, "conductivity": 45.0 }]
                    },
                    "contract": { "transform": { "scale": 20.0, "reduction": "mean" }, "target": { "field": "heat_load" } }
                })),
                cache_policy: None,
                inputs: vec![port(
                    "magnetic_result",
                    "result/magnetostatic_plane_quad_2d",
                )],
                outputs: vec![port("heat_model", "study_model/heat_plane_quad_2d")],
            },
            WorkflowNode {
                id: "solve_heat".to_string(),
                kind: WorkflowNodeKind::Solve,
                operator_id: Some("solve.heat_plane_quad_2d".to_string()),
                name: None,
                description: None,
                config: None,
                cache_policy: None,
                inputs: vec![port("model", "study_model/heat_plane_quad_2d")],
                outputs: vec![port("result", "result/heat_plane_quad_2d")],
            },
            WorkflowNode {
                id: "bridge_thermo".to_string(),
                kind: WorkflowNodeKind::Transform,
                operator_id: Some("bridge.temperature_field_to_thermo_quad_2d".to_string()),
                name: None,
                description: None,
                config: Some(serde_json::json!({
                    "nodes": [
                        { "id": "t0", "x": 0.0, "y": 0.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0, "temperature_delta": 20.0 },
                        { "id": "t1", "x": 1.0, "y": 0.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0, "temperature_delta": 20.0 },
                        { "id": "t2", "x": 1.0, "y": 1.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0, "temperature_delta": 20.0 },
                        { "id": "t3", "x": 0.0, "y": 1.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0, "temperature_delta": 20.0 }
                    ],
                    "elements": [{ "id": "tq0", "node_i": 0, "node_j": 1, "node_k": 2, "node_l": 3, "thickness": 0.02, "youngs_modulus": 70000000000.0, "poisson_ratio": 0.33, "thermal_expansion": 0.000011 }]
                })),
                cache_policy: None,
                inputs: vec![port("heat_result", "result/heat_plane_quad_2d")],
                outputs: vec![port("thermo_model", "study_model/thermal_plane_quad_2d")],
            },
            WorkflowNode {
                id: "solve_thermo".to_string(),
                kind: WorkflowNodeKind::Solve,
                operator_id: Some("solve.thermal_plane_quad_2d".to_string()),
                name: None,
                description: None,
                config: None,
                cache_policy: None,
                inputs: vec![port("model", "study_model/thermal_plane_quad_2d")],
                outputs: vec![port("result", "result/thermal_plane_quad_2d")],
            },
            WorkflowNode {
                id: "thermo_output".to_string(),
                kind: WorkflowNodeKind::Output,
                operator_id: None,
                name: None,
                description: None,
                config: None,
                cache_policy: None,
                inputs: vec![port("result", "result/thermal_plane_quad_2d")],
                outputs: vec![],
            },
        ],
        edges: vec![
            edge(
                "input",
                "magnetic_input",
                "model",
                "solve_magnetic",
                "model",
                "study_model/magnetostatic_plane_quad_2d",
            ),
            edge(
                "magnetic",
                "solve_magnetic",
                "result",
                "bridge_heat",
                "magnetic_result",
                "result/magnetostatic_plane_quad_2d",
            ),
            edge(
                "heat-model",
                "bridge_heat",
                "heat_model",
                "solve_heat",
                "model",
                "study_model/heat_plane_quad_2d",
            ),
            edge(
                "heat-to-thermo",
                "solve_heat",
                "result",
                "bridge_thermo",
                "heat_result",
                "result/heat_plane_quad_2d",
            ),
            edge(
                "thermo-model",
                "bridge_thermo",
                "thermo_model",
                "solve_thermo",
                "model",
                "study_model/thermal_plane_quad_2d",
            ),
            edge(
                "thermo",
                "solve_thermo",
                "result",
                "thermo_output",
                "result",
                "result/thermal_plane_quad_2d",
            ),
        ],
    };
    let run = run_workflow_graph(WorkflowGraphRunRequest {
        graph,
        input_artifacts: BTreeMap::from([("magnetic_input".to_string(), magnetic_quad_model())]),
    })
    .expect("magnetostatic to heat to thermo workflow should run");
    assert_eq!(run.completed_nodes.len(), 7);
    assert!(run.artifacts.contains_key("bridge_heat.heat_model"));
    assert!(run.artifacts.contains_key("bridge_thermo.thermo_model"));
    let result = run
        .artifacts
        .get("thermo_output.result")
        .expect("thermo result");
    assert!(
        result["max_temperature_delta"]
            .as_f64()
            .is_some_and(|value| value > 0.0)
    );
}

fn graph() -> WorkflowGraph {
    graph_for(
        "workflow.magnetostatic-plane-triangle-2d",
        "Magnetostatic plane triangle 2d",
        "study_model/magnetostatic_plane_triangle_2d",
        "solve.magnetostatic_plane_triangle_2d",
        "result/magnetostatic_plane_triangle_2d",
    )
}

fn graph_for(
    graph_id: &str,
    graph_name: &str,
    model_type: &str,
    operator_id: &str,
    result_type: &str,
) -> WorkflowGraph {
    WorkflowGraph {
        schema_version: "kyuubiki.workflow-graph/v1".to_string(),
        id: graph_id.to_string(),
        name: graph_name.to_string(),
        version: "1.0.0".to_string(),
        description: None,
        dataset_contract: None,
        entry_nodes: vec!["magnetic_input".to_string()],
        output_nodes: vec!["magnetic_output".to_string()],
        defaults: WorkflowDefaults {
            cache_policy: Some(WorkflowCachePolicy::Cached),
            orchestrated: Some(true),
        },
        nodes: vec![
            input_node(model_type),
            solve_node(operator_id, model_type, result_type),
            output_node(result_type),
        ],
        edges: vec![
            edge(
                "input_to_solve",
                "magnetic_input",
                "model",
                "solve_magnetic",
                "model",
                model_type,
            ),
            edge(
                "solve_to_output",
                "solve_magnetic",
                "result",
                "magnetic_output",
                "result",
                result_type,
            ),
        ],
    }
}

fn input_node(model_type: &str) -> WorkflowNode {
    WorkflowNode {
        id: "magnetic_input".to_string(),
        kind: WorkflowNodeKind::Input,
        operator_id: None,
        name: None,
        description: None,
        config: None,
        cache_policy: None,
        inputs: vec![],
        outputs: vec![port("model", model_type)],
    }
}

fn solve_node(operator_id: &str, model_type: &str, result_type: &str) -> WorkflowNode {
    WorkflowNode {
        id: "solve_magnetic".to_string(),
        kind: WorkflowNodeKind::Solve,
        operator_id: Some(operator_id.to_string()),
        name: None,
        description: None,
        config: None,
        cache_policy: None,
        inputs: vec![port("model", model_type)],
        outputs: vec![port("result", result_type)],
    }
}

fn output_node(result_type: &str) -> WorkflowNode {
    WorkflowNode {
        id: "magnetic_output".to_string(),
        kind: WorkflowNodeKind::Output,
        operator_id: None,
        name: None,
        description: None,
        config: None,
        cache_policy: None,
        inputs: vec![port("result", result_type)],
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

fn magnetic_model() -> serde_json::Value {
    serde_json::json!({
        "nodes": [
            { "id": "n0", "x": 0.0, "y": 0.0, "fix_vector_potential": true, "vector_potential": 0.0, "current_density": 0.0 },
            { "id": "n1", "x": 1.0, "y": 0.0, "fix_vector_potential": true, "vector_potential": 0.0, "current_density": 0.0 },
            { "id": "n2", "x": 0.0, "y": 1.0, "fix_vector_potential": false, "vector_potential": 0.0, "current_density": 5.0 }
        ],
        "elements": [
            { "id": "m0", "node_i": 0, "node_j": 1, "node_k": 2, "thickness": 0.1, "permeability": 0.0000012566370614359173 }
        ]
    })
}

fn magnetic_quad_model() -> serde_json::Value {
    serde_json::json!({
        "nodes": [
            { "id": "n0", "x": 0.0, "y": 0.0, "fix_vector_potential": true, "vector_potential": 0.0, "current_density": 0.0 },
            { "id": "n1", "x": 1.0, "y": 0.0, "fix_vector_potential": true, "vector_potential": 0.0, "current_density": 0.0 },
            { "id": "n2", "x": 1.0, "y": 1.0, "fix_vector_potential": false, "vector_potential": 0.0, "current_density": 5.0 },
            { "id": "n3", "x": 0.0, "y": 1.0, "fix_vector_potential": false, "vector_potential": 0.0, "current_density": 5.0 }
        ],
        "elements": [
            { "id": "q0", "node_i": 0, "node_j": 1, "node_k": 2, "node_l": 3, "thickness": 0.1, "permeability": 0.0000012566370614359173 }
        ]
    })
}
