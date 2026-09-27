use kyuubiki_engine::run_solve_operator;
use kyuubiki_headless_sdk::{
    HeadlessExecutionBatch, build_execution_plan, engine_solver_headless_bridge_manifest,
};
use serde_json::{Value, json};

fn model(thermal: bool) -> Value {
    let nodes: Vec<_> = (0..2)
        .map(|i| {
            let mut n = json!({"id": format!("n{i}"), "x": 2.0*i as f64, "y": 0.0, "z": 0.0});
            for field in ["fix_x", "fix_y", "fix_z", "fix_rx", "fix_ry", "fix_rz"] {
                n[field] = json!(i == 0);
            }
            for (field, value) in [
                ("load_x", 10.0),
                ("load_y", 20.0),
                ("load_z", -30.0),
                ("moment_x", 4.0),
                ("moment_y", 5.0),
                ("moment_z", 6.0),
            ] {
                n[field] = json!(if i == 1 { value } else { 0.0 });
            }
            if thermal {
                n["temperature_delta"] = json!(0.0);
            }
            n
        })
        .collect();
    let mut e = json!({"id": "member", "node_i": 0, "node_j": 1, "local_y_axis": [0.0, 1.0, 0.0],
        "youngs_modulus": 1000.0, "area": 2.0, "shear_modulus": 400.0, "torsion_constant": 0.5,
        "moment_of_inertia_y": 2.0, "moment_of_inertia_z": 1.0, "section_modulus_y": 2.0, "section_modulus_z": 1.0});
    if thermal {
        for (field, value) in [
            ("thermal_expansion", 1e-5),
            ("section_depth_y", 0.2),
            ("section_depth_z", 0.3),
            ("temperature_gradient_y", 0.0),
            ("temperature_gradient_z", 0.0),
        ] {
            e[field] = json!(value);
        }
    }
    json!({"nodes": nodes, "elements": [e]})
}

fn planned_solve(thermal: bool, input: Value) -> Result<Value, String> {
    let action = if thermal {
        "solve_thermal_frame_3d"
    } else {
        "solve_frame_3d"
    };
    let batch: HeadlessExecutionBatch = serde_json::from_value(json!({
        "schema_version": "kyuubiki.headless-execution-batch/v1", "language": "rust",
        "exported_at": "2026-09-27T00:00:00Z", "workflow_id": "spatial-frame-output",
        "steps": [{"index": 1, "action": action, "risk": "normal", "payload": {"model": input}}]
    }))
    .unwrap();
    let plan = build_execution_plan(&batch);
    assert!(plan.ok, "{:?}", plan.validation);
    let manifest = engine_solver_headless_bridge_manifest();
    let route = manifest
        .routes
        .iter()
        .find(|r| r.action == plan.steps[0].action)
        .unwrap();
    run_solve_operator(
        &route.engine_operator_id,
        plan.steps[0].payload["model"].clone(),
    )
}

fn close(value: &Value, expected: f64, tolerance: f64) {
    let value = value.as_f64().expect("physical output must remain numeric");
    assert!(value.is_finite());
    assert!(
        (value / expected - 1.0).abs() < tolerance,
        "{value:e} != {expected:e}"
    );
}

#[test]
fn headless_spatial_frame_mixed_energy_matches_closed_form() {
    for thermal in [false, true] {
        close(
            &planned_solve(thermal, model(thermal)).unwrap()["total_strain_energy"],
            1.7018333333333333,
            1e-10,
        );
    }
}

#[test]
fn headless_spatial_thermal_background_retains_small_mechanical_energy() {
    let mut input = model(true);
    for n in input["nodes"].as_array_mut().unwrap() {
        n["temperature_delta"] = json!(100.0);
        for field in [
            "load_x", "load_y", "load_z", "moment_x", "moment_y", "moment_z",
        ] {
            n[field] = json!(0.0);
        }
    }
    input["nodes"][1]["load_x"] = json!(0.04);
    input["elements"][0]["youngs_modulus"] = json!(200e9);
    input["elements"][0]["area"] = json!(0.02);
    close(
        &planned_solve(true, input).unwrap()["total_strain_energy"],
        4e-13,
        2e-4,
    );
}

#[test]
fn headless_spatial_frame_numerical_failure_propagates_and_replays() {
    for thermal in [false, true] {
        let mut input = model(thermal);
        input["elements"][0]["youngs_modulus"] = json!(1e200);
        input["elements"][0]["area"] = json!(1e200);
        let error = planned_solve(thermal, input).unwrap_err();
        assert!(
            error.contains("member") && error.contains("representable"),
            "{error}"
        );
        close(
            &planned_solve(thermal, model(thermal)).unwrap()["total_strain_energy"],
            1.7018333333333333,
            1e-10,
        );
    }
}

#[test]
fn headless_spatial_directional_spring_retains_scaled_direction() {
    let mut input = model(true);
    input["directional_springs"] =
        json!([{"id": "spring", "node": 1, "direction": [1e308, 0.0, 0.0], "stiffness": 1000.0}]);
    let result = planned_solve(true, input).unwrap();
    close(&result["nodes"][1]["ux"], 0.005, 1e-10);
    close(
        &result["directional_springs"][0]["reaction_force"],
        -5.0,
        1e-10,
    );
}

fn supported_model(rotational: bool) -> Value {
    let mut input = model(true);
    for n in input["nodes"].as_array_mut().unwrap() {
        for field in [
            "load_x", "load_y", "load_z", "moment_x", "moment_y", "moment_z",
        ] {
            n[field] = json!(0.0);
        }
    }
    let (supports, load) = if rotational {
        ("directional_rotational_constraints", "moment_y")
    } else {
        ("directional_constraints", "load_y")
    };
    input["nodes"][1][load] = json!(10.0);
    input[supports] = json!([
        {"id": "axis", "node": 1, "direction": [1.0, 0.0, 0.0]},
        {"id": "near-axis", "node": 1, "direction": [1.0, 1e-9, 0.0]}
    ]);
    input
}

#[test]
fn headless_spatial_near_parallel_supports_preserve_reactions() {
    for rotational in [false, true] {
        let result = planned_solve(true, supported_model(rotational)).unwrap();
        let (supports, reaction) = if rotational {
            ("directional_rotational_constraints", "reaction_moment")
        } else {
            ("directional_constraints", "reaction_force")
        };
        close(&result[supports][0][reaction], 1e10, 2e-12);
        close(&result[supports][1][reaction], -1e10, 2e-12);
        assert_eq!(result["total_strain_energy"], json!(0.0));
    }
}

#[test]
fn headless_spatial_dependent_supports_fail_and_valid_input_replays() {
    let mut input = supported_model(false);
    input["directional_constraints"][1]["direction"] = json!([1.0, 0.0, 0.0]);
    let error = planned_solve(true, input).unwrap_err();
    assert!(
        error.contains("constraint block 2") && error.contains("linearly dependent"),
        "{error}"
    );
    planned_solve(true, supported_model(false)).unwrap();
}

#[test]
fn headless_spatial_unrepresentable_support_reactions_propagate_failure() {
    let mut input = supported_model(true);
    input["nodes"][1]["moment_y"] = json!(1e300);
    let error = planned_solve(true, input).unwrap_err();
    assert!(
        error.contains("constraint reaction") && error.contains("representable"),
        "{error}"
    );
    planned_solve(true, supported_model(true)).unwrap();
}

#[test]
fn headless_spatial_section_hint_axial_component_preserves_fields() {
    for thermal in [false, true] {
        let mut input = model(thermal);
        input["nodes"][1]["y"] = json!(2.0);
        input["elements"][0]["local_y_axis"] = json!([-1.0, 1.0, 0.0]);
        if thermal {
            for n in input["nodes"].as_array_mut().unwrap() {
                n["temperature_delta"] = json!(100.0);
            }
            input["elements"][0]["temperature_gradient_y"] = json!(35.0);
            input["elements"][0]["temperature_gradient_z"] = json!(-20.0);
        }
        let expected = planned_solve(thermal, input.clone()).unwrap();
        input["elements"][0]["local_y_axis"] = json!([1.0, 1.0 + 1e-10, 0.0]);
        let actual = planned_solve(thermal, input).unwrap();
        for field in ["ux", "uy", "uz", "rx", "ry", "rz"] {
            close(
                &actual["nodes"][1][field],
                expected["nodes"][1][field].as_f64().unwrap(),
                1e-10,
            );
        }
        for field in ["max_stress", "total_strain_energy"] {
            close(&actual[field], expected[field].as_f64().unwrap(), 1e-10);
        }
    }
}

#[test]
fn headless_spatial_parallel_section_hint_fails_and_valid_model_replays() {
    for thermal in [false, true] {
        let mut input = model(thermal);
        input["nodes"][1]["y"] = json!(2.0);
        input["elements"][0]["local_y_axis"] = json!([1e200, 1e200, 0.0]);
        let error = planned_solve(thermal, input).unwrap_err();
        assert!(
            error.contains("local_y_axis must not be parallel"),
            "{error}"
        );
        planned_solve(thermal, model(thermal)).unwrap();
    }
}
