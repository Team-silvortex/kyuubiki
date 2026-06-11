use crate::run_workflow_graph;
use kyuubiki_protocol::{
    WorkflowCachePolicy, WorkflowDefaults, WorkflowEdge, WorkflowGraph, WorkflowGraphRunRequest,
    WorkflowGraphRunResult, WorkflowNode, WorkflowNodeKind, WorkflowNodePortRef, WorkflowPort,
};
use std::collections::BTreeMap;

#[test]
fn runs_truss_3d_extract_export_graph() {
    let run = run_solver_summary_json_graph(
        "workflow.truss-3d-summary-json",
        "Truss 3d summary json",
        "truss_model",
        "study_model/truss_3d",
        "solve_truss",
        "solve.truss_3d",
        "result/truss_3d",
        serde_json::json!({
            "nodes": [
                { "id": "b0", "x": 0.0, "y": 0.0, "z": 0.0, "fix_x": true, "fix_y": true, "fix_z": true, "load_x": 0.0, "load_y": 0.0, "load_z": 0.0 },
                { "id": "b1", "x": 1.2, "y": 0.0, "z": 0.0, "fix_x": true, "fix_y": true, "fix_z": true, "load_x": 0.0, "load_y": 0.0, "load_z": 0.0 },
                { "id": "b2", "x": 0.0, "y": 1.2, "z": 0.0, "fix_x": true, "fix_y": true, "fix_z": true, "load_x": 0.0, "load_y": 0.0, "load_z": 0.0 },
                { "id": "top", "x": 0.35, "y": 0.35, "z": 1.0, "fix_x": false, "fix_y": false, "fix_z": false, "load_x": 0.0, "load_y": 0.0, "load_z": -1600.0 }
            ],
            "elements": [
                { "id": "e0", "node_i": 0, "node_j": 1, "area": 0.01, "youngs_modulus": 70000000000.0 },
                { "id": "e1", "node_i": 1, "node_j": 2, "area": 0.01, "youngs_modulus": 70000000000.0 },
                { "id": "e2", "node_i": 2, "node_j": 0, "area": 0.01, "youngs_modulus": 70000000000.0 },
                { "id": "e3", "node_i": 0, "node_j": 3, "area": 0.01, "youngs_modulus": 70000000000.0 },
                { "id": "e4", "node_i": 1, "node_j": 3, "area": 0.01, "youngs_modulus": 70000000000.0 },
                { "id": "e5", "node_i": 2, "node_j": 3, "area": 0.01, "youngs_modulus": 70000000000.0 }
            ]
        }),
        &["max_displacement", "max_stress"],
    );

    let content = exported_content(&run);
    assert!(content.contains("max_displacement"));
    assert!(content.contains("max_stress"));
}

#[test]
fn runs_frame_2d_extract_export_graph() {
    let run = run_solver_summary_json_graph(
        "workflow.frame-2d-summary-json",
        "Frame 2d summary json",
        "frame_model",
        "study_model/frame_2d",
        "solve_frame",
        "solve.frame_2d",
        "result/frame_2d",
        serde_json::json!({
            "nodes": [
                { "id": "n0", "x": 0.0, "y": 0.0, "fix_x": true, "fix_y": true, "fix_rz": true, "load_x": 0.0, "load_y": 0.0, "moment_z": 0.0 },
                { "id": "n1", "x": 2.0, "y": 0.0, "fix_x": false, "fix_y": false, "fix_rz": false, "load_x": 0.0, "load_y": -1000.0, "moment_z": 0.0 }
            ],
            "elements": [
                { "id": "f0", "node_i": 0, "node_j": 1, "area": 0.02, "youngs_modulus": 210000000000.0, "moment_of_inertia": 0.000008, "section_modulus": 0.00016 }
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
fn runs_beam_1d_extract_export_graph() {
    let run = run_solver_summary_json_graph(
        "workflow.beam-1d-summary-json",
        "Beam 1d summary json",
        "beam_model",
        "study_model/beam_1d",
        "solve_beam",
        "solve.beam_1d",
        "result/beam_1d",
        serde_json::json!({
            "nodes": [
                { "id": "n0", "x": 0.0, "fix_y": true, "fix_rz": true, "load_y": 0.0, "moment_z": 0.0 },
                { "id": "n1", "x": 2.0, "fix_y": false, "fix_rz": false, "load_y": -1000.0, "moment_z": 0.0 }
            ],
            "elements": [
                { "id": "b0", "node_i": 0, "node_j": 1, "youngs_modulus": 210000000000.0, "moment_of_inertia": 0.000008, "section_modulus": 0.00016, "distributed_load_y": 0.0 }
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
fn runs_spring_2d_extract_export_graph() {
    let run = run_solver_summary_json_graph(
        "workflow.spring-2d-summary-json",
        "Spring 2d summary json",
        "spring_model",
        "study_model/spring_2d",
        "solve_spring",
        "solve.spring_2d",
        "result/spring_2d",
        serde_json::json!({
            "nodes": [
                { "id": "s0", "x": 0.0, "y": 0.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0 },
                { "id": "s1", "x": 1.0, "y": 0.0, "fix_x": false, "fix_y": true, "load_x": 0.0, "load_y": 0.0 },
                { "id": "s2", "x": 1.0, "y": 1.0, "fix_x": false, "fix_y": false, "load_x": 1200.0, "load_y": -600.0 },
                { "id": "s3", "x": 0.0, "y": 1.0, "fix_x": true, "fix_y": false, "load_x": 0.0, "load_y": 0.0 }
            ],
            "elements": [
                { "id": "sp0", "node_i": 0, "node_j": 1, "stiffness": 25000.0 },
                { "id": "sp1", "node_i": 1, "node_j": 2, "stiffness": 18000.0 },
                { "id": "sp2", "node_i": 2, "node_j": 3, "stiffness": 22000.0 },
                { "id": "sp3", "node_i": 3, "node_j": 0, "stiffness": 18000.0 },
                { "id": "sp4", "node_i": 0, "node_j": 2, "stiffness": 12000.0 }
            ]
        }),
        &["max_displacement", "max_force"],
    );

    let content = exported_content(&run);
    assert!(content.contains("max_displacement"));
    assert!(content.contains("max_force"));
}

#[test]
fn runs_spring_3d_extract_export_graph() {
    let run = run_solver_summary_json_graph(
        "workflow.spring-3d-summary-json",
        "Spring 3d summary json",
        "spring_model",
        "study_model/spring_3d",
        "solve_spring",
        "solve.spring_3d",
        "result/spring_3d",
        serde_json::json!({
            "nodes": [
                { "id": "s0", "x": 0.0, "y": 0.0, "z": 0.0, "fix_x": true, "fix_y": true, "fix_z": true, "load_x": 0.0, "load_y": 0.0, "load_z": 0.0 },
                { "id": "s1", "x": 1.2, "y": 0.0, "z": 0.0, "fix_x": true, "fix_y": true, "fix_z": true, "load_x": 0.0, "load_y": 0.0, "load_z": 0.0 },
                { "id": "s2", "x": 0.0, "y": 1.0, "z": 0.0, "fix_x": true, "fix_y": true, "fix_z": true, "load_x": 0.0, "load_y": 0.0, "load_z": 0.0 },
                { "id": "top", "x": 0.45, "y": 0.35, "z": 1.1, "fix_x": false, "fix_y": false, "fix_z": false, "load_x": 250.0, "load_y": 0.0, "load_z": -1100.0 }
            ],
            "elements": [
                { "id": "k0", "node_i": 0, "node_j": 3, "stiffness": 18000.0 },
                { "id": "k1", "node_i": 1, "node_j": 3, "stiffness": 22000.0 },
                { "id": "k2", "node_i": 2, "node_j": 3, "stiffness": 16000.0 },
                { "id": "k3", "node_i": 0, "node_j": 1, "stiffness": 9000.0 },
                { "id": "k4", "node_i": 1, "node_j": 2, "stiffness": 7000.0 },
                { "id": "k5", "node_i": 2, "node_j": 0, "stiffness": 8000.0 }
            ]
        }),
        &["max_displacement", "max_force"],
    );

    let content = exported_content(&run);
    assert!(content.contains("max_displacement"));
    assert!(content.contains("max_force"));
}

#[test]
fn runs_thermal_beam_1d_extract_export_graph() {
    let run = run_solver_summary_json_graph(
        "workflow.thermal-beam-1d-summary-json",
        "Thermal beam 1d summary json",
        "thermal_beam_model",
        "study_model/thermal_beam_1d",
        "solve_thermal_beam",
        "solve.thermal_beam_1d",
        "result/thermal_beam_1d",
        serde_json::json!({
            "nodes": [
                { "id": "tb0", "x": 0.0, "fix_y": true, "fix_rz": true, "load_y": 0.0, "moment_z": 0.0 },
                { "id": "tb1", "x": 2.4, "fix_y": false, "fix_rz": false, "load_y": 0.0, "moment_z": 0.0 }
            ],
            "elements": [
                { "id": "tm0", "node_i": 0, "node_j": 1, "youngs_modulus": 210000000000.0, "moment_of_inertia": 0.00012, "section_modulus": 0.0011, "thermal_expansion": 0.000012, "section_depth": 0.3, "distributed_load_y": 0.0, "temperature_gradient_y": 45.0 }
            ]
        }),
        &[
            "max_displacement",
            "max_rotation",
            "max_temperature_gradient",
            "max_moment",
            "max_stress",
        ],
    );

    let content = exported_content(&run);
    assert!(content.contains("max_displacement"));
    assert!(content.contains("max_rotation"));
    assert!(content.contains("max_temperature_gradient"));
    assert!(content.contains("max_moment"));
    assert!(content.contains("max_stress"));
}

#[test]
fn runs_thermal_frame_2d_extract_export_graph() {
    let run = run_solver_summary_json_graph(
        "workflow.thermal-frame-2d-summary-json",
        "Thermal frame 2d summary json",
        "thermal_frame_model",
        "study_model/thermal_frame_2d",
        "solve_thermal_frame",
        "solve.thermal_frame_2d",
        "result/thermal_frame_2d",
        serde_json::json!({
            "nodes": [
                { "id": "tf0", "x": 0.0, "y": 0.0, "fix_x": true, "fix_y": true, "fix_rz": true, "load_x": 0.0, "load_y": 0.0, "moment_z": 0.0, "temperature_delta": 0.0 },
                { "id": "tf1", "x": 0.0, "y": 3.0, "fix_x": false, "fix_y": false, "fix_rz": false, "load_x": 0.0, "load_y": 0.0, "moment_z": 0.0, "temperature_delta": 35.0 },
                { "id": "tf2", "x": 4.0, "y": 3.0, "fix_x": false, "fix_y": false, "fix_rz": false, "load_x": 0.0, "load_y": 0.0, "moment_z": 0.0, "temperature_delta": 35.0 },
                { "id": "tf3", "x": 4.0, "y": 0.0, "fix_x": true, "fix_y": true, "fix_rz": true, "load_x": 0.0, "load_y": 0.0, "moment_z": 0.0, "temperature_delta": 0.0 }
            ],
            "elements": [
                { "id": "te0", "node_i": 0, "node_j": 1, "area": 0.02, "youngs_modulus": 210000000000.0, "moment_of_inertia": 0.00014, "section_modulus": 0.0012, "thermal_expansion": 0.000012, "section_depth": 0.2, "temperature_gradient_y": 0.0 },
                { "id": "te1", "node_i": 1, "node_j": 2, "area": 0.02, "youngs_modulus": 210000000000.0, "moment_of_inertia": 0.00014, "section_modulus": 0.0012, "thermal_expansion": 0.000012, "section_depth": 0.2, "temperature_gradient_y": 30.0 },
                { "id": "te2", "node_i": 2, "node_j": 3, "area": 0.02, "youngs_modulus": 210000000000.0, "moment_of_inertia": 0.00014, "section_modulus": 0.0012, "thermal_expansion": 0.000012, "section_depth": 0.2, "temperature_gradient_y": 0.0 }
            ]
        }),
        &[
            "max_displacement",
            "max_rotation",
            "max_axial_force",
            "max_moment",
            "max_stress",
            "max_temperature_delta",
            "max_temperature_gradient",
        ],
    );

    let content = exported_content(&run);
    assert!(content.contains("max_displacement"));
    assert!(content.contains("max_rotation"));
    assert!(content.contains("max_axial_force"));
    assert!(content.contains("max_moment"));
    assert!(content.contains("max_stress"));
    assert!(content.contains("max_temperature_delta"));
    assert!(content.contains("max_temperature_gradient"));
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
        description: Some(workflow_name.to_string()),
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
    .expect("solver structural workflow should run")
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
