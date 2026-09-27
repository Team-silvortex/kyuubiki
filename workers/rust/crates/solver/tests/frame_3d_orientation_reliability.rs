use kyuubiki_solver::{
    solve_frame_3d, solve_frame_3d_owned, solve_modal_frame_3d, solve_thermal_frame_3d,
    solve_thermal_frame_3d_owned,
};
use serde_json::{Value, json};

fn rotate([x, y, z]: [f64; 3]) -> [f64; 3] {
    let c = std::f64::consts::FRAC_1_SQRT_2;
    [c * (x - y), c * (x + y), z]
}

fn model(thermal: bool, loaded: bool) -> Value {
    let nodes: Vec<_> = (0..2)
        .map(|i| {
            let mut n = json!({"id": format!("node-{i}"), "x": i as f64, "y": i as f64, "z": 0.0});
            for field in ["fix_x", "fix_y", "fix_z", "fix_rx", "fix_ry", "fix_rz"] {
                n[field] = json!(i == 0);
            }
            for (fields, local_load) in [
                (["load_x", "load_y", "load_z"], [300.0, -1000.0, 650.0]),
                (["moment_x", "moment_y", "moment_z"], [120.0, -90.0, 75.0]),
            ] {
                for (field, value) in fields.into_iter().zip(rotate(local_load)) {
                    n[field] = json!(if i == 1 && loaded { value } else { 0.0 });
                }
            }
            if thermal {
                n["temperature_delta"] = json!(100.0);
            }
            n
        })
        .collect();
    let mut member = json!({"id": "member", "node_i": 0, "node_j": 1,
        "local_y_axis": [-1.0, 1.0, 0.0], "youngs_modulus": 200e9, "area": 0.01,
        "shear_modulus": 80e9, "torsion_constant": 4e-6,
        "moment_of_inertia_y": 2e-6, "moment_of_inertia_z": 7e-6,
        "section_modulus_y": 1e-4, "section_modulus_z": 2e-4});
    if thermal {
        for (field, value) in [
            ("thermal_expansion", 1e-5),
            ("section_depth_y", 0.2),
            ("section_depth_z", 0.3),
            ("temperature_gradient_y", 0.0),
            ("temperature_gradient_z", 0.0),
        ] {
            member[field] = json!(value);
        }
    }
    json!({"nodes": nodes, "elements": [member]})
}

fn solve(thermal: bool, input: Value, owned: bool) -> Result<Value, String> {
    if thermal {
        let request = serde_json::from_value(input).unwrap();
        if owned {
            solve_thermal_frame_3d_owned(request)
        } else {
            solve_thermal_frame_3d(&request)
        }
        .map(|result| serde_json::to_value(result).unwrap())
    } else {
        let request = serde_json::from_value(input).unwrap();
        if owned {
            solve_frame_3d_owned(request)
        } else {
            solve_frame_3d(&request)
        }
        .map(|result| serde_json::to_value(result).unwrap())
    }
}

fn close(actual: &Value, expected: f64, tolerance: f64) {
    let actual = actual
        .as_f64()
        .expect("physical output must remain numeric");
    assert!(
        actual.is_finite() && (actual - expected).abs() <= tolerance,
        "actual={actual:e}, expected={expected:e}, tolerance={tolerance:e}"
    );
}

fn compare(actual: &Value, expected: &Value) {
    for (a, e) in actual["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .zip(expected["nodes"].as_array().unwrap())
    {
        for field in ["ux", "uy", "uz", "rx", "ry", "rz"] {
            close(&a[field], e[field].as_f64().unwrap(), 1e-12);
        }
    }
    assert_eq!(
        actual["elements"].as_array().unwrap().len(),
        expected["elements"].as_array().unwrap().len()
    );
    for (index, element) in expected["elements"].as_array().unwrap().iter().enumerate() {
        for (key, e) in element.as_object().unwrap() {
            if let Some(value) = e.as_f64() {
                close(
                    &actual["elements"][index][key],
                    value,
                    1e-9 * value.abs().max(1.0),
                );
            }
        }
    }
    for field in [
        "max_displacement",
        "max_rotation",
        "max_moment",
        "max_stress",
        "total_strain_energy",
    ] {
        let value = expected[field].as_f64().unwrap();
        close(&actual[field], value, 1e-9 * value.abs().max(1e-10));
    }
}

#[test]
fn mechanical_section_hint_axial_component_cannot_change_response() {
    let input = model(false, true);
    let expected = solve(false, input.clone(), false).unwrap();
    for epsilon in [1e-3, 1e-6, 1e-9, 1e-11, 4e-12] {
        let mut alternative = input.clone();
        alternative["elements"][0]["local_y_axis"] = json!([1.0, 1.0 + epsilon, 0.0]);
        let actual = solve(false, alternative.clone(), false).unwrap();
        compare(&actual, &expected);
        assert_eq!(actual, solve(false, alternative, true).unwrap());
    }
}

#[test]
fn thermal_section_hint_axial_component_cannot_change_coupled_response() {
    let mut input = model(true, true);
    input["elements"][0]["temperature_gradient_y"] = json!(35.0);
    input["elements"][0]["temperature_gradient_z"] = json!(-20.0);
    let expected = solve(true, input.clone(), false).unwrap();
    for epsilon in [1e-3, 1e-6, 1e-9, 1e-11, 4e-12] {
        let mut alternative = input.clone();
        alternative["elements"][0]["local_y_axis"] = json!([1.0, 1.0 + epsilon, 0.0]);
        let actual = solve(true, alternative.clone(), false).unwrap();
        compare(&actual, &expected);
        assert_eq!(actual, solve(true, alternative, true).unwrap());
    }
}

#[test]
fn free_uniform_thermal_expansion_remains_parallel_to_the_member() {
    for epsilon in [1e-3, 1e-6, 1e-9, 1e-11, 4e-12] {
        let mut input = model(true, false);
        input["elements"][0]["local_y_axis"] = json!([1.0, 1.0 + epsilon, 0.0]);
        let result = solve(true, input, false).unwrap();
        for field in ["ux", "uy"] {
            close(&result["nodes"][1][field], 0.001, 2e-14);
        }
        for field in ["uz", "rx", "ry", "rz"] {
            close(&result["nodes"][1][field], 0.0, 2e-14);
        }
        close(&result["total_strain_energy"], 0.0, 1e-18);
    }
}

#[test]
fn parallel_and_zero_section_hints_reject_and_valid_models_replay() {
    for thermal in [false, true] {
        for axis in [[1.0, 1.0, 0.0], [1.0, 1.0 + 1e-13, 0.0], [0.0; 3]] {
            let mut input = model(thermal, true);
            input["elements"][0]["local_y_axis"] = json!(axis);
            let error = solve(thermal, input, false).unwrap_err();
            assert!(
                error.contains("local_y_axis must not be parallel"),
                "{error}"
            );
            solve(thermal, model(thermal, true), false).unwrap();
        }
    }
}

#[test]
fn equivalent_section_hints_preserve_refined_frame_fields() {
    for thermal in [false, true] {
        for segments in [2, 4, 8] {
            let mut input = model(thermal, true);
            let root = input["nodes"][0].clone();
            let tip = input["nodes"][1].clone();
            input["nodes"] = json!(
                (0..=segments)
                    .map(|i| {
                        let mut n = if i == segments {
                            tip.clone()
                        } else {
                            root.clone()
                        };
                        n["id"] = json!(format!("n{i}"));
                        n["x"] = json!(i as f64 / segments as f64);
                        n["y"] = n["x"].clone();
                        for field in ["fix_x", "fix_y", "fix_z", "fix_rx", "fix_ry", "fix_rz"] {
                            n[field] = json!(i == 0);
                        }
                        n
                    })
                    .collect::<Vec<_>>()
            );
            let element = input["elements"][0].clone();
            input["elements"] = json!(
                (0..segments)
                    .map(|i| {
                        let mut e = element.clone();
                        e["id"] = json!(format!("member-{i}"));
                        e["node_i"] = json!(i);
                        e["node_j"] = json!(i + 1);
                        e
                    })
                    .collect::<Vec<_>>()
            );
            let expected = solve(thermal, input.clone(), false).unwrap();
            for e in input["elements"].as_array_mut().unwrap() {
                e["local_y_axis"] = json!([1.0, 1.0 + 1e-10, 0.0]);
            }
            compare(&solve(thermal, input, false).unwrap(), &expected);
        }
    }
}

#[test]
fn reversed_section_hint_preserves_physics_when_local_thermal_gradients_follow_it() {
    for thermal in [false, true] {
        let mut input = model(thermal, true);
        if thermal {
            input["elements"][0]["temperature_gradient_y"] = json!(35.0);
            input["elements"][0]["temperature_gradient_z"] = json!(-20.0);
        }
        let expected = solve(thermal, input.clone(), false).unwrap();
        input["elements"][0]["local_y_axis"] = json!([-1.0, -(1.0 + 1e-10), 0.0]);
        if thermal {
            input["elements"][0]["temperature_gradient_y"] = json!(-35.0);
            input["elements"][0]["temperature_gradient_z"] = json!(20.0);
        }
        let mut actual = solve(thermal, input, false).unwrap();
        // Flipping local y also flips local z. Convert signed local output back for comparison.
        for field in [
            "shear_force_y_i",
            "shear_force_z_i",
            "shear_force_y_j",
            "shear_force_z_j",
            "moment_y_i",
            "moment_z_i",
            "moment_y_j",
            "moment_z_j",
            "temperature_gradient_y",
            "temperature_gradient_z",
            "thermal_curvature_y",
            "thermal_curvature_z",
        ] {
            if let Some(value) = actual["elements"][0][field].as_f64() {
                actual["elements"][0][field] = json!(-value);
            }
        }
        compare(&actual, &expected);
    }
}

#[test]
fn implicit_modal_axes_preserve_the_free_cantilever_spectrum_under_rotation() {
    let mut input = model(false, false);
    input["elements"][0]
        .as_object_mut()
        .unwrap()
        .remove("local_y_axis");
    input["elements"][0]["density"] = json!(7850.0);
    input["mode_count"] = json!(6);
    let solve_modal = |input: Value| {
        let request = serde_json::from_value(input).unwrap();
        solve_modal_frame_3d(&request).unwrap()
    };
    let expected = solve_modal(input.clone());
    assert_eq!(expected.modes.len(), 6);
    for direction in [
        [0.36, 0.48, -0.8],
        [0.01, 0.0, 1.0],
        [0.0, 0.0, -1.0],
        [1.0, 0.0, 0.0],
    ] {
        let norm = f64::hypot(f64::hypot(direction[0], direction[1]), direction[2]);
        for (field, value) in ["x", "y", "z"].into_iter().zip(direction) {
            input["nodes"][1][field] = json!(value / norm * 2.0_f64.sqrt());
        }
        let actual = solve_modal(input.clone());
        assert_eq!(actual.modes.len(), expected.modes.len());
        assert_eq!(actual.free_dofs, expected.free_dofs);
        for (a, e) in actual.modes.iter().zip(&expected.modes) {
            assert!((a.natural_frequency_hz / e.natural_frequency_hz - 1.0).abs() < 2e-8);
            assert!((a.participation_norm - 1.0).abs() < 1e-10);
        }
        assert!((actual.total_mass / expected.total_mass - 1.0).abs() < 1e-14);
    }
}
