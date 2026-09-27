use kyuubiki_solver::solver_control::{SolverControl, SolverStage, with_solver_observer};
use kyuubiki_solver::{
    profile_thermal_plane_quad_2d_with_options, profile_thermal_plane_triangle_2d_with_options,
    solve_thermal_plane_quad_2d, solve_thermal_plane_triangle_2d,
};
use serde_json::{Value, json};

fn model(quad: bool, modulus: f64, temperature: f64, expansion: f64) -> Value {
    let points = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    let nodes: Vec<_> = points[..if quad { 4 } else { 3 }]
        .iter()
        .enumerate()
        .map(|(i, [x, y])| {
            json!({
                "id":format!("n{i}"), "x":x, "y":y,
                "fix_x":true, "fix_y":true, "load_x":0.0, "load_y":0.0,
                "temperature_delta":temperature
            })
        })
        .collect();
    let mut element = json!({"id":"thermal-bulk", "node_i":0, "node_j":1, "node_k":2,
        "thickness":1.0, "youngs_modulus":modulus, "poisson_ratio":0.25,
        "thermal_expansion":expansion});
    if quad {
        element["node_l"] = json!(3);
    }
    json!({"nodes":nodes, "elements":[element]})
}

fn rescale(input: &mut Value, length: f64, thickness: f64) {
    for node in input["nodes"].as_array_mut().unwrap() {
        node["x"] = json!(node["x"].as_f64().unwrap() * length);
        node["y"] = json!(node["y"].as_f64().unwrap() * length);
    }
    input["elements"][0]["thickness"] = json!(thickness);
}

fn solve(quad: bool, input: Value) -> Result<Value, String> {
    if quad {
        solve_thermal_plane_quad_2d(&serde_json::from_value(input).unwrap())
            .map(|r| serde_json::to_value(r).unwrap())
    } else {
        solve_thermal_plane_triangle_2d(&serde_json::from_value(input).unwrap())
            .map(|r| serde_json::to_value(r).unwrap())
    }
}

fn numeric_output(value: &Value) {
    match value {
        Value::Null => panic!("nonfinite physical output was serialized as null"),
        Value::Array(values) => values.iter().for_each(numeric_output),
        Value::Object(values) => values.values().for_each(numeric_output),
        _ => {}
    }
}

fn close(actual: &Value, expected: f64, tolerance: f64) {
    let actual = actual.as_f64().expect("physical output must be numeric");
    assert!(actual.is_finite(), "{actual}");
    if expected == 0.0 {
        assert_eq!(actual, 0.0);
    } else {
        assert!(
            (actual / expected - 1.0).abs() < tolerance,
            "actual={actual:e} expected={expected:e}"
        );
    }
}

#[test]
fn thermal_uniform_temperature_means_preserve_extreme_and_subnormal_values() {
    // Arithmetic envelopes, not physically meaningful temperature ranges.
    for quad in [false, true] {
        for temperature in [f64::MAX, -f64::MAX, f64::from_bits(1)] {
            let result = solve(quad, model(quad, 1200.0, temperature, 0.0)).unwrap();
            numeric_output(&result);
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
fn thermal_triangle_temperature_cancellation_is_independent_of_node_order() {
    for order in [
        [0, 1, 2],
        [1, 2, 0],
        [2, 0, 1],
        [0, 2, 1],
        [2, 1, 0],
        [1, 0, 2],
    ] {
        let mut input = model(false, 1200.0, 0.0, 1e-5);
        for (node, temperature) in input["nodes"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .zip([1e16, 1.0, -1e16])
        {
            node["temperature_delta"] = json!(temperature);
        }
        for (key, index) in ["node_i", "node_j", "node_k"].into_iter().zip(order) {
            input["elements"][0][key] = json!(index);
        }
        let result = solve(false, input).unwrap();
        close(
            &result["elements"][0]["average_temperature_delta"],
            1.0 / 3.0,
            1e-14,
        );
        close(
            &result["elements"][0]["stress_x"],
            -1200.0 * 1e-5 / 3.0 / 0.75,
            1e-12,
        );
    }
}

#[test]
fn thermal_area_weighted_means_do_not_overflow_before_normalization() {
    for quad in [false, true] {
        let mut input = model(quad, 1200.0, 1e200, 1e-203);
        rescale(&mut input, 1e60, 1e-120);
        let result = solve(quad, input).unwrap();
        numeric_output(&result);
        close(
            &result["elements"][0]["average_temperature_delta"],
            1e200,
            1e-12,
        );
        close(&result["elements"][0]["stress_x"], -1.6, 1e-12);
        close(
            &result["total_strain_energy"],
            0.0016 * if quad { 1.0 } else { 0.5 },
            1e-12,
        );
    }
}

#[test]
fn thermal_energy_volume_product_preserves_representable_results() {
    for quad in [false, true] {
        for (modulus, length, thickness, tolerance) in
            [(1.2e203, 1e100, 1e-200, 1e-12), (1e-314, 1e-3, 1e6, 0.02)]
        {
            let mut input = model(quad, modulus, 50.0, 1e-5);
            rescale(&mut input, length, thickness);
            let result = solve(quad, input).unwrap();
            numeric_output(&result);
            let density = ((modulus / 0.75) * 0.0005) * 0.0005;
            close(
                &result["elements"][0]["strain_energy_density"],
                density,
                tolerance,
            );
            close(
                &result["total_strain_energy"],
                density * if quad { 1.0 } else { 0.5 },
                tolerance,
            );
        }
    }
}

#[test]
fn thermal_unrepresentable_state_or_total_energy_fails_and_replays_cleanly() {
    for quad in [false, true] {
        // Deliberately outside small-strain validity: rejection tests only.
        for (expansion, length, thickness) in
            [(1e155, 1.0, 1.0), (1e100, 1e60, 1.0), (1e-100, 1.0, 1e-200)]
        {
            let mut input = model(quad, 1.0, 1.0, expansion);
            rescale(&mut input, length, thickness);
            let error = solve(quad, input).unwrap_err();
            assert!(
                error.contains("thermal") && error.contains("representable"),
                "{error}"
            );
            let replay = solve(quad, model(quad, 1200.0, 50.0, 1e-5)).unwrap();
            numeric_output(&replay);
        }
    }
}

#[test]
fn thermal_tiny_displacements_keep_nonzero_magnitudes() {
    for quad in [false, true] {
        let mut input = model(quad, 1e200, 1.0, 1e-200);
        for node in input["nodes"].as_array_mut().unwrap() {
            node["fix_x"] = json!(node["id"] == "n0");
            node["fix_y"] = json!(node["id"] == "n0" || node["id"] == "n1");
        }
        let result = solve(quad, input).unwrap();
        numeric_output(&result);
        close(&result["nodes"][2]["ux"], 1e-200, 1e-12);
        close(&result["nodes"][2]["uy"], 1e-200, 1e-12);
        close(&result["max_displacement"], 2f64.sqrt() * 1e-200, 1e-12);
    }
}

#[test]
fn thermal_result_stages_cancel_before_returning_success_and_allow_replay() {
    for quad in [false, true] {
        let run = || solve(quad, model(quad, 1200.0, 50.0, 1e-5));
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
fn thermal_profile_rhs_norm_preserves_representable_large_and_small_forces() {
    for quad in [false, true] {
        for modulus in [1e-200, 1e200] {
            let mut input = model(quad, modulus, 50.0, 1e-5);
            for node in input["nodes"].as_array_mut().unwrap() {
                node["fix_x"] = json!(node["id"] == "n0");
                node["fix_y"] = json!(node["id"] == "n0" || node["id"] == "n1");
            }
            let norm = if quad {
                profile_thermal_plane_quad_2d_with_options(
                    &serde_json::from_value(input).unwrap(),
                    Default::default(),
                )
                .unwrap()
                .solver_rhs_norm
            } else {
                profile_thermal_plane_triangle_2d_with_options(
                    &serde_json::from_value(input).unwrap(),
                    Default::default(),
                )
                .unwrap()
                .solver_rhs_norm
            };
            let expected =
                modulus * 0.0005 / 0.75 * 0.5 * if quad { 5f64.sqrt() } else { 2f64.sqrt() };
            assert!(
                norm.is_finite() && (norm / expected - 1.0).abs() < 1e-12,
                "{norm:e} != {expected:e}"
            );
        }
    }
}

#[test]
fn nonfinite_equivalent_load_cannot_hide_behind_fully_fixed_supports() {
    for quad in [false, true] {
        let error = solve(quad, model(quad, 1200.0, 1.0, f64::MAX)).unwrap_err();
        assert!(
            error.contains("thermal-bulk")
                && error.contains("thermal load")
                && error.contains("representable"),
            "{error}"
        );
        numeric_output(&solve(quad, model(quad, 1200.0, 50.0, 1e-5)).unwrap());
    }
}
