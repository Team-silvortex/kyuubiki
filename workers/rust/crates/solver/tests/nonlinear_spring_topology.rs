use kyuubiki_protocol::{
    ContactGap1dContactInput, NonlinearSpring1dElementInput, NonlinearSpring1dNodeInput,
    SolveContactGap1dRequest, SolveNonlinearSpring1dRequest,
};
use kyuubiki_solver::solver_control::{SolverControl, SolverStage, with_solver_observer};
use kyuubiki_solver::{
    solve_contact_gap_1d, solve_contact_gap_1d_owned, solve_nonlinear_spring_1d,
    solve_nonlinear_spring_1d_owned,
};

fn node(id: &str, x: f64, fixed: bool, load: f64) -> NonlinearSpring1dNodeInput {
    NonlinearSpring1dNodeInput {
        id: id.into(),
        x,
        fix_x: fixed,
        load_x: load,
    }
}

fn element(id: &str, i: usize, j: usize) -> NonlinearSpring1dElementInput {
    NonlinearSpring1dElementInput {
        id: id.into(),
        node_i: i,
        node_j: j,
        stiffness: 2.0,
        cubic_stiffness: 1.0,
    }
}

fn two_components() -> SolveNonlinearSpring1dRequest {
    SolveNonlinearSpring1dRequest {
        nodes: vec![
            node("base-a", 0.0, true, 0.0),
            node("tip-a", 1.0, false, 0.0),
            node("base-b", 2.0, false, 0.0),
            node("tip-b", 3.0, false, 0.0),
        ],
        elements: vec![element("a", 0, 1), element("b", 2, 3)],
        load_steps: Some(4),
        max_iterations: Some(32),
        tolerance: Some(1e-10),
    }
}

fn with_contact(input: SolveNonlinearSpring1dRequest, gap: f64) -> SolveContactGap1dRequest {
    SolveContactGap1dRequest {
        nodes: input.nodes,
        elements: input.elements,
        load_steps: input.load_steps,
        max_iterations: input.max_iterations,
        tolerance: input.tolerance,
        contacts: vec![ContactGap1dContactInput {
            id: "stop-b".into(),
            node: 3,
            gap,
            normal_stiffness: 10.0,
        }],
    }
}

fn assert_unrestrained(error: String, node_id: &str) {
    assert!(error.contains("unrestrained"), "{error}");
    assert!(error.contains(node_id), "{error}");
}

#[test]
fn zero_residual_cannot_certify_a_disconnected_unrestrained_component() {
    let input = two_components();
    assert_unrestrained(
        solve_nonlinear_spring_1d(&input).expect_err("floating component has a rigid-body mode"),
        "base-b",
    );
    assert_unrestrained(
        solve_nonlinear_spring_1d_owned(input)
            .expect_err("owned entry must enforce the same check"),
        "base-b",
    );
}

#[test]
fn an_unused_free_node_is_not_a_solved_degree_of_freedom() {
    let mut input = two_components();
    input.nodes[2].fix_x = true;
    input.nodes.push(node("unused", 4.0, false, 0.0));
    assert_unrestrained(
        solve_nonlinear_spring_1d(&input).expect_err("unused free node has arbitrary displacement"),
        "unused",
    );
}

#[test]
fn a_fixed_node_outside_the_element_graph_cannot_anchor_a_floating_cycle() {
    let mut input = two_components();
    input.elements = vec![
        element("bc", 1, 2),
        element("cd", 2, 3),
        element("db", 3, 1),
        element("duplicate", 2, 1),
    ];
    assert_unrestrained(
        solve_nonlinear_spring_1d(&input).expect_err("cycles do not eliminate rigid translation"),
        "tip-a",
    );
}

#[test]
fn open_or_just_touching_contact_is_not_a_bilateral_restraint() {
    for gap in [0.0, 0.25, 10.0] {
        let input = with_contact(two_components(), gap);
        assert_unrestrained(
            solve_contact_gap_1d(&input)
                .expect_err("inactive penalty does not remove the nullspace"),
            "base-b",
        );
        assert_unrestrained(
            solve_contact_gap_1d_owned(input).expect_err("owned contact must validate topology"),
            "base-b",
        );
    }
}

#[test]
fn insufficient_restraint_is_rejected_even_below_the_requested_residual_tolerance() {
    let mut input = two_components();
    input.nodes[3].load_x = 1e-12;
    input.tolerance = Some(1e-8);
    assert_unrestrained(
        solve_nonlinear_spring_1d(&input).expect_err("a small residual cannot remove a mechanism"),
        "base-b",
    );
}

#[test]
fn independently_supported_components_keep_their_analytical_equilibria() {
    let mut input = two_components();
    input.nodes[2].fix_x = true;
    input.nodes[1].load_x = 3.0;
    input.nodes[3].load_x = -12.0;
    input.nodes.push(node("unused-fixed", 4.0, true, 17.0));
    for result in [
        solve_nonlinear_spring_1d(&input).unwrap(),
        solve_nonlinear_spring_1d_owned(input).unwrap(),
    ] {
        assert!(result.converged);
        assert!((result.nodes[1].ux - 1.0).abs() < 1e-10);
        assert!((result.nodes[3].ux + 2.0).abs() < 1e-10);
        assert_eq!(result.nodes[4].ux, 0.0);
        assert!((result.elements[0].force - 3.0).abs() < 1e-10);
        assert!((result.elements[1].force + 12.0).abs() < 1e-10);
    }
}

#[test]
fn contact_activation_remains_valid_with_independent_supports() {
    let mut input = two_components();
    input.nodes[2].fix_x = true;
    input.nodes[1].load_x = -3.0;
    input.nodes[3].load_x = 8.0;
    let input = with_contact(input, 0.5);
    for result in [
        solve_contact_gap_1d(&input).unwrap(),
        solve_contact_gap_1d_owned(input).unwrap(),
    ] {
        assert!(result.converged);
        assert!((result.nodes[1].ux + 1.0).abs() < 1e-10);
        assert!((result.nodes[3].ux - 1.0).abs() < 1e-10);
        assert!(result.contacts[0].active);
        assert!((result.contacts[0].force - 5.0).abs() < 1e-10);
        assert!((result.elements[1].force + result.contacts[0].force - 8.0).abs() < 1e-10);
    }
}

#[test]
fn all_four_node_graphs_match_an_independent_reachability_oracle() {
    let possible_edges = [(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)];
    for edges in 1..64 {
        for supports in 1..16 {
            let mut input = two_components();
            let mut reachable = [false; 4];
            for (index, node) in input.nodes.iter_mut().enumerate() {
                node.fix_x = supports & (1 << index) != 0;
                reachable[index] = node.fix_x;
            }
            input.elements = possible_edges
                .iter()
                .enumerate()
                .filter(|(index, _)| edges & (1 << index) != 0)
                .map(|(index, &(i, j))| element(&format!("e{index}"), j, i))
                .rev()
                .collect();
            // Flood from supports rather than duplicating the solver's union-find.
            for _ in 0..4 {
                for edge in &input.elements {
                    if reachable[edge.node_i] || reachable[edge.node_j] {
                        reachable[edge.node_i] = true;
                        reachable[edge.node_j] = true;
                    }
                }
            }
            let expected = reachable.iter().all(|&value| value);
            if expected {
                check_manufactured_equilibrium(&input);
            }
            let contact = with_contact(input.clone(), if edges % 2 == 0 { 0.0 } else { 0.5 });
            let results = [
                solve_nonlinear_spring_1d(&input).map(|result| result.converged),
                solve_nonlinear_spring_1d_owned(input).map(|result| result.converged),
                solve_contact_gap_1d(&contact).map(|result| result.converged),
                solve_contact_gap_1d_owned(contact).map(|result| result.converged),
            ];
            for result in results {
                if expected {
                    assert_eq!(result, Ok(true), "edges={edges}, supports={supports}");
                } else {
                    let error = result.expect_err("unsupported topology cannot be certified");
                    assert!(error.contains("unrestrained"), "{error}");
                }
            }
        }
    }
}

fn check_manufactured_equilibrium(input: &SolveNonlinearSpring1dRequest) {
    let displacement: Vec<_> = input
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| {
            if node.fix_x {
                0.0
            } else {
                0.1 * (index + 1) as f64
            }
        })
        .collect();
    let mut loaded = input.clone();
    // Manufacture loads from an independent force law and a prescribed exact solution.
    for edge in &input.elements {
        let extension = displacement[edge.node_j] - displacement[edge.node_i];
        let force = 2.0 * extension + extension.powi(3);
        loaded.nodes[edge.node_i].load_x -= force;
        loaded.nodes[edge.node_j].load_x += force;
    }
    let result = solve_nonlinear_spring_1d(&loaded).unwrap();
    assert!(result.converged);
    for (node, &expected) in result.nodes.iter().zip(&displacement) {
        assert!((node.ux - expected).abs() < 1e-9, "{node:?} != {expected}");
    }
    let mut contact = with_contact(loaded, 0.25);
    let contact_force = 10.0 * (displacement[3] - 0.25).max(0.0);
    contact.nodes[3].load_x += contact_force;
    let result = solve_contact_gap_1d_owned(contact).unwrap();
    assert!(result.converged);
    for (node, &expected) in result.nodes.iter().zip(&displacement) {
        assert!((node.ux - expected).abs() < 1e-9, "{node:?} != {expected}");
    }
    assert!((result.contacts[0].force - contact_force).abs() < 1e-9);
    assert_eq!(result.contacts[0].active, contact_force > 0.0);
}

#[test]
fn node_permutations_and_reversed_elements_preserve_contact_equilibrium() {
    let mut original = two_components();
    original.nodes[2].fix_x = true;
    original.nodes[1].load_x = -3.0;
    original.nodes[3].load_x = 8.0;
    for reversed in [false, true] {
        for offset in 0..4 {
            let mut order = [0, 1, 2, 3];
            order.rotate_left(offset);
            if reversed {
                order.reverse();
            }
            let mut inverse = [0; 4];
            for (index, &old) in order.iter().enumerate() {
                inverse[old] = index;
            }
            let mut input = with_contact(original.clone(), 0.5);
            input.nodes = order
                .iter()
                .map(|&old| original.nodes[old].clone())
                .collect();
            for edge in &mut input.elements {
                let (i, j) = (edge.node_i, edge.node_j);
                edge.node_i = inverse[j];
                edge.node_j = inverse[i];
            }
            input.elements.reverse();
            input.contacts[0].node = inverse[3];
            let result = solve_contact_gap_1d_owned(input).unwrap();
            assert!(result.converged);
            for (old, expected) in [0.0, -1.0, 0.0, 1.0].into_iter().enumerate() {
                assert!((result.nodes[inverse[old]].ux - expected).abs() < 1e-10);
            }
            assert!((result.contacts[0].force - 5.0).abs() < 1e-10);
        }
    }
}

#[test]
fn validation_can_cancel_in_each_linear_pass_without_starting_assembly() {
    let mut input = two_components();
    input.nodes = (0..130)
        .map(|index| node(&format!("n{index}"), index as f64, index == 129, 0.0))
        .collect();
    input.elements = (1..130)
        .map(|index| element(&format!("e{index}"), index - 1, index))
        .collect();
    for contact_mode in [false, true] {
        for stage in [
            SolverStage::ElementPrecompute,
            SolverStage::ConstraintIndex,
            SolverStage::ConstraintMap,
            SolverStage::ConstraintReduce,
        ] {
            let control = SolverControl::default();
            let cancel = control.clone();
            let result = with_solver_observer(
                &control,
                move |point| {
                    assert_ne!(point.stage, SolverStage::ElementAssembly);
                    if point.stage == stage && point.completed_steps == 64 {
                        cancel.request_cancel();
                    }
                },
                || {
                    if contact_mode {
                        solve_contact_gap_1d_owned(with_contact(input.clone(), 0.5)).map(|_| ())
                    } else {
                        solve_nonlinear_spring_1d_owned(input.clone()).map(|_| ())
                    }
                },
            );
            assert!(result.unwrap_err().contains("cancelled"), "{stage:?}");
            assert!(control.was_interrupted());
        }
    }
    assert!(solve_nonlinear_spring_1d(&input).unwrap().converged);
    assert!(
        solve_contact_gap_1d(&with_contact(input, 0.5))
            .unwrap()
            .converged
    );
}

#[test]
fn malformed_references_are_rejected_before_connectivity_traversal() {
    for invalid_index in [4, usize::MAX] {
        let mut input = two_components();
        input.elements[1].node_j = invalid_index;
        let contact = with_contact(input.clone(), 0.0);
        for error in [
            solve_nonlinear_spring_1d(&input).unwrap_err(),
            solve_contact_gap_1d_owned(contact).unwrap_err(),
        ] {
            assert!(error.contains("out-of-range node"), "{error}");
        }
    }
}
