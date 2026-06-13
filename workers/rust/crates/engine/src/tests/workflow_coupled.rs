use crate::run_workflow_graph;
use kyuubiki_protocol::{
    SolveHeatPlaneQuad2dRequest, SolveThermalPlaneQuad2dRequest, WorkflowCachePolicy,
    WorkflowDefaults, WorkflowEdge, WorkflowGraph, WorkflowGraphRunRequest, WorkflowNode,
    WorkflowNodeKind, WorkflowNodePortRef, WorkflowPort,
};
use std::collections::BTreeMap;

#[test]
fn runs_electrostatic_to_heat_to_thermo_summary_workflow_graph() {
    let graph = WorkflowGraph {
        schema_version: "kyuubiki.workflow-graph/v1".to_string(),
        id: "workflow.electrostatic-heat-thermo-summary".to_string(),
        name: "Electrostatic heat thermo summary".to_string(),
        version: "1.0.0".to_string(),
        description: Some(
            "Full coupled electrostatic -> heat -> thermo workflow with summary export."
                .to_string(),
        ),
        dataset_contract: None,
        entry_nodes: vec!["electrostatic_model".to_string()],
        output_nodes: vec!["summary_output".to_string()],
        defaults: WorkflowDefaults {
            cache_policy: Some(WorkflowCachePolicy::Cached),
            orchestrated: Some(true),
        },
        nodes: vec![
            WorkflowNode {
                id: "electrostatic_model".to_string(),
                kind: WorkflowNodeKind::Input,
                operator_id: None,
                name: Some("Electrostatic model input".to_string()),
                description: None,
                config: None,
                cache_policy: None,
                inputs: vec![],
                outputs: vec![WorkflowPort {
                    id: "model".to_string(),
                    artifact_type: "study_model/electrostatic_plane_quad_2d".to_string(),
                    name: None,
                    required: None,
                    cardinality: None,
                    dataset_value: None,
                }],
            },
            WorkflowNode {
                id: "solve_electrostatic".to_string(),
                kind: WorkflowNodeKind::Solve,
                operator_id: Some("solve.electrostatic_plane_quad_2d".to_string()),
                name: Some("Solve electrostatic".to_string()),
                description: None,
                config: None,
                cache_policy: None,
                inputs: vec![WorkflowPort {
                    id: "model".to_string(),
                    artifact_type: "study_model/electrostatic_plane_quad_2d".to_string(),
                    name: None,
                    required: None,
                    cardinality: None,
                    dataset_value: None,
                }],
                outputs: vec![WorkflowPort {
                    id: "result".to_string(),
                    artifact_type: "result/electrostatic_plane_quad_2d".to_string(),
                    name: None,
                    required: None,
                    cardinality: None,
                    dataset_value: None,
                }],
            },
            WorkflowNode {
                id: "bridge_field_to_heat".to_string(),
                kind: WorkflowNodeKind::Transform,
                operator_id: Some("bridge.electrostatic_field_to_heat_quad_2d".to_string()),
                name: Some("Bridge field to heat".to_string()),
                description: None,
                config: Some(serde_json::json!({
                    "seed_model": {
                        "nodes": [
                            { "id": "h0", "x": 0.0, "y": 0.0, "fix_temperature": true, "temperature": 100.0, "heat_load": 0.0 },
                            { "id": "h1", "x": 1.0, "y": 0.0, "fix_temperature": false, "temperature": 0.0, "heat_load": 0.0 },
                            { "id": "h2", "x": 1.0, "y": 1.0, "fix_temperature": true, "temperature": 20.0, "heat_load": 0.0 },
                            { "id": "h3", "x": 0.0, "y": 1.0, "fix_temperature": true, "temperature": 20.0, "heat_load": 0.0 }
                        ],
                        "elements": [
                            { "id": "hq0", "node_i": 0, "node_j": 1, "node_k": 2, "node_l": 3, "thickness": 0.02, "conductivity": 45.0 }
                        ]
                    },
                    "contract": {
                        "version": "kyuubiki.bridge-contract/v1",
                        "source": {
                            "field": "electric_field_magnitude",
                            "distribution": "element_to_nodes",
                            "node_index_fields": ["node_i", "node_j", "node_k", "node_l"]
                        },
                        "transform": {
                            "scale": 50.0,
                            "reduction": "mean",
                            "default_value": 0.0
                        },
                        "target": { "field": "heat_load" }
                    }
                })),
                cache_policy: None,
                inputs: vec![WorkflowPort {
                    id: "electrostatic_result".to_string(),
                    artifact_type: "result/electrostatic_plane_quad_2d".to_string(),
                    name: None,
                    required: None,
                    cardinality: None,
                    dataset_value: None,
                }],
                outputs: vec![WorkflowPort {
                    id: "heat_model".to_string(),
                    artifact_type: "study_model/heat_plane_quad_2d".to_string(),
                    name: None,
                    required: None,
                    cardinality: None,
                    dataset_value: None,
                }],
            },
            WorkflowNode {
                id: "solve_heat".to_string(),
                kind: WorkflowNodeKind::Solve,
                operator_id: Some("solve.heat_plane_quad_2d".to_string()),
                name: Some("Solve heat".to_string()),
                description: None,
                config: None,
                cache_policy: None,
                inputs: vec![WorkflowPort {
                    id: "model".to_string(),
                    artifact_type: "study_model/heat_plane_quad_2d".to_string(),
                    name: None,
                    required: None,
                    cardinality: None,
                    dataset_value: None,
                }],
                outputs: vec![WorkflowPort {
                    id: "result".to_string(),
                    artifact_type: "result/heat_plane_quad_2d".to_string(),
                    name: None,
                    required: None,
                    cardinality: None,
                    dataset_value: None,
                }],
            },
            WorkflowNode {
                id: "bridge_temperature".to_string(),
                kind: WorkflowNodeKind::Transform,
                operator_id: Some("bridge.temperature_field_to_thermo_quad_2d".to_string()),
                name: Some("Bridge temperature".to_string()),
                description: None,
                config: Some(serde_json::json!({
                    "nodes": [
                        { "id": "n0", "x": 0.0, "y": 0.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0, "temperature_delta": 30.0 },
                        { "id": "n1", "x": 1.0, "y": 0.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0, "temperature_delta": 30.0 },
                        { "id": "n2", "x": 1.0, "y": 1.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0, "temperature_delta": 30.0 },
                        { "id": "n3", "x": 0.0, "y": 1.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0, "temperature_delta": 30.0 }
                    ],
                    "elements": [
                        { "id": "tq0", "node_i": 0, "node_j": 1, "node_k": 2, "node_l": 3, "thickness": 0.02, "youngs_modulus": 70000000000.0, "poisson_ratio": 0.33, "thermal_expansion": 0.000011 }
                    ]
                })),
                cache_policy: None,
                inputs: vec![WorkflowPort {
                    id: "heat_result".to_string(),
                    artifact_type: "result/heat_plane_quad_2d".to_string(),
                    name: None,
                    required: None,
                    cardinality: None,
                    dataset_value: None,
                }],
                outputs: vec![WorkflowPort {
                    id: "thermo_model".to_string(),
                    artifact_type: "study_model/thermal_plane_quad_2d".to_string(),
                    name: None,
                    required: None,
                    cardinality: None,
                    dataset_value: None,
                }],
            },
            WorkflowNode {
                id: "solve_thermo".to_string(),
                kind: WorkflowNodeKind::Solve,
                operator_id: Some("solve.thermal_plane_quad_2d".to_string()),
                name: Some("Solve thermo".to_string()),
                description: None,
                config: None,
                cache_policy: None,
                inputs: vec![WorkflowPort {
                    id: "model".to_string(),
                    artifact_type: "study_model/thermal_plane_quad_2d".to_string(),
                    name: None,
                    required: None,
                    cardinality: None,
                    dataset_value: None,
                }],
                outputs: vec![WorkflowPort {
                    id: "result".to_string(),
                    artifact_type: "result/thermal_plane_quad_2d".to_string(),
                    name: None,
                    required: None,
                    cardinality: None,
                    dataset_value: None,
                }],
            },
            WorkflowNode {
                id: "extract_summary".to_string(),
                kind: WorkflowNodeKind::Extract,
                operator_id: Some("extract.result_summary".to_string()),
                name: Some("Extract summary".to_string()),
                description: None,
                config: Some(serde_json::json!({
                    "fields": [
                        "max_displacement",
                        "max_stress"
                    ]
                })),
                cache_policy: None,
                inputs: vec![WorkflowPort {
                    id: "result".to_string(),
                    artifact_type: "result/thermal_plane_quad_2d".to_string(),
                    name: None,
                    required: None,
                    cardinality: None,
                    dataset_value: None,
                }],
                outputs: vec![WorkflowPort {
                    id: "summary".to_string(),
                    artifact_type: "extract/result_summary".to_string(),
                    name: None,
                    required: None,
                    cardinality: None,
                    dataset_value: None,
                }],
            },
            WorkflowNode {
                id: "export_summary".to_string(),
                kind: WorkflowNodeKind::Export,
                operator_id: Some("export.summary_json".to_string()),
                name: Some("Export summary".to_string()),
                description: None,
                config: None,
                cache_policy: None,
                inputs: vec![WorkflowPort {
                    id: "summary".to_string(),
                    artifact_type: "extract/result_summary".to_string(),
                    name: None,
                    required: None,
                    cardinality: None,
                    dataset_value: None,
                }],
                outputs: vec![WorkflowPort {
                    id: "json".to_string(),
                    artifact_type: "artifact/json".to_string(),
                    name: None,
                    required: None,
                    cardinality: None,
                    dataset_value: None,
                }],
            },
            WorkflowNode {
                id: "summary_output".to_string(),
                kind: WorkflowNodeKind::Output,
                operator_id: None,
                name: Some("Summary output".to_string()),
                description: None,
                config: None,
                cache_policy: None,
                inputs: vec![WorkflowPort {
                    id: "summary".to_string(),
                    artifact_type: "artifact/json".to_string(),
                    name: None,
                    required: None,
                    cardinality: None,
                    dataset_value: None,
                }],
                outputs: vec![],
            },
        ],
        edges: vec![
            WorkflowEdge {
                id: "edge-electrostatic-input".to_string(),
                from: WorkflowNodePortRef {
                    node: "electrostatic_model".to_string(),
                    port: "model".to_string(),
                },
                to: WorkflowNodePortRef {
                    node: "solve_electrostatic".to_string(),
                    port: "model".to_string(),
                },
                artifact_type: "study_model/electrostatic_plane_quad_2d".to_string(),
                dataset_value: None,
            },
            WorkflowEdge {
                id: "edge-electrostatic-result".to_string(),
                from: WorkflowNodePortRef {
                    node: "solve_electrostatic".to_string(),
                    port: "result".to_string(),
                },
                to: WorkflowNodePortRef {
                    node: "bridge_field_to_heat".to_string(),
                    port: "electrostatic_result".to_string(),
                },
                artifact_type: "result/electrostatic_plane_quad_2d".to_string(),
                dataset_value: None,
            },
            WorkflowEdge {
                id: "edge-heat-model".to_string(),
                from: WorkflowNodePortRef {
                    node: "bridge_field_to_heat".to_string(),
                    port: "heat_model".to_string(),
                },
                to: WorkflowNodePortRef {
                    node: "solve_heat".to_string(),
                    port: "model".to_string(),
                },
                artifact_type: "study_model/heat_plane_quad_2d".to_string(),
                dataset_value: None,
            },
            WorkflowEdge {
                id: "edge-heat-result".to_string(),
                from: WorkflowNodePortRef {
                    node: "solve_heat".to_string(),
                    port: "result".to_string(),
                },
                to: WorkflowNodePortRef {
                    node: "bridge_temperature".to_string(),
                    port: "heat_result".to_string(),
                },
                artifact_type: "result/heat_plane_quad_2d".to_string(),
                dataset_value: None,
            },
            WorkflowEdge {
                id: "edge-thermo-model".to_string(),
                from: WorkflowNodePortRef {
                    node: "bridge_temperature".to_string(),
                    port: "thermo_model".to_string(),
                },
                to: WorkflowNodePortRef {
                    node: "solve_thermo".to_string(),
                    port: "model".to_string(),
                },
                artifact_type: "study_model/thermal_plane_quad_2d".to_string(),
                dataset_value: None,
            },
            WorkflowEdge {
                id: "edge-thermo-result".to_string(),
                from: WorkflowNodePortRef {
                    node: "solve_thermo".to_string(),
                    port: "result".to_string(),
                },
                to: WorkflowNodePortRef {
                    node: "extract_summary".to_string(),
                    port: "result".to_string(),
                },
                artifact_type: "result/thermal_plane_quad_2d".to_string(),
                dataset_value: None,
            },
            WorkflowEdge {
                id: "edge-summary".to_string(),
                from: WorkflowNodePortRef {
                    node: "extract_summary".to_string(),
                    port: "summary".to_string(),
                },
                to: WorkflowNodePortRef {
                    node: "export_summary".to_string(),
                    port: "summary".to_string(),
                },
                artifact_type: "extract/result_summary".to_string(),
                dataset_value: None,
            },
            WorkflowEdge {
                id: "edge-summary-output".to_string(),
                from: WorkflowNodePortRef {
                    node: "export_summary".to_string(),
                    port: "json".to_string(),
                },
                to: WorkflowNodePortRef {
                    node: "summary_output".to_string(),
                    port: "summary".to_string(),
                },
                artifact_type: "artifact/json".to_string(),
                dataset_value: None,
            },
        ],
    };

    let run = run_workflow_graph(WorkflowGraphRunRequest {
        graph,
        input_artifacts: BTreeMap::from([(
            "electrostatic_model".to_string(),
            serde_json::json!({
                "nodes": [
                    { "id": "e0", "x": 0.0, "y": 0.0, "fix_potential": true, "potential": 10.0, "charge_density": 0.0 },
                    { "id": "e1", "x": 1.0, "y": 0.0, "fix_potential": false, "potential": 0.0, "charge_density": 0.0 },
                    { "id": "e2", "x": 1.0, "y": 1.0, "fix_potential": true, "potential": 0.0, "charge_density": 0.0 },
                    { "id": "e3", "x": 0.0, "y": 1.0, "fix_potential": true, "potential": 0.0, "charge_density": 0.0 }
                ],
                "elements": [
                    { "id": "eq0", "node_i": 0, "node_j": 1, "node_k": 2, "node_l": 3, "thickness": 0.05, "permittivity": 2.5 }
                ]
            }),
        )]),
    })
    .expect("electrostatic -> heat -> thermo summary workflow should run");

    assert_eq!(
        run.workflow_id,
        "workflow.electrostatic-heat-thermo-summary"
    );
    assert_eq!(run.completed_nodes.len(), 9);

    let heat_model: SolveHeatPlaneQuad2dRequest = serde_json::from_value(
        run.artifacts
            .get("bridge_field_to_heat.heat_model")
            .cloned()
            .expect("bridged heat model"),
    )
    .expect("heat model should decode");
    assert!(heat_model.nodes.iter().all(|node| node.heat_load > 0.0));
    let heat_diagnostics = run
        .artifacts
        .get("bridge_field_to_heat.heat_model")
        .and_then(|value| value.get("__bridge_diagnostics"))
        .expect("bridged heat model should expose bridge diagnostics");
    assert_eq!(
        heat_diagnostics.get("mapped_count"),
        Some(&serde_json::json!(4))
    );
    assert_eq!(
        heat_diagnostics.get("target_field"),
        Some(&serde_json::json!("heat_load"))
    );

    let thermo_model: SolveThermalPlaneQuad2dRequest = serde_json::from_value(
        run.artifacts
            .get("bridge_temperature.thermo_model")
            .cloned()
            .expect("bridged thermo model"),
    )
    .expect("thermo model should decode");
    assert!(
        thermo_model
            .nodes
            .iter()
            .any(|node| node.temperature_delta > 30.0)
    );
    let thermo_diagnostics = run
        .artifacts
        .get("bridge_temperature.thermo_model")
        .and_then(|value| value.get("__bridge_diagnostics"))
        .expect("bridged thermo model should expose bridge diagnostics");
    assert_eq!(
        thermo_diagnostics.get("mapped_count"),
        Some(&serde_json::json!(4))
    );
    assert_eq!(
        thermo_diagnostics.get("target_field"),
        Some(&serde_json::json!("temperature_delta"))
    );

    let exported_summary = run
        .artifacts
        .get("summary_output.summary")
        .and_then(|value| value.get("content"))
        .and_then(|value| value.as_str())
        .expect("summary output should expose JSON content");
    assert!(exported_summary.contains("max_displacement"));
    assert!(exported_summary.contains("max_stress"));
}
