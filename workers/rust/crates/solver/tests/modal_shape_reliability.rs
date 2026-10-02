use kyuubiki_solver::solver_control::{SolverControl, SolverStage, with_solver_observer};
use kyuubiki_solver::{
    solve_modal_frame_2d, solve_modal_frame_2d_owned, solve_modal_frame_3d,
    solve_modal_frame_3d_owned,
};
use serde_json::{Value, json};

fn model(space: bool, combined: bool) -> Value {
    let mut nodes = Vec::new();
    let mut elements = Vec::new();
    for (branch, scale) in [1e307, 1e-318]
        .into_iter()
        .take(if combined { 2 } else { 1 })
        .enumerate()
    {
        for tip in [false, true] {
            let mut node = json!({
                "id":format!("b{branch}-{tip}"), "x":if tip {1.0} else {0.0},
                "y":branch as f64 * 2.0, "fix_x":!tip, "fix_y":!tip, "fix_rz":!tip,
                "load_x":0.0, "load_y":0.0, "moment_z":0.0
            });
            if space {
                for key in ["z", "load_z", "moment_x", "moment_y"] {
                    node[key] = json!(0.0);
                }
                for key in ["fix_z", "fix_rx", "fix_ry"] {
                    node[key] = json!(!tip);
                }
            }
            nodes.push(node);
        }
        let mut element = json!({
            "id":format!("beam{branch}"), "node_i":branch * 2, "node_j":branch * 2 + 1,
            "area":1.0, "youngs_modulus":scale, "density":scale
        });
        if space {
            element["shear_modulus"] = json!(0.4 * scale);
            element["torsion_constant"] = json!(0.01);
            element["moment_of_inertia_y"] = json!(0.02);
            element["moment_of_inertia_z"] = json!(0.01);
        } else {
            element["moment_of_inertia"] = json!(0.01);
            element["section_modulus"] = json!(0.1);
        }
        elements.push(element);
    }
    json!({"nodes":nodes, "elements":elements, "mode_count":12})
}

fn solve(space: bool, input: Value) -> Value {
    if space {
        serde_json::to_value(solve_modal_frame_3d(&serde_json::from_value(input).unwrap()).unwrap())
            .unwrap()
    } else {
        serde_json::to_value(solve_modal_frame_2d(&serde_json::from_value(input).unwrap()).unwrap())
            .unwrap()
    }
}

fn check_independence(space: bool) {
    let separate = solve(space, model(space, false));
    let combined = solve(space, model(space, true));
    let dofs = if space { 6 } else { 3 };
    let active: Vec<_> = combined["modes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|mode| {
            mode["shape"].as_array().unwrap()[dofs..2 * dofs]
                .iter()
                .any(|v| v.as_f64().unwrap() != 0.0)
        })
        .collect();
    let reference = separate["modes"].as_array().unwrap();
    assert_eq!(active.len(), reference.len());
    assert_eq!(active.len(), dofs);
    for (actual, expected) in active.into_iter().zip(reference) {
        let eigenvalue = |mode: &Value| mode["eigenvalue_rad_s_squared"].as_f64().unwrap();
        assert!((eigenvalue(actual) / eigenvalue(expected) - 1.0).abs() < 1e-12);
        for (a, b) in actual["shape"].as_array().unwrap()[..2 * dofs]
            .iter()
            .zip(expected["shape"].as_array().unwrap())
        {
            let (a, b) = (a.as_f64().unwrap(), b.as_f64().unwrap());
            assert!((a - b).abs() < 2e-13, "shape {a:.17e} != {b:.17e}");
        }
        for value in &actual["shape"].as_array().unwrap()[2 * dofs..] {
            assert_eq!(value.as_f64().unwrap(), 0.0);
        }
    }
    for mode in combined["modes"].as_array().unwrap() {
        let norm: f64 = mode["shape"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_f64().unwrap().powi(2))
            .sum();
        assert!((norm - 1.0).abs() < 2e-14);
        assert!((mode["participation_norm"].as_f64().unwrap() - 1.0).abs() < 2e-14);
    }
}

#[test]
fn planar_mode_shapes_are_independent_of_inactive_light_members() {
    check_independence(false);
}

#[test]
fn spatial_mode_shapes_are_independent_of_inactive_light_members() {
    check_independence(true);
}

#[test]
fn heavy_planar_bending_shapes_match_the_discrete_closed_form() {
    let result = solve(false, model(false, true));
    let bending: Vec<_> = result["modes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|mode| mode["shape"][4].as_f64().unwrap() != 0.0)
        .collect();
    assert_eq!(bending.len(), 2);
    for (mode, ratio) in bending
        .into_iter()
        .zip([21.0_f64.sqrt() - 3.0, -21.0_f64.sqrt() - 3.0])
    {
        let displacement = mode["shape"][4].as_f64().unwrap();
        let rotation = mode["shape"][5].as_f64().unwrap();
        assert!((rotation / displacement - ratio).abs() < 2e-13);
        assert!((displacement.abs() - 1.0 / ratio.hypot(1.0)).abs() < 2e-13);
    }
}

#[test]
fn heterogeneous_mass_mode_shapes_match_owned_and_borrowed_routes() {
    for space in [false, true] {
        let input = model(space, true);
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
        assert_eq!(owned, solve(space, input));
    }
}

#[test]
fn heterogeneous_mass_shapes_follow_node_permutation_up_to_mode_sign() {
    for space in [false, true] {
        let mut input = model(space, true);
        // Avoid repeated cross-component axial eigenvalues: their order is not unique.
        input["elements"][1]["youngs_modulus"] = json!(3e-318);
        if space {
            input["elements"][1]["shear_modulus"] = json!(1.2e-318);
        }
        let reference = solve(space, input.clone());
        input["nodes"].as_array_mut().unwrap().reverse();
        for element in input["elements"].as_array_mut().unwrap() {
            element["node_i"] = json!(3 - element["node_i"].as_u64().unwrap());
            element["node_j"] = json!(3 - element["node_j"].as_u64().unwrap());
        }
        let actual = solve(space, input);
        let dofs = if space { 6 } else { 3 };
        for (a, b) in actual["modes"]
            .as_array()
            .unwrap()
            .iter()
            .zip(reference["modes"].as_array().unwrap())
        {
            let mut dot = 0.0;
            for node in 0..4 {
                for dof in 0..dofs {
                    dot += a["shape"][(3 - node) * dofs + dof].as_f64().unwrap()
                        * b["shape"][node * dofs + dof].as_f64().unwrap();
                }
            }
            assert!(
                (dot.abs() - 1.0).abs() < 2e-13,
                "modal direction changed: {dot}"
            );
        }
    }
}

#[test]
fn public_modal_recovery_cancellation_has_no_partial_success_and_replays() {
    for space in [false, true] {
        for stage in [
            SolverStage::ResultNodes,
            SolverStage::ResultNodeSummary,
            SolverStage::ResultTotals,
        ] {
            let control = SolverControl::default();
            let cancel = control.clone();
            let input = model(space, true);
            let error = with_solver_observer(
                &control,
                move |point| {
                    if point.stage == stage && point.completed_steps > 0 {
                        cancel.request_cancel();
                    }
                },
                || {
                    let result = if space {
                        solve_modal_frame_3d(&serde_json::from_value(input.clone()).unwrap())
                            .map(|_| ())
                    } else {
                        solve_modal_frame_2d(&serde_json::from_value(input.clone()).unwrap())
                            .map(|_| ())
                    };
                    assert!(
                        result.is_err(),
                        "public solve must observe recovery cancellation"
                    );
                    result
                },
            )
            .unwrap_err();
            assert!(error.contains("cancel"), "{error}");
            assert_eq!(control.last_checkpoint().unwrap().stage, stage);
            check_independence(space);
        }
    }
}
