use kyuubiki_solver::{
    solve_modal_frame_2d, solve_modal_frame_2d_owned, solve_modal_frame_3d,
    solve_modal_frame_3d_owned,
};
use serde_json::{Value, json};

#[path = "modal_chain_reliability/range.rs"]
mod range;

#[path = "modal_chain_reliability/preparation.rs"]
mod preparation;

fn axial_chain(space: bool, stiffness: [f64; 2], density: f64, mode_count: usize) -> Value {
    let nodes: Vec<_> = (0..3)
        .map(|index| {
            let mut node = json!({
                "id":format!("node{index}"), "x":index as f64, "y":0.0,
                "fix_x":index == 0, "fix_y":true, "fix_rz":true,
                "load_x":0.0, "load_y":0.0, "moment_z":0.0
            });
            if space {
                for key in ["z", "load_z", "moment_x", "moment_y"] {
                    node[key] = json!(0.0);
                }
                for key in ["fix_z", "fix_rx", "fix_ry"] {
                    node[key] = json!(true);
                }
            }
            node
        })
        .collect();
    let elements: Vec<_> = stiffness
        .into_iter()
        .enumerate()
        .map(|(index, modulus)| {
            let mut element = json!({
                "id":format!("beam{index}"), "node_i":index, "node_j":index + 1,
                "area":1.0, "density":density, "youngs_modulus":modulus
            });
            if space {
                element["shear_modulus"] = json!(0.4 * modulus);
                for key in [
                    "moment_of_inertia_y",
                    "moment_of_inertia_z",
                    "torsion_constant",
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
    json!({"nodes":nodes, "elements":elements, "mode_count":mode_count})
}

fn solve(space: bool, input: Value) -> Result<Value, String> {
    if space {
        solve_modal_frame_3d(&serde_json::from_value(input).unwrap())
            .map(|x| serde_json::to_value(x).unwrap())
    } else {
        solve_modal_frame_2d(&serde_json::from_value(input).unwrap())
            .map(|x| serde_json::to_value(x).unwrap())
    }
}

fn eigenvalue(result: &Value) -> f64 {
    result["modes"][0]["eigenvalue_rad_s_squared"]
        .as_f64()
        .unwrap()
}

#[test]
fn large_axial_chain_single_mode_matches_a_finite_complete_spectrum() {
    for space in [false, true] {
        let full = solve(space, axial_chain(space, [5e307; 2], 1.0, 2)).unwrap();
        let single = solve(space, axial_chain(space, [5e307; 2], 1.0, 1)).unwrap();
        assert_eq!(full["modes"].as_array().unwrap().len(), 2);
        let expected = 5e307 * (2.0 - 2.0_f64.sqrt());
        assert!((eigenvalue(&single) / expected - 1.0).abs() < 1e-12);
        assert!((eigenvalue(&single) / eigenvalue(&full) - 1.0).abs() < 1e-12);
    }
}

#[test]
fn tiny_nonuniform_axial_chain_single_mode_matches_complete_spectrum() {
    for space in [false, true] {
        for right_modulus in [2e-318, 1e-318 + 2.0 * f64::from_bits(1)] {
            let stiffness = [1e-318, right_modulus];
            let full = solve(space, axial_chain(space, stiffness, 1e-318, 2)).unwrap();
            let single = solve(space, axial_chain(space, stiffness, 1e-318, 1)).unwrap();
            assert!((eigenvalue(&single) / eigenvalue(&full) - 1.0).abs() < 1e-10);
        }
    }
}

#[test]
fn axial_chain_frequency_and_shape_agree_across_common_scales() {
    for space in [false, true] {
        for scale in [1e-318, 1e-200, 1.0, 1e200] {
            let full = solve(space, axial_chain(space, [scale; 2], scale, 2)).unwrap();
            let single = solve(space, axial_chain(space, [scale; 2], scale, 1)).unwrap();
            assert!((eigenvalue(&single) / eigenvalue(&full) - 1.0).abs() < 1e-10);
            let mode = &single["modes"][0];
            let hz = mode["natural_frequency_hz"].as_f64().unwrap();
            let period = mode["period_s"].as_f64().unwrap();
            assert!((hz * period - 1.0).abs() < 1e-12);
            assert!((mode["participation_norm"].as_f64().unwrap() - 1.0).abs() < 1e-12);
            let dot: f64 = mode["shape"]
                .as_array()
                .unwrap()
                .iter()
                .zip(full["modes"][0]["shape"].as_array().unwrap())
                .map(|(a, b)| a.as_f64().unwrap() * b.as_f64().unwrap())
                .sum();
            assert!((dot.abs() - 1.0).abs() < 1e-12);
        }
    }
}

#[test]
fn tiny_chain_with_heterogeneous_density_uses_the_correct_spectrum() {
    for space in [false, true] {
        let scale = f64::from_bits(1 << 22);
        let mut input = axial_chain(space, [scale; 2], scale, 2);
        input["elements"][1]["density"] = json!(3.0 * scale);
        let full = solve(space, input.clone()).unwrap();
        input["mode_count"] = json!(1);
        let single = solve(space, input).unwrap();
        assert!((eigenvalue(&single) / eigenvalue(&full) - 1.0).abs() < 1e-12);
        let expected = (5.0 - 13.0_f64.sqrt()) / 6.0;
        assert!((eigenvalue(&single) / expected - 1.0).abs() < 1e-12);
    }
}

#[test]
fn axial_chain_node_order_and_direction_do_not_change_the_lowest_mode() {
    for space in [false, true] {
        for (stiffness, density) in [([5e307; 2], 1.0), ([1e-318, 2e-318], 1e-318)] {
            let mut input = axial_chain(space, stiffness, density, 1);
            let reference = solve(space, input.clone()).unwrap();
            input["nodes"].as_array_mut().unwrap().reverse();
            input["elements"].as_array_mut().unwrap().reverse();
            for element in input["elements"].as_array_mut().unwrap() {
                let i = element["node_i"].as_u64().unwrap();
                let j = element["node_j"].as_u64().unwrap();
                element["node_i"] = json!(2 - j);
                element["node_j"] = json!(2 - i);
            }
            let result = solve(space, input).unwrap();
            assert!((eigenvalue(&result) / eigenvalue(&reference) - 1.0).abs() < 1e-10);
            let dofs = if space { 6 } else { 3 };
            let dot: f64 = (0..3)
                .flat_map(|node| (0..dofs).map(move |dof| (node, dof)))
                .map(|(node, dof)| {
                    result["modes"][0]["shape"][(2 - node) * dofs + dof]
                        .as_f64()
                        .unwrap()
                        * reference["modes"][0]["shape"][node * dofs + dof]
                            .as_f64()
                            .unwrap()
                })
                .sum();
            assert!((dot.abs() - 1.0).abs() < 1e-12);
        }
    }
}

#[test]
fn axial_chain_owned_and_borrowed_entry_points_agree() {
    for space in [false, true] {
        for (stiffness, density) in [([5e307; 2], 1.0), ([1e-318, 2e-318], 1e-318)] {
            let input = axial_chain(space, stiffness, density, 1);
            let owned = if space {
                serde_json::to_value(
                    solve_modal_frame_3d_owned(serde_json::from_value(input.clone()).unwrap())
                        .unwrap(),
                )
                .unwrap()
            } else {
                serde_json::to_value(
                    solve_modal_frame_2d_owned(serde_json::from_value(input.clone()).unwrap())
                        .unwrap(),
                )
                .unwrap()
            };
            assert_eq!(owned, solve(space, input).unwrap());
        }
    }
}

#[test]
fn unrepresentable_axial_frequency_is_explicit_and_a_valid_chain_replays() {
    for space in [false, true] {
        let error = solve(space, axial_chain(space, [1e-200; 2], 1e200, 1)).unwrap_err();
        assert!(
            error.contains("eigenvalue") && error.contains("representable"),
            "{error}"
        );
        assert!(solve(space, axial_chain(space, [5e307; 2], 1.0, 1)).is_ok());
    }
}

#[test]
fn soft_axial_segment_single_mode_matches_the_analytic_reference() {
    for space in [false, true] {
        for soft in [1e-24, 1e-100, 1e-200] {
            let input = axial_chain(space, [1.0, soft], 1.0, 1);
            let single = solve(space, input)
                .unwrap_or_else(|error| panic!("space={space}, soft={soft}: {error}"));
            assert!((eigenvalue(&single) / (2.0 * soft) - 1.0).abs() < 1e-12);
        }
    }
}

#[test]
fn nonuniform_chain_single_mode_matches_full_spectrum_in_resolved_range() {
    for space in [false, true] {
        for soft in [1e-2, 1e-4, 1e-8] {
            let input = axial_chain(space, [1.0, soft], 1.0, 2);
            let full = solve(space, input.clone()).unwrap();
            let mut single = input;
            single["mode_count"] = json!(1);
            let single = solve(space, single).unwrap();
            let high = 0.5 * (1.0 + 3.0 * soft + (1.0 - soft).hypot(2.0_f64.sqrt() * 2.0 * soft));
            let expected = 2.0 * soft / high;
            assert!((eigenvalue(&single) / expected - 1.0).abs() < 1e-12);
            assert!((eigenvalue(&single) / eigenvalue(&full) - 1.0).abs() < 1e-12);
        }
    }
}

#[test]
fn soft_segment_modes_preserve_physical_shape_under_reversed_node_order() {
    for space in [false, true] {
        for soft in [1e-24, 1e-100, 1e-200] {
            let mut input = axial_chain(space, [1.0, soft], 1.0, 1);
            let reference = solve(space, input.clone()).unwrap();
            input["nodes"].as_array_mut().unwrap().reverse();
            for element in input["elements"].as_array_mut().unwrap() {
                let i = element["node_i"].as_u64().unwrap();
                let j = element["node_j"].as_u64().unwrap();
                element["node_i"] = json!(2 - j);
                element["node_j"] = json!(2 - i);
            }
            let actual = solve(space, input).unwrap();
            assert!((eigenvalue(&actual) / eigenvalue(&reference) - 1.0).abs() < 1e-12);
            let dofs = if space { 6 } else { 3 };
            let original = &reference["modes"][0]["shape"];
            let reordered = &actual["modes"][0]["shape"];
            for shape in [original, reordered] {
                let middle = shape[dofs].as_f64().unwrap();
                assert!((middle.abs() / soft - 1.0).abs() < 1e-12);
            }
            assert_eq!(original[0], json!(0.0));
            assert_eq!(reordered[2 * dofs], json!(0.0));
            assert!((original[2 * dofs].as_f64().unwrap().abs() - 1.0).abs() < 1e-12);
            assert!((reordered[0].as_f64().unwrap().abs() - 1.0).abs() < 1e-12);
        }
    }
}

#[test]
fn extreme_contrast_complete_axial_spectra_match_analytic_modes_and_single_mode() {
    for space in [false, true] {
        for soft in [1e-24, 1e-100, 1e-200, 1e-300] {
            let full = solve(space, axial_chain(space, [1.0, soft], 1.0, 2))
                .unwrap_or_else(|error| panic!("space={space}, soft={soft:e}: {error}"));
            let single = solve(space, axial_chain(space, [1.0, soft], 1.0, 1)).unwrap();
            assert_eq!(full["modes"].as_array().unwrap().len(), 2);
            assert!((eigenvalue(&full) / (2.0 * soft) - 1.0).abs() < 1e-12);
            assert!((eigenvalue(&full) / eigenvalue(&single) - 1.0).abs() < 1e-12);
            let high = full["modes"][1]["eigenvalue_rad_s_squared"]
                .as_f64()
                .unwrap();
            assert!((high - 1.0).abs() < 1e-12);
            let dofs = if space { 6 } else { 3 };
            let middle = full["modes"][0]["shape"][dofs].as_f64().unwrap();
            assert!((middle.abs() / soft - 1.0).abs() < 1e-12);
            let tip = full["modes"][0]["shape"][2 * dofs].as_f64().unwrap();
            assert!((tip.abs() - 1.0).abs() < 1e-12);
            assert!(middle * tip > 0.0);
            let high_middle = full["modes"][1]["shape"][dofs].as_f64().unwrap();
            let high_tip = full["modes"][1]["shape"][2 * dofs].as_f64().unwrap();
            assert!((high_middle.abs() - 1.0).abs() < 1e-12);
            assert!((high_tip.abs() / (2.0 * soft) - 1.0).abs() < 1e-12);
            assert!(high_middle * high_tip < 0.0);
        }
    }
}

#[test]
fn extreme_contrast_complete_chain_modes_follow_node_and_member_reversal() {
    for space in [false, true] {
        for soft in [1e-24, 1e-100, 1e-200] {
            let mut input = axial_chain(space, [1.0, soft], 1.0, 2);
            let baseline = solve(space, input.clone()).unwrap();
            input["nodes"].as_array_mut().unwrap().reverse();
            input["elements"].as_array_mut().unwrap().reverse();
            for element in input["elements"].as_array_mut().unwrap() {
                let i = element["node_i"].as_u64().unwrap();
                let j = element["node_j"].as_u64().unwrap();
                element["node_i"] = json!(2 - j);
                element["node_j"] = json!(2 - i);
            }
            let reversed = solve(space, input).unwrap();
            let dofs = if space { 6 } else { 3 };
            for mode in 0..2 {
                let reference = &baseline["modes"][mode];
                let actual = &reversed["modes"][mode];
                let a = actual["eigenvalue_rad_s_squared"].as_f64().unwrap();
                let b = reference["eigenvalue_rad_s_squared"].as_f64().unwrap();
                assert!((a / b - 1.0).abs() < 1e-12);
                let anchor = if mode == 0 { 2 * dofs } else { dofs };
                let reference_sign = reference["shape"][anchor].as_f64().unwrap().signum();
                let actual_sign = actual["shape"][2 * dofs - anchor]
                    .as_f64()
                    .unwrap()
                    .signum();
                for index in 0..3 * dofs {
                    let mirrored = (2 - index / dofs) * dofs + index % dofs;
                    let a = actual["shape"][mirrored].as_f64().unwrap() * actual_sign;
                    let b = reference["shape"][index].as_f64().unwrap() * reference_sign;
                    if b == 0.0 {
                        assert_eq!(a, 0.0);
                    } else {
                        assert!((a / b - 1.0).abs() < 1e-12);
                    }
                }
            }
        }
    }
}

#[test]
fn complete_soft_chain_modes_preserve_common_material_scaling() {
    for space in [false, true] {
        for scale in [1e-200, 1.0, 1e200] {
            let full = solve(space, axial_chain(space, [scale, 1e-100 * scale], scale, 2)).unwrap();
            assert!((eigenvalue(&full) / 2e-100 - 1.0).abs() < 1e-12);
            let dofs = if space { 6 } else { 3 };
            let middle = full["modes"][0]["shape"][dofs].as_f64().unwrap();
            assert!((middle.abs() / 1e-100 - 1.0).abs() < 1e-12);
            for mode in full["modes"].as_array().unwrap() {
                let hz = mode["natural_frequency_hz"].as_f64().unwrap();
                let period = mode["period_s"].as_f64().unwrap();
                assert!((hz * period - 1.0).abs() < 1e-12);
                assert!((mode["participation_norm"].as_f64().unwrap() - 1.0).abs() < 1e-12);
            }
        }
    }
}

#[test]
fn complete_soft_spectrum_owned_borrowed_and_default_mode_requests_agree() {
    for space in [false, true] {
        let mut input = axial_chain(space, [1.0, 1e-100], 1.0, 2);
        let mut explicit = solve(space, input.clone()).unwrap();
        input.as_object_mut().unwrap().remove("mode_count");
        let owned = if space {
            serde_json::to_value(
                solve_modal_frame_3d_owned(serde_json::from_value(input.clone()).unwrap()).unwrap(),
            )
            .unwrap()
        } else {
            serde_json::to_value(
                solve_modal_frame_2d_owned(serde_json::from_value(input.clone()).unwrap()).unwrap(),
            )
            .unwrap()
        };
        let mut borrowed = solve(space, input).unwrap();
        assert_eq!(borrowed, owned);
        assert_eq!(explicit["input"]["mode_count"], json!(2));
        assert_eq!(borrowed["input"]["mode_count"], Value::Null);
        explicit.as_object_mut().unwrap().remove("input");
        borrowed.as_object_mut().unwrap().remove("input");
        assert_eq!(explicit, borrowed);
    }
}
