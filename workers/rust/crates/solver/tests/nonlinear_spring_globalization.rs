use kyuubiki_protocol::{
    ContactGap1dContactInput, NonlinearSpring1dElementInput, NonlinearSpring1dNodeInput,
    SolveContactGap1dRequest, SolveNonlinearSpring1dRequest,
};
use kyuubiki_solver::solver_control::{SolverControl, SolverStage, with_solver_observer};
use kyuubiki_solver::{
    solve_contact_gap_1d, solve_contact_gap_1d_owned, solve_nonlinear_spring_1d,
    solve_nonlinear_spring_1d_owned,
};
use std::cell::Cell;
use std::rc::Rc;

fn spring(cubic: f64, expected: f64) -> SolveNonlinearSpring1dRequest {
    SolveNonlinearSpring1dRequest {
        nodes: vec![
            NonlinearSpring1dNodeInput {
                id: "base".into(),
                x: 0.0,
                fix_x: true,
                load_x: 0.0,
            },
            NonlinearSpring1dNodeInput {
                id: "tip".into(),
                x: 1.0,
                fix_x: false,
                load_x: expected + cubic * expected.powi(3),
            },
        ],
        elements: vec![NonlinearSpring1dElementInput {
            id: "hardening".into(),
            node_i: 0,
            node_j: 1,
            stiffness: 1.0,
            cubic_stiffness: cubic,
        }],
        load_steps: Some(1),
        max_iterations: Some(12),
        tolerance: Some(1e-10),
    }
}

fn contact(mut input: SolveNonlinearSpring1dRequest, expected: f64) -> SolveContactGap1dRequest {
    let gap = 0.5 * expected;
    let stiffness = 1.0 / expected;
    input.nodes[1].load_x += stiffness * (expected - gap);
    SolveContactGap1dRequest {
        nodes: input.nodes,
        elements: input.elements,
        load_steps: input.load_steps,
        max_iterations: input.max_iterations,
        tolerance: input.tolerance,
        contacts: vec![ContactGap1dContactInput {
            id: "stop".into(),
            node: 1,
            gap,
            normal_stiffness: stiffness,
        }],
    }
}

fn close(actual: f64, expected: f64) {
    assert!(actual.is_finite());
    assert!(
        (actual / expected - 1.0).abs() < 1e-9,
        "{actual} != {expected}"
    );
}

#[test]
fn strongly_hardening_springs_reach_known_equilibria_within_the_newton_budget() {
    for (cubic, magnitude) in [(1e12, 1e-4), (1e18, 1e-6), (1e24, 1e-8)] {
        for sign in [-1.0, 1.0] {
            let expected = sign * magnitude;
            let input = spring(cubic, expected);
            for result in [
                solve_nonlinear_spring_1d(&input).unwrap(),
                solve_nonlinear_spring_1d_owned(input).unwrap(),
            ] {
                assert!(
                    result.converged,
                    "cubic={cubic}, sign={sign}: {:?}",
                    result.steps
                );
                close(result.nodes[1].ux, expected);
                close(result.elements[0].force, result.input.nodes[1].load_x);
                assert!(result.residual_norm <= 1e-10);
                assert_eq!(result.achieved_load_factor, Some(1.0));
                assert!(result.steps[0].iterations <= 12);
            }
        }
    }
}

#[test]
fn hardening_and_contact_activation_recover_the_manufactured_force_split() {
    for (cubic, expected) in [(1e12, 1e-4), (1e18, 1e-6), (1e24, 1e-8)] {
        let input = contact(spring(cubic, expected), expected);
        for result in [
            solve_contact_gap_1d(&input).unwrap(),
            solve_contact_gap_1d_owned(input).unwrap(),
        ] {
            assert!(result.converged, "cubic={cubic}: {:?}", result.steps);
            close(result.nodes[1].ux, expected);
            close(result.contacts[0].force, 0.5);
            assert!(result.contacts[0].active);
            assert!(result.residual_norm <= 1e-10);
            close(
                result.elements[0].force + result.contacts[0].force,
                result.input.nodes[1].load_x,
            );
        }
    }
}

#[test]
fn damping_can_finish_the_last_permitted_correction() {
    for sign in [-1.0, 1.0] {
        let mut input = spring(1.0, sign);
        input.max_iterations = Some(1);
        let result = solve_nonlinear_spring_1d(&input).unwrap();
        assert!(result.converged);
        assert_eq!(result.steps[0].iterations, 1);
        assert_eq!(result.residual_norm, 0.0);
        close(result.nodes[1].ux, sign);
        let mut input = contact(input, 1.0);
        input.nodes[1].load_x = 2.0 * sign;
        input.contacts[0].gap = 3.0;
        let result = solve_contact_gap_1d(&input).unwrap();
        assert!(result.converged);
        assert_eq!(result.steps[0].iterations, 1);
        assert!(!result.contacts[0].active);
        close(result.nodes[1].ux, sign);
    }
}

#[test]
fn damping_can_complete_a_path_with_a_nonzero_previously_committed_residual() {
    let mut input = contact(spring(1.0, 1.0), 1.0);
    input.nodes[1].load_x = 1.0;
    input.load_steps = Some(2);
    input.max_iterations = Some(1);
    input.tolerance = Some(0.13);
    input.contacts[0].gap = 0.6;
    input.contacts[0].normal_stiffness = 10.0;
    let result = solve_contact_gap_1d(&input).unwrap();
    assert!(result.converged);
    assert_eq!(result.achieved_load_factor, Some(1.0));
    close(result.steps[0].residual_norm, 0.125);
    let expected = 17.0_f64 / 28.0;
    close(result.nodes[1].ux, expected);
    let residual = (expected + expected.powi(3) + 10.0 * (expected - 0.6) - 1.0).abs();
    close(result.residual_norm, residual);
    assert!(residual <= 0.13);
}

#[test]
fn bounded_search_exhaustion_never_commits_a_rejected_trial() {
    let assemblies = Rc::new(Cell::new(0));
    let observed = assemblies.clone();
    let result = with_solver_observer(
        &SolverControl::default(),
        move |point| {
            if point.stage == SolverStage::ElementAssembly && point.completed_steps == 0 {
                observed.set(observed.get() + 1);
            }
        },
        || solve_nonlinear_spring_1d(&spring(1e60, 1e-20)),
    )
    .unwrap();
    assert!(!result.converged);
    assert_eq!(
        assemblies.get(),
        34,
        "one initial assembly plus 33 trial evaluations"
    );
    assert_eq!(result.achieved_load_factor, Some(0.0));
    assert_eq!(result.steps[0].iterations, 1);
    assert!(result.steps[0].residual_norm > 1.0);
    assert_eq!(result.nodes[1].ux, 0.0);
    assert_eq!(result.elements[0].force, 0.0);
    assert_eq!(result.residual_norm, 0.0);
    assert!(
        solve_nonlinear_spring_1d(&spring(1e18, 1e-6))
            .unwrap()
            .converged
    );
}

#[test]
fn exhausted_contact_search_retains_the_last_equilibrated_load_step() {
    let mut input = contact(spring(0.0, 2.0), 1.0);
    input.nodes[1].load_x = 2.0;
    input.load_steps = Some(4);
    input.contacts[0].normal_stiffness = 1e60;
    let result = solve_contact_gap_1d_owned(input).unwrap();
    assert!(!result.converged);
    assert_eq!(result.achieved_load_factor, Some(0.25));
    assert_eq!(result.steps.len(), 2);
    assert!(result.steps[0].converged);
    assert!(!result.steps[1].converged);
    assert_eq!(result.steps[1].iterations, 1);
    assert!(result.steps[1].residual_norm.is_finite());
    assert!(result.steps[1].residual_norm > 1.0);
    close(result.nodes[1].ux, 0.5);
    close(result.elements[0].force, 0.5);
    assert_eq!(result.residual_norm, 0.0);
    assert!(!result.contacts[0].active);
    assert_eq!(result.contacts[0].force, 0.0);
}

#[test]
fn cancellation_during_backtracking_is_not_downgraded_to_partial_convergence() {
    for contact_mode in [false, true] {
        let control = SolverControl::default();
        let cancel = control.clone();
        let updates = Cell::new(0);
        let result = with_solver_observer(
            &control,
            move |point| {
                if point.stage == SolverStage::PcgVectorUpdate && point.completed_steps == 0 {
                    updates.set(updates.get() + 1);
                    if updates.get() == 2 {
                        cancel.request_cancel();
                    }
                }
            },
            || {
                let input = spring(1e18, 1e-6);
                if contact_mode {
                    solve_contact_gap_1d_owned(contact(input, 1e-6)).map(|_| ())
                } else {
                    solve_nonlinear_spring_1d_owned(input).map(|_| ())
                }
            },
        );
        assert!(result.unwrap_err().contains("cancelled"));
        assert!(control.was_interrupted());
        let input = spring(1e18, 1e-6);
        assert!(solve_nonlinear_spring_1d(&input).unwrap().converged);
        assert!(
            solve_contact_gap_1d_owned(contact(input, 1e-6))
                .unwrap()
                .converged
        );
    }
}

#[test]
fn linear_full_steps_reuse_their_accepted_assembly() {
    let assemblies = Rc::new(Cell::new(0));
    let observed = assemblies.clone();
    let mut input = spring(0.0, 2.0);
    input.max_iterations = Some(1);
    let result = with_solver_observer(
        &SolverControl::default(),
        move |point| {
            if point.stage == SolverStage::ElementAssembly && point.completed_steps == 0 {
                observed.set(observed.get() + 1);
            }
        },
        || solve_nonlinear_spring_1d_owned(input),
    )
    .unwrap();
    assert!(result.converged);
    assert_eq!(result.steps[0].iterations, 1);
    assert_eq!(
        assemblies.get(),
        2,
        "accepted state must not be assembled twice"
    );
    close(result.nodes[1].ux, 2.0);
}

#[test]
fn scalar_solutions_agree_with_independent_bracketed_bisection() {
    for cubic in [0.0, 1.0, 1e6, 1e12, 1e18, 1e24] {
        for load in [-100.0, -10.0, -1.0, 1.0, 10.0, 100.0] {
            for (gap, penalty) in [(0.0, 1.0), (1e-7, 1e6), (0.1, 10.0)] {
                let mut input = contact(spring(cubic, 1.0), 1.0);
                input.nodes[1].load_x = load;
                input.max_iterations = Some(32);
                input.contacts[0].gap = gap;
                input.contacts[0].normal_stiffness = penalty;
                let mut lower = load.min(0.0);
                let mut upper = load.max(0.0);
                for _ in 0..160 {
                    let middle: f64 = 0.5 * (lower + upper);
                    let force = middle + cubic * middle.powi(3) + penalty * (middle - gap).max(0.0);
                    if force < load {
                        lower = middle;
                    } else {
                        upper = middle;
                    }
                }
                let result = solve_contact_gap_1d_owned(input).unwrap();
                assert!(
                    result.converged,
                    "cubic={cubic}, load={load}, gap={gap}: {:?}",
                    result.steps
                );
                close(result.nodes[1].ux, 0.5 * (lower + upper));
                assert!(result.residual_norm <= 1e-10);
            }
        }
    }
}

#[test]
fn a_branched_heterogeneous_network_preserves_the_manufactured_equilibrium() {
    let expected = [0.0, 1e-6, -0.5e-6, 2e-6, 0.75e-6, -1e-6];
    let mut input = spring(1.0, 1.0);
    input.max_iterations = Some(32);
    input.nodes = (0..expected.len())
        .map(|index| NonlinearSpring1dNodeInput {
            id: format!("n{index}"),
            x: index as f64,
            fix_x: index == 0,
            load_x: 0.0,
        })
        .collect();
    input.elements = [(0, 1), (1, 2), (2, 3), (1, 4), (4, 5), (3, 5)]
        .into_iter()
        .enumerate()
        .map(|(index, (i, j))| {
            let k = 1.0 + index as f64;
            let c = (index + 1) as f64 * 1e18;
            let extension: f64 = expected[j] - expected[i];
            let force = k * extension + c * extension.powi(3);
            input.nodes[i].load_x -= force;
            input.nodes[j].load_x += force;
            NonlinearSpring1dElementInput {
                id: format!("e{index}"),
                node_i: i,
                node_j: j,
                stiffness: k,
                cubic_stiffness: c,
            }
        })
        .collect();
    let result = solve_nonlinear_spring_1d(&input).unwrap();
    assert!(result.converged, "{:?}", result.steps);
    for (node, displacement) in result.nodes.iter().skip(1).zip(expected.iter().skip(1)) {
        close(node.ux, *displacement);
    }
    let original_load = input.nodes[1].load_x;
    let mut input = contact(input, 1e-6);
    input.nodes[1].load_x = original_load;
    input.nodes[3].load_x += 1.0;
    input.contacts[0].node = 3;
    input.contacts[0].gap = 1e-6;
    input.contacts[0].normal_stiffness = 1e6;
    let result = solve_contact_gap_1d_owned(input).unwrap();
    assert!(result.converged, "{:?}", result.steps);
    for (node, displacement) in result.nodes.iter().skip(1).zip(expected.iter().skip(1)) {
        close(node.ux, *displacement);
    }
    close(result.contacts[0].force, 1.0);
}

#[test]
fn stable_small_softening_branches_keep_their_equilibrium() {
    for cubic in [-0.25, -0.9] {
        for expected in [-0.3, 0.3] {
            let result = solve_nonlinear_spring_1d(&spring(cubic, expected)).unwrap();
            assert!(result.converged);
            close(result.nodes[1].ux, expected);
            assert!(result.elements[0].tangent_stiffness > 0.0);
            assert!(result.residual_norm <= 1e-10);
        }
    }
}
