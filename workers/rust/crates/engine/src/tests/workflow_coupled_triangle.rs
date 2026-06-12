use crate::run_workflow_graph;
use kyuubiki_protocol::{
    WorkflowCachePolicy, WorkflowDefaults, WorkflowEdge, WorkflowGraph, WorkflowGraphRunRequest,
    WorkflowNode, WorkflowNodeKind, WorkflowNodePortRef, WorkflowPort,
};
use std::collections::BTreeMap;

#[test]
fn runs_heat_triangle_to_thermo_triangle_summary_workflow_graph() {
    let graph = WorkflowGraph {
        schema_version: "kyuubiki.workflow-graph/v1".to_string(),
        id: "workflow.heat-triangle-thermo-triangle-summary".to_string(),
        name: "Heat triangle thermo triangle summary".to_string(),
        version: "1.0.0".to_string(),
        description: Some("Heat triangle -> thermo triangle summary workflow.".to_string()),
        dataset_contract: None,
        entry_nodes: vec!["heat_model".to_string()],
        output_nodes: vec!["summary_output".to_string()],
        defaults: WorkflowDefaults {
            cache_policy: Some(WorkflowCachePolicy::Cached),
            orchestrated: Some(true),
        },
        nodes: vec![
            WorkflowNode {
                id: "heat_model".to_string(),
                kind: WorkflowNodeKind::Input,
                operator_id: None,
                name: Some("Heat model input".to_string()),
                description: None,
                config: None,
                cache_policy: None,
                inputs: vec![],
                outputs: vec![WorkflowPort {
                    id: "model".to_string(),
                    artifact_type: "study_model/heat_plane_triangle_2d".to_string(),
                    name: None,
                    required: None,
                    cardinality: None,
                    dataset_value: None,
                }],
            },
            WorkflowNode {
                id: "solve_heat".to_string(),
                kind: WorkflowNodeKind::Solve,
                operator_id: Some("solve.heat_plane_triangle_2d".to_string()),
                name: Some("Solve heat triangle".to_string()),
                description: None,
                config: None,
                cache_policy: None,
                inputs: vec![WorkflowPort {
                    id: "model".to_string(),
                    artifact_type: "study_model/heat_plane_triangle_2d".to_string(),
                    name: None,
                    required: None,
                    cardinality: None,
                    dataset_value: None,
                }],
                outputs: vec![WorkflowPort {
                    id: "result".to_string(),
                    artifact_type: "result/heat_plane_triangle_2d".to_string(),
                    name: None,
                    required: None,
                    cardinality: None,
                    dataset_value: None,
                }],
            },
            WorkflowNode {
                id: "bridge_temperature".to_string(),
                kind: WorkflowNodeKind::Transform,
                operator_id: Some("bridge.temperature_field_to_thermo_triangle_2d".to_string()),
                name: Some("Bridge temperature triangle".to_string()),
                description: None,
                config: Some(serde_json::json!({
                    "nodes": [
                        { "id": "t0", "x": 0.0, "y": 0.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0, "temperature_delta": 30.0 },
                        { "id": "t1", "x": 1.0, "y": 0.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0, "temperature_delta": 30.0 },
                        { "id": "t2", "x": 0.0, "y": 1.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0, "temperature_delta": 30.0 }
                    ],
                    "elements": [
                        { "id": "tt0", "node_i": 0, "node_j": 1, "node_k": 2, "thickness": 0.02, "youngs_modulus": 70000000000.0, "poisson_ratio": 0.33, "thermal_expansion": 0.000011 }
                    ]
                })),
                cache_policy: None,
                inputs: vec![WorkflowPort {
                    id: "source".to_string(),
                    artifact_type: "result/heat_plane_triangle_2d".to_string(),
                    name: None,
                    required: None,
                    cardinality: None,
                    dataset_value: None,
                }],
                outputs: vec![WorkflowPort {
                    id: "bridged_model".to_string(),
                    artifact_type: "study_model/thermal_plane_triangle_2d".to_string(),
                    name: None,
                    required: None,
                    cardinality: None,
                    dataset_value: None,
                }],
            },
            WorkflowNode {
                id: "solve_thermo".to_string(),
                kind: WorkflowNodeKind::Solve,
                operator_id: Some("solve.thermal_plane_triangle_2d".to_string()),
                name: Some("Solve thermo triangle".to_string()),
                description: None,
                config: None,
                cache_policy: None,
                inputs: vec![WorkflowPort {
                    id: "model".to_string(),
                    artifact_type: "study_model/thermal_plane_triangle_2d".to_string(),
                    name: None,
                    required: None,
                    cardinality: None,
                    dataset_value: None,
                }],
                outputs: vec![WorkflowPort {
                    id: "result".to_string(),
                    artifact_type: "result/thermal_plane_triangle_2d".to_string(),
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
                    "fields": ["max_displacement", "max_stress", "max_temperature_delta"]
                })),
                cache_policy: None,
                inputs: vec![WorkflowPort {
                    id: "result".to_string(),
                    artifact_type: "result/thermal_plane_triangle_2d".to_string(),
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
                id: "summary_output".to_string(),
                kind: WorkflowNodeKind::Output,
                operator_id: None,
                name: Some("Summary output".to_string()),
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
                outputs: vec![],
            },
        ],
        edges: vec![
            WorkflowEdge {
                id: "edge-input".to_string(),
                from: WorkflowNodePortRef {
                    node: "heat_model".to_string(),
                    port: "model".to_string(),
                },
                to: WorkflowNodePortRef {
                    node: "solve_heat".to_string(),
                    port: "model".to_string(),
                },
                artifact_type: "study_model/heat_plane_triangle_2d".to_string(),
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
                    port: "source".to_string(),
                },
                artifact_type: "result/heat_plane_triangle_2d".to_string(),
                dataset_value: None,
            },
            WorkflowEdge {
                id: "edge-thermo-model".to_string(),
                from: WorkflowNodePortRef {
                    node: "bridge_temperature".to_string(),
                    port: "bridged_model".to_string(),
                },
                to: WorkflowNodePortRef {
                    node: "solve_thermo".to_string(),
                    port: "model".to_string(),
                },
                artifact_type: "study_model/thermal_plane_triangle_2d".to_string(),
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
                artifact_type: "result/thermal_plane_triangle_2d".to_string(),
                dataset_value: None,
            },
            WorkflowEdge {
                id: "edge-summary".to_string(),
                from: WorkflowNodePortRef {
                    node: "extract_summary".to_string(),
                    port: "summary".to_string(),
                },
                to: WorkflowNodePortRef {
                    node: "summary_output".to_string(),
                    port: "summary".to_string(),
                },
                artifact_type: "extract/result_summary".to_string(),
                dataset_value: None,
            },
        ],
    };

    let run = run_workflow_graph(WorkflowGraphRunRequest {
        graph,
        input_artifacts: BTreeMap::from([(
            "heat_model".to_string(),
            serde_json::json!({
                "nodes": [
                    { "id": "h0", "x": 0.0, "y": 0.0, "fix_temperature": true, "temperature": 100.0, "heat_load": 0.0 },
                    { "id": "h1", "x": 1.0, "y": 0.0, "fix_temperature": true, "temperature": 20.0, "heat_load": 0.0 },
                    { "id": "h2", "x": 0.0, "y": 1.0, "fix_temperature": false, "temperature": 0.0, "heat_load": 0.0 }
                ],
                "elements": [
                    { "id": "ht0", "node_i": 0, "node_j": 1, "node_k": 2, "thickness": 0.02, "conductivity": 45.0 }
                ]
            }),
        )]),
    })
    .expect("heat triangle -> thermo triangle summary workflow should run");

    let summary = run
        .artifacts
        .get("summary_output.summary")
        .cloned()
        .expect("summary artifact should exist");
    assert_eq!(run.completed_nodes.len(), 6);
    assert!(summary.get("max_displacement").is_some());
    assert!(summary.get("max_stress").is_some());
    assert_eq!(summary["max_temperature_delta"], serde_json::json!(100.0));
}
