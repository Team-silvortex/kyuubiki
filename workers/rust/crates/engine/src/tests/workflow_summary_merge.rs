use crate::run_workflow_graph;
use kyuubiki_protocol::{
    WorkflowCachePolicy, WorkflowDefaults, WorkflowEdge, WorkflowGraph, WorkflowGraphRunRequest,
    WorkflowNode, WorkflowNodeKind, WorkflowNodePortRef, WorkflowPort,
};
use std::collections::BTreeMap;

#[test]
fn runs_multi_stage_summary_merge_workflow_graph() {
    let graph = WorkflowGraph {
        schema_version: "kyuubiki.workflow-graph/v1".to_string(),
        id: "workflow.electrostatic-heat-thermo-merged-summary".to_string(),
        name: "Electrostatic heat thermo merged summary".to_string(),
        version: "1.0.0".to_string(),
        description: Some(
            "Aggregate electrostatic, heat, and thermo summaries into one report.".to_string(),
        ),
        dataset_contract: None,
        entry_nodes: vec!["electrostatic_model".to_string()],
        output_nodes: vec!["summary_output".to_string()],
        defaults: WorkflowDefaults {
            cache_policy: Some(WorkflowCachePolicy::Cached),
            orchestrated: Some(true),
        },
        nodes: vec![
            input_node(
                "electrostatic_model",
                "model",
                "study_model/electrostatic_plane_quad_2d",
            ),
            solve_node(
                "solve_electrostatic",
                "solve.electrostatic_plane_quad_2d",
                "study_model/electrostatic_plane_quad_2d",
                "result/electrostatic_plane_quad_2d",
            ),
            extract_node(
                "extract_electrostatic",
                "result/electrostatic_plane_quad_2d",
                &["max_potential", "max_electric_field"],
            ),
            WorkflowNode {
                id: "bridge_field_to_heat".to_string(),
                kind: WorkflowNodeKind::Transform,
                operator_id: Some("bridge.electrostatic_field_to_heat_quad_2d".to_string()),
                name: Some("Bridge field to heat".to_string()),
                description: None,
                config: Some(serde_json::json!({
                    "seed_model": {
                        "nodes": [
                            { "id": "h0", "x": 0.0, "y": 0.0, "fix_temperature": true, "temperature": 100.0, "heat_load": 0.0 },
                            { "id": "h1", "x": 1.0, "y": 0.0, "fix_temperature": false, "temperature": 0.0, "heat_load": 0.0 },
                            { "id": "h2", "x": 1.0, "y": 1.0, "fix_temperature": true, "temperature": 20.0, "heat_load": 0.0 },
                            { "id": "h3", "x": 0.0, "y": 1.0, "fix_temperature": true, "temperature": 20.0, "heat_load": 0.0 }
                        ],
                        "elements": [
                            { "id": "hq0", "node_i": 0, "node_j": 1, "node_k": 2, "node_l": 3, "thickness": 0.02, "conductivity": 45.0 }
                        ]
                    },
                    "contract": {
                        "version": "kyuubiki.bridge-contract/v1",
                        "source": {
                            "field": "electric_field_magnitude",
                            "distribution": "element_to_nodes",
                            "node_index_fields": ["node_i", "node_j", "node_k", "node_l"]
                        },
                        "transform": {
                            "scale": 50.0,
                            "reduction": "mean",
                            "default_value": 0.0
                        },
                        "target": { "field": "heat_load" }
                    }
                })),
                cache_policy: None,
                inputs: vec![port(
                    "electrostatic_result",
                    "result/electrostatic_plane_quad_2d",
                )],
                outputs: vec![port("heat_model", "study_model/heat_plane_quad_2d")],
            },
            solve_node(
                "solve_heat",
                "solve.heat_plane_quad_2d",
                "study_model/heat_plane_quad_2d",
                "result/heat_plane_quad_2d",
            ),
            extract_node(
                "extract_heat",
                "result/heat_plane_quad_2d",
                &["max_temperature", "max_heat_flux"],
            ),
            WorkflowNode {
                id: "bridge_temperature".to_string(),
                kind: WorkflowNodeKind::Transform,
                operator_id: Some("bridge.temperature_field_to_thermo_quad_2d".to_string()),
                name: Some("Bridge temperature".to_string()),
                description: None,
                config: Some(serde_json::json!({
                    "nodes": [
                        { "id": "n0", "x": 0.0, "y": 0.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0, "temperature_delta": 30.0 },
                        { "id": "n1", "x": 1.0, "y": 0.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0, "temperature_delta": 30.0 },
                        { "id": "n2", "x": 1.0, "y": 1.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0, "temperature_delta": 30.0 },
                        { "id": "n3", "x": 0.0, "y": 1.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0, "temperature_delta": 30.0 }
                    ],
                    "elements": [
                        { "id": "tq0", "node_i": 0, "node_j": 1, "node_k": 2, "node_l": 3, "thickness": 0.02, "youngs_modulus": 70000000000.0, "poisson_ratio": 0.33, "thermal_expansion": 0.000011 }
                    ]
                })),
                cache_policy: None,
                inputs: vec![port("heat_result", "result/heat_plane_quad_2d")],
                outputs: vec![port("thermo_model", "study_model/thermal_plane_quad_2d")],
            },
            solve_node(
                "solve_thermo",
                "solve.thermal_plane_quad_2d",
                "study_model/thermal_plane_quad_2d",
                "result/thermal_plane_quad_2d",
            ),
            extract_node(
                "extract_thermo",
                "result/thermal_plane_quad_2d",
                &["max_displacement", "max_stress", "max_temperature_delta"],
            ),
            WorkflowNode {
                id: "merge_field_and_heat".to_string(),
                kind: WorkflowNodeKind::Transform,
                operator_id: Some("transform.merge_summary_pair".to_string()),
                name: Some("Merge field and heat summary".to_string()),
                description: None,
                config: Some(serde_json::json!({
                    "left_prefix": "electrostatic",
                    "right_prefix": "heat",
                    "include_source_count": false
                })),
                cache_policy: None,
                inputs: vec![
                    port("left", "artifact/result_summary"),
                    port("right", "artifact/result_summary"),
                ],
                outputs: vec![port("merged", "artifact/result_summary")],
            },
            WorkflowNode {
                id: "merge_all_summary".to_string(),
                kind: WorkflowNodeKind::Transform,
                operator_id: Some("transform.merge_summary_pair".to_string()),
                name: Some("Merge all stage summary".to_string()),
                description: None,
                config: Some(serde_json::json!({
                    "left_prefix": "",
                    "right_prefix": "thermo",
                    "include_source_count": false
                })),
                cache_policy: None,
                inputs: vec![
                    port("left", "artifact/result_summary"),
                    port("right", "artifact/result_summary"),
                ],
                outputs: vec![port("merged", "artifact/result_summary")],
            },
            output_node("summary_output", "summary", "artifact/result_summary"),
        ],
        edges: vec![
            edge(
                "edge-electrostatic-input",
                "electrostatic_model",
                "model",
                "solve_electrostatic",
                "model",
                "study_model/electrostatic_plane_quad_2d",
            ),
            edge(
                "edge-electrostatic-extract",
                "solve_electrostatic",
                "result",
                "extract_electrostatic",
                "result",
                "result/electrostatic_plane_quad_2d",
            ),
            edge(
                "edge-electrostatic-bridge",
                "solve_electrostatic",
                "result",
                "bridge_field_to_heat",
                "electrostatic_result",
                "result/electrostatic_plane_quad_2d",
            ),
            edge(
                "edge-heat-model",
                "bridge_field_to_heat",
                "heat_model",
                "solve_heat",
                "model",
                "study_model/heat_plane_quad_2d",
            ),
            edge(
                "edge-heat-extract",
                "solve_heat",
                "result",
                "extract_heat",
                "result",
                "result/heat_plane_quad_2d",
            ),
            edge(
                "edge-heat-bridge",
                "solve_heat",
                "result",
                "bridge_temperature",
                "heat_result",
                "result/heat_plane_quad_2d",
            ),
            edge(
                "edge-thermo-model",
                "bridge_temperature",
                "thermo_model",
                "solve_thermo",
                "model",
                "study_model/thermal_plane_quad_2d",
            ),
            edge(
                "edge-thermo-extract",
                "solve_thermo",
                "result",
                "extract_thermo",
                "result",
                "result/thermal_plane_quad_2d",
            ),
            edge(
                "edge-merge-left",
                "extract_electrostatic",
                "summary",
                "merge_field_and_heat",
                "left",
                "artifact/result_summary",
            ),
            edge(
                "edge-merge-right",
                "extract_heat",
                "summary",
                "merge_field_and_heat",
                "right",
                "artifact/result_summary",
            ),
            edge(
                "edge-merge-all-left",
                "merge_field_and_heat",
                "merged",
                "merge_all_summary",
                "left",
                "artifact/result_summary",
            ),
            edge(
                "edge-merge-all-right",
                "extract_thermo",
                "summary",
                "merge_all_summary",
                "right",
                "artifact/result_summary",
            ),
            edge(
                "edge-summary-output",
                "merge_all_summary",
                "merged",
                "summary_output",
                "summary",
                "artifact/result_summary",
            ),
        ],
    };

    let run = run_workflow_graph(WorkflowGraphRunRequest {
        graph,
        input_artifacts: BTreeMap::from([(
            "electrostatic_model".to_string(),
            serde_json::json!({
                "nodes": [
                    { "id": "e0", "x": 0.0, "y": 0.0, "fix_potential": true, "potential": 10.0, "charge_density": 0.0 },
                    { "id": "e1", "x": 1.0, "y": 0.0, "fix_potential": false, "potential": 0.0, "charge_density": 0.0 },
                    { "id": "e2", "x": 1.0, "y": 1.0, "fix_potential": true, "potential": 0.0, "charge_density": 0.0 },
                    { "id": "e3", "x": 0.0, "y": 1.0, "fix_potential": true, "potential": 0.0, "charge_density": 0.0 }
                ],
                "elements": [
                    { "id": "eq0", "node_i": 0, "node_j": 1, "node_k": 2, "node_l": 3, "thickness": 0.05, "permittivity": 2.5 }
                ]
            }),
        )]),
    })
    .expect("multi-stage summary merge workflow should run");

    let summary = run
        .artifacts
        .get("summary_output.summary")
        .cloned()
        .expect("merged summary artifact should exist");
    assert_eq!(run.completed_nodes.len(), 12);
    assert!(summary.get("electrostatic_max_potential").is_some());
    assert!(summary.get("electrostatic_max_electric_field").is_some());
    assert!(summary.get("heat_max_temperature").is_some());
    assert!(summary.get("heat_max_heat_flux").is_some());
    assert!(summary.get("thermo_max_displacement").is_some());
    assert!(summary.get("thermo_max_stress").is_some());
    assert!(summary["thermo_max_temperature_delta"]
        .as_f64()
        .is_some_and(|value| value > 0.0));
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

fn input_node(id: &str, output_id: &str, artifact_type: &str) -> WorkflowNode {
    WorkflowNode {
        id: id.to_string(),
        kind: WorkflowNodeKind::Input,
        operator_id: None,
        name: None,
        description: None,
        config: None,
        cache_policy: None,
        inputs: vec![],
        outputs: vec![port(output_id, artifact_type)],
    }
}

fn solve_node(id: &str, operator_id: &str, input_type: &str, output_type: &str) -> WorkflowNode {
    WorkflowNode {
        id: id.to_string(),
        kind: WorkflowNodeKind::Solve,
        operator_id: Some(operator_id.to_string()),
        name: None,
        description: None,
        config: None,
        cache_policy: None,
        inputs: vec![port("model", input_type)],
        outputs: vec![port("result", output_type)],
    }
}

fn extract_node(id: &str, input_type: &str, fields: &[&str]) -> WorkflowNode {
    WorkflowNode {
        id: id.to_string(),
        kind: WorkflowNodeKind::Extract,
        operator_id: Some("extract.result_summary".to_string()),
        name: None,
        description: None,
        config: Some(serde_json::json!({ "fields": fields })),
        cache_policy: None,
        inputs: vec![port("result", input_type)],
        outputs: vec![port("summary", "artifact/result_summary")],
    }
}

fn output_node(id: &str, input_id: &str, artifact_type: &str) -> WorkflowNode {
    WorkflowNode {
        id: id.to_string(),
        kind: WorkflowNodeKind::Output,
        operator_id: None,
        name: None,
        description: None,
        config: None,
        cache_policy: None,
        inputs: vec![port(input_id, artifact_type)],
        outputs: vec![],
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
