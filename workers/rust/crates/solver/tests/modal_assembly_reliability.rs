use kyuubiki_solver::solver_control::{SolverControl, SolverStage, with_solver_observer};
use kyuubiki_solver::{
    solve_modal_frame_2d, solve_modal_frame_2d_owned, solve_modal_frame_3d,
    solve_modal_frame_3d_owned,
};
use serde_json::{Value, json};

fn model(space: bool, length: f64, free_bending: bool) -> Value {
    let nodes: Vec<_> = (0..2)
        .map(|i| {
            let mut node = json!({"id":format!("n{i}"), "x":i as f64 * length,
                "y":0.0, "fix_x":i == 0, "fix_y":i == 0 || !free_bending,
                "fix_rz":i == 0 || !free_bending,
                "load_x":0.0, "load_y":0.0, "moment_z":0.0});
            if space {
                for key in ["z", "load_z", "moment_x", "moment_y"] {
                    node[key] = json!(0.0);
                }
                for key in ["fix_z", "fix_rx", "fix_ry"] {
                    node[key] = json!(i == 0 || !free_bending);
                }
            }
            node
        })
        .collect();
    let mut element = json!({"id":"beam", "node_i":0, "node_j":1,
        "area":1.0, "youngs_modulus":1.0, "density":1.0});
    if space {
        element["shear_modulus"] = json!(0.4);
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
    json!({"nodes":nodes, "elements":[element], "mode_count":6})
}

fn set_properties(input: &mut Value, space: bool, e: f64, density: f64, area: f64, inertia: f64) {
    let element = &mut input["elements"][0];
    element["youngs_modulus"] = json!(e);
    element["density"] = json!(density);
    element["area"] = json!(area);
    if space {
        element["shear_modulus"] = json!(e * 0.4);
        for key in [
            "torsion_constant",
            "moment_of_inertia_y",
            "moment_of_inertia_z",
        ] {
            element[key] = json!(inertia);
        }
    } else {
        element["moment_of_inertia"] = json!(inertia);
    }
}

fn solve(space: bool, input: Value, owned: bool) -> Result<Value, String> {
    if space {
        let request = serde_json::from_value(input).unwrap();
        let result = if owned {
            solve_modal_frame_3d_owned(request)
        } else {
            solve_modal_frame_3d(&request)
        };
        result.map(|value| serde_json::to_value(value).unwrap())
    } else {
        let request = serde_json::from_value(input).unwrap();
        let result = if owned {
            solve_modal_frame_2d_owned(request)
        } else {
            solve_modal_frame_2d(&request)
        };
        result.map(|value| serde_json::to_value(value).unwrap())
    }
}

fn close(actual: f64, expected: f64) {
    assert!(actual.is_finite() && expected.is_finite());
    assert!(
        (actual / expected - 1.0).abs() < 2.0e-10,
        "{actual:e} != {expected:e}"
    );
}

fn check_modes(result: &Value) {
    let free: Vec<_> = result["free_dofs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_u64().unwrap() as usize)
        .collect();
    for mode in result["modes"].as_array().unwrap() {
        let value = mode["eigenvalue_rad_s_squared"].as_f64().unwrap();
        let radians = mode["natural_frequency_rad_s"].as_f64().unwrap();
        let hz = mode["natural_frequency_hz"].as_f64().unwrap();
        let period = mode["period_s"].as_f64().unwrap();
        close(radians, value.sqrt());
        close(hz, radians / std::f64::consts::TAU);
        close(period * hz, 1.0);
        close(mode["participation_norm"].as_f64().unwrap(), 1.0);
        let mut norm = 0.0_f64;
        for (dof, value) in mode["shape"].as_array().unwrap().iter().enumerate() {
            let value = value.as_f64().expect("mode fields must remain numeric");
            norm = norm.hypot(value);
            if !free.contains(&dof) {
                assert_eq!(value, 0.0);
            }
        }
        close(norm, 1.0);
    }
}

fn check_axial(space: bool, input: Value, mass: f64, eigenvalue: f64) {
    let result = solve(space, input.clone(), false).unwrap();
    assert_eq!(result, solve(space, input, true).unwrap());
    assert_eq!(result["modes"].as_array().unwrap().len(), 1);
    close(result["total_mass"].as_f64().unwrap(), mass);
    close(
        result["modes"][0]["eigenvalue_rad_s_squared"]
            .as_f64()
            .unwrap(),
        eigenvalue,
    );
    check_modes(&result);
}

fn fixed_branch(space: bool, copies: usize) -> Value {
    let mut input = model(space, 1.0, false);
    let mut fixed = input["nodes"][0].clone();
    fixed["id"] = json!("fixed-root");
    fixed["y"] = json!(2.0);
    input["nodes"].as_array_mut().unwrap().push(fixed.clone());
    fixed["id"] = json!("fixed-tip");
    fixed["x"] = json!(1.0);
    input["nodes"].as_array_mut().unwrap().push(fixed);
    for i in 0..copies {
        let mut element = input["elements"][0].clone();
        element["id"] = json!(format!("fixed-{i}"));
        element["node_i"] = json!(2);
        element["node_j"] = json!(3);
        input["elements"].as_array_mut().unwrap().push(element);
    }
    input
}

#[test]
fn common_stiffness_and_density_scale_preserves_full_modal_spectrum() {
    for space in [false, true] {
        let reference = solve(space, model(space, 3.0, true), false).unwrap();
        for scale in [1.0e-200, 1.0, 1.0e200, 1.0e307] {
            let mut input = model(space, 3.0, true);
            set_properties(&mut input, space, scale, scale, 1.0, 0.01);
            let result = solve(space, input.clone(), false).unwrap();
            assert_eq!(result, solve(space, input, true).unwrap());
            close(result["total_mass"].as_f64().unwrap(), 3.0 * scale);
            let actual = result["modes"].as_array().unwrap();
            let expected = reference["modes"].as_array().unwrap();
            assert_eq!(actual.len(), if space { 6 } else { 3 });
            assert_eq!(actual.len(), expected.len());
            for (actual, expected) in actual.iter().zip(expected) {
                close(
                    actual["eigenvalue_rad_s_squared"].as_f64().unwrap(),
                    expected["eigenvalue_rad_s_squared"].as_f64().unwrap(),
                );
            }
            check_modes(&result);
        }
    }
}

#[test]
fn element_mass_recovers_finite_products_after_intermediate_range_loss() {
    for space in [false, true] {
        for (length, e, density, area, inertia, mass, eigenvalue) in [
            (
                1.0e100, 1.0e100, 1.0e-250, 1.0e-100, 1.0e100, 1.0e-250, 2.0e150,
            ),
            (
                1.0e-10, 1.0e-130, 1.0e160, 1.0e150, 1.0e-20, 1.0e300, 2.0e-270,
            ),
        ] {
            let mut input = model(space, length, false);
            set_properties(&mut input, space, e, density, area, inertia);
            check_axial(space, input, mass, eigenvalue);
        }
    }
}

#[test]
fn finite_long_member_length_does_not_require_representable_squared_length() {
    for space in [false, true] {
        let mut input = model(space, 1.0e155, false);
        set_properties(&mut input, space, 1.0e100, 1.0e-150, 1.0e-100, 1.0e100);
        check_axial(space, input, 1.0e-95, 2.0e-60);
    }
}

#[test]
fn constrained_rotations_cannot_hide_unrepresentable_rotary_mass() {
    for space in [false, true] {
        for (length, e, density, inertia) in [
            (1.0e110, 1.0, 0.1, 1.0e100),
            (1.0e-10, 1.0e-300, 1.0e-300, 1.0),
        ] {
            let mut input = model(space, length, false);
            set_properties(&mut input, space, e, density, 1.0, inertia);
            let error =
                solve(space, input, false).expect_err("constrained mass still needs validation");
            assert!(
                error.contains("nodal rotary mass") && error.contains("representable"),
                "{error}"
            );
        }
        check_axial(space, model(space, 1.0, false), 1.0, 2.0);
    }
}

#[test]
fn constraints_cannot_hide_underflowed_local_bending_stiffness() {
    for space in [false, true] {
        let mut input = model(space, 1.0, false);
        set_properties(&mut input, space, 1.0e-100, 1.0e-100, 1.0, 1.0e-300);
        let error = solve(space, input, false).expect_err("lost bending stiffness must fail");
        assert!(
            error.contains("stiffness") && error.contains("representable"),
            "{error}"
        );
        check_axial(space, model(space, 1.0, false), 1.0, 2.0);
    }
}

#[test]
fn fixed_components_cannot_hide_invalid_local_or_assembled_stiffness() {
    for space in [false, true] {
        for copies in [1, 2] {
            let mut input = fixed_branch(space, copies);
            for element in input["elements"].as_array_mut().unwrap().iter_mut().skip(1) {
                element["youngs_modulus"] = json!(1.0e308);
                if copies == 1 {
                    element[if space {
                        "moment_of_inertia_y"
                    } else {
                        "moment_of_inertia"
                    }] = json!(1.0);
                }
            }
            let error =
                solve(space, input, false).expect_err("invalid restrained stiffness must fail");
            assert!(
                error.contains("stiffness") && error.contains("representable"),
                "{error}"
            );
            assert!(
                error.contains(if copies == 1 {
                    "fixed-0: local"
                } else {
                    "assembled"
                }),
                "{error}"
            );
        }
        check_axial(space, model(space, 1.0, false), 1.0, 2.0);
    }
}

#[test]
fn fixed_components_cannot_hide_overflowed_assembled_rotary_mass() {
    for space in [false, true] {
        let mut input = fixed_branch(space, 3);
        input["nodes"][3]["x"] = json!(10.0);
        for element in input["elements"].as_array_mut().unwrap().iter_mut().skip(1) {
            element["density"] = json!(2.0e306);
        }
        let error = solve(space, input, false).expect_err("assembled fixed rotary mass must fail");
        assert!(
            error.contains("assembled mass") && error.contains("representable"),
            "{error}"
        );
        check_axial(space, model(space, 1.0, false), 1.0, 2.0);
    }
}

#[test]
fn large_common_scale_preserves_rotation_and_reversed_connectivity_spectrum() {
    for space in [false, true] {
        let reference = solve(space, model(space, 3.0, true), false).unwrap();
        let mut input = model(space, 3.0, true);
        set_properties(&mut input, space, 1.0e307, 1.0e307, 1.0, 0.01);
        if space {
            input["nodes"][1]["x"] = json!(2.0);
            input["nodes"][1]["y"] = json!(2.0);
            input["nodes"][1]["z"] = json!(1.0);
        } else {
            input["nodes"][1]["x"] = json!(1.8);
            input["nodes"][1]["y"] = json!(2.4);
        }
        for reverse in [false, true] {
            input["elements"][0]["node_i"] = json!(usize::from(reverse));
            input["elements"][0]["node_j"] = json!(usize::from(!reverse));
            let result = solve(space, input.clone(), false).unwrap();
            check_modes(&result);
            let actual = result["modes"].as_array().unwrap();
            let expected = reference["modes"].as_array().unwrap();
            assert_eq!(actual.len(), if space { 6 } else { 3 });
            assert_eq!(actual.len(), expected.len());
            for (actual, expected) in actual.iter().zip(expected) {
                close(
                    actual["eigenvalue_rad_s_squared"].as_f64().unwrap(),
                    expected["eigenvalue_rad_s_squared"].as_f64().unwrap(),
                );
            }
        }
    }
}

#[test]
fn full_modal_assembly_validation_cancels_before_reduction_and_replays() {
    for space in [false, true] {
        let base = model(space, 1.0, false);
        let mut nodes = Vec::new();
        let mut elements = Vec::new();
        for index in 0..40 {
            for node in base["nodes"].as_array().unwrap() {
                let mut node = node.clone();
                node["id"] = json!(format!("n{}", nodes.len()));
                node["y"] = json!(index as f64 * 2.0);
                nodes.push(node);
            }
            let mut element = base["elements"][0].clone();
            element["id"] = json!(format!("e{index}"));
            element["node_i"] = json!(index * 2);
            element["node_j"] = json!(index * 2 + 1);
            elements.push(element);
        }
        let input = json!({"nodes":nodes, "elements":elements, "mode_count":1});
        let control = SolverControl::default();
        let cancel = control.clone();
        let error = with_solver_observer(
            &control,
            move |point| {
                if point.stage == SolverStage::SparseValidateMatrix && point.completed_steps == 64 {
                    cancel.request_cancel();
                }
            },
            || {
                let result = solve(space, input.clone(), false);
                assert!(
                    result.is_err(),
                    "must cancel inside the solve, not only on scope exit"
                );
                result
            },
        )
        .unwrap_err();
        assert!(error.contains("cancel"), "{error}");
        let point = control.last_checkpoint().unwrap();
        assert_eq!(point.stage, SolverStage::SparseValidateMatrix);
        assert_eq!(point.completed_steps, 64);
        check_axial(space, input, 40.0, 2.0);
    }
}
