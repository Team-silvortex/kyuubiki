use crate::run_workflow_graph;
use crate::workflow_reporting::export_alert_markdown;
use kyuubiki_protocol::{
    WorkflowCachePolicy, WorkflowDefaults, WorkflowEdge, WorkflowGraph, WorkflowGraphRunRequest,
    WorkflowNode, WorkflowNodeKind, WorkflowNodePortRef, WorkflowPort,
};
use std::collections::BTreeMap;

#[test]
fn exports_alert_markdown_from_hotspot_summary() {
    let exported = export_alert_markdown(
        serde_json::json!({
            "field_threshold": 12.0,
            "field_hotspot_count": 3,
            "field_hotspot_fraction": 0.2,
            "field_hotspot_samples": [
                { "id": "e9", "electric_field_magnitude": 21.5 },
                { "id": "e3", "electric_field_magnitude": 18.2 }
            ]
        }),
        serde_json::json!({
            "title": "Electrostatic Hotspot Alert",
            "severity": "critical",
            "summary": "Field hotspots exceeded the expected operating envelope.",
            "fields": ["field_threshold", "field_hotspot_count", "field_hotspot_fraction"],
            "sample_count": 2
        }),
    )
    .expect("markdown alert export should succeed");

    let content = exported["content"]
        .as_str()
        .expect("markdown content should be a string");
    assert!(content.contains("# Electrostatic Hotspot Alert"));
    assert!(content.contains("- Severity: critical"));
    assert!(content.contains("- field_hotspot_count: 3"));
    assert!(content.contains("## Sample Context"));
    assert!(content.contains("- e9: electric_field_magnitude=21.5"));
}

#[test]
fn runs_electrostatic_hotspot_alert_export_graph() {
    let graph = WorkflowGraph {
        schema_version: "kyuubiki.workflow-graph/v1".to_string(),
        id: "workflow.electrostatic-hotspot-alert".to_string(),
        name: "Electrostatic hotspot alert".to_string(),
        version: "1.0.0".to_string(),
        description: Some(
            "Solve electrostatic quad, extract hotspots, then export a markdown alert.".to_string(),
        ),
        dataset_contract: None,
        entry_nodes: vec!["electrostatic_model".to_string()],
        output_nodes: vec!["alert_output".to_string()],
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
                    "percentile": 75,
                    "sample_limit": 4,
                    "sample_sort": "value_desc"
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
                id: "export_alert".to_string(),
                kind: WorkflowNodeKind::Export,
                operator_id: Some("export.alert_markdown".to_string()),
                name: Some("Export alert markdown".to_string()),
                description: None,
                config: Some(serde_json::json!({
                    "title": "Electrostatic Hotspot Alert",
                    "severity": "warning",
                    "summary": "Hotspot candidates were detected in the electrostatic field.",
                    "fields": ["field_threshold", "field_hotspot_count", "field_hotspot_fraction"],
                    "sample_count": 4
                })),
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
                    id: "markdown".to_string(),
                    artifact_type: "export/markdown".to_string(),
                    name: None,
                    required: None,
                    cardinality: None,
                    dataset_value: None,
                }],
            },
            WorkflowNode {
                id: "alert_output".to_string(),
                kind: WorkflowNodeKind::Output,
                operator_id: None,
                name: Some("Alert output".to_string()),
                description: None,
                config: None,
                cache_policy: None,
                inputs: vec![WorkflowPort {
                    id: "markdown".to_string(),
                    artifact_type: "export/markdown".to_string(),
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
                    node: "export_alert".to_string(),
                    port: "summary".to_string(),
                },
                artifact_type: "report/summary".to_string(),
                dataset_value: None,
            },
            WorkflowEdge {
                id: "edge-markdown".to_string(),
                from: WorkflowNodePortRef {
                    node: "export_alert".to_string(),
                    port: "markdown".to_string(),
                },
                to: WorkflowNodePortRef {
                    node: "alert_output".to_string(),
                    port: "markdown".to_string(),
                },
                artifact_type: "export/markdown".to_string(),
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
                    { "id": "q0", "node_i": 0, "node_j": 1, "node_k": 2, "node_l": 3, "permittivity": 2.0, "thickness": 0.1 },
                    { "id": "q1", "node_i": 0, "node_j": 1, "node_k": 2, "node_l": 3, "permittivity": 5.0, "thickness": 0.1 }
                ]
            }),
        )]),
    })
    .expect("electrostatic hotspot alert graph should run");

    let exported = run
        .artifacts
        .get("alert_output.markdown")
        .cloned()
        .expect("markdown export artifact should exist");
    let content = exported["content"]
        .as_str()
        .expect("markdown content should be a string");
    assert!(content.contains("# Electrostatic Hotspot Alert"));
    assert!(content.contains("- field_hotspot_count:"));
    assert!(content.contains("- field_hotspot_fraction:"));
    assert!(content.contains("## Sample Context"));
}
