use crate::run_workflow_graph;
use kyuubiki_protocol::{
    WorkflowDefaults, WorkflowEdge, WorkflowGraph, WorkflowGraphRunRequest, WorkflowNode,
    WorkflowNodeKind, WorkflowNodePortRef, WorkflowPort,
};
use std::collections::BTreeMap;

#[test]
fn runs_summary_aggregate_workflow_graph() {
    let graph = WorkflowGraph {
        schema_version: "kyuubiki.workflow-graph/v1".to_string(),
        id: "workflow.summary-aggregate".to_string(),
        name: "Summary aggregate".to_string(),
        version: "1.0.0".to_string(),
        description: Some("Aggregate multiple summary artifacts.".to_string()),
        dataset_contract: None,
        entry_nodes: vec![
            "baseline_summary".to_string(),
            "candidate_summary".to_string(),
            "variant_summary".to_string(),
        ],
        output_nodes: vec!["json_output".to_string()],
        defaults: WorkflowDefaults::default(),
        nodes: vec![
            input_node("baseline_summary"),
            input_node("candidate_summary"),
            input_node("variant_summary"),
            WorkflowNode {
                id: "aggregate_summary".to_string(),
                kind: WorkflowNodeKind::Transform,
                operator_id: Some("transform.aggregate_summary_collection".to_string()),
                name: Some("Aggregate summary collection".to_string()),
                description: None,
                config: Some(serde_json::json!({
                    "fields": ["max_temperature", "max_heat_flux"],
                    "output_prefix": "benchmark",
                    "include_values": true,
                    "include_sources": true
                })),
                cache_policy: None,
                inputs: vec![
                    port("baseline", "artifact/result_summary"),
                    port("candidate", "artifact/result_summary"),
                    port("variant", "artifact/result_summary"),
                ],
                outputs: vec![port("merged", "artifact/result_summary")],
            },
            WorkflowNode {
                id: "export_json".to_string(),
                kind: WorkflowNodeKind::Export,
                operator_id: Some("export.summary_json".to_string()),
                name: Some("Export JSON".to_string()),
                description: None,
                config: None,
                cache_policy: None,
                inputs: vec![port("summary", "artifact/result_summary")],
                outputs: vec![port("json", "artifact/json")],
            },
            WorkflowNode {
                id: "json_output".to_string(),
                kind: WorkflowNodeKind::Output,
                operator_id: None,
                name: Some("JSON output".to_string()),
                description: None,
                config: None,
                cache_policy: None,
                inputs: vec![port("json", "artifact/json")],
                outputs: vec![],
            },
        ],
        edges: vec![
            edge(
                "edge-baseline",
                "baseline_summary",
                "summary",
                "aggregate_summary",
                "baseline",
                "artifact/result_summary",
            ),
            edge(
                "edge-candidate",
                "candidate_summary",
                "summary",
                "aggregate_summary",
                "candidate",
                "artifact/result_summary",
            ),
            edge(
                "edge-variant",
                "variant_summary",
                "summary",
                "aggregate_summary",
                "variant",
                "artifact/result_summary",
            ),
            edge(
                "edge-aggregate-export",
                "aggregate_summary",
                "merged",
                "export_json",
                "summary",
                "artifact/result_summary",
            ),
            edge(
                "edge-export-output",
                "export_json",
                "json",
                "json_output",
                "json",
                "artifact/json",
            ),
        ],
    };

    let run = run_workflow_graph(WorkflowGraphRunRequest {
        graph,
        input_artifacts: BTreeMap::from([
            (
                "baseline_summary".to_string(),
                serde_json::json!({
                    "max_temperature": 100.0,
                    "max_heat_flux": 50.0
                }),
            ),
            (
                "candidate_summary".to_string(),
                serde_json::json!({
                    "max_temperature": 120.0,
                    "max_heat_flux": 40.0
                }),
            ),
            (
                "variant_summary".to_string(),
                serde_json::json!({
                    "max_temperature": 90.0,
                    "max_heat_flux": 60.0
                }),
            ),
        ]),
    })
    .expect("summary aggregate workflow should run");

    let aggregated = run
        .artifacts
        .get("aggregate_summary.merged")
        .cloned()
        .expect("aggregated summary should exist");
    assert_eq!(aggregated["summary_input_count"], serde_json::json!(3));
    assert_eq!(aggregated["summary_aggregated_field_count"], serde_json::json!(2));
    assert_eq!(aggregated["benchmark_max_temperature_min"], serde_json::json!(90.0));
    assert_eq!(aggregated["benchmark_max_temperature_max"], serde_json::json!(120.0));
    assert_eq!(aggregated["benchmark_max_temperature_mean"], serde_json::json!(103.33333333333333));
    assert_eq!(aggregated["benchmark_max_temperature_span"], serde_json::json!(30.0));
    assert_eq!(aggregated["benchmark_max_heat_flux_min"], serde_json::json!(40.0));
    assert_eq!(aggregated["benchmark_max_heat_flux_max"], serde_json::json!(60.0));
    assert_eq!(aggregated["benchmark_max_heat_flux_span"], serde_json::json!(20.0));
    assert_eq!(
        aggregated["benchmark_max_temperature_sources"],
        serde_json::json!(["baseline", "candidate", "variant"])
    );

    let exported = run
        .artifacts
        .get("json_output.json")
        .cloned()
        .expect("json export artifact should exist");
    assert_eq!(exported["format"], serde_json::json!("json"));
    let content = exported["content"]
        .as_str()
        .expect("json content should be a string");
    assert!(content.contains("benchmark_max_temperature_mean"));
    assert!(content.contains("benchmark_max_heat_flux_span"));
}

fn input_node(id: &str) -> WorkflowNode {
    WorkflowNode {
        id: id.to_string(),
        kind: WorkflowNodeKind::Input,
        operator_id: None,
        name: Some("Summary input".to_string()),
        description: None,
        config: None,
        cache_policy: None,
        inputs: vec![],
        outputs: vec![port("summary", "artifact/result_summary")],
    }
}

fn port(id: &str, artifact_type: &str) -> WorkflowPort {
    WorkflowPort {
        id: id.to_string(),
        artifact_type: artifact_type.to_string(),
        name: None,
        required: None,
        cardinality: None,
        dataset_value: Some("result_summary".to_string()),
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
        dataset_value: Some("result_summary".to_string()),
    }
}
