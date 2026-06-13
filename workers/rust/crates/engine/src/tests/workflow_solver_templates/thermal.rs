use super::helpers::{exported_content, run_solver_summary_json_graph};

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
