use kyuubiki_solver::solver_control::{SolverControl, SolverStage, with_solver_observer};
use kyuubiki_solver::{
    solve_frame_2d, solve_frame_2d_owned, solve_thermal_frame_2d, solve_thermal_frame_2d_owned,
    solve_thermal_frame_3d,
};
use serde_json::{Value, json};

const E: f64 = 210e9;
const A: f64 = 0.018;
const I: f64 = 9.5e-6;
const Z: f64 = 1.9e-4;
const L: f64 = 2.4;
const P: f64 = 1350.0;
const N: f64 = 1200.0;
const M: f64 = 400.0;
const ALPHA: f64 = 12e-6;
const DEPTH: f64 = 0.3;

fn model(thermal: bool, segments: usize, angle: f64, loads: [f64; 3]) -> Value {
    let (s, c) = angle.sin_cos();
    let nodes: Vec<_> = (0..=segments)
        .map(|index| {
            let x = L * index as f64 / segments as f64;
            let tip = index == segments;
            let mut node = json!({
                "id": format!("n{index}"), "x": c * x, "y": s * x,
                "fix_x": index == 0, "fix_y": index == 0, "fix_rz": index == 0,
                "load_x": if tip { c * loads[0] - s * loads[1] } else { 0.0 },
                "load_y": if tip { s * loads[0] + c * loads[1] } else { 0.0 },
                "moment_z": if tip { loads[2] } else { 0.0 }
            });
            if thermal {
                node["temperature_delta"] = json!(0.0);
            }
            node
        })
        .collect();
    let elements: Vec<_> = (0..segments)
        .map(|index| {
            let mut element = json!({
                "id": format!("member-{index}"), "node_i": index, "node_j": index + 1,
                "area": A, "youngs_modulus": E, "moment_of_inertia": I, "section_modulus": Z
            });
            if thermal {
                element["thermal_expansion"] = json!(ALPHA);
                element["section_depth"] = json!(DEPTH);
                element["temperature_gradient_y"] = json!(0.0);
            }
            element
        })
        .collect();
    json!({"nodes": nodes, "elements": elements})
}

fn solve(thermal: bool, input: Value, owned: bool) -> Result<Value, String> {
    if thermal {
        let request = serde_json::from_value(input).unwrap();
        if owned {
            solve_thermal_frame_2d_owned(request)
        } else {
            solve_thermal_frame_2d(&request)
        }
        .map(|result| serde_json::to_value(result).unwrap())
    } else {
        let request = serde_json::from_value(input).unwrap();
        if owned {
            solve_frame_2d_owned(request)
        } else {
            solve_frame_2d(&request)
        }
        .map(|result| serde_json::to_value(result).unwrap())
    }
}

fn close(value: &Value, expected: f64) {
    let actual = value.as_f64().expect("physical field must remain numeric");
    assert!(actual.is_finite());
    let tolerance = if expected == 0.0 {
        1e-12
    } else {
        expected.abs() * 2e-7
    };
    assert!(
        (actual - expected).abs() <= tolerance,
        "{actual:e} != {expected:e}"
    );
}

fn elastic_energy([axial, shear, moment]: [f64; 3]) -> f64 {
    axial * axial * L / (2.0 * E * A)
        + (moment * moment * L + moment * shear * L * L + shear * shear * L.powi(3) / 3.0)
            / (2.0 * E * I)
}

fn heat(input: &mut Value, temperature: f64, gradient: f64) {
    for node in input["nodes"].as_array_mut().unwrap() {
        node["temperature_delta"] = json!(temperature);
    }
    for element in input["elements"].as_array_mut().unwrap() {
        element["temperature_gradient_y"] = json!(gradient);
    }
}

fn spatial_planar_model(input: &Value, angle: f64) -> Value {
    let mut spatial = input.clone();
    for node in spatial["nodes"].as_array_mut().unwrap() {
        for field in ["z", "load_z", "moment_x", "moment_y"] {
            node[field] = json!(0.0);
        }
        for field in ["fix_z", "fix_rx", "fix_ry"] {
            node[field] = json!(true);
        }
    }
    for element in spatial["elements"].as_array_mut().unwrap() {
        element["local_y_axis"] = json!([-angle.sin(), angle.cos(), 0.0]);
        element["shear_modulus"] = json!(80e9);
        element["torsion_constant"] = json!(1e-5);
        for field in ["moment_of_inertia_y", "moment_of_inertia_z"] {
            element[field] = json!(I);
        }
        for field in ["section_modulus_y", "section_modulus_z"] {
            element[field] = json!(Z);
        }
        for field in ["section_depth_y", "section_depth_z"] {
            element[field] = json!(DEPTH);
        }
        element["temperature_gradient_z"] = json!(0.0);
    }
    spatial
}

#[test]
fn thermal_frame_transverse_load_energy_includes_curvature_variation() {
    for loads in [[0.0, P, 0.0], [N, P, M], [0.0, 0.0, M]] {
        for segments in [1, 2, 4, 8, 16] {
            let result = solve(true, model(true, segments, 0.0, loads), false).unwrap();
            close(&result["total_strain_energy"], elastic_energy(loads));
        }
    }
}

#[test]
fn free_thermal_expansion_and_curvature_preserve_mechanical_field_energy() {
    for angle in [0.0, 0.71] {
        for segments in [1, 2, 8] {
            let loads = [N, P, M];
            let mut input = model(true, segments, angle, loads);
            heat(&mut input, 35.0, 45.0);
            let result = solve(true, input, false).unwrap();
            close(&result["total_strain_energy"], elastic_energy(loads));
            let axial = N * L / (E * A) + ALPHA * 35.0 * L;
            let transverse = P * L.powi(3) / (3.0 * E * I)
                + M * L * L / (2.0 * E * I)
                + 0.5 * ALPHA * 45.0 / DEPTH * L * L;
            close(
                &result["nodes"][segments]["ux"],
                angle.cos() * axial - angle.sin() * transverse,
            );
            close(
                &result["nodes"][segments]["uy"],
                angle.sin() * axial + angle.cos() * transverse,
            );
        }
    }
}

#[test]
fn planar_thermal_energy_matches_mechanical_and_spatial_reductions() {
    for angle in [0.0, 0.71] {
        for segments in [1, 2, 4] {
            let loads = [N, P, M];
            let input = model(true, segments, angle, loads);
            let planar = solve(true, input.clone(), false).unwrap();
            let mechanical = solve(false, model(false, segments, angle, loads), false).unwrap();
            let spatial = solve_thermal_frame_3d(
                &serde_json::from_value(spatial_planar_model(&input, angle)).unwrap(),
            )
            .unwrap();
            close(&planar["total_strain_energy"], spatial.total_strain_energy);
            close(
                &planar["total_strain_energy"],
                mechanical["total_strain_energy"].as_f64().unwrap(),
            );
            close(&planar["total_strain_energy"], elastic_energy(loads));
        }
    }
}

#[test]
fn thermal_frame_portal_zero_temperature_energy_matches_external_work() {
    let mut input = model(true, 2, 0.0, [N, -P, M]);
    input["nodes"][1]["x"] = json!(0.0);
    input["nodes"][1]["y"] = json!(L);
    input["nodes"][2]["x"] = json!(L);
    input["nodes"][2]["y"] = json!(L);
    let result = solve(true, input, false).unwrap();
    let tip = &result["nodes"][2];
    let work = 0.5
        * (N * tip["ux"].as_f64().unwrap() - P * tip["uy"].as_f64().unwrap()
            + M * tip["rz"].as_f64().unwrap());
    close(&result["total_strain_energy"], work);
}

#[test]
fn restrained_thermal_frame_retains_axial_and_bending_energy() {
    let mut input = model(true, 1, 0.71, [0.0; 3]);
    for node in input["nodes"].as_array_mut().unwrap() {
        for field in ["fix_x", "fix_y", "fix_rz"] {
            node[field] = json!(true);
        }
    }
    heat(&mut input, 35.0, 45.0);
    let result = solve(true, input, false).unwrap();
    close(
        &result["total_strain_energy"],
        0.5 * E * L * (A * (ALPHA * 35.0).powi(2) + I * (ALPHA * 45.0 / DEPTH).powi(2)),
    );
    close(&result["max_displacement"], 0.0);
}

#[test]
fn thermal_frame_temperature_mean_preserves_finite_extremes() {
    let mut input = model(true, 1, 0.0, [0.0; 3]);
    for node in input["nodes"].as_array_mut().unwrap() {
        for field in ["fix_x", "fix_y", "fix_rz"] {
            node[field] = json!(true);
        }
    }
    input["elements"][0]["thermal_expansion"] = json!(1e-310);
    heat(&mut input, 1e308, 0.0);
    let result = solve(true, input, false).unwrap();
    close(&result["elements"][0]["average_temperature_delta"], 1e308);
    close(&result["elements"][0]["thermal_strain"], 0.01);
    close(&result["total_strain_energy"], 0.5 * E * A * 1e-4 * L);
}

#[test]
fn frame_displacement_magnitude_preserves_tiny_nonzero_values() {
    for thermal in [false, true] {
        let mut input = model(thermal, 1, 0.0, [1.0, 0.0, 0.0]);
        input["elements"][0]["youngs_modulus"] = json!(1e200);
        input["elements"][0]["area"] = json!(1.0);
        input["elements"][0]["moment_of_inertia"] = json!(1.0);
        let result = solve(thermal, input, false).unwrap();
        close(&result["max_displacement"], L * 1e-200);
        close(&result["total_strain_energy"], 0.5 * L * 1e-200);
    }
}

#[test]
fn frame_rotated_reversed_connectivity_preserves_energy_and_global_response() {
    for thermal in [false, true] {
        for angle in [0.0, 0.71, 2.8] {
            for segments in [1, 2, 8] {
                let mut input = model(thermal, segments, angle, [N, P, M]);
                if thermal {
                    heat(&mut input, 35.0, 45.0);
                }
                let expected = solve(thermal, input.clone(), false).unwrap();
                for (index, e) in input["elements"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .enumerate()
                {
                    if index % 2 == 0 {
                        let first = e["node_i"].clone();
                        e["node_i"] = e["node_j"].clone();
                        e["node_j"] = first;
                        // Local y reverses too: keep the physical thermal field unchanged.
                        if thermal {
                            e["temperature_gradient_y"] = json!(-45.0);
                        }
                    }
                }
                let result = solve(thermal, input.clone(), false).unwrap();
                assert_eq!(result, solve(thermal, input, true).unwrap());
                close(&result["total_strain_energy"], elastic_energy([N, P, M]));
                for index in 0..=segments {
                    for field in ["ux", "uy", "rz"] {
                        close(
                            &result["nodes"][index][field],
                            expected["nodes"][index][field].as_f64().unwrap(),
                        );
                    }
                }
                for field in ["max_moment", "max_stress", "total_strain_energy"] {
                    close(&result[field], expected[field].as_f64().unwrap());
                }
            }
        }
    }
}

fn fix_all(input: &mut Value) {
    for node in input["nodes"].as_array_mut().unwrap() {
        for field in ["fix_x", "fix_y", "fix_rz"] {
            node[field] = json!(true);
        }
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

fn rejected_then_replay(thermal: bool, input: Value, context: &str) {
    for owned in [false, true] {
        let error = solve(thermal, input.clone(), owned).unwrap_err();
        assert!(
            error.contains(context) && error.contains("representable"),
            "{error}"
        );
        finite_json(&solve(thermal, model(thermal, 1, 0.0, [N, P, M]), owned).unwrap());
    }
}

#[test]
fn fixed_frame_supports_cannot_hide_invalid_element_or_assembled_stiffness() {
    for thermal in [false, true] {
        for (modulus, area, inertia) in [
            (1e200, 1e200, 1.0),
            (1e-200, 1e-200, 1.0),
            (1e200, 1.0, 1e200),
        ] {
            let mut input = model(thermal, 1, 0.0, [0.0; 3]);
            fix_all(&mut input);
            input["elements"][0]["youngs_modulus"] = json!(modulus);
            input["elements"][0]["area"] = json!(area);
            input["elements"][0]["moment_of_inertia"] = json!(inertia);
            rejected_then_replay(thermal, input, "member-0: stiffness");
        }
        let mut input = model(thermal, 1, 0.0, [0.0; 3]);
        fix_all(&mut input);
        input["elements"][0]["youngs_modulus"] = json!(1e308);
        input["elements"][0]["area"] = json!(1.0);
        input["elements"][0]["moment_of_inertia"] = json!(1.0);
        let mut duplicate = input["elements"][0].clone();
        duplicate["id"] = json!("parallel-member");
        input["elements"].as_array_mut().unwrap().push(duplicate);
        rejected_then_replay(thermal, input, "assembled stiffness");
    }
}

#[test]
fn thermal_frame_fixed_supports_do_not_hide_nonfinite_thermal_loads() {
    for (modulus, temperature, context) in [
        (E, 1e308, "equivalent load"),
        (1e200, 1e108, "assembled load"),
    ] {
        let mut input = model(true, 1, 0.0, [0.0; 3]);
        fix_all(&mut input);
        input["elements"][0]["youngs_modulus"] = json!(modulus);
        input["elements"][0]["area"] = json!(1.0);
        input["elements"][0]["thermal_expansion"] = json!(1.0);
        heat(&mut input, temperature, 0.0);
        if context == "assembled load" {
            let mut duplicate = input["elements"][0].clone();
            duplicate["id"] = json!("parallel-member");
            input["elements"].as_array_mut().unwrap().push(duplicate);
        }
        rejected_then_replay(true, input, context);
    }
}

#[test]
fn thermal_frame_evaluates_thermal_strain_before_scaling_by_axial_rigidity() {
    let mut input = model(true, 1, 0.0, [0.0; 3]);
    fix_all(&mut input);
    input["elements"][0]["youngs_modulus"] = json!(1e300);
    input["elements"][0]["area"] = json!(1.0);
    input["elements"][0]["moment_of_inertia"] = json!(1.0);
    input["elements"][0]["thermal_expansion"] = json!(1e10);
    heat(&mut input, 1e-10, 0.0);
    let result = solve(true, input, false).unwrap();
    finite_json(&result);
    close(&result["elements"][0]["thermal_strain"], 1.0);
    close(&result["total_strain_energy"], 0.5 * L * 1e300);
}

#[test]
fn frame_stress_and_energy_range_failures_allow_normal_replay() {
    for thermal in [false, true] {
        let mut input = model(thermal, 1, 0.0, [0.0, 0.0, M]);
        input["elements"][0]["section_modulus"] = json!(1e-320);
        rejected_then_replay(thermal, input, "member-0: stress");
        for (modulus, load) in [(1.0, 1e-200), (1e200, 1e260)] {
            let mut input = model(thermal, 1, 0.0, [load, 0.0, 0.0]);
            input["elements"][0]["youngs_modulus"] = json!(modulus);
            input["elements"][0]["area"] = json!(1.0);
            input["elements"][0]["moment_of_inertia"] = json!(1.0);
            rejected_then_replay(thermal, input, "member-0: strain energy");
        }
    }
}

#[test]
fn frame_energy_total_cannot_overflow_after_finite_member_energies() {
    for thermal in [false, true] {
        let mut input = model(thermal, 2, 0.0, [1.3e154, 0.0, 0.0]);
        for element in input["elements"].as_array_mut().unwrap() {
            element["youngs_modulus"] = json!(1.0);
            element["area"] = json!(1.0);
            element["moment_of_inertia"] = json!(1.0);
        }
        rejected_then_replay(thermal, input, "total strain energy");
    }
}

#[test]
fn long_finite_frame_geometry_avoids_squared_length_overflow() {
    for thermal in [false, true] {
        let mut input = model(thermal, 1, 0.0, [1e10, 0.0, 0.0]);
        input["nodes"][1]["x"] = json!(1e155);
        // Isolate axial arithmetic; unexcited bending at this scale is ill-conditioned.
        input["nodes"][1]["fix_y"] = json!(true);
        input["nodes"][1]["fix_rz"] = json!(true);
        input["elements"][0]["youngs_modulus"] = json!(1e200);
        input["elements"][0]["area"] = json!(1.0);
        input["elements"][0]["moment_of_inertia"] = json!(1e100);
        let result = solve(thermal, input, false).unwrap();
        finite_json(&result);
        close(&result["elements"][0]["length"], 1e155);
        close(&result["max_displacement"], 1e-35);
        close(&result["total_strain_energy"], 5e-26);
    }
}

fn cancelled(thermal: bool, input: Value, stage: SolverStage, steps: usize) {
    let run = || solve(thermal, input.clone(), false);
    let expected = run().unwrap();
    let control = SolverControl::default();
    let cancel = control.clone();
    let result = with_solver_observer(
        &control,
        move |point| {
            if point.stage == stage && point.completed_steps == steps as u64 {
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
    assert!(control.was_interrupted());
    assert_eq!(run().unwrap(), expected);
}

#[test]
fn frame_assembly_and_recovery_cancel_before_success_and_replay() {
    for thermal in [false, true] {
        for (stage, size) in [
            (SolverStage::ElementPrecompute, 3),
            (SolverStage::ElementAssembly, 1),
            (SolverStage::ResultFreeDofs, 3),
            (SolverStage::ResultNodes, 2),
            (SolverStage::ResultElements, 1),
            (SolverStage::ResultNodeSummary, 2),
            (SolverStage::ResultElementSummary, 1),
            (SolverStage::ResultTotals, 1),
        ] {
            for steps in [0, size] {
                cancelled(thermal, model(thermal, 1, 0.0, [N, P, M]), stage, steps);
            }
        }
    }
}

#[test]
fn frame_assembly_and_recovery_cancel_at_chunk_boundaries() {
    for thermal in [false, true] {
        let mut input = model(thermal, 129, 0.0, [0.0; 3]);
        fix_all(&mut input);
        if thermal {
            heat(&mut input, 35.0, 45.0);
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
            cancelled(thermal, input.clone(), stage, 64);
        }
    }
}
