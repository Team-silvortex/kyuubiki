use crate::run_workflow_graph;
use kyuubiki_protocol::{
    OperatorSchemaRef, WorkflowDatasetAxis, WorkflowDatasetContract, WorkflowDatasetShape,
    WorkflowDatasetValueInfo, WorkflowDefaults, WorkflowEdge, WorkflowGraph,
    WorkflowGraphRunRequest, WorkflowNode, WorkflowNodeKind, WorkflowNodePortRef, WorkflowPort,
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

#[test]
fn rejects_workflow_graph_with_duplicate_dataset_value_ids() {
    let mut graph = dataset_contract_smoke_graph();
    graph.dataset_contract = Some(WorkflowDatasetContract {
        id: "dataset.duplicate/v1".to_string(),
        version: "1.0.0".to_string(),
        values: vec![
            dataset_value("result_summary", "report/summary"),
            dataset_value("result_summary", "report/summary"),
        ],
        metadata: BTreeMap::new(),
    });

    let error = run_workflow_graph(WorkflowGraphRunRequest {
        graph,
        input_artifacts: BTreeMap::from([(
            "in".to_string(),
            serde_json::json!({ "max_temperature": 100.0 }),
        )]),
    })
    .expect_err("duplicate dataset value ids should be rejected");

    assert!(error.contains("duplicate dataset value id result_summary"));
}

#[test]
fn rejects_workflow_graph_with_empty_dataset_value_id() {
    let mut graph = dataset_contract_smoke_graph();
    graph.dataset_contract = Some(WorkflowDatasetContract {
        id: "dataset.empty-value/v1".to_string(),
        version: "1.0.0".to_string(),
        values: vec![dataset_value("", "report/summary")],
        metadata: BTreeMap::new(),
    });

    let error = run_workflow_graph(WorkflowGraphRunRequest {
        graph,
        input_artifacts: BTreeMap::from([(
            "in".to_string(),
            serde_json::json!({ "max_temperature": 100.0 }),
        )]),
    })
    .expect_err("empty dataset value ids should be rejected");

    assert!(error.contains("empty dataset value id"));
}

#[test]
fn rejects_workflow_graph_with_unsupported_dataset_data_class() {
    let mut graph = dataset_contract_smoke_graph();
    let mut value = dataset_value("result_summary", "report/summary");
    value.data_class = "mystery_blob".to_string();
    graph.dataset_contract = Some(dataset_contract(vec![value]));

    let error =
        run_smoke_graph(graph).expect_err("unsupported dataset data_class should be rejected");

    assert!(error.contains("unsupported data_class mystery_blob"));
}

#[test]
fn rejects_workflow_graph_with_empty_dataset_element_type() {
    let mut graph = dataset_contract_smoke_graph();
    let mut value = dataset_value("result_summary", "report/summary");
    value.element_type = " ".to_string();
    graph.dataset_contract = Some(dataset_contract(vec![value]));

    let error = run_smoke_graph(graph).expect_err("empty dataset element_type should be rejected");

    assert!(error.contains("empty element_type"));
}

#[test]
fn rejects_workflow_graph_with_duplicate_dataset_shape_axis_ids() {
    let mut graph = dataset_contract_smoke_graph();
    let mut value = dataset_value("result_summary", "report/summary");
    value.shape = WorkflowDatasetShape {
        axes: vec![shape_axis("nodes"), shape_axis("nodes")],
    };
    graph.dataset_contract = Some(dataset_contract(vec![value]));

    let error =
        run_smoke_graph(graph).expect_err("duplicate dataset shape axis ids should be rejected");

    assert!(error.contains("duplicate shape axis id nodes"));
}

#[test]
fn rejects_workflow_graph_with_empty_dataset_schema_ref() {
    let mut graph = dataset_contract_smoke_graph();
    let mut value = dataset_value("result_summary", "report/summary");
    value.schema_ref = Some(OperatorSchemaRef {
        schema: "".to_string(),
        version: "1".to_string(),
    });
    graph.dataset_contract = Some(dataset_contract(vec![value]));

    let error = run_smoke_graph(graph).expect_err("empty dataset schema_ref should be rejected");

    assert!(error.contains("empty schema_ref"));
}

fn dataset_contract_smoke_graph() -> WorkflowGraph {
    WorkflowGraph {
        schema_version: "kyuubiki.workflow-graph/v1".to_string(),
        id: "workflow.dataset-contract-smoke".to_string(),
        name: "Dataset contract smoke".to_string(),
        version: "1.0.0".to_string(),
        description: None,
        dataset_contract: Some(WorkflowDatasetContract {
            id: "dataset.smoke/v1".to_string(),
            version: "1.0.0".to_string(),
            values: vec![dataset_value("result_summary", "report/summary")],
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
                    dataset_value: Some("result_summary".to_string()),
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
                    dataset_value: Some("result_summary".to_string()),
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
            dataset_value: Some("result_summary".to_string()),
        }],
    }
}

fn dataset_value(id: &str, semantic_type: &str) -> WorkflowDatasetValueInfo {
    WorkflowDatasetValueInfo {
        id: id.to_string(),
        data_class: "result".to_string(),
        element_type: "json_object".to_string(),
        shape: WorkflowDatasetShape::default(),
        semantic_type: Some(semantic_type.to_string()),
        unit: None,
        encoding: None,
        schema_ref: None,
    }
}

fn dataset_contract(values: Vec<WorkflowDatasetValueInfo>) -> WorkflowDatasetContract {
    WorkflowDatasetContract {
        id: "dataset.smoke/v1".to_string(),
        version: "1.0.0".to_string(),
        values,
        metadata: BTreeMap::new(),
    }
}

fn shape_axis(id: &str) -> WorkflowDatasetAxis {
    WorkflowDatasetAxis {
        id: id.to_string(),
        label: None,
        size: None,
        semantic: None,
    }
}

fn run_smoke_graph(
    graph: WorkflowGraph,
) -> Result<kyuubiki_protocol::WorkflowGraphRunResult, String> {
    run_workflow_graph(WorkflowGraphRunRequest {
        graph,
        input_artifacts: BTreeMap::from([(
            "in".to_string(),
            serde_json::json!({ "max_temperature": 100.0 }),
        )]),
    })
}
