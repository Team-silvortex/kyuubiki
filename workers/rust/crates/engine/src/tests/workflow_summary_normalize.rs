use crate::run_workflow_graph;
use kyuubiki_protocol::{
    WorkflowDefaults, WorkflowEdge, WorkflowGraph, WorkflowGraphRunRequest, WorkflowNode,
    WorkflowNodeKind, WorkflowNodePortRef, WorkflowPort,
};
use std::collections::BTreeMap;

#[test]
fn runs_summary_normalize_workflow_graph() {
    let graph = WorkflowGraph {
        schema_version: "kyuubiki.workflow-graph/v1".to_string(),
        id: "workflow.summary-normalize".to_string(),
        name: "Summary normalize".to_string(),
        version: "1.0.0".to_string(),
        description: Some("Normalize summary fields before downstream use.".to_string()),
        dataset_contract: None,
        entry_nodes: vec!["summary_input".to_string()],
        output_nodes: vec!["json_output".to_string()],
        defaults: WorkflowDefaults::default(),
        nodes: vec![
            input_node("summary_input"),
            WorkflowNode {
                id: "normalize_summary".to_string(),
                kind: WorkflowNodeKind::Transform,
                operator_id: Some("transform.normalize_summary_fields".to_string()),
                name: Some("Normalize summary".to_string()),
                description: None,
                config: Some(serde_json::json!({
                    "copy_unmapped": false,
                    "rules": [
                        { "source": "max_temperature", "target": "temperature_peak_celsius", "offset": -273.15 },
                        { "source": "max_heat_flux", "target": "heat_flux_peak_kw", "scale": 0.001, "clamp_min": 0.0 },
                        { "source": "max_displacement", "target": "displacement_peak_mm", "scale": 1000.0 }
                    ]
                })),
                cache_policy: None,
                inputs: vec![port("summary", "artifact/result_summary")],
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
                "edge-input-normalize",
                "summary_input",
                "summary",
                "normalize_summary",
                "summary",
                "artifact/result_summary",
            ),
            edge(
                "edge-normalize-export",
                "normalize_summary",
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
        input_artifacts: BTreeMap::from([(
            "summary_input".to_string(),
            serde_json::json!({
                "max_temperature": 373.15,
                "max_heat_flux": 4500.0,
                "max_displacement": 0.003,
                "max_stress": 12.0
            }),
        )]),
    })
    .expect("summary normalize workflow should run");

    let normalized = run
        .artifacts
        .get("normalize_summary.merged")
        .cloned()
        .expect("normalized summary should exist");
    assert_eq!(
        normalized["temperature_peak_celsius"],
        serde_json::json!(100.0)
    );
    assert_eq!(normalized["heat_flux_peak_kw"], serde_json::json!(4.5));
    assert_eq!(normalized["displacement_peak_mm"], serde_json::json!(3.0));
    assert!(normalized.get("max_stress").is_none());

    let exported = run
        .artifacts
        .get("json_output.json")
        .cloned()
        .expect("json export artifact should exist");
    assert_eq!(exported["format"], serde_json::json!("json"));
    let content = exported["content"]
        .as_str()
        .expect("json content should be a string");
    assert!(content.contains("temperature_peak_celsius"));
    assert!(content.contains("heat_flux_peak_kw"));
    assert!(content.contains("displacement_peak_mm"));
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
