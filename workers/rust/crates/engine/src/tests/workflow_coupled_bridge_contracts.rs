use crate::heat_bridge::{
    bridge_heat_result_to_thermal_plane_triangle_model_with_contract,
    resolve_heat_to_thermo_bridge_contract,
};
use crate::run_workflow_graph;
use kyuubiki_protocol::{
    HeatPlaneNodeResult, HeatPlaneTriangleElementInput, HeatPlaneTriangleElementResult,
    SolveHeatPlaneTriangle2dRequest, SolveHeatPlaneTriangle2dResult,
    SolveThermalPlaneTriangle2dRequest, WorkflowCachePolicy, WorkflowDefaults, WorkflowEdge,
    WorkflowGraph, WorkflowGraphRunRequest, WorkflowNode, WorkflowNodeKind,
    WorkflowNodePortRef, WorkflowPort,
};
use std::collections::BTreeMap;

#[test]
fn bridges_heat_triangle_elements_into_thermo_triangle_with_max_reduction() {
    let contract = resolve_heat_to_thermo_bridge_contract(&serde_json::json!({
        "contract": {
            "source": {
                "field": "average_temperature",
                "distribution": "element_to_nodes",
                "node_index_fields": ["node_i", "node_j", "node_k"]
            },
            "transform": { "scale": 1.0, "reduction": "max", "default_value": 0.0 },
            "target": { "field": "temperature_delta" }
        }
    }))
    .expect("heat element contract should resolve");

    let (bridged, diagnostics) = bridge_heat_result_to_thermal_plane_triangle_model_with_contract(
        &heat_triangle_result(),
        &thermo_triangle_seed_model(),
        &contract,
    )
    .expect("heat triangle element bridge should build");

    assert_eq!(bridged.nodes[0].temperature_delta, 30.0);
    assert_eq!(bridged.nodes[1].temperature_delta, 90.0);
    assert_eq!(bridged.nodes[2].temperature_delta, 90.0);
    assert_eq!(bridged.nodes[3].temperature_delta, 90.0);
    assert_eq!(diagnostics.source_field, "average_temperature");
    assert_eq!(diagnostics.reduction.as_deref(), Some("max"));
  }

#[test]
fn runs_electrostatic_to_heat_to_thermo_triangle_workflow_with_contract_bridges() {
    let graph = WorkflowGraph {
        schema_version: "kyuubiki.workflow-graph/v1".to_string(),
        id: "workflow.electrostatic-heat-thermo-contract-bridge".to_string(),
        name: "Electrostatic heat thermo contract bridge".to_string(),
        version: "1.0.0".to_string(),
        description: Some(
            "Electrostatic node bridge plus heat element bridge workflow.".to_string(),
        ),
        dataset_contract: None,
        entry_nodes: vec!["electrostatic_model".to_string()],
        output_nodes: vec!["summary_output".to_string()],
        defaults: WorkflowDefaults {
            cache_policy: Some(WorkflowCachePolicy::Cached),
            orchestrated: Some(true),
        },
        nodes: vec![
            input_node(
                "electrostatic_model",
                "model",
                "study_model/electrostatic_plane_triangle_2d",
            ),
            solve_node(
                "solve_electrostatic",
                "solve.electrostatic_plane_triangle_2d",
                "study_model/electrostatic_plane_triangle_2d",
                "result/electrostatic_plane_triangle_2d",
            ),
            WorkflowNode {
                id: "bridge_field_to_heat".to_string(),
                kind: WorkflowNodeKind::Transform,
                operator_id: Some("bridge.electrostatic_field_to_heat_triangle_2d".to_string()),
                name: Some("Bridge potential to heat".to_string()),
                description: None,
                config: Some(serde_json::json!({
                    "seed_model": {
                        "nodes": [
                            { "id": "h0", "x": 0.0, "y": 0.0, "fix_temperature": false, "temperature": 0.0, "heat_load": 0.0 },
                            { "id": "h1", "x": 1.0, "y": 0.0, "fix_temperature": false, "temperature": 0.0, "heat_load": 0.0 },
                            { "id": "h2", "x": 0.0, "y": 1.0, "fix_temperature": false, "temperature": 0.0, "heat_load": 0.0 },
                            { "id": "h3", "x": 1.0, "y": 1.0, "fix_temperature": true, "temperature": 5.0, "heat_load": 0.0 }
                        ],
                        "elements": [
                            { "id": "ht0", "node_i": 0, "node_j": 1, "node_k": 2, "thickness": 0.02, "conductivity": 45.0 },
                            { "id": "ht1", "node_i": 1, "node_j": 3, "node_k": 2, "thickness": 0.02, "conductivity": 45.0 }
                        ]
                    },
                    "contract": {
                        "source": { "field": "potential", "distribution": "node_to_node" },
                        "transform": { "scale": 2.0, "default_value": 0.0 },
                        "target": { "field": "temperature" }
                    }
                })),
                cache_policy: None,
                inputs: vec![port(
                    "electrostatic_result",
                    "result/electrostatic_plane_triangle_2d",
                )],
                outputs: vec![port("heat_model", "study_model/heat_plane_triangle_2d")],
            },
            solve_node(
                "solve_heat",
                "solve.heat_plane_triangle_2d",
                "study_model/heat_plane_triangle_2d",
                "result/heat_plane_triangle_2d",
            ),
            WorkflowNode {
                id: "bridge_temperature".to_string(),
                kind: WorkflowNodeKind::Transform,
                operator_id: Some("bridge.temperature_field_to_thermo_triangle_2d".to_string()),
                name: Some("Bridge heat elements to thermo".to_string()),
                description: None,
                config: Some(serde_json::json!({
                    "seed_model": {
                        "nodes": [
                            { "id": "t0", "x": 0.0, "y": 0.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0, "temperature_delta": 0.0 },
                            { "id": "t1", "x": 1.0, "y": 0.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0, "temperature_delta": 0.0 },
                            { "id": "t2", "x": 0.0, "y": 1.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0, "temperature_delta": 0.0 },
                            { "id": "t3", "x": 1.0, "y": 1.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0, "temperature_delta": 0.0 }
                        ],
                        "elements": [
                            { "id": "tt0", "node_i": 0, "node_j": 1, "node_k": 2, "thickness": 0.02, "youngs_modulus": 70000000000.0, "poisson_ratio": 0.33, "thermal_expansion": 0.000011 },
                            { "id": "tt1", "node_i": 1, "node_j": 3, "node_k": 2, "thickness": 0.02, "youngs_modulus": 70000000000.0, "poisson_ratio": 0.33, "thermal_expansion": 0.000011 }
                        ]
                    },
                    "contract": {
                        "source": {
                            "field": "average_temperature",
                            "distribution": "element_to_nodes",
                            "node_index_fields": ["node_i", "node_j", "node_k"]
                        },
                        "transform": { "scale": 1.0, "reduction": "max", "default_value": 0.0 },
                        "target": { "field": "temperature_delta" }
                    }
                })),
                cache_policy: None,
                inputs: vec![port("source", "result/heat_plane_triangle_2d")],
                outputs: vec![port("bridged_model", "study_model/thermal_plane_triangle_2d")],
            },
            solve_node(
                "solve_thermo",
                "solve.thermal_plane_triangle_2d",
                "study_model/thermal_plane_triangle_2d",
                "result/thermal_plane_triangle_2d",
            ),
            extract_node(
                "extract_summary",
                "result/thermal_plane_triangle_2d",
                &["max_displacement", "max_stress", "max_temperature_delta"],
            ),
            output_node("summary_output", "summary", "extract/result_summary"),
        ],
        edges: vec![
            edge(
                "edge-input",
                "electrostatic_model",
                "model",
                "solve_electrostatic",
                "model",
                "study_model/electrostatic_plane_triangle_2d",
            ),
            edge(
                "edge-electrostatic-result",
                "solve_electrostatic",
                "result",
                "bridge_field_to_heat",
                "electrostatic_result",
                "result/electrostatic_plane_triangle_2d",
            ),
            edge(
                "edge-heat-model",
                "bridge_field_to_heat",
                "heat_model",
                "solve_heat",
                "model",
                "study_model/heat_plane_triangle_2d",
            ),
            edge(
                "edge-heat-result",
                "solve_heat",
                "result",
                "bridge_temperature",
                "source",
                "result/heat_plane_triangle_2d",
            ),
            edge(
                "edge-thermo-model",
                "bridge_temperature",
                "bridged_model",
                "solve_thermo",
                "model",
                "study_model/thermal_plane_triangle_2d",
            ),
            edge(
                "edge-thermo-result",
                "solve_thermo",
                "result",
                "extract_summary",
                "result",
                "result/thermal_plane_triangle_2d",
            ),
            edge(
                "edge-summary",
                "extract_summary",
                "summary",
                "summary_output",
                "summary",
                "extract/result_summary",
            ),
        ],
    };

    let run = run_workflow_graph(WorkflowGraphRunRequest {
        graph,
        input_artifacts: BTreeMap::from([(
            "electrostatic_model".to_string(),
            serde_json::json!({
                "nodes": [
                    { "id": "e0", "x": 0.0, "y": 0.0, "fix_potential": true, "potential": 12.0, "charge_density": 0.0 },
                    { "id": "e1", "x": 1.0, "y": 0.0, "fix_potential": true, "potential": 6.0, "charge_density": 0.0 },
                    { "id": "e2", "x": 0.0, "y": 1.0, "fix_potential": true, "potential": 3.0, "charge_density": 0.0 },
                    { "id": "e3", "x": 1.0, "y": 1.0, "fix_potential": true, "potential": 1.0, "charge_density": 0.0 }
                ],
                "elements": [
                    { "id": "et0", "node_i": 0, "node_j": 1, "node_k": 2, "thickness": 0.05, "permittivity": 2.5 },
                    { "id": "et1", "node_i": 1, "node_j": 3, "node_k": 2, "thickness": 0.05, "permittivity": 2.5 }
                ]
            }),
        )]),
    })
    .expect("contract-bridged coupled workflow should run");

    let heat_result = run
        .artifacts
        .get("solve_heat.result")
        .cloned()
        .expect("heat result should exist");
    let thermo_model = run
        .artifacts
        .get("bridge_temperature.bridged_model")
        .cloned()
        .expect("thermo model should exist");
    let elements = heat_result["elements"]
        .as_array()
        .expect("heat result elements should exist");
    let thermo_nodes = thermo_model["nodes"]
        .as_array()
        .expect("thermo nodes should exist");
    let first_average = elements[0]["average_temperature"]
        .as_f64()
        .expect("first element average should be numeric");
    let second_average = elements[1]["average_temperature"]
        .as_f64()
        .expect("second element average should be numeric");

    assert_eq!(
        thermo_nodes[0]["temperature_delta"].as_f64(),
        Some(first_average)
    );
    assert_eq!(
        thermo_nodes[1]["temperature_delta"].as_f64(),
        Some(first_average.max(second_average))
    );
    assert_eq!(
        thermo_nodes[2]["temperature_delta"].as_f64(),
        Some(first_average.max(second_average))
    );
    assert_eq!(
        thermo_nodes[3]["temperature_delta"].as_f64(),
        Some(second_average)
    );
    assert!(run
        .artifacts
        .get("summary_output.summary")
        .and_then(|summary| summary.get("max_temperature_delta"))
        .and_then(|value| value.as_f64())
        .is_some_and(|value| value > 0.0));
}

fn heat_triangle_result() -> SolveHeatPlaneTriangle2dResult {
    SolveHeatPlaneTriangle2dResult {
        input: SolveHeatPlaneTriangle2dRequest {
            nodes: vec![],
            elements: vec![
                HeatPlaneTriangleElementInput {
                    id: "ht0".to_string(),
                    node_i: 0,
                    node_j: 1,
                    node_k: 2,
                    thickness: 0.02,
                    conductivity: 45.0,
                },
                HeatPlaneTriangleElementInput {
                    id: "ht1".to_string(),
                    node_i: 1,
                    node_j: 3,
                    node_k: 2,
                    thickness: 0.02,
                    conductivity: 45.0,
                },
            ],
        },
        nodes: vec![
            heat_result_node("h0", 0.0, 0.0, 0.0, 0.0),
            heat_result_node("h1", 1.0, 0.0, 0.0, 0.0),
            heat_result_node("h2", 0.0, 1.0, 0.0, 0.0),
            heat_result_node("h3", 1.0, 1.0, 0.0, 0.0),
        ],
        elements: vec![
            HeatPlaneTriangleElementResult {
                index: 0,
                id: "ht0".to_string(),
                node_i: 0,
                node_j: 1,
                node_k: 2,
                area: 1.0,
                average_temperature: 30.0,
                temperature_gradient_x: 0.0,
                temperature_gradient_y: 0.0,
                heat_flux_x: 5.0,
                heat_flux_y: 0.0,
                heat_flux_magnitude: 5.0,
            },
            HeatPlaneTriangleElementResult {
                index: 1,
                id: "ht1".to_string(),
                node_i: 1,
                node_j: 3,
                node_k: 2,
                area: 2.0,
                average_temperature: 90.0,
                temperature_gradient_x: 0.0,
                temperature_gradient_y: 0.0,
                heat_flux_x: 12.0,
                heat_flux_y: 0.0,
                heat_flux_magnitude: 12.0,
            },
        ],
        max_temperature: 90.0,
        max_heat_flux: 12.0,
    }
}

fn thermo_triangle_seed_model() -> SolveThermalPlaneTriangle2dRequest {
    serde_json::from_value(serde_json::json!({
        "nodes": [
            { "id": "t0", "x": 0.0, "y": 0.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0, "temperature_delta": 0.0 },
            { "id": "t1", "x": 1.0, "y": 0.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0, "temperature_delta": 0.0 },
            { "id": "t2", "x": 0.0, "y": 1.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0, "temperature_delta": 0.0 },
            { "id": "t3", "x": 1.0, "y": 1.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0, "temperature_delta": 0.0 }
        ],
        "elements": [
            { "id": "tt0", "node_i": 0, "node_j": 1, "node_k": 2, "thickness": 0.02, "youngs_modulus": 70000000000.0, "poisson_ratio": 0.33, "thermal_expansion": 0.000011 },
            { "id": "tt1", "node_i": 1, "node_j": 3, "node_k": 2, "thickness": 0.02, "youngs_modulus": 70000000000.0, "poisson_ratio": 0.33, "thermal_expansion": 0.000011 }
        ]
    }))
    .expect("thermo seed model should decode")
}

fn heat_result_node(id: &str, x: f64, y: f64, temperature: f64, heat_load: f64) -> HeatPlaneNodeResult {
    HeatPlaneNodeResult {
        index: 0,
        id: id.to_string(),
        x,
        y,
        temperature,
        heat_load,
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

fn input_node(id: &str, output_id: &str, artifact_type: &str) -> WorkflowNode {
    WorkflowNode {
        id: id.to_string(),
        kind: WorkflowNodeKind::Input,
        operator_id: None,
        name: None,
        description: None,
        config: None,
        cache_policy: None,
        inputs: vec![],
        outputs: vec![port(output_id, artifact_type)],
    }
}

fn solve_node(id: &str, operator_id: &str, input_type: &str, output_type: &str) -> WorkflowNode {
    WorkflowNode {
        id: id.to_string(),
        kind: WorkflowNodeKind::Solve,
        operator_id: Some(operator_id.to_string()),
        name: None,
        description: None,
        config: None,
        cache_policy: None,
        inputs: vec![port("model", input_type)],
        outputs: vec![port("result", output_type)],
    }
}

fn extract_node(id: &str, input_type: &str, fields: &[&str]) -> WorkflowNode {
    WorkflowNode {
        id: id.to_string(),
        kind: WorkflowNodeKind::Extract,
        operator_id: Some("extract.result_summary".to_string()),
        name: None,
        description: None,
        config: Some(serde_json::json!({ "fields": fields })),
        cache_policy: None,
        inputs: vec![port("result", input_type)],
        outputs: vec![port("summary", "extract/result_summary")],
    }
}

fn output_node(id: &str, input_id: &str, artifact_type: &str) -> WorkflowNode {
    WorkflowNode {
        id: id.to_string(),
        kind: WorkflowNodeKind::Output,
        operator_id: None,
        name: None,
        description: None,
        config: None,
        cache_policy: None,
        inputs: vec![port(input_id, artifact_type)],
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
