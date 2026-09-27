use kyuubiki_solver::solver_control::{SolverControl, SolverStage, with_solver_observer};
use kyuubiki_solver::{solve_thermal_truss_2d, solve_thermal_truss_3d};
use serde_json::{Value, json};

fn model(spatial: bool, modulus: f64, temperature: f64, expansion: f64) -> Value {
    let nodes: Vec<_> = (0..3)
        .map(|index| {
            let mut node = json!({
                "id": format!("n{index}"), "x": index as f64, "y": 0.0,
                "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0,
                "temperature_delta": temperature
            });
            if spatial {
                node["z"] = json!(0.0);
                node["fix_z"] = json!(true);
                node["load_z"] = json!(0.0);
            }
            node
        })
        .collect();
    let elements: Vec<_> = (0..2)
        .map(|index| {
            json!({
                "id": format!("member-{index}"), "node_i": index, "node_j": index + 1,
                "area": 1.0, "youngs_modulus": modulus, "thermal_expansion": expansion
            })
        })
        .collect();
    json!({"nodes": nodes, "elements": elements})
}

fn solve(spatial: bool, input: Value) -> Result<Value, String> {
    if spatial {
        solve_thermal_truss_3d(&serde_json::from_value(input).unwrap())
            .map(|result| serde_json::to_value(result).unwrap())
    } else {
        solve_thermal_truss_2d(&serde_json::from_value(input).unwrap())
            .map(|result| serde_json::to_value(result).unwrap())
    }
}

fn rescale(input: &mut Value, length: f64, area: f64) {
    for (index, node) in input["nodes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .enumerate()
    {
        node["x"] = json!(index as f64 * length);
    }
    for element in input["elements"].as_array_mut().unwrap() {
        element["area"] = json!(area);
    }
}

fn release_axial_supports(input: &mut Value) {
    input["nodes"][1]["fix_x"] = json!(false);
    input["nodes"][2]["fix_x"] = json!(false);
}

fn close(actual: &Value, expected: f64, tolerance: f64) {
    let actual = actual.as_f64().expect("physical field must remain numeric");
    assert!(actual.is_finite());
    if expected == 0.0 {
        assert_eq!(actual, 0.0);
    } else {
        assert!(
            (actual / expected - 1.0).abs() < tolerance,
            "{actual:e} != {expected:e}"
        );
    }
}

fn finite_json(value: &Value) {
    match value {
        Value::Null => panic!("nonfinite result serialized as null"),
        Value::Array(items) => items.iter().for_each(finite_json),
        Value::Object(items) => items.values().for_each(finite_json),
        _ => {}
    }
}

#[test]
fn restrained_truss_closed_form_survives_connectivity_reversal() {
    for spatial in [false, true] {
        for reverse in [false, true] {
            let mut input = model(spatial, 1200.0, 50.0, 1e-5);
            if reverse {
                for element in input["elements"].as_array_mut().unwrap() {
                    let start = element["node_i"].clone();
                    element["node_i"] = element["node_j"].clone();
                    element["node_j"] = start;
                }
            }
            let result = solve(spatial, input).unwrap();
            finite_json(&result);
            for element in result["elements"].as_array().unwrap() {
                close(&element["thermal_strain"], 0.0005, 1e-12);
                close(&element["mechanical_strain"], -0.0005, 1e-12);
                close(&element["total_strain"], 0.0, 0.0);
                close(&element["stress"], -0.6, 1e-12);
                close(&element["strain_energy_density"], 0.00015, 1e-12);
            }
            close(&result["total_strain_energy"], 0.0003, 1e-12);
        }
    }
}

#[test]
fn thermal_truss_temperature_mean_preserves_finite_extremes() {
    // Synthetic arithmetic envelopes, not plausible material temperatures.
    for spatial in [false, true] {
        for temperature in [f64::MAX, -f64::MAX, f64::from_bits(1)] {
            let result = solve(spatial, model(spatial, 1.0, temperature, 0.0)).unwrap();
            finite_json(&result);
            close(
                &result["elements"][0]["average_temperature_delta"],
                temperature,
                1e-14,
            );
            close(&result["total_strain_energy"], 0.0, 0.0);
        }
    }
}

#[test]
fn thermal_truss_nonfinite_state_or_energy_fails_then_valid_input_replays() {
    for spatial in [false, true] {
        for (expansion, area) in [(1e155, 1.0), (1e100, 1e120), (1e-100, 1e-200)] {
            let mut input = model(spatial, 1.0, 1.0, expansion);
            rescale(&mut input, 1.0, area);
            let error = solve(spatial, input).unwrap_err();
            assert!(
                error.contains("member-0") && error.contains("representable"),
                "{error}"
            );
            finite_json(&solve(spatial, model(spatial, 1200.0, 50.0, 1e-5)).unwrap());
        }
    }
}

#[test]
fn thermal_truss_volume_products_preserve_representable_energy() {
    for spatial in [false, true] {
        for (modulus, expansion, length, area, expected) in [
            (1e100, 1e100, 1e-5, 1e10, 1e305),
            (1e-100, 1e-100, 1e50, 1e-40, 1e-290),
        ] {
            let mut input = model(spatial, modulus, 1.0, expansion);
            rescale(&mut input, length, area);
            let result = solve(spatial, input).unwrap();
            finite_json(&result);
            close(&result["total_strain_energy"], expected, 1e-12);
        }
    }
}

#[test]
fn thermal_truss_energy_sum_overflow_is_not_success() {
    for spatial in [false, true] {
        let mut input = model(spatial, 8e307, 1.0, 1.0);
        rescale(&mut input, 2.0, 2.0);
        let error = solve(spatial, input).unwrap_err();
        assert!(
            error.contains("total strain energy") && error.contains("representable"),
            "{error}"
        );
    }
}

#[test]
fn thermal_truss_tiny_displacements_retain_nonzero_norms() {
    for spatial in [false, true] {
        let mut input = model(spatial, 1e200, 1.0, 1e-200);
        release_axial_supports(&mut input);
        let result = solve(spatial, input).unwrap();
        finite_json(&result);
        close(&result["nodes"][2]["ux"], 2e-200, 1e-12);
        close(&result["max_displacement"], 2e-200, 1e-12);
    }
}

#[test]
fn thermal_truss_long_finite_members_do_not_overflow_the_length_norm() {
    for spatial in [false, true] {
        let mut input = model(spatial, 1.0, 1.0, 1e-3);
        rescale(&mut input, 1e200, 1.0);
        let result = solve(spatial, input).unwrap();
        finite_json(&result);
        close(&result["elements"][0]["length"], 1e200, 1e-12);
        close(&result["total_strain_energy"], 1e194, 1e-12);
    }
}

#[test]
fn thermal_truss_small_deformation_guard_is_translation_and_scale_objective() {
    for spatial in [false, true] {
        for length in [1.0, 1e-3] {
            for shift in [0.0, 1e6, -1e6] {
                for load in [0.01, 1.0] {
                    let mut input = model(spatial, 1.0, 0.0, 0.0);
                    rescale(&mut input, length, 1.0);
                    for node in input["nodes"].as_array_mut().unwrap() {
                        node["x"] = json!(node["x"].as_f64().unwrap() + shift);
                    }
                    release_axial_supports(&mut input);
                    input["nodes"][2]["load_x"] = json!(load);
                    let result = solve(spatial, input);
                    if load < 0.25 {
                        close(
                            &result.unwrap()["max_displacement"],
                            2.0 * length * load,
                            1e-6,
                        );
                    } else {
                        let error = result.unwrap_err();
                        assert!(error.contains("small-deformation limit"), "{error}");
                    }
                }
            }
        }
    }
}

#[test]
fn thermal_truss_fixed_supports_cannot_hide_invalid_coefficients() {
    for spatial in [false, true] {
        for (modulus, area, expansion, context) in [
            (1e200, 1e200, 0.0, "stiffness"),
            (1e-200, 1e-200, 0.0, "stiffness"),
            (1.0, 1e200, 1e150, "thermal load"),
        ] {
            let mut input = model(spatial, modulus, 1.0, expansion);
            rescale(&mut input, 1.0, area);
            let error = solve(spatial, input).unwrap_err();
            assert!(
                error.contains("member-0") && error.contains(context),
                "{error}"
            );
        }
    }
}

#[test]
fn thermal_truss_fixed_supports_cannot_hide_assembled_load_overflow() {
    for spatial in [false, true] {
        let mut input = model(spatial, 1e308, 1.0, 1.0);
        input["elements"][1]["node_i"] = json!(0);
        let error = solve(spatial, input).unwrap_err();
        assert!(
            error.contains("member-1") && error.contains("assembled thermal load"),
            "{error}"
        );
    }
}

#[test]
fn thermal_truss_load_uses_the_same_scaled_thermal_strain_as_recovery() {
    for spatial in [false, true] {
        for (modulus, temperature, expansion) in [(1e-200, 1e200, 1e-203), (1e200, 1e-203, 1e200)] {
            let mut input = model(spatial, modulus, temperature, expansion);
            release_axial_supports(&mut input);
            let result = solve(spatial, input).unwrap();
            finite_json(&result);
            close(&result["max_displacement"], 0.002, 1e-12);
            for element in result["elements"].as_array().unwrap() {
                close(&element["thermal_strain"], 0.001, 1e-12);
                assert!(element["mechanical_strain"].as_f64().unwrap().abs() < 1e-15);
            }
        }
    }
}

#[test]
fn thermal_truss_fixed_supports_cannot_hide_assembled_stiffness_overflow() {
    for spatial in [false, true] {
        let error = solve(spatial, model(spatial, 1e308, 0.0, 0.0)).unwrap_err();
        assert!(
            error.contains("assembled stiffness") && error.contains("representable"),
            "{error}"
        );
    }
}

#[test]
fn thermal_truss_result_stages_cancel_before_success_and_allow_replay() {
    for spatial in [false, true] {
        let run = || solve(spatial, model(spatial, 1200.0, 50.0, 1e-5));
        let expected = run().unwrap();
        for stage in [
            SolverStage::ResultNodes,
            SolverStage::ResultElements,
            SolverStage::ResultTotals,
        ] {
            for terminal in [false, true] {
                let control = SolverControl::default();
                let cancel = control.clone();
                let result = with_solver_observer(
                    &control,
                    move |point| {
                        if point.stage == stage && (point.completed_steps > 0) == terminal {
                            cancel.request_cancel();
                        }
                    },
                    || {
                        let raw = run();
                        assert!(raw.is_err(), "raw solver ignored {stage:?}");
                        raw
                    },
                );
                assert!(result.unwrap_err().starts_with("solver cancelled"));
                assert!(control.was_interrupted());
                assert_eq!(run().unwrap(), expected);
            }
        }
    }
}

#[test]
fn thermal_truss_assembly_and_recovery_cancel_at_chunk_boundaries() {
    for spatial in [false, true] {
        let mut input = model(spatial, 1200.0, 50.0, 1e-5);
        input["nodes"] = (0..130)
            .map(|index| {
                let mut node = input["nodes"][0].clone();
                node["id"] = json!(format!("n{index}"));
                node["x"] = json!(index as f64);
                node
            })
            .collect();
        input["elements"] = (0..129)
            .map(|index| {
                let mut element = input["elements"][0].clone();
                element["id"] = json!(format!("member-{index}"));
                element["node_i"] = json!(index);
                element["node_j"] = json!(index + 1);
                element
            })
            .collect();
        let run = || solve(spatial, input.clone());
        let expected = run().unwrap();
        for stage in [
            SolverStage::ElementAssembly,
            SolverStage::SparseValidateMatrix,
            SolverStage::ResultNodes,
            SolverStage::ResultElements,
            SolverStage::ResultTotals,
        ] {
            let control = SolverControl::default();
            let cancel = control.clone();
            let result = with_solver_observer(
                &control,
                move |point| {
                    if point.stage == stage && point.completed_steps == 64 {
                        cancel.request_cancel();
                    }
                },
                || {
                    let raw = run();
                    assert!(raw.is_err(), "raw solver ignored {stage:?} at 64");
                    raw
                },
            );
            assert!(result.unwrap_err().starts_with("solver cancelled"));
            assert!(control.was_interrupted());
            assert_eq!(run().unwrap(), expected);
        }
    }
}
