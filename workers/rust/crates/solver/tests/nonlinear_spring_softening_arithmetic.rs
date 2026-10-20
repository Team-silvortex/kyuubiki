use kyuubiki_protocol::{
    ContactGap1dContactInput, NonlinearSpring1dElementInput, NonlinearSpring1dNodeInput,
    SolveContactGap1dRequest, SolveNonlinearSpring1dRequest,
};
use kyuubiki_solver::{
    solve_contact_gap_1d, solve_contact_gap_1d_owned, solve_nonlinear_spring_1d,
    solve_nonlinear_spring_1d_owned,
};

fn model(unit: f64, k: f64, c: f64, load: f64) -> SolveNonlinearSpring1dRequest {
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
                load_x: load * unit,
            },
        ],
        elements: vec![NonlinearSpring1dElementInput {
            id: "softening".into(),
            node_i: 0,
            node_j: 1,
            stiffness: k * unit,
            cubic_stiffness: c * unit,
        }],
        load_steps: Some(1),
        max_iterations: Some(32),
        tolerance: Some(2.0_f64.powi(-40) * unit),
    }
}

fn contact(
    input: SolveNonlinearSpring1dRequest,
    unit: f64,
    gap: f64,
    penalty: f64,
) -> SolveContactGap1dRequest {
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
            normal_stiffness: penalty * unit,
        }],
    }
}

fn close(actual: f64, expected: f64) {
    assert!(actual.is_finite());
    assert!((actual - expected).abs() < 1e-10, "{actual} != {expected}");
}

#[test]
fn active_contact_stabilizes_softening_without_spurious_tangent_overflow() {
    check_contact(3.0, -1.5, 2.5, 0.5, 2.0, 1.0);
}

#[test]
fn active_contact_stabilizes_softening_without_spurious_force_overflow() {
    check_contact(2.5, -0.25, 3.5, 1.5, 1.0, 2.0);
}

fn check_contact(k: f64, c: f64, load: f64, gap: f64, penalty: f64, u: f64) {
    let spring_force = (k + c * u * u) * u;
    let spring_tangent = k + 3.0 * c * u * u;
    assert!(spring_tangent < 0.0 && spring_tangent + penalty > 0.0);
    for unit in [
        2.0_f64.powi(-500),
        1.0,
        2.0_f64.powi(500),
        2.0_f64.powi(1022),
    ] {
        let input = contact(model(unit, k, c, load), unit, gap, penalty);
        for result in [
            solve_contact_gap_1d(&input).unwrap(),
            solve_contact_gap_1d_owned(input).unwrap(),
        ] {
            assert!(result.converged, "unit={unit}: {:?}", result.steps);
            assert_eq!(result.achieved_load_factor, Some(1.0));
            assert!(result.residual_norm <= result.input.tolerance.unwrap());
            close(result.nodes[1].ux, u);
            close(result.elements[0].force / unit, spring_force);
            close(result.elements[0].tangent_stiffness / unit, spring_tangent);
            assert!(result.contacts[0].active);
            close(result.contacts[0].force / unit, penalty * (u - gap));
            close(
                result.elements[0].force / unit + result.contacts[0].force / unit,
                load,
            );
        }
    }
}

#[test]
fn stable_softening_without_contact_keeps_all_public_entries_consistent() {
    for unit in [2.0_f64.powi(-500), 1.0, 2.0_f64.powi(1022)] {
        for sign in [-1.0, 1.0] {
            let input = model(unit, 3.0, -1.5, sign * 1.3125);
            for result in [
                solve_nonlinear_spring_1d(&input).unwrap(),
                solve_nonlinear_spring_1d_owned(input.clone()).unwrap(),
            ] {
                assert!(result.converged);
                close(result.nodes[1].ux, sign * 0.5);
                close(result.elements[0].tangent_stiffness / unit, 1.875);
            }
            let input = contact(input, unit, 2.0, 1.0);
            for result in [
                solve_contact_gap_1d(&input).unwrap(),
                solve_contact_gap_1d_owned(input).unwrap(),
            ] {
                assert!(result.converged);
                assert!(!result.contacts[0].active);
                close(result.nodes[1].ux, sign * 0.5);
                close(result.elements[0].tangent_stiffness / unit, 1.875);
            }
        }
    }
}

#[test]
fn bounded_softening_failure_retains_the_committed_state_and_can_replay() {
    for unit in [1.0, 2.0_f64.powi(1022)] {
        let mut input = contact(model(unit, 2.5, -0.25, 3.5), unit, 1.5, 1.0);
        input.max_iterations = Some(2);
        let failed = solve_contact_gap_1d(&input).unwrap();
        assert!(!failed.converged);
        assert_eq!(failed.achieved_load_factor, Some(0.0));
        assert_eq!(failed.nodes[1].ux, 0.0);
        assert_eq!(failed.elements[0].force, 0.0);
        assert_eq!(failed.contacts[0].force, 0.0);
        assert_eq!(failed.residual_norm, 0.0);
        assert_eq!(failed.steps[0].iterations, 2);
        assert!(failed.steps[0].residual_norm.is_finite());
        assert!(failed.steps[0].residual_norm > input.tolerance.unwrap());
        input.max_iterations = Some(32);
        let replay = solve_contact_gap_1d_owned(input).unwrap();
        assert!(replay.converged);
        close(replay.nodes[1].ux, 2.0);
    }
}
