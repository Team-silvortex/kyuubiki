use kyuubiki_solver::solver_control::{SolverControl, SolverStage, with_solver_observer};
use kyuubiki_solver::{
    profile_truss_2d, solve_truss_2d, solve_truss_2d_owned, solve_truss_3d, solve_truss_3d_owned,
};
use serde_json::{Value, json};

fn model(spatial: bool, modulus: f64, area: f64, length: f64, load: f64) -> Value {
    let nodes: Vec<_> = (0..3)
        .map(|index| {
            let mut node = json!({
                "id": format!("n{index}"), "x": index as f64 * length, "y": 0.0,
                "fix_x": index == 0, "fix_y": true,
                "load_x": if index == 2 { load } else { 0.0 }, "load_y": 0.0
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
                "area": area, "youngs_modulus": modulus
            })
        })
        .collect();
    json!({"nodes": nodes, "elements": elements})
}

fn fix_all(input: &mut Value) {
    for node in input["nodes"].as_array_mut().unwrap() {
        node["fix_x"] = json!(true);
    }
}

fn solve(spatial: bool, input: Value, owned: bool) -> Result<Value, String> {
    if spatial {
        let request = serde_json::from_value(input).unwrap();
        if owned {
            solve_truss_3d_owned(request)
        } else {
            solve_truss_3d(&request)
        }
        .map(|result| serde_json::to_value(result).unwrap())
    } else {
        let request = serde_json::from_value(input).unwrap();
        if owned {
            solve_truss_2d_owned(request)
        } else {
            solve_truss_2d(&request)
        }
        .map(|result| serde_json::to_value(result).unwrap())
    }
}

fn close(value: &Value, expected: f64, tolerance: f64) {
    let actual = value.as_f64().expect("physical field must remain numeric");
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
fn mechanical_truss_closed_form_and_work_survive_connectivity_reversal() {
    for spatial in [false, true] {
        for load in [-1.2, 1.2] {
            for reverse in [false, true] {
                let mut input = model(spatial, 1200.0, 1.0, 1.0, load);
                if reverse {
                    for element in input["elements"].as_array_mut().unwrap() {
                        let first = element["node_i"].clone();
                        element["node_i"] = element["node_j"].clone();
                        element["node_j"] = first;
                    }
                }
                let borrowed = solve(spatial, input.clone(), false).unwrap();
                assert_eq!(borrowed, solve(spatial, input, true).unwrap());
                finite_json(&borrowed);
                close(&borrowed["nodes"][2]["ux"], load / 600.0, 1e-12);
                close(&borrowed["max_displacement"], 0.002, 1e-12);
                close(&borrowed["total_strain_energy"], 0.0012, 1e-12);
                close(
                    &borrowed["total_strain_energy"],
                    0.5 * load * borrowed["nodes"][2]["ux"].as_f64().unwrap(),
                    1e-12,
                );
                for element in borrowed["elements"].as_array().unwrap() {
                    close(&element["stress"], load, 1e-12);
                    close(&element["strain"], load / 1200.0, 1e-12);
                    close(&element["axial_force"], load, 1e-12);
                }
            }
        }
    }
}

#[test]
fn mechanical_truss_tiny_displacements_keep_nonzero_norms() {
    for spatial in [false, true] {
        let result = solve(spatial, model(spatial, 1e200, 1.0, 1.0, 1.0), false).unwrap();
        finite_json(&result);
        close(&result["nodes"][2]["ux"], 2e-200, 1e-12);
        close(&result["max_displacement"], 2e-200, 1e-12);
        close(&result["total_strain_energy"], 1e-200, 1e-12);
    }
}

#[test]
fn mechanical_truss_long_members_preserve_finite_length_and_displacement() {
    for spatial in [false, true] {
        let result = solve(spatial, model(spatial, 1.0, 1.0, 1e200, 1e-3), false).unwrap();
        finite_json(&result);
        close(&result["elements"][0]["length"], 1e200, 1e-12);
        close(&result["max_displacement"], 2e197, 1e-12);
        close(&result["total_strain_energy"], 1e194, 1e-12);
    }
}

#[test]
fn mechanical_truss_volume_product_retains_representable_energy() {
    for spatial in [false, true] {
        let result = solve(
            spatial,
            model(spatial, 1e-100, 1e-100, 1e100, 1e-300),
            false,
        )
        .unwrap();
        finite_json(&result);
        close(&result["max_displacement"], 2.0, 1e-12);
        close(&result["total_strain_energy"], 1e-300, 1e-12);
    }
}

#[test]
fn mechanical_truss_energy_range_errors_allow_valid_input_replay() {
    for spatial in [false, true] {
        for (modulus, area, length, load, context) in [
            (1e308, 0.5, 1e4, 1e306, "total strain energy"),
            (1e308, 0.5, 1e5, 1e306, "member-0"),
            (1e-100, 1e-100, 1e-6, 1e-300, "member-0"),
        ] {
            let error =
                solve(spatial, model(spatial, modulus, area, length, load), false).unwrap_err();
            assert!(
                error.contains(context) && error.contains("representable"),
                "{error}"
            );
            finite_json(&solve(spatial, model(spatial, 1200.0, 1.0, 1.0, 1.2), false).unwrap());
        }
    }
}

#[test]
fn mechanical_truss_fixed_supports_cannot_hide_invalid_stiffness() {
    for spatial in [false, true] {
        for (modulus, area, context) in [
            (1e200, 1e200, "member-0"),
            (1e-200, 1e-200, "member-0"),
            (1e308, 1.0, "assembled stiffness"),
        ] {
            let mut input = model(spatial, modulus, area, 1.0, 0.0);
            fix_all(&mut input);
            for owned in [false, true] {
                let error = solve(spatial, input.clone(), owned).unwrap_err();
                assert!(
                    error.contains(context) && error.contains("representable"),
                    "{error}"
                );
            }
        }
    }
}

#[test]
fn mechanical_truss_displacement_guard_uses_actual_translated_scaled_extent() {
    for spatial in [false, true] {
        for length in [1.0, 1e-3] {
            for offset in [0.0, 1e6, -1e6] {
                for load in [0.01, 1.0] {
                    let mut input = model(spatial, 1.0, 1.0, length, load);
                    for node in input["nodes"].as_array_mut().unwrap() {
                        node["x"] = json!(node["x"].as_f64().unwrap() + offset);
                    }
                    for owned in [false, true] {
                        let result = solve(spatial, input.clone(), owned);
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
}

#[test]
fn mechanical_truss_profile_matches_public_result_and_rejects_invalid_output() {
    let input = model(false, 1200.0, 1.0, 1.0, 1.2);
    let result = profile_truss_2d(&serde_json::from_value(input.clone()).unwrap()).unwrap();
    assert!(result.solver_residual_norm.is_finite());
    assert_eq!(
        serde_json::to_value(&result.result).unwrap(),
        solve(false, input, false).unwrap()
    );
    assert!(
        result
            .stages
            .iter()
            .any(|stage| stage.label == "assemble_result")
    );
    let bad = model(false, 1e308, 0.5, 1e4, 1e306);
    let error = profile_truss_2d(&serde_json::from_value(bad).unwrap()).unwrap_err();
    assert!(error.contains("total strain energy"), "{error}");
}

fn cancelled(spatial: bool, input: Value, stage: SolverStage, steps: usize) {
    let run = || solve(spatial, input.clone(), false);
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
fn mechanical_truss_result_stages_cancel_before_success_and_replay() {
    for spatial in [false, true] {
        for (stage, size) in [
            (SolverStage::ResultFreeDofs, 2),
            (SolverStage::ResultNodes, 3),
            (SolverStage::ResultElements, 2),
            (SolverStage::ResultNodeSummary, 3),
            (SolverStage::ResultElementSummary, 2),
            (SolverStage::ResultTotals, 2),
        ] {
            for steps in [0, size] {
                cancelled(spatial, model(spatial, 1200.0, 1.0, 1.0, 1.2), stage, steps);
            }
        }
    }
}

#[test]
fn mechanical_truss_assembly_and_recovery_cancel_at_chunk_boundaries() {
    for spatial in [false, true] {
        let mut input = model(spatial, 1200.0, 1.0, 1.0, 0.0);
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
        for stage in [
            SolverStage::ElementAssembly,
            SolverStage::SparseValidateMatrix,
            SolverStage::ResultNodes,
            SolverStage::ResultElements,
            SolverStage::ResultTotals,
        ] {
            cancelled(spatial, input.clone(), stage, 64);
        }
    }
}
