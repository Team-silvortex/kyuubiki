use kyuubiki_engine::run_solve_operator;
use serde_json::json;

#[test]
fn workflow_route_executes_precritical_imperfection_amplification() {
    let result = run_solve_operator(
        "solve.frame_2d_p_delta",
        json!({
            "buckling": {
                "frame": {
                    "nodes": [
                        {"id": "n0", "x": 0.0, "y": 0.0, "fix_x": true, "fix_y": true, "fix_rz": false, "load_x": 0.0, "load_y": 0.0, "moment_z": 0.0},
                        {"id": "n1", "x": 0.0, "y": 1.0, "fix_x": false, "fix_y": false, "fix_rz": false, "load_x": 0.0, "load_y": 0.0, "moment_z": 0.0},
                        {"id": "n2", "x": 0.0, "y": 2.0, "fix_x": true, "fix_y": false, "fix_rz": false, "load_x": 0.0, "load_y": -100000.0, "moment_z": 0.0}
                    ],
                    "elements": [
                        {"id": "e0", "node_i": 0, "node_j": 1, "area": 0.01, "youngs_modulus": 210000000000.0, "moment_of_inertia": 0.000008, "section_modulus": 0.0001},
                        {"id": "e1", "node_i": 1, "node_j": 2, "area": 0.01, "youngs_modulus": 210000000000.0, "moment_of_inertia": 0.000008, "section_modulus": 0.0001}
                    ]
                },
                "mode_count": 1
            },
            "imperfection_amplitude": 0.002,
            "imperfection_mode_index": 0,
            "load_steps": 4
        }),
    )
    .expect("workflow p-delta route should solve");

    assert_eq!(result["steps"].as_array().unwrap().len(), 4);
    assert!(result["max_imperfection_amplification"].as_f64().unwrap() > 1.0);
    assert_eq!(result["imperfection_source"], "buckling_mode");
    assert_eq!(result["kinematics"], "linearized_p_delta");
    assert_eq!(result["converged"], true);
    assert_eq!(result["critical_factor_limit_ratio"], 0.95);
    assert_eq!(
        result["_solver_provenance"]["operator_id"],
        "solve.frame_2d_p_delta"
    );
}

#[test]
fn workflow_route_executes_corotational_equilibrium() {
    let result = run_solve_operator(
        "solve.frame_2d_p_delta",
        json!({
            "buckling": {
                "frame": {
                    "nodes": [
                        {"id": "n0", "x": 0.0, "y": 0.0, "fix_x": true, "fix_y": true, "fix_rz": false, "load_x": 0.0, "load_y": 0.0, "moment_z": 0.0},
                        {"id": "n1", "x": 0.0, "y": 1.0, "fix_x": false, "fix_y": false, "fix_rz": false, "load_x": 0.0, "load_y": 0.0, "moment_z": 0.0},
                        {"id": "n2", "x": 0.0, "y": 2.0, "fix_x": true, "fix_y": false, "fix_rz": false, "load_x": 0.0, "load_y": -100000.0, "moment_z": 0.0}
                    ],
                    "elements": [
                        {"id": "e0", "node_i": 0, "node_j": 1, "area": 0.01, "youngs_modulus": 210000000000.0, "moment_of_inertia": 0.000008, "section_modulus": 0.0001},
                        {"id": "e1", "node_i": 1, "node_j": 2, "area": 0.01, "youngs_modulus": 210000000000.0, "moment_of_inertia": 0.000008, "section_modulus": 0.0001}
                    ]
                },
                "mode_count": 1
            },
            "imperfection_amplitude": 0.002,
            "imperfection_mode_index": 0,
            "kinematics": "corotational",
            "load_steps": 4
        }),
    )
    .expect("workflow corotational route should solve");

    assert_eq!(result["kinematics"], "corotational");
    assert_eq!(result["converged"], true);
    assert_eq!(result["steps"].as_array().unwrap().len(), 4);
    assert!(
        result["steps"]
            .as_array()
            .unwrap()
            .iter()
            .all(|step| step["converged"] == true)
    );
    assert!(
        result["steps"]
            .as_array()
            .unwrap()
            .iter()
            .any(|step| step["iterations"].as_u64().unwrap() > 1)
    );
}
