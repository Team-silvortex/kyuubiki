use crate::run_workflow_graph;
use kyuubiki_protocol::{
    WorkflowDatasetContract, WorkflowDatasetShape, WorkflowDatasetValueInfo, WorkflowDefaults,
    WorkflowEdge, WorkflowGraph, WorkflowGraphRunRequest, WorkflowNode, WorkflowNodeKind,
    WorkflowNodePortRef, WorkflowPort,
};
use std::collections::BTreeMap;

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
