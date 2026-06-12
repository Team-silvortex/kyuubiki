use crate::run_workflow_graph;
use crate::workflow_reporting::extract_field_statistics;
use kyuubiki_protocol::{
    WorkflowCachePolicy, WorkflowDefaults, WorkflowEdge, WorkflowGraph, WorkflowGraphRunRequest,
    WorkflowNode, WorkflowNodeKind, WorkflowNodePortRef, WorkflowPort,
};
use std::collections::BTreeMap;

#[test]
fn runs_electrostatic_plane_field_statistics_graph() {
    let graph = WorkflowGraph {
        schema_version: "kyuubiki.workflow-graph/v1".to_string(),
        id: "workflow.electrostatic-plane-field-statistics".to_string(),
        name: "Electrostatic plane field statistics".to_string(),
        version: "1.0.0".to_string(),
        description: Some("Solve an electrostatic quad model then extract field statistics.".to_string()),
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
                id: "field_stats".to_string(),
                kind: WorkflowNodeKind::Extract,
                operator_id: Some("extract.field_statistics".to_string()),
                name: Some("Extract field statistics".to_string()),
                description: None,
                config: Some(serde_json::json!({
                    "source": "elements",
                    "field": "electric_field_magnitude",
                    "output_prefix": "field",
                    "percentiles": [50, 90]
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
                name: Some("Export statistics JSON".to_string()),
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
                from: WorkflowNodePortRef { node: "electrostatic_model".to_string(), port: "model".to_string() },
                to: WorkflowNodePortRef { node: "solve_electrostatic".to_string(), port: "model".to_string() },
                artifact_type: "study_model/electrostatic_plane_quad_2d".to_string(),
                dataset_value: None,
            },
            WorkflowEdge {
                id: "edge-result".to_string(),
                from: WorkflowNodePortRef { node: "solve_electrostatic".to_string(), port: "result".to_string() },
                to: WorkflowNodePortRef { node: "field_stats".to_string(), port: "result".to_string() },
                artifact_type: "result/electrostatic_plane_quad_2d".to_string(),
                dataset_value: None,
            },
            WorkflowEdge {
                id: "edge-summary".to_string(),
                from: WorkflowNodePortRef { node: "field_stats".to_string(), port: "summary".to_string() },
                to: WorkflowNodePortRef { node: "export_json".to_string(), port: "summary".to_string() },
                artifact_type: "report/summary".to_string(),
                dataset_value: None,
            },
            WorkflowEdge {
                id: "edge-json".to_string(),
                from: WorkflowNodePortRef { node: "export_json".to_string(), port: "json".to_string() },
                to: WorkflowNodePortRef { node: "json_output".to_string(), port: "json".to_string() },
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
    .expect("electrostatic plane field statistics graph should run");

    let exported = run
        .artifacts
        .get("json_output.json")
        .cloned()
        .expect("json export artifact should exist");
    let content = exported["content"].as_str().expect("json content should be a string");
    assert!(content.contains("field_min"));
    assert!(content.contains("field_max"));
    assert!(content.contains("field_mean"));
    assert!(content.contains("field_sum"));
    assert!(content.contains("field_count"));
    assert!(content.contains("field_stddev"));
    assert!(content.contains("field_p50"));
    assert!(content.contains("field_p90"));
}

#[test]
fn computes_stddev_and_percentiles_for_numeric_fields() {
    let summary = extract_field_statistics(
        serde_json::json!({
            "nodes": [
                { "temperature": 1.0 },
                { "temperature": 2.0 },
                { "temperature": 3.0 },
                { "temperature": 4.0 }
            ]
        }),
        serde_json::json!({
            "source": "nodes",
            "field": "temperature",
            "output_prefix": "temp",
            "percentiles": [50, 75, 90]
        }),
    )
    .expect("field statistics should be computed");

    assert_eq!(summary["temp_count"], serde_json::json!(4));
    assert_eq!(summary["temp_min"], serde_json::json!(1.0));
    assert_eq!(summary["temp_max"], serde_json::json!(4.0));
    assert_eq!(summary["temp_sum"], serde_json::json!(10.0));
    assert_eq!(summary["temp_mean"], serde_json::json!(2.5));
    assert_eq!(summary["temp_stddev"], serde_json::json!(1.118033988749895));
    assert_eq!(summary["temp_p50"], serde_json::json!(2.5));
    assert_eq!(summary["temp_p75"], serde_json::json!(3.25));
    assert_eq!(summary["temp_p90"], serde_json::json!(3.7));
}
