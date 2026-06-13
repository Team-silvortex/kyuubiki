use crate::run_workflow_graph;
use kyuubiki_protocol::{
    WorkflowDefaults, WorkflowEdge, WorkflowGraph, WorkflowGraphRunRequest, WorkflowNode,
    WorkflowNodeKind, WorkflowNodePortRef, WorkflowPort,
};
use std::collections::BTreeMap;

#[test]
fn runs_summary_select_workflow_graph() {
    let graph = WorkflowGraph {
        schema_version: "kyuubiki.workflow-graph/v1".to_string(),
        id: "workflow.summary-select".to_string(),
        name: "Summary select".to_string(),
        version: "1.0.0".to_string(),
        description: Some("Select the best candidate summary.".to_string()),
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
                id: "select_summary".to_string(),
                kind: WorkflowNodeKind::Transform,
                operator_id: Some("transform.select_best_summary".to_string()),
                name: Some("Select best summary".to_string()),
                description: None,
                config: Some(serde_json::json!({
                    "criteria": [
                        { "field": "max_temperature", "goal": "min", "weight": 1.0 },
                        { "field": "max_heat_flux", "goal": "max", "weight": 0.5 }
                    ],
                    "include_breakdown": true,
                    "include_all_scores": true
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
                "select_summary",
                "baseline",
                "artifact/result_summary",
            ),
            edge(
                "edge-candidate",
                "candidate_summary",
                "summary",
                "select_summary",
                "candidate",
                "artifact/result_summary",
            ),
            edge(
                "edge-variant",
                "variant_summary",
                "summary",
                "select_summary",
                "variant",
                "artifact/result_summary",
            ),
            edge(
                "edge-select-export",
                "select_summary",
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
                    "max_heat_flux": 40.0,
                    "max_displacement": 3.0
                }),
            ),
            (
                "candidate_summary".to_string(),
                serde_json::json!({
                    "max_temperature": 95.0,
                    "max_heat_flux": 30.0,
                    "max_displacement": 2.0
                }),
            ),
            (
                "variant_summary".to_string(),
                serde_json::json!({
                    "max_temperature": 92.0,
                    "max_heat_flux": 50.0,
                    "max_displacement": 2.5
                }),
            ),
        ]),
    })
    .expect("summary select workflow should run");

    let selected = run
        .artifacts
        .get("select_summary.merged")
        .cloned()
        .expect("selected summary should exist");
    assert_eq!(
        selected["selected_summary_source"],
        serde_json::json!("variant")
    );
    assert_eq!(selected["max_temperature"], serde_json::json!(92.0));
    assert_eq!(selected["max_heat_flux"], serde_json::json!(50.0));
    assert!(selected["selected_summary_score"].as_f64().is_some());
    assert_eq!(
        selected["selected_summary_candidates"]
            .as_array()
            .map(|entries| entries.len()),
        Some(3)
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
    assert!(content.contains("selected_summary_source"));
    assert!(content.contains("variant"));
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
