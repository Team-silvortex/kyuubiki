use kyuubiki_engine::run_solve_operator;
use kyuubiki_headless_sdk::{
    HeadlessExecutionBatch, build_execution_plan, engine_solver_headless_bridge_manifest,
};
use serde_json::{Value, json};

#[path = "support/frame_2d_equilibrium.rs"]
mod equilibrium_support;

fn model(scale: f64) -> Value {
    json!({
        "stability": {"buckling": {"frame": {
            "nodes": (0..=4).map(|i| json!({
                "id": format!("n{i}"), "x": 0.0, "y": i as f64,
                "fix_x": i == 0 || i == 4, "fix_y": i == 0, "fix_rz": false,
                "load_x": 0.0, "load_y": if i == 4 { -2.5e6 * scale } else { 0.0 }, "moment_z": 0.0
            })).collect::<Vec<_>>(),
            "elements": (0..4).map(|i| json!({
                "id": format!("e{i}"), "node_i": i, "node_j": i + 1,
                "area": 0.01, "youngs_modulus": 2e11 * scale,
                "moment_of_inertia": 0.1, "section_modulus": 0.1
            })).collect::<Vec<_>>()
        }, "mode_count": 1},
        "imperfection_amplitude": 1e-8, "kinematics": "corotational",
        "max_iterations": 64, "tolerance": 1e-10, "max_step_cutbacks": 12},
        "materials": (0..4).map(|i| json!({
            "element_id": format!("e{i}"), "yield_strength": 2.5e8 * scale, "hardening_ratio": 0.05
        })).collect::<Vec<_>>(),
        "load_factor_schedule": [1.3, 0.0, -1.3, 0.0, 1.3]
    })
}

fn planned_solve(model: Value) -> Result<Value, String> {
    let batch: HeadlessExecutionBatch = serde_json::from_value(json!({
        "schema_version": "kyuubiki.headless-execution-batch/v1",
        "exported_at": "2026-09-23T00:00:00Z", "language": "rust", "workflow_id": "material-reliability",
        "steps": [{"index": 1, "action": "solve_frame_2d_material_p_delta", "risk": "normal", "payload": {"model": model}}]
    })).unwrap();
    let plan = build_execution_plan(&batch);
    assert!(plan.ok, "{:?}", plan.validation);
    let manifest = engine_solver_headless_bridge_manifest();
    let route = manifest
        .routes
        .iter()
        .find(|route| route.action == plan.steps[0].action)
        .unwrap();
    run_solve_operator(
        &route.engine_operator_id,
        plan.steps[0].payload["model"].clone(),
    )
}

#[test]
fn headless_cyclic_material_history_is_finite_and_scale_invariant() {
    let baseline = planned_solve(model(1.0)).unwrap();
    for scale in [1e145, 1e160] {
        let result = planned_solve(model(scale)).unwrap();
        assert_eq!(
            result["stability_result"]["converged"], true,
            "scale {scale}: {}",
            result["stability_result"]
        );
        assert_eq!(result["material_history"].as_array().unwrap().len(), 5);
        for (actual, expected) in result["material_history"]
            .as_array()
            .unwrap()
            .iter()
            .zip(baseline["material_history"].as_array().unwrap())
        {
            for (state, reference) in actual["material_states"]
                .as_array()
                .unwrap()
                .iter()
                .zip(expected["material_states"].as_array().unwrap())
            {
                for key in ["axial_stress", "backstress", "tangent_modulus"] {
                    let value = state[key]
                        .as_f64()
                        .expect("material values must never become JSON null");
                    let expected = reference[key].as_f64().unwrap();
                    assert!(value.is_finite());
                    assert!(
                        (value / scale - expected).abs() <= expected.abs().max(1.0) * 1e-6,
                        "{key}: {value} vs {expected}"
                    );
                }
                let plastic = state["equivalent_plastic_strain"].as_f64().unwrap();
                assert!(
                    (plastic - reference["equivalent_plastic_strain"].as_f64().unwrap()).abs()
                        < 1e-9
                );
            }
        }
    }
}

#[test]
fn headless_failed_reversal_exposes_the_last_committed_state_and_replays_cleanly() {
    let mut input = model(1.0);
    // Resolve the small initial bending residual, then exhaust the budget on
    // a genuinely nonlinear reversal rather than an almost straight column.
    input["stability"]["max_iterations"] = json!(4);
    input["stability"]["imperfection_amplitude"] = json!(0.1);
    input["stability"]["max_step_cutbacks"] = json!(0);
    input["load_factor_schedule"] = json!([1.3, -10.0]);
    let failed = planned_solve(input.clone()).unwrap();
    assert_eq!(failed["stability_result"]["converged"], false);
    assert_eq!(
        failed["material_history"][0]["converged"], true,
        "{}",
        failed["stability_result"]
    );
    assert_eq!(failed["material_history"][1]["converged"], false);
    assert!(
        failed["stability_result"]["steps"][0]["residual_norm"]
            .as_f64()
            .unwrap()
            < 1e-10
    );
    assert!(
        failed["stability_result"]["steps"][1]["residual_norm"]
            .as_f64()
            .unwrap()
            > 1e-10
    );
    assert_eq!(failed["material_history"][1]["achieved_load_factor"], 1.3);
    assert_eq!(
        failed["material_states"],
        failed["material_history"][0]["material_states"]
    );
    assert_eq!(
        failed["material_history"][1]["material_states"],
        failed["material_states"]
    );
    input["load_factor_schedule"] = json!([1.3]);
    let replay = planned_solve(input).unwrap();
    assert_eq!(replay["stability_result"]["converged"], true);
    assert_eq!(failed["material_states"], replay["material_states"]);
}

#[test]
fn headless_invalid_material_is_rejected_without_affecting_the_next_run() {
    let baseline = planned_solve(model(1.0)).unwrap();
    let mut invalid = model(1.0);
    invalid["materials"][0]["hardening_ratio"] = json!(1.0);
    let error = planned_solve(invalid).unwrap_err();
    assert!(error.contains("hardening_ratio"), "{error}");
    assert_eq!(planned_solve(model(1.0)).unwrap(), baseline);
}

fn adaptive_model(scale: f64) -> Value {
    let mut input = model(scale);
    input["stability"]["buckling"]["frame"] = json!({
        "nodes": (0..3).map(|i| json!({
            "id": format!("n{i}"), "x": 0.0, "y": 2.0 * i as f64,
            "fix_x": i == 0, "fix_y": i == 0, "fix_rz": i == 0,
            "load_x": 0.0, "load_y": if i == 2 { -2.5e6 * scale } else { 0.0 },
            "moment_z": if i == 2 { (1e6 / 3.0) * scale } else { 0.0 }
        })).collect::<Vec<_>>(),
        "elements": (0..2).map(|i| json!({
            "id": format!("e{i}"), "node_i": i, "node_j": i + 1,
            "area": 0.01, "youngs_modulus": 2e11 * scale,
            "moment_of_inertia": 5e-4, "section_modulus": 5e-4 / 0.3
        })).collect::<Vec<_>>()
    });
    input["materials"] = json!((0..2).map(|i| json!({
        "element_id": format!("e{i}"), "yield_strength": 2.5e8 * scale, "hardening_ratio": 0.05,
        "adaptive_longitudinal_integration": true, "longitudinal_integration_tolerance": 1e-3,
        "section_fibers": ([-0.3_f64, -0.1, 0.1, 0.3].into_iter().map(|y| json!({
            "y": y, "area": 0.0025, "material_id": if y.abs() > 0.2 { "soft" } else { "stiff" }
        })).collect::<Vec<_>>()),
        "fiber_materials": [
            {"id": "soft", "youngs_modulus": 1.5e11 * scale, "yield_strength": 2e8 * scale, "hardening_ratio": 0.05},
            {"id": "stiff", "youngs_modulus": 2.5e11 * scale, "yield_strength": 3e8 * scale, "hardening_ratio": 0.05}
        ]
    })).collect::<Vec<_>>());
    input["load_factor_schedule"] = json!([0.6, 0.0, -0.6]);
    input
}

#[test]
fn headless_adaptive_mixed_fibers_keep_finite_scale_invariant_diagnostics() {
    let baseline = planned_solve(adaptive_model(1.0)).unwrap();
    assert_eq!(baseline["stability_result"]["converged"], true);
    let scaled = planned_solve(adaptive_model(1e160)).unwrap();
    assert_eq!(
        scaled["stability_result"]["converged"], true,
        "{}",
        scaled["stability_result"]
    );
    assert_eq!(scaled["material_history"].as_array().unwrap().len(), 3);
    for (actual, expected) in scaled["material_history"]
        .as_array()
        .unwrap()
        .iter()
        .zip(baseline["material_history"].as_array().unwrap())
    {
        for (actual, expected) in actual["material_states"]
            .as_array()
            .unwrap()
            .iter()
            .zip(expected["material_states"].as_array().unwrap())
        {
            let error = actual["longitudinal_integration_error"]
                .as_f64()
                .expect("integration error must not serialize as null");
            assert!(error.is_finite() && (0.0..=2.0).contains(&error));
            assert!(
                (error - expected["longitudinal_integration_error"].as_f64().unwrap()).abs() < 1e-6
            );
            assert_eq!(
                actual["active_longitudinal_integration_points"],
                expected["active_longitudinal_integration_points"]
            );
            for field in [
                "section_axial_force",
                "section_end_moment_i",
                "section_end_moment_j",
            ] {
                let value = actual[field].as_f64().unwrap() / 1e160;
                let reference = expected[field].as_f64().unwrap();
                assert!(
                    (value - reference).abs() < 1e-6 * reference.abs().max(1.0),
                    "{field}: {value} vs {reference}"
                );
            }
        }
    }
}

#[test]
fn headless_unused_parent_defaults_do_not_change_fully_overridden_quadrature() {
    let baseline = planned_solve(adaptive_model(1.0)).unwrap();
    let mut changed = adaptive_model(1.0);
    for material in changed["materials"].as_array_mut().unwrap() {
        material["yield_strength"] = json!(1e30);
        material["hardening_ratio"] = json!(0.9);
    }
    let actual = planned_solve(changed).unwrap();
    assert_eq!(actual["stability_result"]["converged"], true);
    assert_eq!(actual["material_states"], baseline["material_states"]);
    assert_eq!(actual["material_history"], baseline["material_history"]);
    assert_eq!(
        actual["stability_result"]["final_displacements"],
        baseline["stability_result"]["final_displacements"]
    );
}

#[test]
fn headless_rejects_tiny_initial_imbalance_then_replays_a_valid_plan() {
    let reference = planned_solve(model(1.0)).unwrap();
    let mut invalid = model(1.0);
    invalid["materials"][0]["initial_axial_stress"] = json!(1e-6);
    let error = planned_solve(invalid).unwrap_err();
    assert!(
        error.contains("not self-equilibrated on free DOFs"),
        "{error}"
    );
    assert!(
        error.contains("node ") && error.contains("translation"),
        "{error}"
    );
    assert_eq!(planned_solve(model(1.0)).unwrap(), reference);
}

#[test]
fn headless_unused_parent_strength_cannot_hide_initial_fiber_imbalance() {
    for strength in [2.5e8, 1e30] {
        let mut invalid = adaptive_model(1.0);
        for material in invalid["materials"].as_array_mut().unwrap() {
            material["yield_strength"] = json!(strength);
            for fiber in material["section_fibers"].as_array_mut().unwrap() {
                fiber["initial_axial_stress"] = json!(1e6);
            }
        }
        let error = planned_solve(invalid).unwrap_err();
        assert!(
            error.contains("not self-equilibrated on free DOFs"),
            "{error}"
        );
    }
}

#[test]
fn headless_balanced_mixed_fiber_prestress_remains_observable() {
    let mut input = adaptive_model(1.0);
    for material in input["materials"].as_array_mut().unwrap() {
        material["yield_strength"] = json!(1e30);
        for fiber in material["section_fibers"].as_array_mut().unwrap() {
            let y = fiber["y"].as_f64().unwrap();
            fiber["initial_axial_stress"] = json!(if y.abs() > 0.2 { -1e6 } else { 1e6 });
        }
    }
    input["load_factor_schedule"] = json!([0.0, 0.1]);
    let result = planned_solve(input).unwrap();
    assert_eq!(result["stability_result"]["converged"], true);
    for state in result["material_history"][0]["material_states"]
        .as_array()
        .unwrap()
    {
        assert_eq!(state["min_fiber_initial_axial_stress"], -1e6);
        assert_eq!(state["max_fiber_initial_axial_stress"], 1e6);
        assert!(state["section_axial_force"].as_f64().unwrap().abs() < 1e-8);
        assert!(state["section_end_moment_i"].as_f64().unwrap().abs() < 1e-8);
        assert!(state["section_end_moment_j"].as_f64().unwrap().abs() < 1e-8);
    }
}

#[test]
fn headless_support_loads_do_not_erase_cyclic_material_history() {
    let baseline = planned_solve(model(1.0)).unwrap();
    let mut input = model(1.0);
    input["stability"]["buckling"]["frame"]["nodes"][0]["load_x"] = json!(1e100);
    input["stability"]["buckling"]["frame"]["nodes"][0]["load_y"] = json!(-1e100);
    let result = planned_solve(input).unwrap();
    assert_eq!(result["stability_result"]["converged"], true);
    assert_eq!(result["material_history"], baseline["material_history"]);
    assert_eq!(result["material_states"], baseline["material_states"]);
    assert_eq!(
        result["stability_result"]["final_displacements"],
        baseline["stability_result"]["final_displacements"]
    );
    assert!(result["max_equivalent_plastic_strain"].as_f64().unwrap() > 0.0);
}

#[test]
fn headless_support_loads_cannot_certify_a_failed_trial_or_commit_its_material_state() {
    let mut input = model(1.0);
    input["stability"]["max_iterations"] = json!(1);
    input["stability"]["max_step_cutbacks"] = json!(0);
    let baseline = planned_solve(input.clone()).unwrap();
    assert_eq!(baseline["stability_result"]["converged"], false);
    input["stability"]["buckling"]["frame"]["nodes"][0]["load_y"] = json!(-1e100);
    let failed = planned_solve(input).unwrap();
    assert_eq!(failed["stability_result"]["converged"], false);
    assert_eq!(
        failed["stability_result"]["steps"],
        baseline["stability_result"]["steps"]
    );
    assert_eq!(failed["material_states"], baseline["material_states"]);
    assert_eq!(failed["material_history"], baseline["material_history"]);
    assert_eq!(
        planned_solve(model(1.0)).unwrap()["stability_result"]["converged"],
        true
    );
}

#[test]
fn headless_unrelated_free_load_does_not_erase_cyclic_material_history() {
    let baseline = planned_solve(model(1.0)).unwrap();
    let mut input = model(1.0);
    equilibrium_support::add_independent_axial_member(&mut input["stability"]);
    let actual = planned_solve(input).unwrap();
    assert_eq!(
        actual["stability_result"]["converged"], true,
        "{}",
        actual["stability_result"]["steps"]
    );
    let history = actual["material_history"].as_array().unwrap();
    let expected = baseline["material_history"].as_array().unwrap();
    assert_eq!(history.len(), expected.len());
    for (step, expected) in history.iter().zip(expected) {
        assert_eq!(step["converged"], true);
        assert_eq!(
            step["achieved_load_factor"],
            expected["achieved_load_factor"]
        );
        assert_eq!(
            step["material_states"].as_array().unwrap().len(),
            expected["material_states"].as_array().unwrap().len()
        );
        for (state, expected) in step["material_states"]
            .as_array()
            .unwrap()
            .iter()
            .zip(expected["material_states"].as_array().unwrap())
        {
            assert_eq!(state["element_id"], expected["element_id"]);
            for field in [
                "axial_stress",
                "plastic_strain",
                "backstress",
                "equivalent_plastic_strain",
            ] {
                let actual = state[field].as_f64().unwrap();
                let expected = expected[field].as_f64().unwrap();
                assert!(
                    (actual - expected).abs() < 1e-8 * expected.abs().max(1.0),
                    "{field}: {actual} vs {expected}"
                );
            }
        }
    }
    assert!(actual["max_equivalent_plastic_strain"].as_f64().unwrap() > 0.0);
}

#[test]
fn headless_unrelated_free_load_cannot_commit_an_unbalanced_material_trial() {
    let mut input = model(1.0);
    input["stability"]["max_iterations"] = json!(2);
    input["stability"]["max_step_cutbacks"] = json!(0);
    let reference = planned_solve(input.clone()).unwrap();
    assert_eq!(reference["stability_result"]["converged"], false);
    equilibrium_support::add_independent_axial_member(&mut input["stability"]);
    let actual = planned_solve(input).unwrap();
    assert_eq!(actual["stability_result"]["converged"], false);
    assert_eq!(actual["material_history"][0]["achieved_load_factor"], 0.0);
    assert_eq!(
        actual["material_states"].as_array().unwrap().len(),
        reference["material_states"].as_array().unwrap().len()
    );
    for (state, expected) in actual["material_states"]
        .as_array()
        .unwrap()
        .iter()
        .zip(reference["material_states"].as_array().unwrap())
    {
        assert_eq!(state, expected);
    }
    assert_eq!(actual["max_equivalent_plastic_strain"], 0.0);
    assert_eq!(
        planned_solve(model(1.0)).unwrap()["stability_result"]["converged"],
        true
    );
}
