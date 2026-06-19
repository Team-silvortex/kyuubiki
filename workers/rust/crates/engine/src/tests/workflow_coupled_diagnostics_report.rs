use crate::run_workflow_graph;
use kyuubiki_protocol::{
    WorkflowCachePolicy, WorkflowDefaults, WorkflowEdge, WorkflowGraph, WorkflowGraphRunRequest,
    WorkflowNode, WorkflowNodeKind, WorkflowNodePortRef, WorkflowPort,
};
use std::collections::BTreeMap;

#[test]
fn runs_electrostatic_heat_thermo_diagnostics_markdown_workflow_graph() {
    let graph = WorkflowGraph {
        schema_version: "kyuubiki.workflow-graph/v1".to_string(),
        id: "workflow.electrostatic-heat-thermo-diagnostics-markdown".to_string(),
        name: "Electrostatic heat thermo diagnostics markdown".to_string(),
        version: "1.0.0".to_string(),
        description: Some(
            "Run electrostatic -> heat -> thermo, extract diagnostics from every stage, evaluate a unified guard, and export markdown."
                .to_string(),
        ),
        dataset_contract: None,
        entry_nodes: vec!["electrostatic_model".to_string()],
        output_nodes: vec!["markdown_output".to_string()],
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
            WorkflowNode {
                id: "bridge_field_to_heat".to_string(),
                kind: WorkflowNodeKind::Transform,
                operator_id: Some("bridge.electrostatic_field_to_heat_quad_2d".to_string()),
                name: Some("Bridge electrostatic field to heat".to_string()),
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
            WorkflowNode {
                id: "bridge_temperature".to_string(),
                kind: WorkflowNodeKind::Transform,
                operator_id: Some("bridge.temperature_field_to_thermo_quad_2d".to_string()),
                name: Some("Bridge heat to thermo".to_string()),
                description: None,
                config: Some(serde_json::json!({
                    "seed_model": {
                        "nodes": [
                            { "id": "t0", "x": 0.0, "y": 0.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0, "temperature_delta": 30.0 },
                            { "id": "t1", "x": 1.0, "y": 0.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0, "temperature_delta": 30.0 },
                            { "id": "t2", "x": 1.0, "y": 1.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0, "temperature_delta": 30.0 },
                            { "id": "t3", "x": 0.0, "y": 1.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0, "temperature_delta": 30.0 }
                        ],
                        "elements": [
                            { "id": "tq0", "node_i": 0, "node_j": 1, "node_k": 2, "node_l": 3, "thickness": 0.02, "youngs_modulus": 70000000000.0, "poisson_ratio": 0.33, "thermal_expansion": 0.000011 }
                        ]
                    }
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
                "extract_electrostatic_diagnostics",
                "extract.electrostatic_result_diagnostics",
                "result/electrostatic_plane_quad_2d",
            ),
            extract_node(
                "extract_thermal_diagnostics",
                "extract.thermal_result_diagnostics",
                "result/heat_plane_quad_2d",
            ),
            extract_node(
                "extract_thermo_diagnostics",
                "extract.thermo_result_diagnostics",
                "result/thermal_plane_quad_2d",
            ),
            WorkflowNode {
                id: "bundle".to_string(),
                kind: WorkflowNodeKind::Transform,
                operator_id: Some("transform.compose_diagnostics_bundle".to_string()),
                name: Some("Compose diagnostics bundle".to_string()),
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
                name: Some("Evaluate diagnostics guard".to_string()),
                description: None,
                config: Some(serde_json::json!({
                    "rules": [
                        { "source": "electrostatic", "field": "electrostatic_field_peak_magnitude", "comparison": "gt", "threshold": 9.0, "severity": "warn", "label": "field ceiling" },
                        { "source": "thermal", "field": "thermal_temperature_max", "comparison": "gt", "threshold": 120.0, "severity": "warn", "label": "thermal temperature" },
                        { "source": "thermo", "field": "thermo_stress_peak", "comparison": "gt", "threshold": 180.0, "severity": "block", "label": "stress ceiling" }
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
                name: Some("Compose diagnostics report payload".to_string()),
                description: None,
                config: Some(serde_json::json!({})),
                cache_policy: None,
                inputs: vec![port("bundle", "artifact/json"), port("guard", "artifact/json")],
                outputs: vec![port("result", "artifact/json")],
            },
            WorkflowNode {
                id: "export".to_string(),
                kind: WorkflowNodeKind::Export,
                operator_id: Some("export.diagnostics_bundle_markdown".to_string()),
                name: Some("Export diagnostics markdown".to_string()),
                description: None,
                config: Some(serde_json::json!({
                    "title": "Electrostatic Heat Thermo Diagnostics"
                })),
                cache_policy: None,
                inputs: vec![port("bundle", "artifact/json")],
                outputs: vec![port("markdown", "export/markdown")],
            },
            WorkflowNode {
                id: "markdown_output".to_string(),
                kind: WorkflowNodeKind::Output,
                operator_id: None,
                name: Some("Markdown output".to_string()),
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
                "electrostatic_model",
                "model",
                "solve_electrostatic",
                "model",
                "study_model/electrostatic_plane_quad_2d",
            ),
            edge(
                "e1",
                "solve_electrostatic",
                "result",
                "bridge_field_to_heat",
                "electrostatic_result",
                "result/electrostatic_plane_quad_2d",
            ),
            edge(
                "e2",
                "bridge_field_to_heat",
                "heat_model",
                "solve_heat",
                "model",
                "study_model/heat_plane_quad_2d",
            ),
            edge(
                "e3",
                "solve_heat",
                "result",
                "bridge_temperature",
                "heat_result",
                "result/heat_plane_quad_2d",
            ),
            edge(
                "e4",
                "bridge_temperature",
                "thermo_model",
                "solve_thermo",
                "model",
                "study_model/thermal_plane_quad_2d",
            ),
            edge(
                "e5",
                "solve_electrostatic",
                "result",
                "extract_electrostatic_diagnostics",
                "result",
                "result/electrostatic_plane_quad_2d",
            ),
            edge(
                "e6",
                "solve_heat",
                "result",
                "extract_thermal_diagnostics",
                "result",
                "result/heat_plane_quad_2d",
            ),
            edge(
                "e7",
                "solve_thermo",
                "result",
                "extract_thermo_diagnostics",
                "result",
                "result/thermal_plane_quad_2d",
            ),
            edge(
                "e8",
                "extract_electrostatic_diagnostics",
                "summary",
                "bundle",
                "electrostatic",
                "artifact/json",
            ),
            edge(
                "e9",
                "extract_thermal_diagnostics",
                "summary",
                "bundle",
                "thermal",
                "artifact/json",
            ),
            edge(
                "e10",
                "extract_thermo_diagnostics",
                "summary",
                "bundle",
                "thermo",
                "artifact/json",
            ),
            edge("e11", "bundle", "result", "guard", "bundle", "artifact/json"),
            edge("e12", "bundle", "result", "report", "bundle", "artifact/json"),
            edge("e13", "guard", "result", "report", "guard", "artifact/json"),
            edge("e14", "report", "result", "export", "bundle", "artifact/json"),
            edge(
                "e15",
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
    .expect("coupled diagnostics workflow should run");

    assert_eq!(
        run.workflow_id,
        "workflow.electrostatic-heat-thermo-diagnostics-markdown"
    );

    let guard = run
        .artifacts
        .get("guard.result")
        .cloned()
        .expect("guard result should exist");
    assert_eq!(guard["guard_status"].as_str(), Some("block"));
    assert_eq!(guard["guard_passed"].as_bool(), Some(false));

    let bundle = run
        .artifacts
        .get("bundle.result")
        .cloned()
        .expect("bundle result should exist");
    assert_eq!(bundle["bundle_source_count"].as_u64(), Some(3));

    let markdown = run
        .artifacts
        .get("markdown_output.markdown")
        .and_then(|value| value.get("content"))
        .and_then(|value| value.as_str())
        .expect("markdown output should expose content");
    assert!(markdown.contains("# Electrostatic Heat Thermo Diagnostics"));
    assert!(markdown.contains("## Diagnostics Sources"));
    assert!(markdown.contains("## Guard Decision"));
    assert!(markdown.contains("Status: block"));
    assert!(markdown.contains("stress ceiling"));
}

fn input_node(id: &str, port_id: &str, artifact_type: &str) -> WorkflowNode {
    WorkflowNode {
        id: id.to_string(),
        kind: WorkflowNodeKind::Input,
        operator_id: None,
        name: Some(id.replace('_', " ")),
        description: None,
        config: None,
        cache_policy: None,
        inputs: vec![],
        outputs: vec![port(port_id, artifact_type)],
    }
}

fn solve_node(id: &str, operator_id: &str, input_type: &str, output_type: &str) -> WorkflowNode {
    WorkflowNode {
        id: id.to_string(),
        kind: WorkflowNodeKind::Solve,
        operator_id: Some(operator_id.to_string()),
        name: Some(id.replace('_', " ")),
        description: None,
        config: None,
        cache_policy: None,
        inputs: vec![port("model", input_type)],
        outputs: vec![port("result", output_type)],
    }
}

fn extract_node(id: &str, operator_id: &str, input_type: &str) -> WorkflowNode {
    WorkflowNode {
        id: id.to_string(),
        kind: WorkflowNodeKind::Extract,
        operator_id: Some(operator_id.to_string()),
        name: Some(id.replace('_', " ")),
        description: None,
        config: Some(serde_json::json!({})),
        cache_policy: None,
        inputs: vec![port("result", input_type)],
        outputs: vec![port("summary", "artifact/json")],
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
