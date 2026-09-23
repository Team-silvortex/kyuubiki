use kyuubiki_solver::solver_control::{SolverControl, SolverStage, with_solver_observer};
use kyuubiki_solver::{solve_modal_frame_2d, solve_modal_frame_3d};
use serde_json::{Value, json};

fn frame_node(space: bool, id: &str, x: f64, y: f64, fixed: bool) -> Value {
    let mut node = json!({"id":id, "x":x, "y":y,
        "fix_x":fixed, "fix_y":true, "fix_rz":true,
        "load_x":0.0, "load_y":0.0, "moment_z":0.0});
    if space {
        for key in ["z", "load_z", "moment_x", "moment_y"] {
            node[key] = json!(0.0);
        }
        for key in ["fix_z", "fix_rx", "fix_ry"] {
            node[key] = json!(true);
        }
    }
    node
}

fn frame_element(space: bool, id: &str, i: usize, j: usize, modulus: f64) -> Value {
    let mut element = json!({"id":id, "node_i":i, "node_j":j, "area":1.0,
        "youngs_modulus":modulus, "density":1.0});
    if space {
        element["shear_modulus"] = json!(0.4 * modulus);
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
    element
}

fn two_branches(space: bool, stiff_modulus: f64, mode_count: usize) -> Value {
    json!({"nodes":[
        frame_node(space, "soft-root", 0.0, 0.0, true),
        frame_node(space, "soft-tip", 1.0, 0.0, false),
        frame_node(space, "stiff-root", 0.0, 2.0, true),
        frame_node(space, "stiff-tip", 1.0, 2.0, false)
    ], "elements":[
        frame_element(space, "soft", 0, 1, 1.0),
        frame_element(space, "stiff", 2, 3, stiff_modulus)
    ], "mode_count":mode_count})
}

fn solve(space: bool, input: Value) -> Result<Value, String> {
    if space {
        solve_modal_frame_3d(&serde_json::from_value(input).unwrap())
            .map(|value| serde_json::to_value(value).unwrap())
    } else {
        solve_modal_frame_2d(&serde_json::from_value(input).unwrap())
            .map(|value| serde_json::to_value(value).unwrap())
    }
}

fn eigenvalue(result: &Value, index: usize) -> f64 {
    result["modes"][index]["eigenvalue_rad_s_squared"]
        .as_f64()
        .unwrap()
}

fn close(actual: f64, expected: f64) {
    assert!(
        (actual / expected - 1.0).abs() < 1e-8,
        "{actual:e} != {expected:e}"
    );
}

#[test]
fn modal_count_does_not_remove_a_resolved_soft_branch() {
    for space in [false, true] {
        for stiffness in [1.0, 1e8, 1e14] {
            let first = solve(space, two_branches(space, stiffness, 1)).unwrap();
            let both = solve(space, two_branches(space, stiffness, 2)).unwrap();
            assert_eq!(both["modes"].as_array().unwrap().len(), 2, "{both}");
            close(eigenvalue(&first, 0), 2.0);
            close(eigenvalue(&both, 0), 2.0);
            close(eigenvalue(&both, 1), 2.0 * stiffness);
        }
    }
}

#[test]
fn soft_bending_block_is_not_approximated_as_diagonal_beside_a_stiff_branch() {
    let mut input = two_branches(false, 1e14, 3);
    for index in [1, 3] {
        input["nodes"][index]["fix_y"] = json!(false);
        input["nodes"][index]["fix_rz"] = json!(false);
    }
    let result = solve(false, input).unwrap();
    for (index, expected) in [
        0.01 * (60.0 - 12.0 * 21.0_f64.sqrt()),
        0.01 * (60.0 + 12.0 * 21.0_f64.sqrt()),
        2.0,
    ]
    .into_iter()
    .enumerate()
    {
        close(eigenvalue(&result, index), expected);
    }
}

#[test]
fn rotated_frame_first_mode_is_transverse_not_the_uniform_axial_seed() {
    // 44 copies leave 132 free dofs, beyond the bounded single-mode dense check.
    for copies in [1, 44] {
        let mut input = rotated_frame(copies);
        let first = solve(true, input.clone()).unwrap();
        input["mode_count"] = json!(3);
        let spectrum = solve(true, input).unwrap();
        close(eigenvalue(&spectrum, 0), 240.0);
        close(eigenvalue(&first, 0), 240.0);
    }
}

fn rotated_frame(copies: usize) -> Value {
    let direction = 1.0 / 3.0_f64.sqrt();
    let mut nodes = Vec::new();
    let mut elements = Vec::new();
    for index in 0..copies {
        let offset = index as f64 * 2.0;
        nodes.push(frame_node(
            true,
            &format!("root-{index}"),
            offset,
            0.0,
            true,
        ));
        let mut tip = frame_node(
            true,
            &format!("tip-{index}"),
            offset + direction,
            direction,
            false,
        );
        tip["z"] = json!(direction);
        tip["fix_y"] = json!(false);
        tip["fix_z"] = json!(false);
        nodes.push(tip);
        elements.push(frame_element(
            true,
            &format!("beam-{index}"),
            index * 2,
            index * 2 + 1,
            1000.0,
        ));
    }
    json!({"nodes":nodes, "elements":elements, "mode_count":1})
}

#[test]
fn floating_components_cannot_hide_behind_the_positive_modes_of_an_anchored_body() {
    for space in [false, true] {
        let mut input = two_branches(space, 1.0, 3);
        input["nodes"][2]["fix_x"] = json!(false);
        let error = solve(space, input).unwrap_err();
        assert!(
            error.contains("rigid-body") || error.contains("unrestrained"),
            "{error}"
        );
        assert!(solve(space, two_branches(space, 1.0, 2)).is_ok());
    }
}

#[test]
fn distributed_restraints_are_valid_without_a_fully_clamped_node() {
    for space in [false, true] {
        let mut input = two_branches(space, 1.0, 3);
        input["nodes"].as_array_mut().unwrap().truncate(2);
        input["elements"].as_array_mut().unwrap().truncate(1);
        for node in input["nodes"].as_array_mut().unwrap() {
            node["fix_rz"] = json!(false);
            if space {
                node["fix_ry"] = json!(false);
            }
        }
        if space {
            input["nodes"][0]["fix_rx"] = json!(false);
        }
        let result = solve(space, input).expect("restraint rank, not a clamped-node shortcut");
        assert_eq!(result["modes"].as_array().unwrap().len(), 3);
    }
}

#[test]
fn redundant_restraints_and_orphan_nodes_are_rejected_and_can_be_corrected() {
    for space in [false, true] {
        let healthy = two_branches(space, 1.0, 2);
        let mut input = healthy.clone();
        for node in input["nodes"].as_array_mut().unwrap() {
            node["fix_x"] = json!(false);
        }
        let error = solve(space, input).unwrap_err();
        assert!(error.contains("restraint rank"), "{error}");
        let mut orphan = healthy.clone();
        orphan["nodes"]
            .as_array_mut()
            .unwrap()
            .push(frame_node(space, "orphan", 9.0, 9.0, true));
        let error = solve(space, orphan).unwrap_err();
        assert!(error.contains("orphan"), "{error}");
        close(eigenvalue(&solve(space, healthy).unwrap(), 0), 2.0);
    }
}

#[test]
fn node_permutation_and_reversed_members_preserve_the_spectrum() {
    for space in [false, true] {
        let mut input = two_branches(space, 1.0e14, 2);
        input["nodes"].as_array_mut().unwrap().reverse();
        let elements = input["elements"].as_array_mut().unwrap();
        elements.reverse();
        for element in elements {
            let i = element["node_i"].as_u64().unwrap();
            let j = element["node_j"].as_u64().unwrap();
            element["node_i"] = json!(3 - j);
            element["node_j"] = json!(3 - i);
        }
        let result = solve(space, input).unwrap();
        close(eigenvalue(&result, 0), 2.0);
        close(eigenvalue(&result, 1), 2.0e14);
    }
}

#[test]
fn nonrepresentable_aggregate_mass_is_an_error_not_a_successful_null_field() {
    for space in [false, true] {
        let mut input = two_branches(space, 1.0, 2);
        for element in input["elements"].as_array_mut().unwrap() {
            element["density"] = json!(1.0e308);
        }
        let error = solve(space, input).unwrap_err();
        assert!(error.contains("total mass"), "{error}");
        close(
            eigenvalue(&solve(space, two_branches(space, 1.0, 2)).unwrap(), 0),
            2.0,
        );
    }
}

#[test]
fn modal_cancellation_is_observed_inside_each_phase_and_replay_is_clean() {
    for stage in [
        SolverStage::ModalSweep,
        SolverStage::ModalIteration,
        SolverStage::ModalValidation,
    ] {
        let input = rotated_frame(if stage == SolverStage::ModalIteration {
            44
        } else {
            1
        });
        let control = SolverControl::default();
        let cancel = control.clone();
        let error = with_solver_observer(
            &control,
            move |point| {
                if point.stage == stage {
                    cancel.request_cancel();
                }
            },
            || {
                let result = solve(true, input.clone());
                assert!(
                    result.is_err(),
                    "{stage:?} must propagate cancellation before scope exit"
                );
                result
            },
        )
        .unwrap_err();
        assert!(error.contains("cancel"), "{error}");
        close(eigenvalue(&solve(true, input).unwrap(), 0), 240.0);
    }
}
