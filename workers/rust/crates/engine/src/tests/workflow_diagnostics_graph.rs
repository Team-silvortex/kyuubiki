use crate::run_workflow_graph;
use kyuubiki_protocol::{
    WorkflowCachePolicy, WorkflowDefaults, WorkflowEdge, WorkflowGraph, WorkflowGraphRunRequest,
    WorkflowNode, WorkflowNodeKind, WorkflowNodePortRef, WorkflowPort,
};
use std::collections::BTreeMap;

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

#[test]
fn runs_diagnostics_bundle_guard_report_markdown_workflow_graph() {
    let graph = WorkflowGraph {
        schema_version: "kyuubiki.workflow-graph/v1".to_string(),
        id: "workflow.diagnostics-bundle-guard-report-markdown".to_string(),
        name: "Diagnostics bundle guard report markdown".to_string(),
        version: "1.0.0".to_string(),
        description: Some(
            "Compose multi-domain diagnostics, evaluate a unified guard, and export markdown."
                .to_string(),
        ),
        dataset_contract: None,
        entry_nodes: vec![
            "electrostatic_input".to_string(),
            "thermal_input".to_string(),
            "thermo_input".to_string(),
        ],
        output_nodes: vec!["markdown_output".to_string()],
        defaults: WorkflowDefaults {
            cache_policy: Some(WorkflowCachePolicy::Cached),
            orchestrated: Some(true),
        },
        nodes: vec![
            WorkflowNode {
                id: "electrostatic_input".to_string(),
                kind: WorkflowNodeKind::Input,
                operator_id: None,
                name: None,
                description: None,
                config: None,
                cache_policy: None,
                inputs: vec![],
                outputs: vec![port("summary", "artifact/json")],
            },
            WorkflowNode {
                id: "thermal_input".to_string(),
                kind: WorkflowNodeKind::Input,
                operator_id: None,
                name: None,
                description: None,
                config: None,
                cache_policy: None,
                inputs: vec![],
                outputs: vec![port("summary", "artifact/json")],
            },
            WorkflowNode {
                id: "thermo_input".to_string(),
                kind: WorkflowNodeKind::Input,
                operator_id: None,
                name: None,
                description: None,
                config: None,
                cache_policy: None,
                inputs: vec![],
                outputs: vec![port("summary", "artifact/json")],
            },
            WorkflowNode {
                id: "bundle".to_string(),
                kind: WorkflowNodeKind::Transform,
                operator_id: Some("transform.compose_diagnostics_bundle".to_string()),
                name: None,
                description: None,
                config: Some(serde_json::json!({})),
                cache_policy: None,
                inputs: vec![
                    port("electrostatic", "artifact/json"),
                    port("thermal", "artifact/json"),
                    port("thermo", "artifact/json"),
                ],
                outputs: vec![port("result", "artifact/json")],
            },
            WorkflowNode {
                id: "guard".to_string(),
                kind: WorkflowNodeKind::Transform,
                operator_id: Some("transform.evaluate_diagnostics_bundle_guard".to_string()),
                name: None,
                description: None,
                config: Some(serde_json::json!({
                    "rules": [
                        { "source": "thermal", "field": "thermal_temperature_max", "threshold": 120.0, "severity": "warn", "label": "thermal temperature" },
                        { "source": "thermo", "field": "thermo_stress_peak", "comparison": "gt", "threshold": 180.0, "severity": "block", "label": "stress ceiling" },
                        { "source": "electrostatic", "field": "electrostatic_field_peak_magnitude", "comparison": "gt", "threshold": 9.0, "severity": "warn", "label": "field ceiling" }
                    ]
                })),
                cache_policy: None,
                inputs: vec![port("bundle", "artifact/json")],
                outputs: vec![port("result", "artifact/json")],
            },
            WorkflowNode {
                id: "report".to_string(),
                kind: WorkflowNodeKind::Transform,
                operator_id: Some("transform.compose_diagnostics_report_payload".to_string()),
                name: None,
                description: None,
                config: Some(serde_json::json!({})),
                cache_policy: None,
                inputs: vec![
                    port("bundle", "artifact/json"),
                    port("guard", "artifact/json"),
                ],
                outputs: vec![port("result", "artifact/json")],
            },
            WorkflowNode {
                id: "export".to_string(),
                kind: WorkflowNodeKind::Export,
                operator_id: Some("export.diagnostics_bundle_markdown".to_string()),
                name: None,
                description: None,
                config: Some(serde_json::json!({
                    "title": "Diagnostics Bundle Report"
                })),
                cache_policy: None,
                inputs: vec![port("bundle", "artifact/json")],
                outputs: vec![port("markdown", "export/markdown")],
            },
            WorkflowNode {
                id: "markdown_output".to_string(),
                kind: WorkflowNodeKind::Output,
                operator_id: None,
                name: None,
                description: None,
                config: None,
                cache_policy: None,
                inputs: vec![port("markdown", "export/markdown")],
                outputs: vec![],
            },
        ],
        edges: vec![
            edge(
                "e0",
                "electrostatic_input",
                "summary",
                "bundle",
                "electrostatic",
                "artifact/json",
            ),
            edge(
                "e1",
                "thermal_input",
                "summary",
                "bundle",
                "thermal",
                "artifact/json",
            ),
            edge(
                "e2",
                "thermo_input",
                "summary",
                "bundle",
                "thermo",
                "artifact/json",
            ),
            edge("e3", "bundle", "result", "guard", "bundle", "artifact/json"),
            edge(
                "e4",
                "bundle",
                "result",
                "report",
                "bundle",
                "artifact/json",
            ),
            edge("e5", "guard", "result", "report", "guard", "artifact/json"),
            edge(
                "e6",
                "report",
                "result",
                "export",
                "bundle",
                "artifact/json",
            ),
            edge(
                "e7",
                "export",
                "markdown",
                "markdown_output",
                "markdown",
                "export/markdown",
            ),
        ],
    };

    let run = run_workflow_graph(WorkflowGraphRunRequest {
        graph,
        input_artifacts: BTreeMap::from([
            (
                "electrostatic_input".to_string(),
                serde_json::json!({
                    "diagnostic_contract": "kyuubiki.workflow_diagnostics/v1",
                    "diagnostic_domain": "electrostatic",
                    "diagnostic_subject": "electrostatic_result",
                    "diagnostic_prefix": "electrostatic",
                    "diagnostic_node_count": 4,
                    "diagnostic_element_count": 1,
                    "diagnostic_metric_groups": ["field"],
                    "electrostatic_field_peak_magnitude": 12.0
                }),
            ),
            (
                "thermal_input".to_string(),
                serde_json::json!({
                    "diagnostic_contract": "kyuubiki.workflow_diagnostics/v1",
                    "diagnostic_domain": "thermal",
                    "diagnostic_subject": "thermal_result",
                    "diagnostic_prefix": "thermal",
                    "diagnostic_node_count": 4,
                    "diagnostic_element_count": 1,
                    "diagnostic_metric_groups": ["temperature", "flux"],
                    "thermal_temperature_max": 125.0
                }),
            ),
            (
                "thermo_input".to_string(),
                serde_json::json!({
                    "diagnostic_contract": "kyuubiki.workflow_diagnostics/v1",
                    "diagnostic_domain": "thermo_mechanical",
                    "diagnostic_subject": "thermo_result",
                    "diagnostic_prefix": "thermo",
                    "diagnostic_node_count": 4,
                    "diagnostic_element_count": 1,
                    "diagnostic_metric_groups": ["temperature_delta", "stress"],
                    "thermo_stress_peak": 220.0
                }),
            ),
        ]),
    })
    .expect("diagnostics graph should run");

    assert_eq!(
        run.completed_nodes,
        vec![
            "electrostatic_input".to_string(),
            "thermal_input".to_string(),
            "thermo_input".to_string(),
            "bundle".to_string(),
            "guard".to_string(),
            "report".to_string(),
            "export".to_string(),
            "markdown_output".to_string(),
        ]
    );

    let markdown = run
        .artifacts
        .get("export.markdown")
        .and_then(|value| value.get("content"))
        .and_then(serde_json::Value::as_str)
        .expect("expected markdown export content");
    assert!(markdown.contains("# Diagnostics Bundle Report"));
    assert!(markdown.contains("## Diagnostics Sources"));
    assert!(markdown.contains("## Guard Decision"));
    assert_eq!(
        run.artifacts
            .get("guard.result")
            .and_then(|value| value.get("guard_status"))
            .and_then(serde_json::Value::as_str),
        Some("block")
    );
    assert!(
        markdown.contains("Status: block"),
        "markdown was:\n{markdown}"
    );
    assert!(
        markdown.contains("thermo.stress ceiling"),
        "markdown was:\n{markdown}"
    );
    assert!(run.artifacts.contains_key("bundle.result"));
    assert!(run.artifacts.contains_key("guard.result"));
    assert!(run.artifacts.contains_key("report.result"));
    assert!(run.artifacts.contains_key("markdown_output.markdown"));
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
