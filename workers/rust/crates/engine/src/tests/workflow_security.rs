use crate::run_workflow_graph;
use kyuubiki_protocol::{
    WorkflowDatasetAxis, WorkflowDatasetContract, WorkflowDatasetShape, WorkflowDatasetValueInfo,
    WorkflowDefaults, WorkflowEdge, WorkflowGraph, WorkflowGraphRunRequest, WorkflowNode,
    WorkflowNodeKind, WorkflowNodePortRef, WorkflowPort,
};
use serde_json::Value;
use std::collections::BTreeMap;

#[test]
fn rejects_duplicate_workflow_node_ids_before_execution() {
    let mut graph = minimal_graph();
    graph.nodes[1].id = "input".to_string();

    assert_engine_security_error(graph, "duplicate workflow node id input");
}

#[test]
fn rejects_missing_edge_target_before_execution() {
    let mut graph = minimal_graph();
    graph.edges[0].to.node = "missing".to_string();

    assert_engine_security_error(graph, "references missing target node missing");
}

#[test]
fn rejects_edge_artifact_type_mismatch_before_execution() {
    let mut graph = minimal_graph();
    graph.edges[0].artifact_type = "study_model/wrong".to_string();

    assert_engine_security_error(graph, "does not match source port input.model type");
}

#[test]
fn rejects_duplicate_target_port_edges_before_execution() {
    let mut graph = minimal_graph();
    graph.nodes.insert(
        1,
        WorkflowNode {
            id: "input_b".to_string(),
            kind: WorkflowNodeKind::Input,
            operator_id: None,
            name: None,
            description: None,
            config: None,
            cache_policy: None,
            inputs: vec![],
            outputs: vec![port("model", "study_model/bar_1d")],
        },
    );
    graph.entry_nodes.push("input_b".to_string());
    graph.edges.push(edge(
        "edge_input_b",
        "input_b",
        "model",
        "solve",
        "model",
        "study_model/bar_1d",
    ));

    assert_engine_security_error(graph, "duplicate incoming edge for target port solve.model");
}

#[test]
fn rejects_non_input_entry_node_before_execution() {
    let mut graph = minimal_graph();
    graph.entry_nodes = vec!["solve".to_string()];

    assert_engine_security_error(graph, "workflow entry node solve must be an input node");
}

#[test]
fn rejects_non_output_output_node_before_execution() {
    let mut graph = minimal_graph();
    graph.output_nodes = vec!["solve".to_string()];

    assert_engine_security_error(graph, "workflow output node solve must be an output node");
}

#[test]
fn rejects_cycle_before_execution() {
    let mut graph = minimal_graph();
    graph.nodes[0].inputs = vec![port("feedback", "result/bar_1d")];
    graph.nodes[2].outputs = vec![port("result", "result/bar_1d")];
    graph.edges.push(edge(
        "edge_feedback",
        "output",
        "result",
        "input",
        "feedback",
        "result/bar_1d",
    ));

    assert_engine_security_error(graph, "workflow graph must be acyclic");
}

#[test]
fn rejects_unsupported_operator_before_execution() {
    let mut graph = minimal_graph();
    graph.nodes[1].operator_id = Some("solve.not_real".to_string());

    assert_engine_security_error(graph, "uses unsupported operator solve.not_real");
}

#[test]
fn rejects_unknown_recovery_policy_before_execution() {
    let mut graph = minimal_graph();
    graph.nodes[1].config = Some(serde_json::json!({ "on_error": "restart_everything" }));

    assert_engine_security_error(
        graph,
        "workflow node solve config.on_error has unsupported recovery policy restart_everything",
    );
}

#[test]
fn rejects_non_string_recovery_policy_before_execution() {
    let mut graph = minimal_graph();
    graph.nodes[1].config = Some(serde_json::json!({ "recovery": { "on_error": true } }));

    assert_engine_security_error(
        graph,
        "workflow node solve config.recovery.on_error must be a string recovery policy",
    );
}

#[test]
fn rejects_wrong_workflow_schema_version_before_execution() {
    let mut graph = minimal_graph();
    graph.schema_version = "kyuubiki.workflow-graph/v0".to_string();

    assert_engine_security_error(graph, "schema_version must be kyuubiki.workflow-graph/v1");
}

#[test]
fn rejects_extra_input_artifact_for_non_input_node() {
    let graph = minimal_graph();
    let error = run_workflow_graph(WorkflowGraphRunRequest {
        graph,
        input_artifacts: BTreeMap::from([
            ("input".to_string(), serde_json::json!({})),
            ("output".to_string(), serde_json::json!({})),
        ]),
    })
    .expect_err("non-input artifact target should be rejected");

    assert!(
        error.contains("must target an input node"),
        "unexpected error: {error}"
    );
}

#[test]
fn rejects_node_id_with_artifact_key_separator() {
    let mut graph = minimal_graph();
    graph.nodes[0].id = "input.bad".to_string();

    assert_engine_security_error(graph, "workflow node id contains unsupported characters");
}

#[test]
fn rejects_graphs_over_node_security_budget() {
    let mut graph = minimal_graph();
    graph.nodes = (0..2049)
        .map(|index| WorkflowNode {
            id: format!("input_{index}"),
            kind: WorkflowNodeKind::Input,
            operator_id: None,
            name: None,
            description: None,
            config: None,
            cache_policy: None,
            inputs: vec![],
            outputs: vec![port("model", "study_model/bar_1d")],
        })
        .collect();
    graph.edges.clear();
    graph.entry_nodes = vec!["input_0".to_string()];
    graph.output_nodes.clear();

    assert_engine_security_error(graph, "exceeds node security budget");
}

#[test]
fn rejects_node_config_over_json_depth_budget() {
    let mut graph = minimal_graph();
    graph.nodes[1].config = Some(nested_json(65));

    assert_engine_security_error(graph, "exceeds JSON depth security budget");
}

#[test]
fn rejects_dataset_contract_over_json_depth_budget() {
    let mut graph = minimal_graph();
    graph.dataset_contract = Some(WorkflowDatasetContract {
        id: "dataset.security".to_string(),
        version: "1.0.0".to_string(),
        values: vec![WorkflowDatasetValueInfo {
            id: "summary".to_string(),
            data_class: "report".to_string(),
            element_type: "json_object".to_string(),
            shape: WorkflowDatasetShape {
                axes: vec![WorkflowDatasetAxis {
                    id: "axis".to_string(),
                    label: None,
                    size: None,
                    semantic: Some("x".repeat(1_000_001)),
                }],
            },
            semantic_type: Some("report/security".to_string()),
            unit: None,
            encoding: None,
            schema_ref: None,
        }],
        metadata: BTreeMap::new(),
    });

    assert_engine_security_error(
        graph,
        "workflow dataset_contract string exceeds length security budget",
    );
}

#[test]
fn rejects_input_artifact_over_string_budget() {
    let graph = minimal_graph();
    let oversized = "x".repeat(1_000_001);
    let error = run_workflow_graph(WorkflowGraphRunRequest {
        graph,
        input_artifacts: BTreeMap::from([("input".to_string(), Value::String(oversized))]),
    })
    .expect_err("oversized input artifact should be rejected");

    assert!(
        error.contains("string exceeds length security budget"),
        "unexpected error: {error}"
    );
}

#[test]
fn rejects_input_artifact_over_object_key_budget() {
    let graph = minimal_graph();
    let long_key = "k".repeat(257);
    let error = run_workflow_graph(WorkflowGraphRunRequest {
        graph,
        input_artifacts: BTreeMap::from([(
            "input".to_string(),
            serde_json::json!({ long_key: true }),
        )]),
    })
    .expect_err("oversized object key should be rejected");

    assert!(
        error.contains("object key exceeds length security budget"),
        "unexpected error: {error}"
    );
}

#[test]
fn rejects_operator_output_over_string_budget() {
    let graph = oversized_markdown_export_graph();
    let error = run_workflow_graph(WorkflowGraphRunRequest {
        graph,
        input_artifacts: BTreeMap::from([(
            "input".to_string(),
            serde_json::json!({ "status": "warn" }),
        )]),
    })
    .expect_err("oversized operator output should be rejected");

    assert!(
        error.contains("workflow node export output string exceeds length security budget"),
        "unexpected error: {error}"
    );
}

fn assert_engine_security_error(graph: WorkflowGraph, expected: &str) {
    let error = run_workflow_graph(WorkflowGraphRunRequest {
        graph,
        input_artifacts: BTreeMap::from([("input".to_string(), serde_json::json!({}))]),
    })
    .expect_err("workflow security guard should reject graph");

    assert!(
        error.contains(expected),
        "expected {expected:?}, got {error:?}"
    );
}

fn oversized_markdown_export_graph() -> WorkflowGraph {
    WorkflowGraph {
        schema_version: "kyuubiki.workflow-graph/v1".to_string(),
        id: "workflow.security-output-budget".to_string(),
        name: "Security output budget".to_string(),
        version: "1.0.0".to_string(),
        description: None,
        dataset_contract: None,
        entry_nodes: vec!["input".to_string()],
        output_nodes: vec!["output".to_string()],
        defaults: WorkflowDefaults::default(),
        nodes: vec![
            WorkflowNode {
                id: "input".to_string(),
                kind: WorkflowNodeKind::Input,
                operator_id: None,
                name: None,
                description: None,
                config: None,
                cache_policy: None,
                inputs: vec![],
                outputs: vec![port("summary", "summary/generic")],
            },
            WorkflowNode {
                id: "export".to_string(),
                kind: WorkflowNodeKind::Export,
                operator_id: Some("export.alert_markdown".to_string()),
                name: None,
                description: None,
                config: Some(serde_json::json!({
                    "title": "x".repeat(500_001),
                    "summary": "output budget smoke"
                })),
                cache_policy: None,
                inputs: vec![port("summary", "summary/generic")],
                outputs: vec![port("report", "report/markdown")],
            },
            WorkflowNode {
                id: "output".to_string(),
                kind: WorkflowNodeKind::Output,
                operator_id: None,
                name: None,
                description: None,
                config: None,
                cache_policy: None,
                inputs: vec![port("report", "report/markdown")],
                outputs: vec![],
            },
        ],
        edges: vec![
            edge(
                "edge_input_export",
                "input",
                "summary",
                "export",
                "summary",
                "summary/generic",
            ),
            edge(
                "edge_export_output",
                "export",
                "report",
                "output",
                "report",
                "report/markdown",
            ),
        ],
    }
}

fn nested_json(depth: usize) -> Value {
    let mut value = Value::Null;
    for _ in 0..depth {
        value = serde_json::json!({ "next": value });
    }
    value
}

fn minimal_graph() -> WorkflowGraph {
    WorkflowGraph {
        schema_version: "kyuubiki.workflow-graph/v1".to_string(),
        id: "workflow.security-smoke".to_string(),
        name: "Security smoke".to_string(),
        version: "1.0.0".to_string(),
        description: None,
        dataset_contract: None,
        entry_nodes: vec!["input".to_string()],
        output_nodes: vec!["output".to_string()],
        defaults: WorkflowDefaults::default(),
        nodes: vec![
            WorkflowNode {
                id: "input".to_string(),
                kind: WorkflowNodeKind::Input,
                operator_id: None,
                name: None,
                description: None,
                config: None,
                cache_policy: None,
                inputs: vec![],
                outputs: vec![port("model", "study_model/bar_1d")],
            },
            WorkflowNode {
                id: "solve".to_string(),
                kind: WorkflowNodeKind::Solve,
                operator_id: Some("solve.bar_1d".to_string()),
                name: None,
                description: None,
                config: None,
                cache_policy: None,
                inputs: vec![port("model", "study_model/bar_1d")],
                outputs: vec![port("result", "result/bar_1d")],
            },
            WorkflowNode {
                id: "output".to_string(),
                kind: WorkflowNodeKind::Output,
                operator_id: None,
                name: None,
                description: None,
                config: None,
                cache_policy: None,
                inputs: vec![port("result", "result/bar_1d")],
                outputs: vec![],
            },
        ],
        edges: vec![
            edge(
                "edge_input",
                "input",
                "model",
                "solve",
                "model",
                "study_model/bar_1d",
            ),
            edge(
                "edge_output",
                "solve",
                "result",
                "output",
                "result",
                "result/bar_1d",
            ),
        ],
    }
}

fn port(id: &str, artifact_type: &str) -> WorkflowPort {
    WorkflowPort {
        id: id.to_string(),
        artifact_type: artifact_type.to_string(),
        name: None,
        required: None,
        cardinality: None,
        dataset_value: None,
    }
}

fn edge(
    id: &str,
    from_node: &str,
    from_port: &str,
    to_node: &str,
    to_port: &str,
    artifact_type: &str,
) -> WorkflowEdge {
    WorkflowEdge {
        id: id.to_string(),
        from: WorkflowNodePortRef {
            node: from_node.to_string(),
            port: from_port.to_string(),
        },
        to: WorkflowNodePortRef {
            node: to_node.to_string(),
            port: to_port.to_string(),
        },
        artifact_type: artifact_type.to_string(),
        dataset_value: None,
    }
}
