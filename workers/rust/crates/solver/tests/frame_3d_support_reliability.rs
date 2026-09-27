use kyuubiki_solver::solver_control::{SolverControl, SolverStage, with_solver_observer};
use kyuubiki_solver::{solve_thermal_frame_3d, solve_thermal_frame_3d_owned};
use serde_json::{Value, json};

fn model() -> Value {
    let nodes: Vec<_> = (0..2)
        .map(|i| {
            let mut n = json!({"id": format!("n{i}"), "x": 2.0*i as f64,
                "y": 0.0, "z": 0.0, "temperature_delta": 0.0});
            for field in ["fix_x", "fix_y", "fix_z", "fix_rx", "fix_ry", "fix_rz"] {
                n[field] = json!(i == 0);
            }
            for field in [
                "load_x", "load_y", "load_z", "moment_x", "moment_y", "moment_z",
            ] {
                n[field] = json!(0.0);
            }
            n
        })
        .collect();
    json!({"nodes": nodes, "elements": [{"id": "member", "node_i": 0, "node_j": 1,
        "local_y_axis": [0.0, 1.0, 0.0], "youngs_modulus": 1000.0, "area": 2.0,
        "shear_modulus": 400.0, "torsion_constant": 0.5, "moment_of_inertia_y": 2.0,
        "moment_of_inertia_z": 1.0, "section_modulus_y": 2.0, "section_modulus_z": 1.0,
        "thermal_expansion": 1e-5, "section_depth_y": 0.2, "section_depth_z": 0.3,
        "temperature_gradient_y": 0.0, "temperature_gradient_z": 0.0}]})
}

fn solve(input: Value, owned: bool) -> Result<Value, String> {
    let request = serde_json::from_value(input).unwrap();
    let result = if owned {
        solve_thermal_frame_3d_owned(request)
    } else {
        solve_thermal_frame_3d(&request)
    }?;
    Ok(serde_json::to_value(result).unwrap())
}

fn number(value: &Value) -> f64 {
    let value = value.as_f64().expect("physical output must be numeric");
    assert!(value.is_finite());
    value
}

fn close(actual: f64, expected: f64, tolerance: f64) {
    assert!(actual.is_finite());
    assert!(
        (actual - expected).abs() <= tolerance,
        "actual={actual:e}, expected={expected:e}, tolerance={tolerance:e}"
    );
}

fn constraints(rotational: bool) -> &'static str {
    if rotational {
        "directional_rotational_constraints"
    } else {
        "directional_constraints"
    }
}

fn reaction(rotational: bool) -> &'static str {
    if rotational {
        "reaction_moment"
    } else {
        "reaction_force"
    }
}

fn constrained_response(rotational: bool) -> &'static str {
    if rotational {
        "rotation"
    } else {
        "displacement"
    }
}

fn pair(input: &mut Value, rotational: bool, epsilon: f64) {
    input[constraints(rotational)] = json!([
        {"id": "axis", "node": 1, "direction": [1.0, 0.0, 0.0]},
        {"id": "near-axis", "node": 1, "direction": [1.0, epsilon, 0.0]}
    ]);
    input["nodes"][1][if rotational { "moment_y" } else { "load_y" }] = json!(10.0);
}

#[test]
fn independent_near_parallel_supports_recover_analytic_reactions() {
    for rotational in [false, true] {
        for epsilon in [1e-3_f64, 1e-5, 1e-7, 1e-9, 2e-10] {
            let mut input = model();
            pair(&mut input, rotational, epsilon);
            let result = solve(input.clone(), false).unwrap();
            assert_eq!(result, solve(input, true).unwrap());
            let supports = &result[constraints(rotational)];
            let expected = 10.0 / epsilon;
            close(
                number(&supports[0][reaction(rotational)]),
                expected,
                expected * 2e-12,
            );
            close(
                number(&supports[1][reaction(rotational)]),
                -expected * epsilon.hypot(1.0),
                expected * 2e-12,
            );
            for support in supports.as_array().unwrap() {
                close(
                    number(&support[constrained_response(rotational)]),
                    0.0,
                    1e-14,
                );
            }
            close(number(&result["total_strain_energy"]), 0.0, 1e-20);
        }
    }
}

#[test]
fn fixed_axes_and_near_parallel_supports_share_reaction_basis() {
    for rotational in [false, true] {
        let mut input = model();
        pair(&mut input, rotational, 1e-9);
        input[constraints(rotational)]
            .as_array_mut()
            .unwrap()
            .remove(0);
        input["nodes"][1][if rotational { "fix_rx" } else { "fix_x" }] = json!(true);
        let result = solve(input, false).unwrap();
        close(
            number(&result[constraints(rotational)][0][reaction(rotational)]),
            -1e10,
            0.02,
        );
        close(number(&result["total_strain_energy"]), 0.0, 1e-20);
    }
}

fn rotate(v: [f64; 3]) -> [f64; 3] {
    [[0.36, -0.8, 0.48], [0.48, 0.6, 0.64], [-0.8, 0.0, 0.6]]
        .map(|row| row.into_iter().zip(v).map(|(a, b)| a * b).sum())
}

#[test]
fn rotated_support_plane_preserves_free_motion_energy_and_reaction_balance() {
    let epsilon = 1e-8;
    let mut input = model();
    pair(&mut input, false, epsilon);
    input["nodes"][1]["load_z"] = json!(3.0);
    let expected_energy = 0.5 * 3.0 * (3.0 * 8.0 / (3.0 * 2000.0));
    let x = rotate([2.0, 0.0, 0.0]);
    let load = rotate([0.0, 10.0, 3.0]);
    for ((coordinate, force), index) in ["x", "y", "z"]
        .into_iter()
        .zip(["load_x", "load_y", "load_z"])
        .zip(0..3)
    {
        input["nodes"][1][coordinate] = json!(x[index]);
        input["nodes"][1][force] = json!(load[index]);
    }
    input["elements"][0]["local_y_axis"] = json!(rotate([0.0, 1.0, 0.0]));
    input["directional_constraints"][0]["direction"] = json!(rotate([1.0, 0.0, 0.0]));
    input["directional_constraints"][1]["direction"] = json!(rotate([1.0, epsilon, 0.0]));
    for reverse in [false, true] {
        if reverse {
            input["directional_constraints"]
                .as_array_mut()
                .unwrap()
                .reverse();
        }
        let result = solve(input.clone(), false).unwrap();
        close(
            number(&result["total_strain_energy"]),
            expected_energy,
            expected_energy * 2e-6,
        );
        let expected = rotate([0.0, 0.0, 0.004]);
        for (index, field) in ["ux", "uy", "uz"].into_iter().enumerate() {
            close(number(&result["nodes"][1][field]), expected[index], 1e-8);
        }
        let mut balance = [0.0; 3];
        for support in result["directional_constraints"].as_array().unwrap() {
            close(number(&support["displacement"]), 0.0, 1e-13);
            for (i, sum) in balance.iter_mut().enumerate() {
                *sum += number(&support["reaction_force"]) * number(&support["direction"][i]);
            }
        }
        for (actual, expected) in balance.into_iter().zip(rotate([0.0, -10.0, 0.0])) {
            close(actual, expected, 2e-6);
        }
    }
}

#[test]
fn three_oblique_supports_remove_all_three_physical_dofs() {
    let epsilon = 1e-8;
    for rotational in [false, true] {
        let mut input = model();
        input[constraints(rotational)] = json!(
            [[1.0, 0.0, 0.0], [1.0, epsilon, 0.0], [0.0, 0.0, 1.0]]
                .into_iter()
                .enumerate()
                .map(|(i, v)| json!({
                    "id": format!("support-{i}"), "node": 1, "direction": rotate(v)
                }))
                .collect::<Vec<_>>()
        );
        let load = rotate([0.0, 10.0, 3.0]);
        let fields = if rotational {
            ["moment_x", "moment_y", "moment_z"]
        } else {
            ["load_x", "load_y", "load_z"]
        };
        for (field, value) in fields.into_iter().zip(load) {
            input["nodes"][1][field] = json!(value);
        }
        let result = solve(input, false).unwrap();
        for field in ["ux", "uy", "uz", "rx", "ry", "rz"] {
            close(number(&result["nodes"][1][field]), 0.0, 1e-14);
        }
        close(number(&result["total_strain_energy"]), 0.0, 1e-20);
    }
}

#[test]
fn dependent_supports_reject_without_poisoning_a_valid_replay() {
    for rotational in [false, true] {
        for epsilon in [0.0, 1e-12, 0.5e-10] {
            let mut input = model();
            pair(&mut input, rotational, epsilon);
            let error = solve(input, false).unwrap_err();
            assert!(error.contains("linearly dependent"), "{error}");
            let mut valid = model();
            pair(&mut valid, rotational, 0.1);
            solve(valid, false).unwrap();
        }
    }
}

#[test]
fn four_support_directions_reject_before_constructing_a_negative_free_dimension() {
    let mut input = model();
    input["directional_constraints"] = json!(
        [
            [1.0, 0.0, 0.0],
            [1.0, 1e-8, 0.0],
            [0.0, 0.0, 1.0],
            [0.0, 1.0, 0.0]
        ]
        .into_iter()
        .enumerate()
        .map(|(i, v)| json!({
            "id": format!("support-{i}"), "node": 1, "direction": rotate(v)
        }))
        .collect::<Vec<_>>()
    );
    let result = std::panic::catch_unwind(|| solve(input, false));
    let error = result
        .expect("invalid supports must not panic")
        .unwrap_err();
    assert!(error.contains("linearly dependent"), "{error}");
}

#[test]
fn support_reparameterization_preserves_thermal_and_spring_response() {
    let mut input = model();
    pair(&mut input, false, 1.0);
    input["directional_constraints"][1]["direction"] = json!([0.0, 1.0, 0.0]);
    input["nodes"][1]["load_z"] = json!(3.0);
    input["nodes"][0]["temperature_delta"] = json!(75.0);
    input["nodes"][1]["temperature_delta"] = json!(75.0);
    input["elements"][0]["temperature_gradient_y"] = json!(5.0);
    input["elements"][0]["temperature_gradient_z"] = json!(-9.0);
    input["directional_springs"] = json!([
        {"id": "spring", "node": 1, "direction": [2.0, 3.0, 4.0], "stiffness": 300.0}
    ]);
    input["directional_rotational_springs"] = json!([
        {"id": "torsion-spring", "node": 1, "direction": [2.0, -3.0, 5.0], "stiffness": 200.0}
    ]);
    let expected = solve(input.clone(), false).unwrap();
    for scale in [1.0, -1.0, 1e200, -1e308] {
        input["directional_constraints"][1]["direction"] = json!([scale, scale * 1e-9, 0.0]);
        let result = solve(input.clone(), false).unwrap();
        for key in [
            "nodes",
            "elements",
            "directional_springs",
            "directional_rotational_springs",
            "total_strain_energy",
            "max_displacement",
            "max_rotation",
            "max_stress",
        ] {
            assert_eq!(
                result[key], expected[key],
                "support coordinates must not change {key}"
            );
        }
        let vector_sum = |result: &Value, component: usize| {
            result["directional_constraints"]
                .as_array()
                .unwrap()
                .iter()
                .map(|s| number(&s["reaction_force"]) * number(&s["direction"][component]))
                .sum::<f64>()
        };
        for component in 0..3 {
            close(
                vector_sum(&result, component),
                vector_sum(&expected, component),
                5e-6,
            );
        }
    }
}

#[test]
fn unrepresentable_support_reactions_fail_and_finite_loads_replay() {
    for rotational in [false, true] {
        for owned in [false, true] {
            let mut input = model();
            pair(&mut input, rotational, 1e-9);
            let field = if rotational { "moment_y" } else { "load_y" };
            input["nodes"][1][field] = json!(1e300);
            let error = solve(input.clone(), owned).unwrap_err();
            assert!(
                error.contains("constraint reaction") && error.contains("representable"),
                "{error}"
            );
            input["nodes"][1][field] = json!(10.0);
            solve(input, owned).unwrap();
        }
    }
}

#[test]
fn fixed_supports_cannot_hide_spring_assembly_overflow() {
    for rotational in [false, true] {
        let mut input = model();
        let field = if rotational {
            "directional_rotational_springs"
        } else {
            "directional_springs"
        };
        input[field] = json!(
            (0..3)
                .map(|i| json!({"id": format!("spring-{i}"),
            "node": 0, "direction": [1.0, 0.0, 0.0], "stiffness": 1e308}))
                .collect::<Vec<_>>()
        );
        let error = solve(input, false).unwrap_err();
        assert!(
            error.contains("assembled stiffness") && error.contains("representable"),
            "{error}"
        );
        solve(model(), false).unwrap();
    }
}

#[test]
fn finite_member_and_spring_energy_groups_cannot_overflow_total() {
    let mut input = model();
    for field in ["youngs_modulus", "shear_modulus"] {
        input["elements"][0][field] = json!(2.0);
    }
    for field in [
        "area",
        "torsion_constant",
        "moment_of_inertia_y",
        "moment_of_inertia_z",
    ] {
        input["elements"][0][field] = json!(1.0);
    }
    for field in ["fix_y", "fix_z", "fix_ry", "fix_rz"] {
        input["nodes"][1][field] = json!(true);
    }
    for field in ["load_x", "moment_x"] {
        input["nodes"][1][field] = json!(1e154);
    }
    for field in ["directional_springs", "directional_rotational_springs"] {
        input[field] =
            json!([{"id": field, "node": 1, "direction": [1.0, 0.0, 0.0], "stiffness": 1.0}]);
    }
    let finite = solve(input.clone(), false).unwrap();
    close(
        number(&finite["elements"][0]["strain_energy"]),
        2.5e307,
        1e295,
    );
    for field in ["directional_springs", "directional_rotational_springs"] {
        close(number(&finite[field][0]["strain_energy"]), 1.25e307, 1e295);
    }
    close(number(&finite["total_strain_energy"]), 5e307, 1e295);
    for field in ["load_x", "moment_x"] {
        input["nodes"][1][field] = json!(2e154);
    }
    let error = solve(input, false).unwrap_err();
    assert!(
        error.contains("summary: total strain energy is not representable"),
        "{error}"
    );
    solve(model(), false).unwrap();
}

#[test]
fn support_reaction_factorization_and_substitution_cancel_and_replay() {
    let mut input = model();
    pair(&mut input, false, 1e-9);
    let expected = solve(input.clone(), false).unwrap();
    for stage in [SolverStage::DenseFactor, SolverStage::DenseSubstitution] {
        for steps in [0, 2, 3] {
            let control = SolverControl::default();
            let cancel = control.clone();
            let result = with_solver_observer(
                &control,
                move |p| {
                    if p.stage == stage && p.completed_steps == steps {
                        cancel.request_cancel();
                    }
                },
                || {
                    let raw = solve(input.clone(), false);
                    assert!(raw.is_err(), "raw solve ignored {stage:?} at {steps}");
                    raw
                },
            );
            assert!(result.unwrap_err().starts_with("solver cancelled"));
            assert_eq!(solve(input.clone(), false).unwrap(), expected);
        }
    }
}
