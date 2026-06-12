use crate::run_workflow_graph;
use kyuubiki_protocol::{
    WorkflowCachePolicy, WorkflowDefaults, WorkflowEdge, WorkflowGraph, WorkflowGraphRunRequest,
    WorkflowGraphRunResult, WorkflowNode, WorkflowNodeKind, WorkflowNodePortRef, WorkflowPort,
};
use std::collections::BTreeMap;

#[test]
fn runs_bar_1d_extract_export_graph() {
    assert_solver_summary(
        "workflow.bar-1d-summary-json",
        "study_model/bar_1d",
        "solve.bar_1d",
        "result/bar_1d",
        serde_json::json!({ "length": 1.0, "area": 0.01, "youngs_modulus": 210000000000.0, "elements": 2, "tip_force": 1200.0 }),
        &["max_displacement", "max_stress"],
    );
}

#[test]
fn runs_thermal_bar_1d_extract_export_graph() {
    assert_solver_summary(
        "workflow.thermal-bar-1d-summary-json",
        "study_model/thermal_bar_1d",
        "solve.thermal_bar_1d",
        "result/thermal_bar_1d",
        serde_json::json!({
            "nodes": [
                { "id": "n0", "x": 0.0, "fix_x": true, "load_x": 0.0, "temperature_delta": 0.0 },
                { "id": "n1", "x": 1.0, "fix_x": true, "load_x": 0.0, "temperature_delta": 35.0 }
            ],
            "elements": [
                { "id": "e0", "node_i": 0, "node_j": 1, "area": 0.01, "youngs_modulus": 210000000000.0, "thermal_expansion": 0.000012 }
            ]
        }),
        &[
            "max_displacement",
            "max_stress",
            "max_axial_force",
            "max_temperature_delta",
        ],
    );
}

#[test]
fn runs_heat_bar_1d_extract_export_graph() {
    assert_solver_summary(
        "workflow.heat-bar-1d-summary-json",
        "study_model/heat_bar_1d",
        "solve.heat_bar_1d",
        "result/heat_bar_1d",
        serde_json::json!({
            "nodes": [
                { "id": "h0", "x": 0.0, "fix_temperature": true, "temperature": 100.0, "heat_load": 0.0 },
                { "id": "h1", "x": 1.0, "fix_temperature": true, "temperature": 20.0, "heat_load": 0.0 }
            ],
            "elements": [
                { "id": "he0", "node_i": 0, "node_j": 1, "area": 0.02, "conductivity": 45.0 }
            ]
        }),
        &["max_temperature", "max_heat_flux"],
    );
}

#[test]
fn runs_heat_plane_triangle_extract_export_graph() {
    assert_solver_summary(
        "workflow.heat-plane-triangle-summary-json",
        "study_model/heat_plane_triangle_2d",
        "solve.heat_plane_triangle_2d",
        "result/heat_plane_triangle_2d",
        serde_json::json!({
            "nodes": [
                { "id": "h0", "x": 0.0, "y": 0.0, "fix_temperature": true, "temperature": 100.0, "heat_load": 0.0 },
                { "id": "h1", "x": 1.0, "y": 0.0, "fix_temperature": true, "temperature": 20.0, "heat_load": 0.0 },
                { "id": "h2", "x": 0.0, "y": 1.0, "fix_temperature": false, "temperature": 0.0, "heat_load": 0.0 }
            ],
            "elements": [
                { "id": "ht0", "node_i": 0, "node_j": 1, "node_k": 2, "thickness": 0.02, "conductivity": 45.0 }
            ]
        }),
        &["max_temperature", "max_heat_flux"],
    );
}

#[test]
fn runs_thermal_truss_2d_extract_export_graph() {
    assert_solver_summary(
        "workflow.thermal-truss-2d-summary-json",
        "study_model/thermal_truss_2d",
        "solve.thermal_truss_2d",
        "result/thermal_truss_2d",
        serde_json::json!({
            "nodes": [
                { "id": "n0", "x": 0.0, "y": 0.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0, "temperature_delta": 20.0 },
                { "id": "n1", "x": 1.0, "y": 0.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0, "temperature_delta": 40.0 },
                { "id": "n2", "x": 0.5, "y": 0.8, "fix_x": false, "fix_y": false, "load_x": 0.0, "load_y": 0.0, "temperature_delta": 40.0 }
            ],
            "elements": [
                { "id": "tt0", "node_i": 0, "node_j": 2, "area": 0.01, "youngs_modulus": 210000000000.0, "thermal_expansion": 0.000012 },
                { "id": "tt1", "node_i": 1, "node_j": 2, "area": 0.01, "youngs_modulus": 210000000000.0, "thermal_expansion": 0.000012 }
            ]
        }),
        &[
            "max_displacement",
            "max_stress",
            "max_axial_force",
            "max_temperature_delta",
        ],
    );
}

#[test]
fn runs_torsion_1d_extract_export_graph() {
    assert_solver_summary(
        "workflow.torsion-1d-summary-json",
        "study_model/torsion_1d",
        "solve.torsion_1d",
        "result/torsion_1d",
        serde_json::json!({
            "nodes": [
                { "id": "t0", "x": 0.0, "fix_rz": true, "torque_z": 0.0 },
                { "id": "t1", "x": 1.0, "fix_rz": false, "torque_z": 500.0 }
            ],
            "elements": [
                { "id": "te0", "node_i": 0, "node_j": 1, "shear_modulus": 80000000000.0, "polar_moment": 0.000005, "section_modulus": 0.00016 }
            ]
        }),
        &["max_rotation", "max_torque", "max_stress"],
    );
}

#[test]
fn runs_plane_triangle_2d_extract_export_graph() {
    assert_solver_summary(
        "workflow.plane-triangle-2d-summary-json",
        "study_model/plane_triangle_2d",
        "solve.plane_triangle_2d",
        "result/plane_triangle_2d",
        serde_json::json!({
            "nodes": [
                { "id": "p0", "x": 0.0, "y": 0.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0 },
                { "id": "p1", "x": 1.0, "y": 0.0, "fix_x": false, "fix_y": true, "load_x": 0.0, "load_y": 0.0 },
                { "id": "p2", "x": 0.0, "y": 1.0, "fix_x": false, "fix_y": false, "load_x": 0.0, "load_y": -1000.0 }
            ],
            "elements": [
                { "id": "pt0", "node_i": 0, "node_j": 1, "node_k": 2, "thickness": 0.02, "youngs_modulus": 70000000000.0, "poisson_ratio": 0.33 }
            ]
        }),
        &["max_displacement", "max_stress"],
    );
}

#[test]
fn runs_thermal_plane_triangle_2d_extract_export_graph() {
    assert_solver_summary(
        "workflow.thermal-plane-triangle-2d-summary-json",
        "study_model/thermal_plane_triangle_2d",
        "solve.thermal_plane_triangle_2d",
        "result/thermal_plane_triangle_2d",
        serde_json::json!({
            "nodes": [
                { "id": "tp0", "x": 0.0, "y": 0.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0, "temperature_delta": 20.0 },
                { "id": "tp1", "x": 1.0, "y": 0.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0, "temperature_delta": 40.0 },
                { "id": "tp2", "x": 0.0, "y": 1.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0, "temperature_delta": 40.0 }
            ],
            "elements": [
                { "id": "tpt0", "node_i": 0, "node_j": 1, "node_k": 2, "thickness": 0.02, "youngs_modulus": 70000000000.0, "poisson_ratio": 0.33, "thermal_expansion": 0.000011 }
            ]
        }),
        &["max_displacement", "max_stress", "max_temperature_delta"],
    );
}

#[test]
fn runs_plane_quad_2d_extract_export_graph() {
    assert_solver_summary(
        "workflow.plane-quad-2d-summary-json",
        "study_model/plane_quad_2d",
        "solve.plane_quad_2d",
        "result/plane_quad_2d",
        serde_json::json!({
            "nodes": [
                { "id": "q0", "x": 0.0, "y": 0.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0 },
                { "id": "q1", "x": 1.0, "y": 0.0, "fix_x": false, "fix_y": true, "load_x": 0.0, "load_y": 0.0 },
                { "id": "q2", "x": 1.0, "y": 1.0, "fix_x": false, "fix_y": false, "load_x": 0.0, "load_y": -1000.0 },
                { "id": "q3", "x": 0.0, "y": 1.0, "fix_x": true, "fix_y": false, "load_x": 0.0, "load_y": 0.0 }
            ],
            "elements": [
                { "id": "pq0", "node_i": 0, "node_j": 1, "node_k": 2, "node_l": 3, "thickness": 0.02, "youngs_modulus": 70000000000.0, "poisson_ratio": 0.33 }
            ]
        }),
        &["max_displacement", "max_stress"],
    );
}

fn assert_solver_summary(
    workflow_id: &str,
    input_artifact_type: &str,
    solve_operator_id: &str,
    result_artifact_type: &str,
    model: serde_json::Value,
    summary_fields: &[&str],
) {
    let run = run_solver_summary_json_graph(
        workflow_id,
        input_artifact_type,
        solve_operator_id,
        result_artifact_type,
        model,
        summary_fields,
    );
    let content = exported_content(&run);
    for field in summary_fields {
        assert!(
            content.contains(field),
            "expected exported summary to contain {field}"
        );
    }
}

fn run_solver_summary_json_graph(
    workflow_id: &str,
    input_artifact_type: &str,
    solve_operator_id: &str,
    result_artifact_type: &str,
    model: serde_json::Value,
    summary_fields: &[&str],
) -> WorkflowGraphRunResult {
    let graph = WorkflowGraph {
        schema_version: "kyuubiki.workflow-graph/v1".to_string(),
        id: workflow_id.to_string(),
        name: workflow_id.to_string(),
        version: "1.0.0".to_string(),
        description: Some(workflow_id.to_string()),
        dataset_contract: None,
        entry_nodes: vec!["solver_input".to_string()],
        output_nodes: vec!["json_output".to_string()],
        defaults: WorkflowDefaults {
            cache_policy: Some(WorkflowCachePolicy::Cached),
            orchestrated: Some(true),
        },
        nodes: vec![
            WorkflowNode {
                id: "solver_input".to_string(),
                kind: WorkflowNodeKind::Input,
                operator_id: None,
                name: Some("Solver input".to_string()),
                description: None,
                config: None,
                cache_policy: None,
                inputs: vec![],
                outputs: vec![WorkflowPort {
                    id: "model".to_string(),
                    artifact_type: input_artifact_type.to_string(),
                    name: None,
                    required: None,
                    cardinality: None,
                    dataset_value: None,
                }],
            },
            WorkflowNode {
                id: "solve".to_string(),
                kind: WorkflowNodeKind::Solve,
                operator_id: Some(solve_operator_id.to_string()),
                name: Some("Solve".to_string()),
                description: None,
                config: None,
                cache_policy: None,
                inputs: vec![WorkflowPort {
                    id: "model".to_string(),
                    artifact_type: input_artifact_type.to_string(),
                    name: None,
                    required: None,
                    cardinality: None,
                    dataset_value: None,
                }],
                outputs: vec![WorkflowPort {
                    id: "result".to_string(),
                    artifact_type: result_artifact_type.to_string(),
                    name: None,
                    required: None,
                    cardinality: None,
                    dataset_value: None,
                }],
            },
            WorkflowNode {
                id: "extract".to_string(),
                kind: WorkflowNodeKind::Extract,
                operator_id: Some("extract.result_summary".to_string()),
                name: Some("Extract summary".to_string()),
                description: None,
                config: Some(serde_json::json!({ "fields": summary_fields })),
                cache_policy: None,
                inputs: vec![WorkflowPort {
                    id: "result".to_string(),
                    artifact_type: result_artifact_type.to_string(),
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
                id: "export".to_string(),
                kind: WorkflowNodeKind::Export,
                operator_id: Some("export.summary_json".to_string()),
                name: Some("Export JSON".to_string()),
                description: None,
                config: None,
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
                    id: "json".to_string(),
                    artifact_type: "export/json".to_string(),
                    name: None,
                    required: None,
                    cardinality: None,
                    dataset_value: None,
                }],
            },
            WorkflowNode {
                id: "json_output".to_string(),
                kind: WorkflowNodeKind::Output,
                operator_id: None,
                name: Some("JSON output".to_string()),
                description: None,
                config: None,
                cache_policy: None,
                inputs: vec![WorkflowPort {
                    id: "json".to_string(),
                    artifact_type: "export/json".to_string(),
                    name: None,
                    required: None,
                    cardinality: None,
                    dataset_value: None,
                }],
                outputs: vec![],
            },
        ],
        edges: vec![
            WorkflowEdge {
                id: "edge-input".to_string(),
                from: WorkflowNodePortRef {
                    node: "solver_input".to_string(),
                    port: "model".to_string(),
                },
                to: WorkflowNodePortRef {
                    node: "solve".to_string(),
                    port: "model".to_string(),
                },
                artifact_type: input_artifact_type.to_string(),
                dataset_value: None,
            },
            WorkflowEdge {
                id: "edge-result".to_string(),
                from: WorkflowNodePortRef {
                    node: "solve".to_string(),
                    port: "result".to_string(),
                },
                to: WorkflowNodePortRef {
                    node: "extract".to_string(),
                    port: "result".to_string(),
                },
                artifact_type: result_artifact_type.to_string(),
                dataset_value: None,
            },
            WorkflowEdge {
                id: "edge-summary".to_string(),
                from: WorkflowNodePortRef {
                    node: "extract".to_string(),
                    port: "summary".to_string(),
                },
                to: WorkflowNodePortRef {
                    node: "export".to_string(),
                    port: "summary".to_string(),
                },
                artifact_type: "report/summary".to_string(),
                dataset_value: None,
            },
            WorkflowEdge {
                id: "edge-json".to_string(),
                from: WorkflowNodePortRef {
                    node: "export".to_string(),
                    port: "json".to_string(),
                },
                to: WorkflowNodePortRef {
                    node: "json_output".to_string(),
                    port: "json".to_string(),
                },
                artifact_type: "export/json".to_string(),
                dataset_value: None,
            },
        ],
    };

    run_workflow_graph(WorkflowGraphRunRequest {
        graph,
        input_artifacts: BTreeMap::from([("solver_input".to_string(), model)]),
    })
    .expect("extended solver workflow should run")
}

fn exported_content(run: &WorkflowGraphRunResult) -> &str {
    let exported = run
        .artifacts
        .get("json_output.json")
        .expect("json export artifact should exist");
    assert_eq!(run.completed_nodes.len(), 5);
    exported["content"]
        .as_str()
        .expect("json content should be a string")
}
