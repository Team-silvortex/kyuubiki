use crate::run_workflow_graph;
use kyuubiki_protocol::{
    WorkflowCachePolicy, WorkflowDefaults, WorkflowEdge, WorkflowGraph, WorkflowGraphRunRequest,
    WorkflowGraphRunResult, WorkflowNode, WorkflowNodeKind, WorkflowNodePortRef, WorkflowPort,
};
use std::collections::BTreeMap;

pub(super) fn run_solver_summary_json_graph(
    workflow_id: &str,
    workflow_name: &str,
    input_node_id: &str,
    input_artifact_type: &str,
    solve_node_id: &str,
    solve_operator_id: &str,
    result_artifact_type: &str,
    model: serde_json::Value,
    summary_fields: &[&str],
) -> WorkflowGraphRunResult {
    let graph = WorkflowGraph {
        schema_version: "kyuubiki.workflow-graph/v1".to_string(),
        id: workflow_id.to_string(),
        name: workflow_name.to_string(),
        version: "1.0.0".to_string(),
        description: Some(workflow_name.to_string()),
        dataset_contract: None,
        entry_nodes: vec![input_node_id.to_string()],
        output_nodes: vec!["json_output".to_string()],
        defaults: WorkflowDefaults {
            cache_policy: Some(WorkflowCachePolicy::Cached),
            orchestrated: Some(true),
        },
        nodes: vec![
            WorkflowNode {
                id: input_node_id.to_string(),
                kind: WorkflowNodeKind::Input,
                operator_id: None,
                name: Some("Solver input".to_string()),
                description: None,
                config: None,
                cache_policy: None,
                inputs: vec![],
                outputs: vec![WorkflowPort {
                    id: "model".to_string(),
                    artifact_type: input_artifact_type.to_string(),
                    name: None,
                    required: None,
                    cardinality: None,
                    dataset_value: None,
                }],
            },
            WorkflowNode {
                id: solve_node_id.to_string(),
                kind: WorkflowNodeKind::Solve,
                operator_id: Some(solve_operator_id.to_string()),
                name: Some("Solve".to_string()),
                description: None,
                config: None,
                cache_policy: None,
                inputs: vec![WorkflowPort {
                    id: "model".to_string(),
                    artifact_type: input_artifact_type.to_string(),
                    name: None,
                    required: None,
                    cardinality: None,
                    dataset_value: None,
                }],
                outputs: vec![WorkflowPort {
                    id: "result".to_string(),
                    artifact_type: result_artifact_type.to_string(),
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
                    "fields": summary_fields
                })),
                cache_policy: None,
                inputs: vec![WorkflowPort {
                    id: "result".to_string(),
                    artifact_type: result_artifact_type.to_string(),
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
                name: Some("Export JSON".to_string()),
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
                    node: input_node_id.to_string(),
                    port: "model".to_string(),
                },
                to: WorkflowNodePortRef {
                    node: solve_node_id.to_string(),
                    port: "model".to_string(),
                },
                artifact_type: input_artifact_type.to_string(),
                dataset_value: None,
            },
            WorkflowEdge {
                id: "edge-result".to_string(),
                from: WorkflowNodePortRef {
                    node: solve_node_id.to_string(),
                    port: "result".to_string(),
                },
                to: WorkflowNodePortRef {
                    node: "extract_summary".to_string(),
                    port: "result".to_string(),
                },
                artifact_type: result_artifact_type.to_string(),
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

    run_workflow_graph(WorkflowGraphRunRequest {
        graph,
        input_artifacts: BTreeMap::from([(input_node_id.to_string(), model)]),
    })
    .expect("solver template workflow should run")
}

pub(super) fn exported_content(run: &WorkflowGraphRunResult) -> &str {
    let exported = run
        .artifacts
        .get("json_output.json")
        .expect("json export artifact should exist");
    assert_eq!(run.completed_nodes.len(), 5);
    assert_eq!(exported["format"], serde_json::json!("json"));
    exported["content"]
        .as_str()
        .expect("json content should be a string")
}
