use kyuubiki_solver::solver_control::{SolverControl, SolverStage, with_solver_observer};
use kyuubiki_solver::{
    solve_beam_1d, solve_beam_1d_owned, solve_thermal_beam_1d, solve_thermal_beam_1d_owned,
};
use serde_json::{Value, json};

const E: f64 = 210e9;
const I: f64 = 9.5e-6;
const Z: f64 = 1.9e-4;
const L: f64 = 2.4;
const Q: f64 = 1350.0;
const ALPHA: f64 = 12e-6;
const GRADIENT: f64 = 45.0;
const DEPTH: f64 = 0.3;

fn model(thermal: bool, segments: usize, support: &str, q: f64) -> Value {
    let nodes: Vec<_> = (0..=segments)
        .map(|index| {
            let end = index == 0 || index == segments;
            json!({
                "id": format!("n{index}"), "x": L * index as f64 / segments as f64,
                "fix_y": if support == "cantilever" { index == 0 } else { end },
                "fix_rz": support != "pinned" && (support == "fixed" && end || index == 0),
                "load_y": 0.0, "moment_z": 0.0
            })
        })
        .collect();
    let elements: Vec<_> = (0..segments)
        .map(|index| {
            let mut element = json!({
                "id": format!("beam-{index}"), "node_i": index, "node_j": index + 1,
                "youngs_modulus": E, "moment_of_inertia": I, "section_modulus": Z,
                "distributed_load_y": q
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
            solve_thermal_beam_1d_owned(request)
        } else {
            solve_thermal_beam_1d(&request)
        }
        .map(|result| serde_json::to_value(result).unwrap())
    } else {
        let request = serde_json::from_value(input).unwrap();
        if owned {
            solve_beam_1d_owned(request)
        } else {
            solve_beam_1d(&request)
        }
        .map(|result| serde_json::to_value(result).unwrap())
    }
}

fn close(value: &Value, expected: f64) {
    let actual = value.as_f64().expect("physical field must remain numeric");
    assert!(actual.is_finite());
    assert!(
        (actual - expected).abs() <= 2e-7 * expected.abs().max(1e-10),
        "{actual:e} != {expected:e}"
    );
}

fn reverse(element: &mut Value) {
    let first = element["node_i"].clone();
    element["node_i"] = element["node_j"].clone();
    element["node_j"] = first;
}

fn heat(input: &mut Value, gradient: f64) {
    for element in input["elements"].as_array_mut().unwrap() {
        element["temperature_gradient_y"] = json!(gradient);
    }
}

#[test]
fn uniform_cantilever_has_positive_exact_field_energy() {
    for thermal in [false, true] {
        for q in [-Q, Q] {
            let result = solve(thermal, model(thermal, 1, "cantilever", q), false).unwrap();
            close(&result["nodes"][1]["uy"], q * L.powi(4) / (8.0 * E * I));
            close(&result["nodes"][1]["rz"], q * L.powi(3) / (6.0 * E * I));
            close(&result["max_moment"], q.abs() * L * L / 2.0);
            close(
                &result["total_strain_energy"],
                q * q * L.powi(5) / (40.0 * E * I),
            );
        }
    }
}

#[test]
fn simply_supported_uniform_beam_recovers_interior_moment_and_stress() {
    for thermal in [false, true] {
        for q in [-Q, Q] {
            let result = solve(thermal, model(thermal, 1, "pinned", q), false).unwrap();
            close(&result["nodes"][0]["rz"], q * L.powi(3) / (24.0 * E * I));
            close(&result["nodes"][1]["rz"], -q * L.powi(3) / (24.0 * E * I));
            close(&result["max_moment"], q.abs() * L * L / 8.0);
            close(&result["max_stress"], q.abs() * L * L / (8.0 * Z));
            close(
                &result["elements"][0]["max_bending_stress"],
                q.abs() * L * L / (8.0 * Z),
            );
            close(
                &result["total_strain_energy"],
                q * q * L.powi(5) / (240.0 * E * I),
            );
        }
    }
}

#[test]
fn fixed_uniform_beam_keeps_load_particular_solution_energy() {
    for thermal in [false, true] {
        let result = solve(thermal, model(thermal, 1, "fixed", Q), false).unwrap();
        close(&result["max_displacement"], 0.0);
        close(&result["max_moment"], Q * L * L / 12.0);
        close(
            &result["total_strain_energy"],
            Q * Q * L.powi(5) / (1440.0 * E * I),
        );
    }
}

#[test]
fn restrained_thermal_curvature_stores_elastic_energy_without_displacement() {
    for segments in [1, 2, 4, 8, 16] {
        for gradient in [-GRADIENT, GRADIENT] {
            let mut input = model(true, segments, "fixed", 0.0);
            heat(&mut input, gradient);
            let result = solve(true, input, false).unwrap();
            let curvature = ALPHA * gradient / DEPTH;
            assert!(result["max_displacement"].as_f64().unwrap() < 1e-12);
            close(&result["max_moment"], E * I * curvature.abs());
            close(
                &result["total_strain_energy"],
                0.5 * E * I * curvature * curvature * L,
            );
        }
    }
}

#[test]
fn free_thermal_curvature_does_not_add_elastic_energy_to_mechanical_loading() {
    for segments in [1, 2, 4, 8, 16] {
        for q in [0.0, Q] {
            let mut input = model(true, segments, "cantilever", q);
            heat(&mut input, GRADIENT);
            let result = solve(true, input, false).unwrap();
            let curvature = ALPHA * GRADIENT / DEPTH;
            close(
                &result["nodes"][segments]["uy"],
                0.5 * curvature * L * L + q * L.powi(4) / (8.0 * E * I),
            );
            close(
                &result["nodes"][segments]["rz"],
                curvature * L + q * L.powi(3) / (6.0 * E * I),
            );
            if q == 0.0 {
                assert!(result["total_strain_energy"].as_f64().unwrap() < 1e-12);
            } else {
                close(
                    &result["total_strain_energy"],
                    q * q * L.powi(5) / (40.0 * E * I),
                );
            }
        }
    }
}

#[test]
fn uniform_load_field_recovery_is_invariant_under_mesh_refinement() {
    for thermal in [false, true] {
        for (support, energy_divisor, moment_divisor) in [
            ("cantilever", 40.0, 2.0),
            ("pinned", 240.0, 8.0),
            ("fixed", 1440.0, 12.0),
        ] {
            for segments in [1, 2, 4, 8, 16] {
                let result = solve(thermal, model(thermal, segments, support, Q), false).unwrap();
                close(
                    &result["total_strain_energy"],
                    Q * Q * L.powi(5) / (energy_divisor * E * I),
                );
                close(&result["max_moment"], Q * L * L / moment_divisor);
                let sum: f64 = result["elements"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|element| element["strain_energy"].as_f64().unwrap())
                    .sum();
                close(&result["total_strain_energy"], sum);
            }
        }
    }
}

#[test]
fn connectivity_reversal_preserves_global_displacements_and_endpoint_actions() {
    for thermal in [false, true] {
        for (q, force, moment) in [(0.0, Q, 0.0), (0.0, 0.0, Q), (Q, 0.0, 0.0)] {
            for segments in [1, 2, 8] {
                let mut input = model(thermal, segments, "cantilever", q);
                input["nodes"][segments]["load_y"] = json!(force);
                input["nodes"][segments]["moment_z"] = json!(moment);
                if thermal {
                    heat(&mut input, GRADIENT);
                }
                let expected = solve(thermal, input.clone(), false).unwrap();
                for index in (0..segments).step_by(2) {
                    reverse(&mut input["elements"][index]);
                }
                let result = solve(thermal, input.clone(), false).unwrap();
                assert_eq!(result, solve(thermal, input, true).unwrap());
                for index in 0..=segments {
                    for field in ["uy", "rz"] {
                        close(
                            &result["nodes"][index][field],
                            expected["nodes"][index][field].as_f64().unwrap(),
                        );
                    }
                }
                for index in 0..segments {
                    for (left, right) in
                        [("moment_i", "moment_j"), ("shear_force_i", "shear_force_j")]
                    {
                        let field = if index % 2 == 0 { right } else { left };
                        let actual = result["elements"][index][field].as_f64().unwrap();
                        let wanted = expected["elements"][index][left].as_f64().unwrap();
                        assert!((actual - wanted).abs() < 1e-6 * Q);
                    }
                }
                for field in ["max_moment", "max_stress", "total_strain_energy"] {
                    close(&result[field], expected[field].as_f64().unwrap());
                }
            }
        }
    }
}

#[test]
fn zero_load_keeps_zero_energy_in_both_ownership_paths() {
    for thermal in [false, true] {
        for owned in [false, true] {
            let result = solve(thermal, model(thermal, 1, "cantilever", 0.0), owned).unwrap();
            for field in ["max_moment", "max_stress", "total_strain_energy"] {
                close(&result[field], 0.0);
            }
        }
    }
}

#[test]
fn mixed_loading_recovers_an_off_center_moment_extremum() {
    let force = -Q * L / 3.0;
    let moment = -Q * L * L / 12.0;
    let energy = (moment * moment * L
        + moment * force * L * L
        + (force * force + moment * Q) * L.powi(3) / 3.0
        + force * Q * L.powi(4) / 4.0
        + Q * Q * L.powi(5) / 20.0)
        / (2.0 * E * I);
    for thermal in [false, true] {
        let mut input = model(thermal, 1, "cantilever", Q);
        input["nodes"][1]["load_y"] = json!(force);
        input["nodes"][1]["moment_z"] = json!(moment);
        let result = solve(thermal, input, false).unwrap();
        close(&result["max_moment"], 5.0 * Q * L * L / 36.0);
        close(&result["total_strain_energy"], energy);
    }
}

#[test]
fn piecewise_stiffness_and_nonuniform_lengths_preserve_energy_and_equilibrium() {
    for thermal in [false, true] {
        let mut input = model(thermal, 2, "cantilever", Q);
        input["nodes"][1]["x"] = json!(L / 3.0);
        input["elements"][0]["youngs_modulus"] = json!(1.7 * E);
        input["elements"][1]["youngs_modulus"] = json!(0.8 * E);
        reverse(&mut input["elements"][1]);
        let result = solve(thermal, input, false).unwrap();
        let split = (2.0 * L / 3.0).powi(5);
        let energy = Q * Q / (40.0 * E * I) * ((L.powi(5) - split) / 1.7 + split / 0.8);
        close(&result["total_strain_energy"], energy);
        close(&result["max_moment"], Q * L * L / 2.0);
        close(&result["elements"][0]["shear_force_i"], -Q * L);
        close(&result["elements"][0]["moment_i"], -Q * L * L / 2.0);
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
        finite_json(&solve(thermal, model(thermal, 1, "cantilever", Q), owned).unwrap());
    }
}

#[test]
fn fixed_supports_cannot_hide_invalid_element_or_assembled_stiffness() {
    for thermal in [false, true] {
        for (modulus, inertia) in [(1e200, 1e200), (1e-200, 1e-200)] {
            let mut input = model(thermal, 1, "fixed", 0.0);
            input["elements"][0]["youngs_modulus"] = json!(modulus);
            input["elements"][0]["moment_of_inertia"] = json!(inertia);
            rejected_then_replay(thermal, input, "beam-0: stiffness");
        }
        let mut input = model(thermal, 1, "fixed", 0.0);
        input["elements"][0]["youngs_modulus"] = json!(1e308);
        input["elements"][0]["moment_of_inertia"] = json!(1.0);
        let mut duplicate = input["elements"][0].clone();
        duplicate["id"] = json!("parallel-beam");
        input["elements"].as_array_mut().unwrap().push(duplicate);
        rejected_then_replay(thermal, input, "assembled stiffness");
    }
}

#[test]
fn fixed_supports_cannot_hide_nonfinite_assembled_loads() {
    for thermal in [false, true] {
        let mut input = model(thermal, 1, "fixed", 1e308);
        let mut duplicate = input["elements"][0].clone();
        duplicate["id"] = json!("parallel-beam");
        input["elements"].as_array_mut().unwrap().push(duplicate);
        rejected_then_replay(thermal, input, "parallel-beam: assembled load");
    }
}

#[test]
fn thermal_curvature_and_equivalent_moment_overflow_are_explicit_errors() {
    for (modulus, alpha, gradient) in [(E, 1e308, 1e308), (1e200, 1.0, 1e200)] {
        let mut input = model(true, 1, "fixed", 0.0);
        input["elements"][0]["youngs_modulus"] = json!(modulus);
        input["elements"][0]["moment_of_inertia"] = json!(1.0);
        input["elements"][0]["thermal_expansion"] = json!(alpha);
        heat(&mut input, gradient);
        rejected_then_replay(true, input, "beam-0: equivalent load");
    }
}

#[test]
fn stress_and_energy_range_errors_are_not_serialized_as_success() {
    for thermal in [false, true] {
        let mut input = model(thermal, 1, "fixed", Q);
        input["elements"][0]["section_modulus"] = json!(1e-320);
        rejected_then_replay(thermal, input, "beam-0: moment or stress");
        for q in [1e-190, 1e200] {
            let mut input = model(thermal, 1, "fixed", q);
            input["elements"][0]["youngs_modulus"] = json!(1.0);
            input["elements"][0]["moment_of_inertia"] = json!(1.0);
            rejected_then_replay(thermal, input, "beam-0: strain energy");
        }
    }
}

#[test]
fn finite_large_moments_do_not_overflow_when_their_energy_is_representable() {
    for thermal in [false, true] {
        let mut input = model(thermal, 1, "fixed", 1e200);
        input["elements"][0]["youngs_modulus"] = json!(1e200);
        input["elements"][0]["moment_of_inertia"] = json!(1.0);
        input["elements"][0]["section_modulus"] = json!(1.0);
        let result = solve(thermal, input, false).unwrap();
        finite_json(&result);
        close(&result["total_strain_energy"], 1e200 * L.powi(5) / 1440.0);
    }
}

#[test]
fn thermal_element_and_total_energy_overflow_are_both_rejected() {
    for (segments, context) in [(1, "beam-0: strain energy"), (2, "total strain energy")] {
        let mut input = model(true, segments, "fixed", 0.0);
        for element in input["elements"].as_array_mut().unwrap() {
            element["youngs_modulus"] = json!(1e200);
            element["moment_of_inertia"] = json!(1.0);
            element["thermal_expansion"] = json!(1.0);
            element["section_depth"] = json!(1.0);
        }
        heat(&mut input, 1.3e54);
        rejected_then_replay(true, input, context);
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
fn beam_assembly_and_result_stages_cancel_before_success_and_replay() {
    for thermal in [false, true] {
        for (stage, size) in [
            (SolverStage::ElementPrecompute, 3),
            (SolverStage::ElementAssembly, 1),
            (SolverStage::ResultFreeDofs, 2),
            (SolverStage::ResultNodes, 2),
            (SolverStage::ResultElements, 1),
            (SolverStage::ResultNodeSummary, 2),
            (SolverStage::ResultElementSummary, 1),
            (SolverStage::ResultTotals, 1),
        ] {
            for steps in [0, size] {
                cancelled(thermal, model(thermal, 1, "cantilever", Q), stage, steps);
            }
        }
    }
}

#[test]
fn beam_assembly_and_recovery_cancel_at_chunk_boundaries() {
    for thermal in [false, true] {
        let mut input = model(thermal, 129, "fixed", Q);
        for node in input["nodes"].as_array_mut().unwrap() {
            node["fix_y"] = json!(true);
            node["fix_rz"] = json!(true);
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
