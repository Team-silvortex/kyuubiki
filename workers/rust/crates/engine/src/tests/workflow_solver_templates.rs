use crate::run_workflow_graph;
use kyuubiki_protocol::{
    WorkflowCachePolicy, WorkflowDefaults, WorkflowEdge, WorkflowGraph, WorkflowGraphRunRequest,
    WorkflowGraphRunResult, WorkflowNode, WorkflowNodeKind, WorkflowNodePortRef, WorkflowPort,
};
use std::collections::BTreeMap;

#[test]
fn runs_frame_3d_extract_export_graph() {
    let run = run_solver_summary_json_graph(
        "workflow.frame-3d-summary-json",
        "Frame 3d summary json",
        "frame_model",
        "study_model/frame_3d",
        "solve_frame",
        "solve.frame_3d",
        "result/frame_3d",
        serde_json::json!({
            "nodes": [
                {
                    "id": "n0",
                    "x": 0.0,
                    "y": 0.0,
                    "z": 0.0,
                    "fix_x": true,
                    "fix_y": true,
                    "fix_z": true,
                    "fix_rx": true,
                    "fix_ry": true,
                    "fix_rz": true,
                    "load_x": 0.0,
                    "load_y": 0.0,
                    "load_z": 0.0,
                    "moment_x": 0.0,
                    "moment_y": 0.0,
                    "moment_z": 0.0
                },
                {
                    "id": "n1",
                    "x": 2.0,
                    "y": 0.0,
                    "z": 0.0,
                    "fix_x": false,
                    "fix_y": false,
                    "fix_z": false,
                    "fix_rx": false,
                    "fix_ry": false,
                    "fix_rz": false,
                    "load_x": 0.0,
                    "load_y": -1000.0,
                    "load_z": 0.0,
                    "moment_x": 0.0,
                    "moment_y": 0.0,
                    "moment_z": 0.0
                }
            ],
            "elements": [
                {
                    "id": "f0",
                    "node_i": 0,
                    "node_j": 1,
                    "area": 0.02,
                    "youngs_modulus": 210000000000.0,
                    "shear_modulus": 80000000000.0,
                    "torsion_constant": 0.000005,
                    "moment_of_inertia_y": 0.000008,
                    "moment_of_inertia_z": 0.000008,
                    "section_modulus_y": 0.00016,
                    "section_modulus_z": 0.00016
                }
            ]
        }),
        &[
            "max_displacement",
            "max_rotation",
            "max_moment",
            "max_stress",
        ],
    );

    let content = exported_content(&run);
    assert!(content.contains("max_displacement"));
    assert!(content.contains("max_rotation"));
    assert!(content.contains("max_moment"));
    assert!(content.contains("max_stress"));
}

#[test]
fn runs_thermal_frame_3d_extract_export_graph() {
    let run = run_solver_summary_json_graph(
        "workflow.thermal-frame-3d-summary-json",
        "Thermal frame 3d summary json",
        "thermal_frame_model",
        "study_model/thermal_frame_3d",
        "solve_thermal_frame",
        "solve.thermal_frame_3d",
        "result/thermal_frame_3d",
        serde_json::json!({
            "nodes": [
                {
                    "id": "n0",
                    "x": 0.0,
                    "y": 0.0,
                    "z": 0.0,
                    "fix_x": true,
                    "fix_y": true,
                    "fix_z": true,
                    "fix_rx": true,
                    "fix_ry": true,
                    "fix_rz": true,
                    "load_x": 0.0,
                    "load_y": 0.0,
                    "load_z": 0.0,
                    "moment_x": 0.0,
                    "moment_y": 0.0,
                    "moment_z": 0.0,
                    "temperature_delta": 35.0
                },
                {
                    "id": "n1",
                    "x": 2.0,
                    "y": 0.0,
                    "z": 0.0,
                    "fix_x": true,
                    "fix_y": true,
                    "fix_z": true,
                    "fix_rx": true,
                    "fix_ry": true,
                    "fix_rz": true,
                    "load_x": 0.0,
                    "load_y": 0.0,
                    "load_z": 0.0,
                    "moment_x": 0.0,
                    "moment_y": 0.0,
                    "moment_z": 0.0,
                    "temperature_delta": 35.0
                }
            ],
            "elements": [
                {
                    "id": "tf3-0",
                    "node_i": 0,
                    "node_j": 1,
                    "area": 0.02,
                    "youngs_modulus": 210000000000.0,
                    "shear_modulus": 80000000000.0,
                    "torsion_constant": 0.000005,
                    "moment_of_inertia_y": 0.000008,
                    "moment_of_inertia_z": 0.000006,
                    "section_modulus_y": 0.00016,
                    "section_modulus_z": 0.00012,
                    "thermal_expansion": 0.000012,
                    "section_depth_y": 0.2,
                    "section_depth_z": 0.15,
                    "temperature_gradient_y": 30.0,
                    "temperature_gradient_z": 20.0
                }
            ]
        }),
        &[
            "max_displacement",
            "max_rotation",
            "max_axial_force",
            "max_moment",
            "max_stress",
        ],
    );

    let content = exported_content(&run);
    assert!(content.contains("max_axial_force"));
    assert!(content.contains("max_moment"));
    assert!(content.contains("max_stress"));
}

#[test]
fn runs_thermal_truss_3d_extract_export_graph() {
    let run = run_solver_summary_json_graph(
        "workflow.thermal-truss-3d-summary-json",
        "Thermal truss 3d summary json",
        "thermal_truss_model",
        "study_model/thermal_truss_3d",
        "solve_thermal_truss",
        "solve.thermal_truss_3d",
        "result/thermal_truss_3d",
        serde_json::json!({
            "nodes": [
                {
                    "id": "n0",
                    "x": 0.0,
                    "y": 0.0,
                    "z": 0.0,
                    "fix_x": true,
                    "fix_y": true,
                    "fix_z": true,
                    "load_x": 0.0,
                    "load_y": 0.0,
                    "load_z": 0.0,
                    "temperature_delta": 40.0
                },
                {
                    "id": "n1",
                    "x": 1.0,
                    "y": 0.0,
                    "z": 0.0,
                    "fix_x": true,
                    "fix_y": true,
                    "fix_z": true,
                    "load_x": 0.0,
                    "load_y": 0.0,
                    "load_z": 0.0,
                    "temperature_delta": 40.0
                },
                {
                    "id": "n2",
                    "x": 0.0,
                    "y": 1.0,
                    "z": 0.0,
                    "fix_x": true,
                    "fix_y": true,
                    "fix_z": true,
                    "load_x": 0.0,
                    "load_y": 0.0,
                    "load_z": 0.0,
                    "temperature_delta": 40.0
                }
            ],
            "elements": [
                {
                    "id": "tt3-0",
                    "node_i": 0,
                    "node_j": 1,
                    "area": 0.01,
                    "youngs_modulus": 210000000000.0,
                    "thermal_expansion": 0.000012
                },
                {
                    "id": "tt3-1",
                    "node_i": 1,
                    "node_j": 2,
                    "area": 0.01,
                    "youngs_modulus": 210000000000.0,
                    "thermal_expansion": 0.000012
                },
                {
                    "id": "tt3-2",
                    "node_i": 2,
                    "node_j": 0,
                    "area": 0.01,
                    "youngs_modulus": 210000000000.0,
                    "thermal_expansion": 0.000012
                }
            ]
        }),
        &["max_displacement", "max_axial_force", "max_stress"],
    );

    let content = exported_content(&run);
    assert!(content.contains("max_axial_force"));
    assert!(content.contains("max_stress"));
}

#[test]
fn runs_electrostatic_plane_triangle_extract_export_graph() {
    let run = run_solver_summary_json_graph(
        "workflow.electrostatic-plane-triangle-summary-json",
        "Electrostatic plane triangle summary json",
        "electrostatic_triangle_model",
        "study_model/electrostatic_plane_triangle_2d",
        "solve_electrostatic_triangle",
        "solve.electrostatic_plane_triangle_2d",
        "result/electrostatic_plane_triangle_2d",
        serde_json::json!({
            "nodes": [
                { "id": "n0", "x": 0.0, "y": 0.0, "fix_potential": true, "potential": 10.0, "charge_density": 0.0 },
                { "id": "n1", "x": 1.0, "y": 0.0, "fix_potential": true, "potential": 0.0, "charge_density": 0.0 },
                { "id": "n2", "x": 0.0, "y": 1.0, "fix_potential": true, "potential": 0.0, "charge_density": 0.0 }
            ],
            "elements": [
                { "id": "et0", "node_i": 0, "node_j": 1, "node_k": 2, "thickness": 0.01, "permittivity": 2.5 }
            ]
        }),
        &["max_potential", "max_electric_field", "max_flux_density"],
    );

    let content = exported_content(&run);
    assert!(content.contains("max_potential"));
    assert!(content.contains("max_electric_field"));
    assert!(content.contains("max_flux_density"));
}

#[test]
fn runs_spring_1d_extract_export_graph() {
    let run = run_solver_summary_json_graph(
        "workflow.spring-1d-summary-json",
        "Spring 1d summary json",
        "spring_model",
        "study_model/spring_1d",
        "solve_spring",
        "solve.spring_1d",
        "result/spring_1d",
        serde_json::json!({
            "nodes": [
                { "id": "s0", "x": 0.0, "fix_x": true, "load_x": 0.0 },
                { "id": "s1", "x": 1.2, "fix_x": false, "load_x": 0.0 },
                { "id": "s2", "x": 2.4, "fix_x": false, "load_x": 1200.0 }
            ],
            "elements": [
                { "id": "k0", "node_i": 0, "node_j": 1, "stiffness": 35000.0 },
                { "id": "k1", "node_i": 1, "node_j": 2, "stiffness": 20000.0 }
            ]
        }),
        &["max_displacement", "max_force"],
    );

    let content = exported_content(&run);
    assert!(content.contains("max_displacement"));
    assert!(content.contains("max_force"));
}

#[test]
fn runs_truss_2d_extract_export_graph() {
    let run = run_solver_summary_json_graph(
        "workflow.truss-2d-summary-json",
        "Truss 2d summary json",
        "truss_model",
        "study_model/truss_2d",
        "solve_truss",
        "solve.truss_2d",
        "result/truss_2d",
        serde_json::json!({
            "nodes": [
                { "id": "n0", "x": 0.0, "y": 0.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0 },
                { "id": "n1", "x": 1.0, "y": 0.0, "fix_x": false, "fix_y": true, "load_x": 0.0, "load_y": 0.0 },
                { "id": "n2", "x": 0.5, "y": 0.75, "fix_x": false, "fix_y": false, "load_x": 0.0, "load_y": -1000.0 }
            ],
            "elements": [
                { "id": "e0", "node_i": 0, "node_j": 2, "area": 0.01, "youngs_modulus": 70000000000.0 },
                { "id": "e1", "node_i": 1, "node_j": 2, "area": 0.01, "youngs_modulus": 70000000000.0 },
                { "id": "e2", "node_i": 0, "node_j": 1, "area": 0.01, "youngs_modulus": 70000000000.0 }
            ]
        }),
        &["max_displacement", "max_stress"],
    );

    let content = exported_content(&run);
    assert!(content.contains("max_displacement"));
    assert!(content.contains("max_stress"));
}

fn run_solver_summary_json_graph(
    workflow_id: &str,
    workflow_name: &str,
    input_node_id: &str,
    input_artifact_type: &str,
    solve_node_id: &str,
    solve_operator_id: &str,
    result_artifact_type: &str,
    model: serde_json::Value,
    summary_fields: &[&str],
) -> WorkflowGraphRunResult {
    let graph = WorkflowGraph {
        schema_version: "kyuubiki.workflow-graph/v1".to_string(),
        id: workflow_id.to_string(),
        name: workflow_name.to_string(),
        version: "1.0.0".to_string(),
        description: Some(format!("{workflow_name}")),
        dataset_contract: None,
        entry_nodes: vec![input_node_id.to_string()],
        output_nodes: vec!["json_output".to_string()],
        defaults: WorkflowDefaults {
            cache_policy: Some(WorkflowCachePolicy::Cached),
            orchestrated: Some(true),
        },
        nodes: vec![
            WorkflowNode {
                id: input_node_id.to_string(),
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
                id: solve_node_id.to_string(),
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
                id: "extract_summary".to_string(),
                kind: WorkflowNodeKind::Extract,
                operator_id: Some("extract.result_summary".to_string()),
                name: Some("Extract summary".to_string()),
                description: None,
                config: Some(serde_json::json!({
                    "fields": summary_fields
                })),
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
                id: "export_json".to_string(),
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
                    node: input_node_id.to_string(),
                    port: "model".to_string(),
                },
                to: WorkflowNodePortRef {
                    node: solve_node_id.to_string(),
                    port: "model".to_string(),
                },
                artifact_type: input_artifact_type.to_string(),
                dataset_value: None,
            },
            WorkflowEdge {
                id: "edge-result".to_string(),
                from: WorkflowNodePortRef {
                    node: solve_node_id.to_string(),
                    port: "result".to_string(),
                },
                to: WorkflowNodePortRef {
                    node: "extract_summary".to_string(),
                    port: "result".to_string(),
                },
                artifact_type: result_artifact_type.to_string(),
                dataset_value: None,
            },
            WorkflowEdge {
                id: "edge-summary".to_string(),
                from: WorkflowNodePortRef {
                    node: "extract_summary".to_string(),
                    port: "summary".to_string(),
                },
                to: WorkflowNodePortRef {
                    node: "export_json".to_string(),
                    port: "summary".to_string(),
                },
                artifact_type: "report/summary".to_string(),
                dataset_value: None,
            },
            WorkflowEdge {
                id: "edge-json".to_string(),
                from: WorkflowNodePortRef {
                    node: "export_json".to_string(),
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
        input_artifacts: BTreeMap::from([(input_node_id.to_string(), model)]),
    })
    .expect("solver template workflow should run")
}

fn exported_content(run: &WorkflowGraphRunResult) -> &str {
    let exported = run
        .artifacts
        .get("json_output.json")
        .expect("json export artifact should exist");
    assert_eq!(run.completed_nodes.len(), 5);
    assert_eq!(exported["format"], serde_json::json!("json"));
    exported["content"]
        .as_str()
        .expect("json content should be a string")
}
