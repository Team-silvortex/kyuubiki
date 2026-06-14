use crate::run_workflow_graph;
use kyuubiki_protocol::{
    WorkflowCachePolicy, WorkflowDefaults, WorkflowEdge, WorkflowGraph, WorkflowGraphRunRequest,
    WorkflowNode, WorkflowNodeKind, WorkflowNodePortRef, WorkflowPort,
};
use std::collections::BTreeMap;

fn build_hotspot_condition_graph(predicate_value: usize) -> WorkflowGraph {
    WorkflowGraph {
        schema_version: "kyuubiki.workflow-graph/v1".to_string(),
        id: format!("workflow.electrostatic-hotspot-condition-{predicate_value}"),
        name: "Electrostatic hotspot condition alert".to_string(),
        version: "1.0.0".to_string(),
        description: Some(
            "Route hotspot summaries into alert or clear markdown exports.".to_string(),
        ),
        dataset_contract: None,
        entry_nodes: vec!["electrostatic_model".to_string()],
        output_nodes: vec!["final_output".to_string()],
        defaults: WorkflowDefaults {
            cache_policy: Some(WorkflowCachePolicy::Cached),
            orchestrated: Some(true),
        },
        nodes: vec![
            WorkflowNode {
                id: "electrostatic_model".to_string(),
                kind: WorkflowNodeKind::Input,
                operator_id: None,
                name: Some("Electrostatic plane input".to_string()),
                description: None,
                config: None,
                cache_policy: None,
                inputs: vec![],
                outputs: vec![WorkflowPort {
                    id: "model".to_string(),
                    artifact_type: "study_model/electrostatic_plane_quad_2d".to_string(),
                    name: None,
                    required: None,
                    cardinality: None,
                    dataset_value: None,
                }],
            },
            WorkflowNode {
                id: "solve_electrostatic".to_string(),
                kind: WorkflowNodeKind::Solve,
                operator_id: Some("solve.electrostatic_plane_quad_2d".to_string()),
                name: Some("Solve electrostatic quad".to_string()),
                description: None,
                config: None,
                cache_policy: None,
                inputs: vec![WorkflowPort {
                    id: "model".to_string(),
                    artifact_type: "study_model/electrostatic_plane_quad_2d".to_string(),
                    name: None,
                    required: None,
                    cardinality: None,
                    dataset_value: None,
                }],
                outputs: vec![WorkflowPort {
                    id: "result".to_string(),
                    artifact_type: "result/electrostatic_plane_quad_2d".to_string(),
                    name: None,
                    required: None,
                    cardinality: None,
                    dataset_value: None,
                }],
            },
            WorkflowNode {
                id: "field_hotspots".to_string(),
                kind: WorkflowNodeKind::Extract,
                operator_id: Some("extract.field_hotspots".to_string()),
                name: Some("Extract field hotspots".to_string()),
                description: None,
                config: Some(serde_json::json!({
                    "source": "elements",
                    "field": "electric_field_magnitude",
                    "output_prefix": "field",
                    "percentile": 75,
                    "sample_limit": 4,
                    "sample_sort": "value_desc"
                })),
                cache_policy: None,
                inputs: vec![WorkflowPort {
                    id: "result".to_string(),
                    artifact_type: "result/electrostatic_plane_quad_2d".to_string(),
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
                id: "gate".to_string(),
                kind: WorkflowNodeKind::Condition,
                operator_id: None,
                name: Some("Check hotspot count".to_string()),
                description: None,
                config: Some(serde_json::json!({
                    "predicate": {
                        "path": "field_hotspot_count",
                        "operator": "gt",
                        "value": predicate_value
                    }
                })),
                cache_policy: None,
                inputs: vec![WorkflowPort {
                    id: "value".to_string(),
                    artifact_type: "report/summary".to_string(),
                    name: None,
                    required: None,
                    cardinality: None,
                    dataset_value: None,
                }],
                outputs: vec![
                    WorkflowPort {
                        id: "if_true".to_string(),
                        artifact_type: "report/summary".to_string(),
                        name: None,
                        required: None,
                        cardinality: None,
                        dataset_value: None,
                    },
                    WorkflowPort {
                        id: "if_false".to_string(),
                        artifact_type: "report/summary".to_string(),
                        name: None,
                        required: None,
                        cardinality: None,
                        dataset_value: None,
                    },
                ],
            },
            WorkflowNode {
                id: "alert_export".to_string(),
                kind: WorkflowNodeKind::Export,
                operator_id: Some("export.alert_markdown".to_string()),
                name: Some("Export hotspot alert".to_string()),
                description: None,
                config: Some(serde_json::json!({
                    "title": "Electrostatic Hotspot Alert",
                    "severity": "warning",
                    "summary": "Hotspot candidates exceeded the workflow threshold.",
                    "fields": ["field_hotspot_count", "field_hotspot_fraction", "field_threshold"],
                    "sample_count": 4
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
                    id: "markdown".to_string(),
                    artifact_type: "export/markdown".to_string(),
                    name: None,
                    required: None,
                    cardinality: None,
                    dataset_value: None,
                }],
            },
            WorkflowNode {
                id: "clear_export".to_string(),
                kind: WorkflowNodeKind::Export,
                operator_id: Some("export.alert_markdown".to_string()),
                name: Some("Export clear state".to_string()),
                description: None,
                config: Some(serde_json::json!({
                    "title": "Electrostatic Field Clear",
                    "severity": "info",
                    "summary": "Hotspot count stayed within the configured workflow threshold.",
                    "fields": ["field_hotspot_count", "field_hotspot_fraction", "field_threshold"],
                    "sample_count": 4
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
                    id: "markdown".to_string(),
                    artifact_type: "export/markdown".to_string(),
                    name: None,
                    required: None,
                    cardinality: None,
                    dataset_value: None,
                }],
            },
            WorkflowNode {
                id: "merge_output".to_string(),
                kind: WorkflowNodeKind::Transform,
                operator_id: Some("transform.first_available".to_string()),
                name: Some("Merge active branch".to_string()),
                description: None,
                config: None,
                cache_policy: None,
                inputs: vec![
                    WorkflowPort {
                        id: "left".to_string(),
                        artifact_type: "export/markdown".to_string(),
                        name: None,
                        required: None,
                        cardinality: None,
                        dataset_value: None,
                    },
                    WorkflowPort {
                        id: "right".to_string(),
                        artifact_type: "export/markdown".to_string(),
                        name: None,
                        required: None,
                        cardinality: None,
                        dataset_value: None,
                    },
                ],
                outputs: vec![WorkflowPort {
                    id: "merged".to_string(),
                    artifact_type: "export/markdown".to_string(),
                    name: None,
                    required: None,
                    cardinality: None,
                    dataset_value: None,
                }],
            },
            WorkflowNode {
                id: "final_output".to_string(),
                kind: WorkflowNodeKind::Output,
                operator_id: None,
                name: Some("Final markdown output".to_string()),
                description: None,
                config: None,
                cache_policy: None,
                inputs: vec![WorkflowPort {
                    id: "markdown".to_string(),
                    artifact_type: "export/markdown".to_string(),
                    name: None,
                    required: None,
                    cardinality: None,
                    dataset_value: None,
                }],
                outputs: vec![],
            },
        ],
        edges: vec![
            edge(
                "edge-input",
                ("electrostatic_model", "model"),
                ("solve_electrostatic", "model"),
                "study_model/electrostatic_plane_quad_2d",
            ),
            edge(
                "edge-result",
                ("solve_electrostatic", "result"),
                ("field_hotspots", "result"),
                "result/electrostatic_plane_quad_2d",
            ),
            edge(
                "edge-summary",
                ("field_hotspots", "summary"),
                ("gate", "value"),
                "report/summary",
            ),
            edge(
                "edge-true",
                ("gate", "if_true"),
                ("alert_export", "summary"),
                "report/summary",
            ),
            edge(
                "edge-false",
                ("gate", "if_false"),
                ("clear_export", "summary"),
                "report/summary",
            ),
            edge(
                "edge-alert",
                ("alert_export", "markdown"),
                ("merge_output", "left"),
                "export/markdown",
            ),
            edge(
                "edge-clear",
                ("clear_export", "markdown"),
                ("merge_output", "right"),
                "export/markdown",
            ),
            edge(
                "edge-output",
                ("merge_output", "merged"),
                ("final_output", "markdown"),
                "export/markdown",
            ),
        ],
    }
}

fn edge(id: &str, from: (&str, &str), to: (&str, &str), artifact_type: &str) -> WorkflowEdge {
    WorkflowEdge {
        id: id.to_string(),
        from: WorkflowNodePortRef {
            node: from.0.to_string(),
            port: from.1.to_string(),
        },
        to: WorkflowNodePortRef {
            node: to.0.to_string(),
            port: to.1.to_string(),
        },
        artifact_type: artifact_type.to_string(),
        dataset_value: None,
    }
}

fn electrostatic_input() -> serde_json::Value {
    serde_json::json!({
        "nodes": [
            { "id": "n0", "x": 0.0, "y": 0.0, "fix_potential": true, "potential": 10.0, "charge_density": 0.0 },
            { "id": "n1", "x": 1.0, "y": 0.0, "fix_potential": false, "potential": 0.0, "charge_density": 0.0 },
            { "id": "n2", "x": 1.0, "y": 1.0, "fix_potential": false, "potential": 0.0, "charge_density": 0.0 },
            { "id": "n3", "x": 0.0, "y": 1.0, "fix_potential": true, "potential": 5.0, "charge_density": 0.0 }
        ],
        "elements": [
            { "id": "q0", "node_i": 0, "node_j": 1, "node_k": 2, "node_l": 3, "permittivity": 2.0, "thickness": 0.1 },
            { "id": "q1", "node_i": 0, "node_j": 1, "node_k": 2, "node_l": 3, "permittivity": 5.0, "thickness": 0.1 }
        ]
    })
}

#[test]
fn routes_hotspot_summary_to_alert_branch() {
    let run = run_workflow_graph(WorkflowGraphRunRequest {
        graph: build_hotspot_condition_graph(0),
        input_artifacts: BTreeMap::from([(
            "electrostatic_model".to_string(),
            electrostatic_input(),
        )]),
    })
    .expect("hotspot condition alert workflow should run");

    let content = run.artifacts["final_output.markdown"]["content"]
        .as_str()
        .expect("final markdown should be a string");
    assert!(content.contains("# Electrostatic Hotspot Alert"));
    assert!(content.contains("## Sample Context"));
    assert!(run.completed_nodes.contains(&"alert_export".to_string()));
    assert!(run.skipped_nodes.contains(&"clear_export".to_string()));
}

#[test]
fn routes_hotspot_summary_to_clear_branch() {
    let run = run_workflow_graph(WorkflowGraphRunRequest {
        graph: build_hotspot_condition_graph(2),
        input_artifacts: BTreeMap::from([(
            "electrostatic_model".to_string(),
            electrostatic_input(),
        )]),
    })
    .expect("hotspot clear workflow should run");

    let content = run.artifacts["final_output.markdown"]["content"]
        .as_str()
        .expect("final markdown should be a string");
    assert!(content.contains("# Electrostatic Field Clear"));
    assert!(content.contains("## Sample Context"));
    assert!(run.completed_nodes.contains(&"clear_export".to_string()));
    assert!(run.skipped_nodes.contains(&"alert_export".to_string()));
}
