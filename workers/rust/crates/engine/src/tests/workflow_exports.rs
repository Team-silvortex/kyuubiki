use crate::run_workflow_graph;
use kyuubiki_protocol::{
    WorkflowCachePolicy, WorkflowDatasetContract, WorkflowDatasetShape, WorkflowDatasetValueInfo,
    WorkflowDefaults, WorkflowEdge, WorkflowGraph, WorkflowGraphRunRequest, WorkflowNode,
    WorkflowNodeKind, WorkflowNodePortRef, WorkflowPort,
};
use std::collections::BTreeMap;

#[test]
fn runs_solve_extract_export_output_graph() {
    let graph = WorkflowGraph {
        schema_version: "kyuubiki.workflow-graph/v1".to_string(),
        id: "workflow.heat-summary-export-csv".to_string(),
        name: "Heat summary export csv".to_string(),
        version: "1.0.0".to_string(),
        description: Some("Solve then extract summary and export CSV".to_string()),
        dataset_contract: None,
        entry_nodes: vec!["heat_model".to_string()],
        output_nodes: vec!["csv_output".to_string()],
        defaults: WorkflowDefaults {
            cache_policy: Some(WorkflowCachePolicy::Cached),
            orchestrated: Some(true),
        },
        nodes: vec![
            WorkflowNode {
                id: "heat_model".to_string(),
                kind: WorkflowNodeKind::Input,
                operator_id: None,
                name: Some("Heat input".to_string()),
                description: None,
                config: None,
                cache_policy: None,
                inputs: vec![],
                outputs: vec![WorkflowPort {
                    id: "model".to_string(),
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
                id: "extract_summary".to_string(),
                kind: WorkflowNodeKind::Extract,
                operator_id: Some("extract.result_summary".to_string()),
                name: Some("Extract result summary".to_string()),
                description: None,
                config: Some(serde_json::json!({
                    "fields": ["max_temperature", "max_heat_flux"]
                })),
                cache_policy: None,
                inputs: vec![WorkflowPort {
                    id: "result".to_string(),
                    artifact_type: "result/heat_plane_quad_2d".to_string(),
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
                id: "export_csv".to_string(),
                kind: WorkflowNodeKind::Export,
                operator_id: Some("export.summary_csv".to_string()),
                name: Some("Export summary CSV".to_string()),
                description: None,
                config: Some(serde_json::json!({
                    "fields": ["max_temperature", "max_heat_flux"]
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
                    id: "csv".to_string(),
                    artifact_type: "export/csv".to_string(),
                    name: None,
                    required: None,
                    cardinality: None,
                    dataset_value: None,
                }],
            },
            WorkflowNode {
                id: "csv_output".to_string(),
                kind: WorkflowNodeKind::Output,
                operator_id: None,
                name: Some("CSV output".to_string()),
                description: None,
                config: None,
                cache_policy: None,
                inputs: vec![WorkflowPort {
                    id: "csv".to_string(),
                    artifact_type: "export/csv".to_string(),
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
                id: "edge-heat-input".to_string(),
                from: WorkflowNodePortRef {
                    node: "heat_model".to_string(),
                    port: "model".to_string(),
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
                    node: "extract_summary".to_string(),
                    port: "result".to_string(),
                },
                artifact_type: "result/heat_plane_quad_2d".to_string(),
                dataset_value: None,
            },
            WorkflowEdge {
                id: "edge-summary".to_string(),
                from: WorkflowNodePortRef {
                    node: "extract_summary".to_string(),
                    port: "summary".to_string(),
                },
                to: WorkflowNodePortRef {
                    node: "export_csv".to_string(),
                    port: "summary".to_string(),
                },
                artifact_type: "report/summary".to_string(),
                dataset_value: None,
            },
            WorkflowEdge {
                id: "edge-csv".to_string(),
                from: WorkflowNodePortRef {
                    node: "export_csv".to_string(),
                    port: "csv".to_string(),
                },
                to: WorkflowNodePortRef {
                    node: "csv_output".to_string(),
                    port: "csv".to_string(),
                },
                artifact_type: "export/csv".to_string(),
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
                    { "id": "h0", "x": 0, "y": 0, "fix_temperature": true, "temperature": 100, "heat_load": 0 },
                    { "id": "h1", "x": 1, "y": 0, "fix_temperature": false, "temperature": 0, "heat_load": 0 },
                    { "id": "h2", "x": 1, "y": 1, "fix_temperature": true, "temperature": 20, "heat_load": 0 },
                    { "id": "h3", "x": 0, "y": 1, "fix_temperature": true, "temperature": 20, "heat_load": 0 }
                ],
                "elements": [
                    { "id": "hq0", "node_i": 0, "node_j": 1, "node_k": 2, "node_l": 3, "thickness": 0.02, "conductivity": 45 }
                ]
            }),
        )]),
    })
    .expect("solve -> extract -> export -> output graph should run");

    let exported = run
        .artifacts
        .get("csv_output.csv")
        .cloned()
        .expect("csv export artifact should exist");
    assert_eq!(run.completed_nodes.len(), 5);
    assert_eq!(exported["format"], serde_json::json!("csv"));
    let content = exported["content"]
        .as_str()
        .expect("csv content should be a string");
    assert!(content.contains("key,value"));
    assert!(content.contains("max_temperature,100"));
    assert!(content.contains("max_heat_flux"));
}

#[test]
fn runs_electrostatic_bar_extract_export_graph() {
    let graph = WorkflowGraph {
        schema_version: "kyuubiki.workflow-graph/v1".to_string(),
        id: "workflow.electrostatic-bar-summary-json".to_string(),
        name: "Electrostatic bar summary json".to_string(),
        version: "1.0.0".to_string(),
        description: Some("Solve electrostatic bar, extract summary, and export JSON.".to_string()),
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
                name: Some("Electrostatic bar input".to_string()),
                description: None,
                config: None,
                cache_policy: None,
                inputs: vec![],
                outputs: vec![WorkflowPort {
                    id: "model".to_string(),
                    artifact_type: "study_model/electrostatic_bar_1d".to_string(),
                    name: None,
                    required: None,
                    cardinality: None,
                    dataset_value: None,
                }],
            },
            WorkflowNode {
                id: "solve_electrostatic".to_string(),
                kind: WorkflowNodeKind::Solve,
                operator_id: Some("solve.electrostatic_bar_1d".to_string()),
                name: Some("Solve electrostatic bar".to_string()),
                description: None,
                config: None,
                cache_policy: None,
                inputs: vec![WorkflowPort {
                    id: "model".to_string(),
                    artifact_type: "study_model/electrostatic_bar_1d".to_string(),
                    name: None,
                    required: None,
                    cardinality: None,
                    dataset_value: None,
                }],
                outputs: vec![WorkflowPort {
                    id: "result".to_string(),
                    artifact_type: "result/electrostatic_bar_1d".to_string(),
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
                name: Some("Extract electrostatic summary".to_string()),
                description: None,
                config: Some(serde_json::json!({
                    "fields": ["max_potential", "max_electric_field", "max_flux_density"]
                })),
                cache_policy: None,
                inputs: vec![WorkflowPort {
                    id: "result".to_string(),
                    artifact_type: "result/electrostatic_bar_1d".to_string(),
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
                name: Some("Export summary JSON".to_string()),
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
                id: "edge-electrostatic-input".to_string(),
                from: WorkflowNodePortRef {
                    node: "electrostatic_model".to_string(),
                    port: "model".to_string(),
                },
                to: WorkflowNodePortRef {
                    node: "solve_electrostatic".to_string(),
                    port: "model".to_string(),
                },
                artifact_type: "study_model/electrostatic_bar_1d".to_string(),
                dataset_value: None,
            },
            WorkflowEdge {
                id: "edge-electrostatic-result".to_string(),
                from: WorkflowNodePortRef {
                    node: "solve_electrostatic".to_string(),
                    port: "result".to_string(),
                },
                to: WorkflowNodePortRef {
                    node: "extract_summary".to_string(),
                    port: "result".to_string(),
                },
                artifact_type: "result/electrostatic_bar_1d".to_string(),
                dataset_value: None,
            },
            WorkflowEdge {
                id: "edge-summary".to_string(),
                from: WorkflowNodePortRef {
                    node: "extract_summary".to_string(),
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
                    { "id": "e0", "x": 0.0, "fix_potential": true, "potential": 10.0, "charge_density": 0.0 },
                    { "id": "e1", "x": 1.0, "fix_potential": true, "potential": 0.0, "charge_density": 0.0 }
                ],
                "elements": [
                    { "id": "eb0", "node_i": 0, "node_j": 1, "area": 0.01, "permittivity": 2.5 }
                ]
            }),
        )]),
    })
    .expect("electrostatic bar -> extract -> export graph should run");

    let exported = run
        .artifacts
        .get("json_output.json")
        .cloned()
        .expect("json export artifact should exist");
    assert_eq!(run.completed_nodes.len(), 5);
    assert_eq!(exported["format"], serde_json::json!("json"));
    let content = exported["content"]
        .as_str()
        .expect("json content should be a string");
    assert!(content.contains("max_potential"));
    assert!(content.contains("max_electric_field"));
    assert!(content.contains("max_flux_density"));
}

#[test]
fn rejects_workflow_graph_with_mismatched_dataset_contract() {
    let graph = WorkflowGraph {
        schema_version: "kyuubiki.workflow-graph/v1".to_string(),
        id: "workflow.invalid-dataset-contract".to_string(),
        name: "Invalid dataset contract".to_string(),
        version: "1.0.0".to_string(),
        description: Some("Graph with mismatched artifact and dataset semantic type".to_string()),
        dataset_contract: Some(WorkflowDatasetContract {
            id: "dataset.invalid/v1".to_string(),
            version: "1.0.0".to_string(),
            values: vec![WorkflowDatasetValueInfo {
                id: "bad_summary".to_string(),
                data_class: "result".to_string(),
                element_type: "json_object".to_string(),
                shape: WorkflowDatasetShape::default(),
                semantic_type: Some("result/thermal_plane_quad_2d".to_string()),
                unit: None,
                encoding: None,
                schema_ref: None,
            }],
            metadata: BTreeMap::new(),
        }),
        entry_nodes: vec!["in".to_string()],
        output_nodes: vec!["out".to_string()],
        defaults: WorkflowDefaults::default(),
        nodes: vec![
            WorkflowNode {
                id: "in".to_string(),
                kind: WorkflowNodeKind::Input,
                operator_id: None,
                name: None,
                description: None,
                config: None,
                cache_policy: None,
                inputs: vec![],
                outputs: vec![WorkflowPort {
                    id: "value".to_string(),
                    artifact_type: "report/summary".to_string(),
                    name: None,
                    required: None,
                    cardinality: None,
                    dataset_value: Some("bad_summary".to_string()),
                }],
            },
            WorkflowNode {
                id: "out".to_string(),
                kind: WorkflowNodeKind::Output,
                operator_id: None,
                name: None,
                description: None,
                config: None,
                cache_policy: None,
                inputs: vec![WorkflowPort {
                    id: "value".to_string(),
                    artifact_type: "report/summary".to_string(),
                    name: None,
                    required: None,
                    cardinality: None,
                    dataset_value: Some("bad_summary".to_string()),
                }],
                outputs: vec![],
            },
        ],
        edges: vec![WorkflowEdge {
            id: "e0".to_string(),
            from: WorkflowNodePortRef {
                node: "in".to_string(),
                port: "value".to_string(),
            },
            to: WorkflowNodePortRef {
                node: "out".to_string(),
                port: "value".to_string(),
            },
            artifact_type: "report/summary".to_string(),
            dataset_value: Some("bad_summary".to_string()),
        }],
    };

    let error = run_workflow_graph(WorkflowGraphRunRequest {
        graph,
        input_artifacts: BTreeMap::from([(
            "in".to_string(),
            serde_json::json!({ "max_temperature": 100.0 }),
        )]),
    })
    .expect_err("dataset contract mismatch should be rejected");

    assert!(error.contains("semantic_type"));
}
