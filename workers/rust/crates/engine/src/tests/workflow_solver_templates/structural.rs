use super::helpers::{exported_content, run_solver_summary_json_graph};

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
