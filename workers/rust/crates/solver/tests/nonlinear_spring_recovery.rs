use kyuubiki_protocol::{
    ContactGap1dContactInput, NonlinearSpring1dElementInput, NonlinearSpring1dNodeInput,
    SolveContactGap1dRequest, SolveNonlinearSpring1dRequest,
};
use kyuubiki_solver::solver_control::{SolverControl, SolverStage, with_solver_observer};
use kyuubiki_solver::{
    solve_contact_gap_1d, solve_contact_gap_1d_owned, solve_nonlinear_spring_1d,
    solve_nonlinear_spring_1d_owned,
};

fn spring(cubic: f64, load: f64, steps: usize, iterations: usize) -> SolveNonlinearSpring1dRequest {
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
                load_x: load,
            },
        ],
        elements: vec![NonlinearSpring1dElementInput {
            id: "spring".into(),
            node_i: 0,
            node_j: 1,
            stiffness: 1.0,
            cubic_stiffness: cubic,
        }],
        load_steps: Some(steps),
        max_iterations: Some(iterations),
        tolerance: Some(1e-12),
    }
}

fn contact(input: SolveNonlinearSpring1dRequest, gap: f64) -> SolveContactGap1dRequest {
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
            normal_stiffness: 10.0,
        }],
    }
}

fn close(actual: f64, expected: f64) {
    assert!(actual.is_finite());
    assert!(
        (actual - expected).abs() <= 1e-11 * expected.abs().max(1.0),
        "{actual} != {expected}"
    );
}

#[test]
fn final_allowed_correction_is_checked_before_declaring_budget_exhaustion() {
    for sign in [-1.0, 1.0] {
        let input = spring(0.0, sign * 2.0, 1, 1);
        for result in [
            solve_nonlinear_spring_1d(&input).unwrap(),
            solve_nonlinear_spring_1d_owned(input.clone()).unwrap(),
        ] {
            assert!(result.converged);
            assert_eq!(result.achieved_load_factor, Some(1.0));
            close(result.nodes[1].ux, sign * 2.0);
            close(result.residual_norm, 0.0);
            assert_eq!(result.steps[0].iterations, 1);
        }
        let input = contact(input, 3.0);
        for result in [
            solve_contact_gap_1d(&input).unwrap(),
            solve_contact_gap_1d_owned(input).unwrap(),
        ] {
            assert!(result.converged);
            assert_eq!(result.achieved_load_factor, Some(1.0));
            close(result.nodes[1].ux, sign * 2.0);
            close(result.residual_norm, 0.0);
        }
    }
}

#[test]
fn failed_first_step_retains_zero_state_and_reports_the_evaluated_trial_residual() {
    for load in [-2.0, 2.0] {
        let input = spring(1.0, load, 1, 1);
        let result = solve_nonlinear_spring_1d(&input).unwrap();
        assert!(!result.converged);
        assert_eq!(result.achieved_load_factor, Some(0.0));
        close(result.steps[0].residual_norm, 8.0);
        close(result.nodes[1].ux, 0.0);
        close(result.elements[0].force, 0.0);
        close(result.max_displacement, 0.0);
        close(result.max_force, 0.0);
        close(result.residual_norm, 0.0);
        let result = solve_contact_gap_1d(&contact(input, 3.0)).unwrap();
        assert!(!result.converged);
        assert_eq!(result.achieved_load_factor, Some(0.0));
        close(result.steps[0].residual_norm, 8.0);
        close(result.nodes[1].ux, 0.0);
        close(result.residual_norm, 0.0);
        assert!(!result.contacts[0].active);
    }
}

#[test]
fn failed_contact_activation_restores_the_last_equilibrated_load_step() {
    let input = contact(spring(0.0, 2.0, 4, 1), 1.0);
    let result = solve_contact_gap_1d(&input).unwrap();
    assert!(!result.converged);
    assert_eq!(result.achieved_load_factor, Some(0.5));
    assert_eq!(result.steps.len(), 3);
    assert!(result.steps[..2].iter().all(|step| step.converged));
    assert!(!result.steps[2].converged);
    close(result.steps[2].load_factor, 0.75);
    close(result.steps[2].residual_norm, 5.0);
    let baseline = solve_contact_gap_1d(&contact(spring(0.0, 1.0, 2, 1), 1.0)).unwrap();
    assert!(baseline.converged);
    assert_eq!(result.nodes, baseline.nodes);
    assert_eq!(result.elements, baseline.elements);
    assert_eq!(result.contacts, baseline.contacts);
    close(result.max_contact_force, 0.0);
    assert_eq!(result.active_contact_count, 0);
    close(result.residual_norm, 0.0);
}

#[test]
fn last_allowed_contact_correction_can_commit_the_full_load() {
    let result = solve_contact_gap_1d(&contact(spring(0.0, 2.0, 4, 2), 1.0)).unwrap();
    assert!(result.converged);
    assert_eq!(result.achieved_load_factor, Some(1.0));
    assert_eq!(result.steps.len(), 4);
    close(result.nodes[1].ux, 12.0 / 11.0);
    close(result.contacts[0].force, 10.0 / 11.0);
    close(result.elements[0].force + result.contacts[0].force, 2.0);
    assert!(result.steps.iter().all(|step| step.iterations <= 2));
}

#[test]
fn rollback_preserves_a_nonzero_committed_residual_instead_of_the_failed_trial_norm() {
    let mut input = contact(spring(1.0, 1.0, 2, 1), 0.6);
    input.tolerance = Some(0.13);
    let result = solve_contact_gap_1d_owned(input).unwrap();
    assert!(!result.converged);
    assert_eq!(result.achieved_load_factor, Some(0.5));
    close(result.nodes[1].ux, 0.5);
    close(result.elements[0].force, 0.625);
    close(result.residual_norm, 0.125);
    let last_trial = 5.0_f64 / 7.0;
    let trial_residual = (last_trial + last_trial.powi(3) + 10.0 * (last_trial - 0.6) - 1.0).abs();
    close(result.steps[1].residual_norm, trial_residual);
    assert!(result.steps[1].residual_norm > 0.13);
    assert!(!result.contacts[0].active);
}

#[test]
fn hardening_spring_failure_restores_its_own_nonzero_committed_state() {
    let mut input = spring(1.0, 1.0, 4, 1);
    input.tolerance = Some(0.02);
    let result = solve_nonlinear_spring_1d_owned(input).unwrap();
    assert!(!result.converged);
    assert_eq!(result.achieved_load_factor, Some(0.25));
    assert_eq!(result.steps.len(), 2);
    close(result.nodes[1].ux, 0.25);
    close(result.elements[0].force, 0.265625);
    close(result.residual_norm, 0.015625);
    let trial = 17.0_f64 / 38.0;
    close(
        result.steps[1].residual_norm,
        (trial + trial.powi(3) - 0.5).abs(),
    );
    assert!(result.steps[1].residual_norm > 0.02);
    let mut preload = spring(1.0, 0.25, 1, 1);
    preload.tolerance = Some(0.02);
    let baseline = solve_nonlinear_spring_1d(&preload).unwrap();
    assert!(baseline.converged);
    assert_eq!(result.nodes, baseline.nodes);
    assert_eq!(result.elements, baseline.elements);
    assert_eq!(result.residual_norm, baseline.residual_norm);
}

#[test]
fn failure_then_a_larger_budget_matches_a_clean_full_solve_without_leaking_state() {
    let mut input = contact(spring(0.0, 2.0, 4, 1), 1.0);
    let failed = solve_contact_gap_1d(&input).unwrap();
    assert!(!failed.converged);
    input.max_iterations = Some(2);
    let recovered = solve_contact_gap_1d_owned(input).unwrap();
    let clean = solve_contact_gap_1d(&contact(spring(0.0, 2.0, 4, 2), 1.0)).unwrap();
    assert!(recovered.converged);
    assert_eq!(recovered, clean);
}

#[test]
fn cancellation_after_a_committed_step_is_not_downgraded_to_partial_success() {
    let input = contact(spring(0.0, 2.0, 4, 2), 1.0);
    let control = SolverControl::default();
    let cancel = control.clone();
    let result = with_solver_observer(
        &control,
        move |point| {
            if point.stage == SolverStage::StabilityStep && point.completed_steps == 3 {
                cancel.request_cancel();
            }
        },
        || solve_contact_gap_1d_owned(input.clone()),
    );
    assert!(result.unwrap_err().contains("cancelled"));
    assert!(control.was_interrupted());
    assert!(solve_contact_gap_1d_owned(input).unwrap().converged);
}

#[test]
fn zero_load_has_a_full_achieved_factor_without_spending_a_newton_correction() {
    let input = spring(1.0, 0.0, 4, 1);
    let result = solve_nonlinear_spring_1d(&input).unwrap();
    assert!(result.converged);
    assert_eq!(result.achieved_load_factor, Some(1.0));
    assert_eq!(result.steps.len(), 4);
    assert!(
        result
            .steps
            .iter()
            .all(|step| step.iterations == 0 && step.converged)
    );
    let result = solve_contact_gap_1d(&contact(input, 0.0)).unwrap();
    assert!(result.converged);
    assert_eq!(result.achieved_load_factor, Some(1.0));
    assert!(
        result
            .steps
            .iter()
            .all(|step| step.iterations == 0 && step.converged)
    );
}

#[test]
fn copying_a_committed_chain_state_is_cancellable_between_batches() {
    let mut input = spring(0.0, 0.0, 2, 1);
    for index in 2..=130 {
        input.nodes.push(NonlinearSpring1dNodeInput {
            id: format!("n{index}"),
            x: index as f64,
            fix_x: false,
            load_x: 0.0,
        });
        input.elements.push(NonlinearSpring1dElementInput {
            id: format!("e{index}"),
            node_i: index - 1,
            node_j: index,
            stiffness: 1.0,
            cubic_stiffness: 0.0,
        });
    }
    input.nodes[130].load_x = 1.0;
    input.tolerance = Some(1e-9);
    let control = SolverControl::default();
    let cancel = control.clone();
    let step = std::cell::Cell::new(0);
    let result = with_solver_observer(
        &control,
        move |point| {
            if point.stage == SolverStage::StabilityStep {
                step.set(point.completed_steps);
            }
            if step.get() == 2
                && point.stage == SolverStage::StabilityRecovery
                && point.completed_steps == 64
            {
                cancel.request_cancel();
            }
        },
        || solve_nonlinear_spring_1d_owned(input.clone()),
    );
    assert!(result.unwrap_err().contains("cancelled"));
    assert!(control.was_interrupted());
    let replay = solve_nonlinear_spring_1d_owned(input).unwrap();
    assert!(replay.converged);
    close(replay.nodes[130].ux, 130.0);
}
