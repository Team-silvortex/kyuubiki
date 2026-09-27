use kyuubiki_solver::solver_control::{SolverControl, SolverStage, with_solver_observer};
use kyuubiki_solver::{
    solve_frame_3d, solve_frame_3d_owned, solve_thermal_frame_3d, solve_thermal_frame_3d_owned,
};
use serde_json::{Value, json};

const L: f64 = 2.0;
const E: f64 = 200e9;
const A: f64 = 0.02;
const G: f64 = 80e9;
const J: f64 = 1e-5;
const IY: f64 = 8e-6;
const IZ: f64 = 5e-6;

fn model(thermal: bool, segments: usize, loads: [f64; 6]) -> Value {
    let nodes: Vec<_> = (0..=segments)
        .map(|i| {
            let mut node = json!({"id": format!("n{i}"), "x": L*i as f64/segments as f64,
            "y": 0.0, "z": 0.0});
            for field in ["fix_x", "fix_y", "fix_z", "fix_rx", "fix_ry", "fix_rz"] {
                node[field] = json!(i == 0);
            }
            for (field, load) in [
                "load_x", "load_y", "load_z", "moment_x", "moment_y", "moment_z",
            ]
            .into_iter()
            .zip(loads)
            {
                node[field] = json!(if i == segments { load } else { 0.0 });
            }
            if thermal {
                node["temperature_delta"] = json!(0.0);
            }
            node
        })
        .collect();
    let elements: Vec<_> = (0..segments)
        .map(|i| {
            let mut e = json!({"id": format!("member-{i}"), "node_i": i, "node_j": i+1,
            "local_y_axis": [0.0, 1.0, 0.0], "youngs_modulus": E, "area": A,
            "shear_modulus": G, "torsion_constant": J, "moment_of_inertia_y": IY,
            "moment_of_inertia_z": IZ, "section_modulus_y": 1.6e-4, "section_modulus_z": 1e-4});
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
            e
        })
        .collect();
    json!({"nodes": nodes, "elements": elements})
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

fn close(actual: &Value, expected: f64, relative: f64) {
    let actual = actual.as_f64().expect("physical output must be numeric");
    assert!(actual.is_finite());
    let tolerance = if expected == 0.0 {
        1e-14
    } else {
        expected.abs() * relative
    };
    assert!(
        (actual - expected).abs() <= tolerance,
        "{actual:e} != {expected:e}"
    );
}

fn heat(input: &mut Value, temperature: f64, gradients: [f64; 2]) {
    for n in input["nodes"].as_array_mut().unwrap() {
        n["temperature_delta"] = json!(temperature);
    }
    for e in input["elements"].as_array_mut().unwrap() {
        e["temperature_gradient_y"] = json!(gradients[0]);
        e["temperature_gradient_z"] = json!(gradients[1]);
    }
}

fn fix_all(input: &mut Value) {
    for n in input["nodes"].as_array_mut().unwrap() {
        for field in ["fix_x", "fix_y", "fix_z", "fix_rx", "fix_ry", "fix_rz"] {
            n[field] = json!(true);
        }
        for field in [
            "load_x", "load_y", "load_z", "moment_x", "moment_y", "moment_z",
        ] {
            n[field] = json!(0.0);
        }
    }
}

fn energy([n, py, pz, t, my, mz]: [f64; 6]) -> f64 {
    let bending = |p: f64, m: f64, ei: f64| {
        (m * m * L + m * p * L * L + p * p * L.powi(3) / 3.0) / (2.0 * ei)
    };
    n * n * L / (2.0 * E * A)
        + t * t * L / (2.0 * G * J)
        + bending(py, mz, E * IZ)
        + bending(-pz, my, E * IY)
}

#[test]
fn spatial_frame_mixed_loads_match_closed_form_and_nodal_work() {
    let loads = [1200.0, 1350.0, -750.0, 230.0, 180.0, 400.0];
    for thermal in [false, true] {
        for segments in [1, 2, 4, 8] {
            let result = solve(thermal, model(thermal, segments, loads), false).unwrap();
            close(&result["total_strain_energy"], energy(loads), 2e-8);
            let work = loads
                .into_iter()
                .zip(["ux", "uy", "uz", "rx", "ry", "rz"])
                .map(|(load, field)| {
                    0.5 * load * result["nodes"][segments][field].as_f64().unwrap()
                })
                .sum::<f64>();
            close(&result["total_strain_energy"], work, 2e-8);
        }
    }
}

#[test]
fn spatial_thermal_background_does_not_erase_small_mechanical_energy() {
    for loads in [
        [0.04, 0.0, 0.0, 0.0, 0.0, 0.0],
        [0.0, 1e-3, -1e-3, 1e-3, 1e-3, 1e-3],
    ] {
        for segments in [1, 2, 4] {
            let mut input = model(true, segments, loads);
            heat(&mut input, 100.0, [40.0, 60.0]);
            let result = solve(true, input, false).unwrap();
            close(&result["total_strain_energy"], energy(loads), 2e-4);
        }
    }
}

#[test]
fn spatial_frame_tiny_displacement_and_rotation_remain_nonzero() {
    for thermal in [false, true] {
        let mut input = model(thermal, 1, [1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
        for field in ["youngs_modulus", "shear_modulus"] {
            input["elements"][0][field] = json!(1e200);
        }
        for field in [
            "area",
            "torsion_constant",
            "moment_of_inertia_y",
            "moment_of_inertia_z",
        ] {
            input["elements"][0][field] = json!(1.0);
        }
        let result = solve(thermal, input, false).unwrap();
        close(&result["max_displacement"], 2e-200, 1e-10);
        close(&result["max_rotation"], 2e-200, 1e-10);
        close(&result["total_strain_energy"], 2e-200, 1e-10);
    }
}

#[test]
fn spatial_thermal_temperature_mean_does_not_overflow_finite_inputs() {
    let mut input = model(true, 1, [0.0; 6]);
    fix_all(&mut input);
    input["elements"][0]["thermal_expansion"] = json!(1e-310);
    heat(&mut input, 1e308, [0.0; 2]);
    let result = solve(true, input, false).unwrap();
    close(
        &result["elements"][0]["average_temperature_delta"],
        1e308,
        1e-10,
    );
    close(
        &result["total_strain_energy"],
        0.5 * E * A * L * 1e-4,
        1e-10,
    );
}

#[test]
fn spatial_frame_fixed_supports_cannot_hide_invalid_rigidity() {
    for thermal in [false, true] {
        let mut input = model(thermal, 1, [0.0; 6]);
        fix_all(&mut input);
        input["elements"][0]["youngs_modulus"] = json!(1e200);
        input["elements"][0]["area"] = json!(1e200);
        assert!(solve(thermal, input, false).is_err());
    }
}

#[test]
fn spatial_directional_spring_is_invariant_under_finite_direction_scaling() {
    for rotational in [false, true] {
        for scale in [1.0, 1e200, 1e308] {
            let mut loads = [0.0; 6];
            loads[if rotational { 3 } else { 0 }] = 10.0;
            let mut input = model(true, 1, loads);
            let k = if rotational { G * J / L } else { E * A / L };
            let (key, field) = if rotational {
                ("directional_rotational_springs", "rx")
            } else {
                ("directional_springs", "ux")
            };
            input[key] = json!([{"id": "support", "node": 1, "direction": [scale, 0.0, 0.0], "stiffness": k}]);
            let result = solve(true, input, false).unwrap();
            close(&result["nodes"][1][field], 10.0 / (2.0 * k), 1e-9);
            close(&result["total_strain_energy"], 25.0 / k, 1e-9);
        }
    }
}

#[test]
fn spatial_frame_result_cancellation_reaches_raw_solver() {
    for thermal in [false, true] {
        let input = model(thermal, 1, [10.0; 6]);
        let expected = solve(thermal, input.clone(), false).unwrap();
        let control = SolverControl::default();
        let cancel = control.clone();
        let result = with_solver_observer(
            &control,
            move |point| {
                if point.stage == SolverStage::ResultElements {
                    cancel.request_cancel();
                }
            },
            || {
                let raw = solve(thermal, input.clone(), false);
                assert!(raw.is_err(), "raw solver ignored result cancellation");
                raw
            },
        );
        assert!(result.unwrap_err().starts_with("solver cancelled"));
        assert_eq!(solve(thermal, input, false).unwrap(), expected);
    }
}

fn reject_and_replay(thermal: bool, input: Value, context: &str) {
    for owned in [false, true] {
        let error = solve(thermal, input.clone(), owned).unwrap_err();
        assert!(
            error.contains(context) && error.contains("representable"),
            "{error}"
        );
        let loads = [1200.0, 1350.0, -750.0, 230.0, 180.0, 400.0];
        let result = solve(thermal, model(thermal, 1, loads), owned).unwrap();
        close(&result["total_strain_energy"], energy(loads), 1e-9);
    }
}

#[test]
fn spatial_frame_full_matrix_overflow_and_rigidity_underflow_are_rejected() {
    for thermal in [false, true] {
        let mut input = model(thermal, 1, [0.0; 6]);
        fix_all(&mut input);
        for field in ["youngs_modulus", "shear_modulus"] {
            input["elements"][0][field] = json!(5e307);
        }
        for field in [
            "area",
            "torsion_constant",
            "moment_of_inertia_y",
            "moment_of_inertia_z",
        ] {
            input["elements"][0][field] = json!(1.0);
        }
        for i in 1..3 {
            let mut duplicate = input["elements"][0].clone();
            duplicate["id"] = json!(format!("parallel-{i}"));
            input["elements"].as_array_mut().unwrap().push(duplicate);
        }
        reject_and_replay(thermal, input, "assembled stiffness");
        for (a, b) in [
            ("youngs_modulus", "area"),
            ("shear_modulus", "torsion_constant"),
        ] {
            let mut input = model(thermal, 1, [0.0; 6]);
            fix_all(&mut input);
            input["elements"][0][a] = json!(1e-200);
            input["elements"][0][b] = json!(1e-200);
            reject_and_replay(thermal, input, "stiffness");
        }
    }
}

#[test]
fn spatial_thermal_nonfinite_loads_are_rejected_before_constraint_projection() {
    for (modulus, temperature, context) in [
        (E, 1e308, "equivalent load"),
        (1e200, 1e108, "assembled load"),
    ] {
        let mut input = model(true, 1, [0.0; 6]);
        fix_all(&mut input);
        input["elements"][0]["youngs_modulus"] = json!(modulus);
        input["elements"][0]["area"] = json!(1.0);
        input["elements"][0]["thermal_expansion"] = json!(1.0);
        heat(&mut input, temperature, [0.0; 2]);
        if context == "assembled load" {
            let mut duplicate = input["elements"][0].clone();
            duplicate["id"] = json!("parallel");
            input["elements"].as_array_mut().unwrap().push(duplicate);
        }
        reject_and_replay(true, input, context);
    }
}

#[test]
fn spatial_frame_stress_and_energy_range_errors_allow_replay() {
    for thermal in [false, true] {
        let mut input = model(thermal, 1, [10.0; 6]);
        input["elements"][0]["section_modulus_z"] = json!(1e-320);
        reject_and_replay(thermal, input, "stress");
        for (e, load) in [(1.0, 1e-200), (1e200, 1e260)] {
            let mut input = model(thermal, 1, [load, 0.0, 0.0, 0.0, 0.0, 0.0]);
            input["elements"][0]["youngs_modulus"] = json!(e);
            input["elements"][0]["shear_modulus"] = json!(e);
            for field in [
                "area",
                "torsion_constant",
                "moment_of_inertia_y",
                "moment_of_inertia_z",
            ] {
                input["elements"][0][field] = json!(1.0);
            }
            reject_and_replay(thermal, input, "strain energy");
        }
    }
}

#[test]
fn spatial_frame_finite_member_energies_cannot_overflow_total() {
    for thermal in [false, true] {
        let mut input = model(thermal, 2, [1.4e154, 0.0, 0.0, 0.0, 0.0, 0.0]);
        for e in input["elements"].as_array_mut().unwrap() {
            for field in [
                "youngs_modulus",
                "shear_modulus",
                "area",
                "torsion_constant",
                "moment_of_inertia_y",
                "moment_of_inertia_z",
            ] {
                e[field] = json!(1.0);
            }
        }
        reject_and_replay(thermal, input, "total strain energy");
    }
}

#[test]
fn spatial_frame_large_section_axis_is_a_direction_not_a_scale() {
    for thermal in [false, true] {
        let input = model(thermal, 1, [10.0; 6]);
        let expected = solve(thermal, input.clone(), false).unwrap();
        for scale in [1e200, 1e308] {
            let mut input = input.clone();
            input["elements"][0]["local_y_axis"] = json!([0.0, scale, 0.0]);
            let result = solve(thermal, input, true).unwrap();
            assert_eq!(result["nodes"], expected["nodes"]);
            close(
                &result["total_strain_energy"],
                expected["total_strain_energy"].as_f64().unwrap(),
                1e-12,
            );
        }
    }
}

#[test]
fn spatial_exact_directional_constraints_preserve_scaled_reactions() {
    for rotational in [false, true] {
        for scale in [1.0, 1e200, 1e308] {
            let mut loads = [0.0; 6];
            loads[if rotational { 3 } else { 0 }] = 10.0;
            let mut input = model(true, 1, loads);
            let key = if rotational {
                "directional_rotational_constraints"
            } else {
                "directional_constraints"
            };
            let field = if rotational {
                "reaction_moment"
            } else {
                "reaction_force"
            };
            input[key] = json!([{"id": "support", "node": 1, "direction": [scale, 0.0, 0.0]}]);
            let result = solve(true, input, false).unwrap();
            close(&result[key][0][field], -10.0, 1e-10);
            close(&result["total_strain_energy"], 0.0, 0.0);
        }
    }
}

fn rotate(vector: [f64; 3]) -> [f64; 3] {
    [[0.36, -0.8, 0.48], [0.48, 0.6, 0.64], [-0.8, 0.0, 0.6]]
        .map(|row| row.into_iter().zip(vector).map(|(a, b)| a * b).sum())
}

#[test]
fn spatial_frame_rotation_and_reversed_connectivity_preserve_response() {
    for thermal in [false, true] {
        let loads = [1200.0, 1350.0, -750.0, 230.0, 180.0, 400.0];
        let mut input = model(thermal, 4, loads);
        if thermal {
            heat(&mut input, 35.0, [40.0, 60.0]);
        }
        let expected = solve(thermal, input.clone(), false).unwrap();
        for n in input["nodes"].as_array_mut().unwrap() {
            for fields in [
                ["x", "y", "z"],
                ["load_x", "load_y", "load_z"],
                ["moment_x", "moment_y", "moment_z"],
            ] {
                let rotated = rotate(fields.map(|f| n[f].as_f64().unwrap()));
                for (field, value) in fields.into_iter().zip(rotated) {
                    n[field] = json!(value);
                }
            }
        }
        for (index, e) in input["elements"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .enumerate()
        {
            e["local_y_axis"] = json!(rotate([0.0, 1.0, 0.0]));
            if index % 2 == 0 {
                let first = e["node_i"].clone();
                e["node_i"] = e["node_j"].clone();
                e["node_j"] = first;
                // Explicit local y stays fixed; reversing x reverses local z.
                if thermal {
                    e["temperature_gradient_z"] = json!(-60.0);
                }
            }
        }
        let result = solve(thermal, input.clone(), false).unwrap();
        assert_eq!(result, solve(thermal, input, true).unwrap());
        close(&result["total_strain_energy"], energy(loads), 1e-8);
        for index in 0..5 {
            for fields in [["ux", "uy", "uz"], ["rx", "ry", "rz"]] {
                let rotated = rotate(fields.map(|f| expected["nodes"][index][f].as_f64().unwrap()));
                for (field, value) in fields.into_iter().zip(rotated) {
                    close(&result["nodes"][index][field], value, 2e-8);
                }
            }
        }
    }
}

#[test]
fn spatial_long_geometry_retains_finite_axial_response() {
    for thermal in [false, true] {
        let mut input = model(thermal, 1, [1e10, 0.0, 0.0, 0.0, 0.0, 0.0]);
        input["nodes"][1]["x"] = json!(1e155);
        // Isolate axial arithmetic from an extremely ill-conditioned bending block.
        for field in ["fix_y", "fix_z", "fix_rx", "fix_ry", "fix_rz"] {
            input["nodes"][1][field] = json!(true);
        }
        input["elements"][0]["youngs_modulus"] = json!(1e200);
        input["elements"][0]["area"] = json!(1.0);
        for field in ["moment_of_inertia_y", "moment_of_inertia_z"] {
            input["elements"][0][field] = json!(1e100);
        }
        let result = solve(thermal, input, false).unwrap();
        close(&result["max_displacement"], 1e-35, 1e-10);
        close(&result["total_strain_energy"], 5e-26, 1e-10);
    }
}

fn cancel_at(thermal: bool, input: Value, stage: SolverStage, steps: usize) {
    let run = || solve(thermal, input.clone(), false);
    let expected = run().unwrap();
    let control = SolverControl::default();
    let cancel = control.clone();
    let result = with_solver_observer(
        &control,
        move |p| {
            if p.stage == stage && p.completed_steps == steps as u64 {
                cancel.request_cancel();
            }
        },
        || {
            let raw = run();
            assert!(raw.is_err(), "raw solver ignored {stage:?} at {steps}");
            raw
        },
    );
    assert!(result.unwrap_err().starts_with("solver cancelled"));
    assert_eq!(run().unwrap(), expected);
}

#[test]
fn spatial_frame_stage_entry_and_terminal_cancellation_replay() {
    for thermal in [false, true] {
        for (stage, size) in [
            (SolverStage::ElementPrecompute, 3),
            (SolverStage::LinearPrepare, 2),
            (SolverStage::ElementAssembly, 1),
            (SolverStage::SparseValidateMatrix, 12),
            (SolverStage::ResultNodes, 2),
            (SolverStage::ResultElements, 1),
            (SolverStage::ResultNodeSummary, 2),
            (SolverStage::ResultElementSummary, 1),
            (SolverStage::ResultTotals, 1),
        ] {
            for steps in [0, size] {
                cancel_at(thermal, model(thermal, 1, [10.0; 6]), stage, steps);
            }
        }
    }
}

#[test]
fn spatial_frame_chunk_cancellation_includes_constraint_projection() {
    for thermal in [false, true] {
        let mut input = model(thermal, 129, [0.0; 6]);
        fix_all(&mut input);
        if thermal {
            heat(&mut input, 35.0, [40.0, 60.0]);
        }
        for stage in [
            SolverStage::ElementPrecompute,
            SolverStage::LinearPrepare,
            SolverStage::ElementAssembly,
            SolverStage::SparseValidateMatrix,
            SolverStage::ResultNodes,
            SolverStage::ResultElements,
            SolverStage::ResultTotals,
        ] {
            cancel_at(thermal, input.clone(), stage, 64);
        }
        if thermal {
            for stage in [
                SolverStage::ConstraintIndex,
                SolverStage::ConstraintMap,
                SolverStage::ConstraintReduce,
                SolverStage::ResultFreeDofs,
                SolverStage::ResidualValidate,
            ] {
                cancel_at(thermal, input.clone(), stage, 64);
            }
        }
    }
}
