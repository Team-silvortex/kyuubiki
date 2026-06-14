use crate::run_workflow_graph;
use crate::workflow_reporting::extract_field_hotspots;
use kyuubiki_protocol::{
    WorkflowCachePolicy, WorkflowDefaults, WorkflowEdge, WorkflowGraph, WorkflowGraphRunRequest,
    WorkflowNode, WorkflowNodeKind, WorkflowNodePortRef, WorkflowPort,
};
use std::collections::BTreeMap;

#[test]
fn extracts_hotspot_summary_from_numeric_field() {
    let summary = extract_field_hotspots(
        serde_json::json!({
            "elements": [
                { "id": "e0", "field": 1.0 },
                { "id": "e1", "field": 2.0 },
                { "id": "e2", "field": 3.0 },
                { "id": "e3", "field": 10.0 }
            ]
        }),
        serde_json::json!({
            "source": "elements",
            "field": "field",
            "output_prefix": "field",
            "percentile": 75,
            "sample_limit": 2
        }),
    )
    .expect("hotspot extraction should succeed");

    assert_eq!(summary["field_threshold"], serde_json::json!(4.75));
    assert_eq!(summary["field_hotspot_count"], serde_json::json!(1));
    assert_eq!(summary["field_hotspot_fraction"], serde_json::json!(0.25));
    assert_eq!(summary["field_hotspot_mean"], serde_json::json!(10.0));
    assert_eq!(summary["field_hotspot_max"], serde_json::json!(10.0));
    assert_eq!(summary["field_hotspot_ids"], serde_json::json!(["e3"]));
}

#[test]
fn runs_electrostatic_plane_field_hotspots_graph() {
    let graph = WorkflowGraph {
        schema_version: "kyuubiki.workflow-graph/v1".to_string(),
        id: "workflow.electrostatic-plane-field-hotspots".to_string(),
        name: "Electrostatic plane field hotspots".to_string(),
        version: "1.0.0".to_string(),
        description: Some(
            "Solve an electrostatic quad model then extract hotspot samples.".to_string(),
        ),
        dataset_contract: None,
        entry_nodes: vec!["electrostatic_model".to_string()],
        output_nodes: vec!["json_output".to_string()],
        defaults: WorkflowDefaults {
            cache_policy: Some(WorkflowCachePolicy::Cached),
            orchestrated: Some(true),
        },
        nodes: vec![
            WorkflowNode {
                id: "electrostatic_model".to_string(),
                kind: WorkflowNodeKind::Input,
                operator_id: None,
                name: Some("Electrostatic plane input".to_string()),
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
                name: Some("Solve electrostatic quad".to_string()),
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
                id: "field_hotspots".to_string(),
                kind: WorkflowNodeKind::Extract,
                operator_id: Some("extract.field_hotspots".to_string()),
                name: Some("Extract field hotspots".to_string()),
                description: None,
                config: Some(serde_json::json!({
                    "source": "elements",
                    "field": "electric_field_magnitude",
                    "output_prefix": "field",
                    "percentile": 90,
                    "sample_limit": 4
                })),
                cache_policy: None,
                inputs: vec![WorkflowPort {
                    id: "result".to_string(),
                    artifact_type: "result/electrostatic_plane_quad_2d".to_string(),
                    name: None,
                    required: None,
                    cardinality: None,
                    dataset_value: None,
                }],
                outputs: vec![WorkflowPort {
                    id: "summary".to_string(),
                    artifact_type: "report/summary".to_string(),
                    name: None,
                    required: None,
                    cardinality: None,
                    dataset_value: None,
                }],
            },
            WorkflowNode {
                id: "export_json".to_string(),
                kind: WorkflowNodeKind::Export,
                operator_id: Some("export.summary_json".to_string()),
                name: Some("Export hotspot JSON".to_string()),
                description: None,
                config: None,
                cache_policy: None,
                inputs: vec![WorkflowPort {
                    id: "summary".to_string(),
                    artifact_type: "report/summary".to_string(),
                    name: None,
                    required: None,
                    cardinality: None,
                    dataset_value: None,
                }],
                outputs: vec![WorkflowPort {
                    id: "json".to_string(),
                    artifact_type: "export/json".to_string(),
                    name: None,
                    required: None,
                    cardinality: None,
                    dataset_value: None,
                }],
            },
            WorkflowNode {
                id: "json_output".to_string(),
                kind: WorkflowNodeKind::Output,
                operator_id: None,
                name: Some("JSON output".to_string()),
                description: None,
                config: None,
                cache_policy: None,
                inputs: vec![WorkflowPort {
                    id: "json".to_string(),
                    artifact_type: "export/json".to_string(),
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
                id: "edge-result".to_string(),
                from: WorkflowNodePortRef {
                    node: "solve_electrostatic".to_string(),
                    port: "result".to_string(),
                },
                to: WorkflowNodePortRef {
                    node: "field_hotspots".to_string(),
                    port: "result".to_string(),
                },
                artifact_type: "result/electrostatic_plane_quad_2d".to_string(),
                dataset_value: None,
            },
            WorkflowEdge {
                id: "edge-summary".to_string(),
                from: WorkflowNodePortRef {
                    node: "field_hotspots".to_string(),
                    port: "summary".to_string(),
                },
                to: WorkflowNodePortRef {
                    node: "export_json".to_string(),
                    port: "summary".to_string(),
                },
                artifact_type: "report/summary".to_string(),
                dataset_value: None,
            },
            WorkflowEdge {
                id: "edge-json".to_string(),
                from: WorkflowNodePortRef {
                    node: "export_json".to_string(),
                    port: "json".to_string(),
                },
                to: WorkflowNodePortRef {
                    node: "json_output".to_string(),
                    port: "json".to_string(),
                },
                artifact_type: "export/json".to_string(),
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
                    { "id": "n0", "x": 0.0, "y": 0.0, "fix_potential": true, "potential": 10.0, "charge_density": 0.0 },
                    { "id": "n1", "x": 1.0, "y": 0.0, "fix_potential": false, "potential": 0.0, "charge_density": 0.0 },
                    { "id": "n2", "x": 1.0, "y": 1.0, "fix_potential": false, "potential": 0.0, "charge_density": 0.0 },
                    { "id": "n3", "x": 0.0, "y": 1.0, "fix_potential": true, "potential": 5.0, "charge_density": 0.0 }
                ],
                "elements": [
                    { "id": "q0", "node_i": 0, "node_j": 1, "node_k": 2, "node_l": 3, "permittivity": 2.0, "thickness": 0.1 }
                ]
            }),
        )]),
    })
    .expect("electrostatic plane field hotspots graph should run");

    let exported = run
        .artifacts
        .get("json_output.json")
        .cloned()
        .expect("json export artifact should exist");
    let content = exported["content"]
        .as_str()
        .expect("json content should be a string");
    assert!(content.contains("field_threshold"));
    assert!(content.contains("field_hotspot_count"));
    assert!(content.contains("field_hotspot_samples"));
}
