use crate::run_workflow_graph;
use kyuubiki_protocol::{
    WorkflowCachePolicy, WorkflowDefaults, WorkflowEdge, WorkflowGraph, WorkflowGraphRunRequest,
    WorkflowNode, WorkflowNodeKind, WorkflowNodePortRef, WorkflowPort,
};
use std::collections::BTreeMap;

#[test]
fn runs_solve_extract_output_graph() {
    let graph = WorkflowGraph {
        schema_version: "kyuubiki.workflow-graph/v1".to_string(),
        id: "workflow.heat-summary-quad-2d".to_string(),
        name: "Heat summary quad".to_string(),
        version: "1.0.0".to_string(),
        description: Some("Solve then extract summary".to_string()),
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
                id: "summary_output".to_string(),
                kind: WorkflowNodeKind::Output,
                operator_id: None,
                name: Some("Summary output".to_string()),
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
                    node: "summary_output".to_string(),
                    port: "summary".to_string(),
                },
                artifact_type: "report/summary".to_string(),
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
    .expect("solve -> extract -> output graph should run");

    let summary = run
        .artifacts
        .get("summary_output.summary")
        .cloned()
        .expect("summary artifact should exist");
    assert_eq!(run.completed_nodes.len(), 4);
    assert_eq!(summary["max_temperature"], serde_json::json!(100.0));
    assert!(summary.get("max_heat_flux").is_some());
}
