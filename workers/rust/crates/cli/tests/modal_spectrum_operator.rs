use kyuubiki_engine::run_solve_operator;
use kyuubiki_headless_sdk::{
    HeadlessExecutionBatch, build_execution_plan, engine_solver_headless_bridge_manifest,
};
use serde_json::{Value, json};

#[path = "modal_spectrum_operator/clustered.rs"]
mod clustered;

#[path = "modal_spectrum_operator/connected.rs"]
mod connected;

#[path = "modal_spectrum_operator/mass_scaling.rs"]
mod mass_scaling;

fn model(space: bool) -> Value {
    let nodes: Vec<_> = (0..4)
        .map(|index| {
            let mut node = json!({"id":format!("n{index}"), "x":(index % 2) as f64,
            "y":(index / 2) as f64, "fix_x":index % 2 == 0, "fix_y":true, "fix_rz":true,
            "load_x":0.0, "load_y":0.0, "moment_z":0.0});
            if space {
                node["z"] = json!(0.0);
                for key in ["load_z", "moment_x", "moment_y"] {
                    node[key] = json!(0.0);
                }
                for key in ["fix_z", "fix_rx", "fix_ry"] {
                    node[key] = json!(true);
                }
            }
            node
        })
        .collect();
    let elements: Vec<_> = (0..2)
        .map(|index| {
            let modulus = if index == 0 { 1.0 } else { 1.0e14 };
            let mut element = json!({"id":format!("beam{index}"), "node_i":index * 2,
            "node_j":index * 2 + 1, "area":1.0, "youngs_modulus":modulus, "density":1.0});
            if space {
                element["shear_modulus"] = json!(modulus * 0.4);
                for key in [
                    "torsion_constant",
                    "moment_of_inertia_y",
                    "moment_of_inertia_z",
                ] {
                    element[key] = json!(0.01);
                }
            } else {
                element["moment_of_inertia"] = json!(0.01);
                element["section_modulus"] = json!(0.1);
            }
            element
        })
        .collect();
    json!({"nodes":nodes, "elements":elements, "mode_count":2})
}

fn planned_solve(space: bool, model: Value) -> Result<Value, String> {
    let action = if space {
        "solve_modal_frame_3d"
    } else {
        "solve_modal_frame_2d"
    };
    let batch: HeadlessExecutionBatch = serde_json::from_value(json!({
        "schema_version":"kyuubiki.headless-execution-batch/v1",
        "exported_at":"2026-09-23T00:00:00Z", "language":"rust", "workflow_id":"modal-spectrum",
        "steps":[{"index":1, "action":action, "risk":"normal", "payload":{"model":model}}]
    }))
    .unwrap();
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

fn assert_spectrum(result: Value) {
    assert_eq!(result["modes"].as_array().unwrap().len(), 2);
    for (index, expected) in [2.0, 2.0e14].into_iter().enumerate() {
        let mode = &result["modes"][index];
        let actual = mode["eigenvalue_rad_s_squared"].as_f64().unwrap();
        assert!((actual / expected - 1.0).abs() < 1.0e-10);
        for key in [
            "natural_frequency_rad_s",
            "natural_frequency_hz",
            "period_s",
            "participation_norm",
        ] {
            let value = mode[key]
                .as_f64()
                .expect("successful mode must contain finite numbers");
            assert!(value.is_finite() && value > 0.0, "{key}: {value}");
        }
    }
}

#[test]
fn headless_plan_preserves_both_soft_and_stiff_modal_branches() {
    for space in [false, true] {
        assert_spectrum(planned_solve(space, model(space)).unwrap());
    }
}

#[test]
fn engine_returns_restraint_errors_and_the_corrected_model_can_replay() {
    for space in [false, true] {
        let mut floating = model(space);
        floating["nodes"][2]["fix_x"] = json!(false);
        let error = planned_solve(space, floating).unwrap_err();
        assert!(error.contains("unrestrained rigid-body motion"), "{error}");
        assert_spectrum(planned_solve(space, model(space)).unwrap());
    }
}

#[test]
fn headless_modal_common_mass_stiffness_scale_preserves_frequencies() {
    for space in [false, true] {
        let mut input = model(space);
        input["nodes"].as_array_mut().unwrap().truncate(2);
        input["elements"].as_array_mut().unwrap().truncate(1);
        input["mode_count"] = json!(6);
        input["nodes"][1]["x"] = json!(3.0);
        for key in ["fix_y", "fix_rz"] {
            input["nodes"][1][key] = json!(false);
        }
        if space {
            for key in ["fix_z", "fix_rx", "fix_ry"] {
                input["nodes"][1][key] = json!(false);
            }
        }
        let reference = planned_solve(space, input.clone()).unwrap();
        input["elements"][0]["density"] = json!(1.0e307);
        input["elements"][0]["youngs_modulus"] = json!(1.0e307);
        if space {
            input["elements"][0]["shear_modulus"] = json!(4.0e306);
        }
        let result = planned_solve(space, input).unwrap();
        assert!((result["total_mass"].as_f64().unwrap() / 3.0e307 - 1.0).abs() < 1.0e-12);
        let actual = result["modes"].as_array().unwrap();
        let expected = reference["modes"].as_array().unwrap();
        assert_eq!(actual.len(), if space { 6 } else { 3 });
        assert_eq!(actual.len(), expected.len());
        for (actual, expected) in actual.iter().zip(expected) {
            for key in [
                "eigenvalue_rad_s_squared",
                "natural_frequency_rad_s",
                "natural_frequency_hz",
                "period_s",
                "participation_norm",
            ] {
                let ratio = actual[key].as_f64().unwrap() / expected[key].as_f64().unwrap();
                assert!((ratio - 1.0).abs() < 2.0e-10, "{key}: {ratio}");
            }
        }
    }
}

#[test]
fn headless_modal_restrained_mass_failure_is_not_success_and_replays() {
    for space in [false, true] {
        let mut input = model(space);
        input["nodes"].as_array_mut().unwrap().truncate(2);
        input["elements"].as_array_mut().unwrap().truncate(1);
        input["nodes"][1]["x"] = json!(1.0e110);
        input["elements"][0]["density"] = json!(0.1);
        if space {
            for key in [
                "moment_of_inertia_y",
                "moment_of_inertia_z",
                "torsion_constant",
            ] {
                input["elements"][0][key] = json!(1.0e100);
            }
        } else {
            input["elements"][0]["moment_of_inertia"] = json!(1.0e100);
        }
        let error = planned_solve(space, input).expect_err("restrained rotary mass is invalid");
        assert!(
            error.contains("nodal rotary mass") && error.contains("representable"),
            "{error}"
        );
        assert_spectrum(planned_solve(space, model(space)).unwrap());
    }
}

#[test]
fn headless_modal_restrained_stiffness_overflow_fails_before_reduction() {
    for space in [false, true] {
        let mut input = model(space);
        input["nodes"][3]["fix_x"] = json!(true);
        input["elements"][1]["youngs_modulus"] = json!(1.0e308);
        let mut duplicate = input["elements"][1].clone();
        duplicate["id"] = json!("fixed-parallel-member");
        input["elements"].as_array_mut().unwrap().push(duplicate);
        let error = planned_solve(space, input).expect_err("restrained assembly is invalid");
        assert!(
            error.contains("assembled stiffness") && error.contains("representable"),
            "{error}"
        );
        assert_spectrum(planned_solve(space, model(space)).unwrap());
    }
}

#[test]
fn headless_modal_independent_components_preserve_extreme_spectrum() {
    for space in [false, true] {
        for count in [1, 2] {
            let mut input = model(space);
            input["mode_count"] = json!(count);
            for (index, scale) in [1.0e-200, 1.0e200].into_iter().enumerate() {
                input["elements"][index]["youngs_modulus"] = json!(scale);
                if space {
                    input["elements"][index]["shear_modulus"] = json!(0.4 * scale);
                }
            }
            let result = planned_solve(space, input).unwrap();
            assert_eq!(result["modes"].as_array().unwrap().len(), count);
            for (index, expected) in [2.0e-200, 2.0e200].into_iter().take(count).enumerate() {
                let actual = result["modes"][index]["eigenvalue_rad_s_squared"]
                    .as_f64()
                    .unwrap();
                assert!((actual / expected - 1.0).abs() < 1.0e-12);
            }
        }
    }
}

#[test]
fn headless_modal_connected_range_failure_propagates_and_replays() {
    for space in [false, true] {
        let mut input = model(space);
        input["nodes"].as_array_mut().unwrap().truncate(3);
        input["nodes"][2]["x"] = json!(2.0);
        input["nodes"][2]["y"] = json!(0.0);
        input["nodes"][2]["fix_x"] = json!(false);
        input["elements"][0]["node_i"] = json!(1);
        input["elements"][0]["node_j"] = json!(2);
        input["elements"][1]["node_i"] = json!(0);
        input["elements"][1]["node_j"] = json!(2);
        for (index, scale) in [1.0e-200, 1.0e200].into_iter().enumerate() {
            input["elements"][index]["youngs_modulus"] = json!(scale);
            if space {
                input["elements"][index]["shear_modulus"] = json!(0.4 * scale);
            }
        }
        let error = planned_solve(space, input).expect_err("range failure is not success");
        assert!(
            error.contains("scaling") && error.contains("representable"),
            "{error}"
        );
        assert_spectrum(planned_solve(space, model(space)).unwrap());
    }
}

fn heterogeneous_mass_model(space: bool) -> Value {
    let mut input = model(space);
    input["mode_count"] = json!(12);
    for index in [1, 3] {
        for key in ["fix_y", "fix_rz"] {
            input["nodes"][index][key] = json!(false);
        }
        if space {
            for key in ["fix_z", "fix_rx", "fix_ry"] {
                input["nodes"][index][key] = json!(false);
            }
        }
    }
    for (index, scale) in [1e307, 1e-318].into_iter().enumerate() {
        input["elements"][index]["density"] = json!(scale);
        input["elements"][index]["youngs_modulus"] = json!(scale);
        if space {
            input["elements"][index]["shear_modulus"] = json!(0.4 * scale);
            input["elements"][index]["moment_of_inertia_y"] = json!(0.02);
        }
    }
    input
}

#[test]
fn headless_modal_inactive_mass_cannot_change_recovered_shapes() {
    for space in [false, true] {
        let input = heterogeneous_mass_model(space);
        let combined = planned_solve(space, input.clone()).unwrap();
        let mut isolated = input;
        isolated["nodes"].as_array_mut().unwrap().truncate(2);
        isolated["elements"].as_array_mut().unwrap().truncate(1);
        let separate = planned_solve(space, isolated).unwrap();
        let dofs = if space { 6 } else { 3 };
        let active: Vec<_> = combined["modes"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|mode| {
                mode["shape"].as_array().unwrap()[dofs..2 * dofs]
                    .iter()
                    .any(|v| v.as_f64().unwrap() != 0.0)
            })
            .collect();
        assert_eq!(active.len(), dofs);
        for (actual, expected) in active
            .into_iter()
            .zip(separate["modes"].as_array().unwrap())
        {
            for (a, b) in actual["shape"]
                .as_array()
                .unwrap()
                .iter()
                .zip(expected["shape"].as_array().unwrap())
            {
                assert!((a.as_f64().unwrap() - b.as_f64().unwrap()).abs() < 2e-13);
            }
            assert!((actual["participation_norm"].as_f64().unwrap() - 1.0).abs() < 2e-14);
        }
    }
}

#[test]
fn headless_modal_shape_recovery_cancellation_propagates_and_replays() {
    use kyuubiki_solver::solver_control::{SolverControl, SolverStage, with_solver_observer};

    for space in [false, true] {
        let control = SolverControl::default();
        let cancel = control.clone();
        let error = with_solver_observer(
            &control,
            move |point| {
                if point.stage == SolverStage::ResultNodeSummary && point.completed_steps > 0 {
                    cancel.request_cancel();
                }
            },
            || {
                let result = planned_solve(space, heterogeneous_mass_model(space));
                assert!(
                    result.is_err(),
                    "engine must not publish a partially recovered mode"
                );
                result
            },
        )
        .unwrap_err();
        assert!(error.contains("cancel"), "{error}");
        assert_eq!(
            control.last_checkpoint().unwrap().stage,
            SolverStage::ResultNodeSummary
        );
        assert_spectrum(planned_solve(space, model(space)).unwrap());
    }
}

fn axial_chain_model(space: bool, stiffness: [f64; 2], density: f64, count: usize) -> Value {
    let mut input = model(space);
    input["nodes"].as_array_mut().unwrap().truncate(3);
    input["nodes"][2]["x"] = json!(2.0);
    input["nodes"][2]["y"] = json!(0.0);
    input["nodes"][2]["fix_x"] = json!(false);
    input["elements"][1]["node_i"] = json!(1);
    input["elements"][1]["node_j"] = json!(2);
    input["mode_count"] = json!(count);
    for (index, modulus) in stiffness.into_iter().enumerate() {
        input["elements"][index]["youngs_modulus"] = json!(modulus);
        input["elements"][index]["density"] = json!(density);
        if space {
            input["elements"][index]["shear_modulus"] = json!(0.4 * modulus);
        }
    }
    input
}

#[test]
fn headless_modal_axial_fast_path_agrees_with_complete_spectrum_at_extreme_scales() {
    for space in [false, true] {
        for (stiffness, density) in [
            ([5e307; 2], 1.0),
            ([1e-318, 2e-318], 1e-318),
            ([1e-318, 1e-318 + 2.0 * f64::from_bits(1)], 1e-318),
        ] {
            let full =
                planned_solve(space, axial_chain_model(space, stiffness, density, 2)).unwrap();
            let single =
                planned_solve(space, axial_chain_model(space, stiffness, density, 1)).unwrap();
            assert_eq!(full["modes"].as_array().unwrap().len(), 2);
            assert_eq!(single["modes"].as_array().unwrap().len(), 1);
            for key in [
                "eigenvalue_rad_s_squared",
                "natural_frequency_rad_s",
                "natural_frequency_hz",
                "period_s",
                "participation_norm",
            ] {
                let (a, b) = (
                    single["modes"][0][key].as_f64().unwrap(),
                    full["modes"][0][key].as_f64().unwrap(),
                );
                assert!(a.is_finite() && a > 0.0, "{key}: {a}");
                assert!((a / b - 1.0).abs() < 1e-10, "{key}: {a} != {b}");
            }
        }
    }
}

#[test]
fn headless_modal_unrepresentable_axial_frequency_fails_and_replays() {
    for space in [false, true] {
        let error =
            planned_solve(space, axial_chain_model(space, [1e-200; 2], 1e200, 1)).unwrap_err();
        assert!(
            error.contains("eigenvalue") && error.contains("representable"),
            "{error}"
        );
        let replay = planned_solve(space, axial_chain_model(space, [5e307; 2], 1.0, 1)).unwrap();
        let expected = 5e307 * (2.0 - 2.0_f64.sqrt());
        let actual = replay["modes"][0]["eigenvalue_rad_s_squared"]
            .as_f64()
            .unwrap();
        assert!((actual / expected - 1.0).abs() < 1e-12);
    }
}

#[test]
fn headless_nonuniform_chain_retains_low_frequency_and_small_shape_components() {
    for space in [false, true] {
        for soft in [1e-24, 1e-100, 1e-200] {
            let result =
                planned_solve(space, axial_chain_model(space, [1.0, soft], 1.0, 1)).unwrap();
            let mode = &result["modes"][0];
            let actual = mode["eigenvalue_rad_s_squared"].as_f64().unwrap();
            assert!((actual / (2.0 * soft) - 1.0).abs() < 1e-12);
            let dofs = if space { 6 } else { 3 };
            let middle = mode["shape"][dofs].as_f64().unwrap();
            assert!((middle.abs() / soft - 1.0).abs() < 1e-12);
            assert!((mode["participation_norm"].as_f64().unwrap() - 1.0).abs() < 1e-12);
        }
    }
}

#[test]
fn headless_general_tridiagonal_iteration_cancels_and_replays() {
    use kyuubiki_solver::solver_control::{SolverControl, SolverStage, with_solver_observer};
    for space in [false, true] {
        let control = SolverControl::default();
        let cancel = control.clone();
        let error = with_solver_observer(
            &control,
            move |point| {
                if point.stage == SolverStage::ModalIteration && point.completed_steps == 1 {
                    cancel.request_cancel();
                }
            },
            || {
                let result = planned_solve(space, axial_chain_model(space, [1.0, 1e-100], 1.0, 1));
                assert!(
                    result.is_err(),
                    "Engine must not publish a cancelled tridiagonal mode"
                );
                result
            },
        )
        .unwrap_err();
        assert!(error.contains("cancel"), "{error}");
        assert_eq!(
            control.last_checkpoint().unwrap().stage,
            SolverStage::ModalIteration
        );
        let result = planned_solve(space, axial_chain_model(space, [1.0, 1e-100], 1.0, 1)).unwrap();
        let actual = result["modes"][0]["eigenvalue_rad_s_squared"]
            .as_f64()
            .unwrap();
        assert!((actual / 2e-100 - 1.0).abs() < 1e-12);
    }
}

#[test]
fn headless_complete_soft_chain_spectrum_matches_single_mode_and_analytic_shape() {
    for space in [false, true] {
        for soft in [1e-24, 1e-100, 1e-200, 1e-300] {
            let full = planned_solve(space, axial_chain_model(space, [1.0, soft], 1.0, 2)).unwrap();
            let single =
                planned_solve(space, axial_chain_model(space, [1.0, soft], 1.0, 1)).unwrap();
            assert_eq!(full["modes"].as_array().unwrap().len(), 2);
            let low = full["modes"][0]["eigenvalue_rad_s_squared"]
                .as_f64()
                .unwrap();
            let first = single["modes"][0]["eigenvalue_rad_s_squared"]
                .as_f64()
                .unwrap();
            let high = full["modes"][1]["eigenvalue_rad_s_squared"]
                .as_f64()
                .unwrap();
            assert!((low / (2.0 * soft) - 1.0).abs() < 1e-12);
            assert!((low / first - 1.0).abs() < 1e-12);
            assert!((high - 1.0).abs() < 1e-12);
            let dofs = if space { 6 } else { 3 };
            let middle = full["modes"][0]["shape"][dofs].as_f64().unwrap();
            let high_tip = full["modes"][1]["shape"][2 * dofs].as_f64().unwrap();
            assert!((middle.abs() / soft - 1.0).abs() < 1e-12);
            assert!((high_tip.abs() / (2.0 * soft) - 1.0).abs() < 1e-12);
        }
    }
}

#[test]
fn headless_complete_soft_spectrum_rotation_cancels_and_replays() {
    use kyuubiki_solver::solver_control::{SolverControl, SolverStage, with_solver_observer};
    for space in [false, true] {
        let control = SolverControl::default();
        let cancel = control.clone();
        let error = with_solver_observer(
            &control,
            move |point| {
                if point.stage == SolverStage::ModalSweep && point.completed_steps == 1 {
                    cancel.request_cancel();
                }
            },
            || {
                let result = planned_solve(space, axial_chain_model(space, [1.0, 1e-100], 1.0, 2));
                assert!(
                    result.is_err(),
                    "Engine must not publish a cancelled complete spectrum"
                );
                result
            },
        )
        .unwrap_err();
        assert!(error.contains("cancel"), "{error}");
        assert_eq!(control.last_checkpoint().unwrap().completed_steps, 1);
        let replay = planned_solve(space, axial_chain_model(space, [1.0, 1e-100], 1.0, 2)).unwrap();
        let low = replay["modes"][0]["eigenvalue_rad_s_squared"]
            .as_f64()
            .unwrap();
        assert!((low / 2e-100 - 1.0).abs() < 1e-12);
    }
}
