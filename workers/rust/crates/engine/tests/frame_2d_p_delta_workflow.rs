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
            "load_steps": 4,
            "max_iterations": 24,
            "tolerance": 1.0e-9,
            "max_step_cutbacks": 6
        }),
    )
    .expect("workflow corotational route should solve");

    assert_eq!(result["kinematics"], "corotational");
    assert_eq!(result["converged"], true);
    assert_eq!(result["input"]["max_iterations"], 24);
    assert_eq!(result["input"]["max_step_cutbacks"], 6);
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
            .all(|step| step["achieved_load_factor"] == step["load_factor"])
    );
    assert!(
        result["steps"]
            .as_array()
            .unwrap()
            .iter()
            .all(|step| step["failure_reason"].is_null())
    );
    assert!(
        result["steps"]
            .as_array()
            .unwrap()
            .iter()
            .any(|step| step["iterations"].as_u64().unwrap() > 1)
    );
}

#[test]
fn workflow_route_executes_arc_length_continuation() {
    let mut payload = json!({
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
        "path_control": "arc_length",
        "load_steps": 4,
        "max_iterations": 32,
        "arc_length_target_iterations": 4,
        "tolerance": 1.0e-8
    });
    let result = run_solve_operator("solve.frame_2d_p_delta", payload.clone())
        .expect("workflow arc-length route should solve");

    assert_eq!(result["path_control"], "arc_length");
    assert_eq!(result["input"]["arc_length_target_iterations"], 4);
    assert_eq!(result["converged"], true);
    assert_eq!(result["steps"].as_array().unwrap().len(), 4);
    assert!(result["steps"].as_array().unwrap().iter().all(|step| {
        step["arc_length_constraint_error"]
            .as_f64()
            .is_some_and(|error| error < 1.0e-8)
    }));
    assert!(
        result["steps"]
            .as_array()
            .unwrap()
            .iter()
            .all(|step| step["arc_length_radius"].as_f64().is_some())
    );
    assert!(result["steps"].as_array().unwrap().iter().all(|step| {
        step["load_factor_increment"]
            .as_f64()
            .is_some_and(|value| value > 0.0)
            && step["path_event"].is_null()
    }));
    assert!(result["steps"].as_array().unwrap().iter().all(|step| {
        step["tangent_stability"].as_str().is_some()
            && step["tangent_negative_pivots"].as_u64().is_some()
            && step["tangent_near_zero_pivots"].as_u64().is_some()
    }));
    let steps = result["steps"].as_array().unwrap();
    assert!(steps[0]["tangent_negative_pivot_delta"].is_null());
    assert!(
        steps
            .iter()
            .skip(1)
            .all(|step| { step["tangent_negative_pivot_delta"].as_i64() == Some(0) })
    );
    assert!(steps.iter().all(|step| {
        step["tangent_critical_eigenvalue"].is_null()
            && step["tangent_critical_mode_residual"].is_null()
            && step["tangent_critical_mode"].is_null()
            && step["tangent_transition_load_factor_min"].is_null()
            && step["tangent_transition_load_factor_max"].is_null()
            && step["tangent_transition_load_factor_width"].is_null()
            && step["tangent_transition_refinements"].is_null()
            && step["tangent_critical_load_factor"].is_null()
            && step["branch_switch_probes"]
                .as_array()
                .is_some_and(Vec::is_empty)
    }));

    let exported_state = result["continuation_state"].clone();
    assert_eq!(exported_state["displacements"].as_array().unwrap().len(), 9);
    assert_eq!(
        exported_state["displacement_increment"]
            .as_array()
            .unwrap()
            .len(),
        9
    );
    payload["load_steps"] = json!(2);
    payload["continuation_state"] = exported_state.clone();
    let continued = run_solve_operator("solve.frame_2d_p_delta", payload)
        .expect("workflow route should accept its exported continuation state");
    assert_eq!(continued["input"]["continuation_state"], exported_state);
    assert_eq!(continued["steps"].as_array().unwrap().len(), 2);
    assert_eq!(continued["converged"], true);
    assert!(
        continued["continuation_state_correction_norm"]
            .as_f64()
            .is_some_and(f64::is_finite)
    );
}

#[test]
fn workflow_route_executes_a_state_seeded_parameter_path() {
    let result = run_solve_operator(
        "solve.frame_2d_p_delta_path",
        json!({
            "points": [
                parameter_path_point(0.000008),
                parameter_path_point(0.0000082),
                parameter_path_point(0.0000079)
            ],
            "max_subdivisions": 3,
            "minimum_step_fraction": 0.03125,
            "minimum_branch_shape_overlap": 0.75
        }),
    )
    .expect("workflow parameter path route should solve");

    assert_eq!(result["completed_point_count"], 3);
    assert_eq!(result["converged"], true);
    assert_eq!(result["attempts"].as_array().unwrap().len(), 3);
    assert!(
        result["attempts"]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
            .all(|(index, attempt)| {
                attempt["converged"] == true
                    && attempt["result"]["continuation_state"]["displacements"]
                        .as_array()
                        .is_some_and(|values| values.len() == 9)
                    && (index == 0
                        || attempt["branch_shape_overlap"]
                            .as_f64()
                            .is_some_and(|overlap| overlap >= 0.75))
            })
    );
    assert_eq!(
        result["_solver_provenance"]["operator_id"],
        "solve.frame_2d_p_delta_path"
    );
}

fn parameter_path_point(moment_of_inertia: f64) -> serde_json::Value {
    json!({
        "buckling": {
            "frame": {
                "nodes": [
                    {"id": "n0", "x": 0.0, "y": 0.0, "fix_x": true, "fix_y": true, "fix_rz": false, "load_x": 0.0, "load_y": 0.0, "moment_z": 0.0},
                    {"id": "n1", "x": 0.0, "y": 1.0, "fix_x": false, "fix_y": false, "fix_rz": false, "load_x": 0.0, "load_y": 0.0, "moment_z": 0.0},
                    {"id": "n2", "x": 0.0, "y": 2.0, "fix_x": true, "fix_y": false, "fix_rz": false, "load_x": 0.0, "load_y": -100000.0, "moment_z": 0.0}
                ],
                "elements": [
                    {"id": "e0", "node_i": 0, "node_j": 1, "area": 0.01, "youngs_modulus": 210000000000.0, "moment_of_inertia": moment_of_inertia, "section_modulus": 0.0001},
                    {"id": "e1", "node_i": 1, "node_j": 2, "area": 0.01, "youngs_modulus": 210000000000.0, "moment_of_inertia": moment_of_inertia, "section_modulus": 0.0001}
                ]
            },
            "mode_count": 1
        },
        "imperfection_amplitude": 0.002,
        "imperfection_shape": [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        "imperfection_mode_index": 0,
        "kinematics": "corotational",
        "path_control": "arc_length",
        "load_steps": 4,
        "max_iterations": 32,
        "tolerance": 1.0e-8
    })
}
