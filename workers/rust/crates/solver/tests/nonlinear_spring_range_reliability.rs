use kyuubiki_protocol::{
    ContactGap1dContactInput, NonlinearSpring1dElementInput, NonlinearSpring1dNodeInput,
    SolveContactGap1dRequest, SolveNonlinearSpring1dRequest,
};
use kyuubiki_solver::solver_control::{SolverControl, SolverStage, with_solver_observer};
use kyuubiki_solver::{
    solve_contact_gap_1d, solve_contact_gap_1d_owned, solve_nonlinear_spring_1d,
    solve_nonlinear_spring_1d_owned,
};

fn spring(stiffness: f64, cubic: f64, load: f64) -> SolveNonlinearSpring1dRequest {
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
            stiffness,
            cubic_stiffness: cubic,
        }],
        load_steps: Some(1),
        max_iterations: Some(64),
        tolerance: Some(1e-12 * load.abs().max(1.0)),
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
            normal_stiffness: 1.0,
        }],
    }
}

fn close(actual: f64, expected: f64) {
    assert!(actual.is_finite(), "non-finite result: {actual}");
    assert!(
        (actual / expected - 1.0).abs() < 1e-11,
        "{actual} != {expected}"
    );
}

#[test]
fn zero_cubic_law_does_not_evaluate_overflowing_displacement_powers() {
    for load in [-1e200, 1e200] {
        let input = spring(1.0, 0.0, load);
        for result in [
            solve_nonlinear_spring_1d(&input).unwrap(),
            solve_nonlinear_spring_1d_owned(input).unwrap(),
        ] {
            assert!(result.converged);
            close(result.nodes[1].ux, load);
            close(result.elements[0].force, load);
            close(result.elements[0].tangent_stiffness, 1.0);
            close(result.max_force, load.abs());
            assert_eq!(result.residual_norm, 0.0);
        }
    }
}

#[test]
fn contact_active_set_keeps_finite_linear_force_split_at_large_displacement() {
    for load in [-1e200, 1e200] {
        let input = contact(spring(1.0, 0.0, load), 0.0);
        for result in [
            solve_contact_gap_1d(&input).unwrap(),
            solve_contact_gap_1d_owned(input).unwrap(),
        ] {
            let expected = if load > 0.0 { load / 2.0 } else { load };
            assert!(result.converged);
            close(result.nodes[1].ux, expected);
            close(result.elements[0].force, expected);
            close(result.elements[0].tangent_stiffness, 1.0);
            assert_eq!(result.contacts[0].active, load > 0.0);
            assert!(result.contacts[0].force.is_finite());
            close(result.elements[0].force + result.contacts[0].force, load);
        }
    }
}

#[test]
fn small_cubic_coefficient_preserves_a_representable_cubic_force_and_tangent() {
    for sign in [-1.0, 1.0] {
        let input = spring(1.0, 1e-300, sign * 2e150);
        let result = solve_nonlinear_spring_1d(&input).unwrap();
        assert!(result.converged);
        close(result.nodes[1].ux, sign * 1e150);
        close(result.elements[0].force, sign * 2e150);
        close(result.elements[0].tangent_stiffness, 4.0);
        let result = solve_contact_gap_1d(&contact(input, 1e151)).unwrap();
        assert!(result.converged);
        assert!(!result.contacts[0].active);
        close(result.nodes[1].ux, sign * 1e150);
        close(result.elements[0].tangent_stiffness, 4.0);
    }
}

#[test]
fn large_cubic_coefficient_at_zero_extension_keeps_the_linear_tangent() {
    let input = spring(1.0, 1e308, 0.0);
    let result = solve_nonlinear_spring_1d(&input).unwrap();
    assert!(result.converged);
    close(result.elements[0].tangent_stiffness, 1.0);
    let result = solve_contact_gap_1d(&contact(input, 0.0)).unwrap();
    assert!(result.converged);
    close(result.elements[0].tangent_stiffness, 1.0);
}

#[test]
fn exhausted_iteration_budget_cannot_publish_an_unrepresentable_force() {
    let mut input = spring(1.0, 1e200, 1e100);
    input.max_iterations = Some(1);
    for error in [
        solve_nonlinear_spring_1d(&input).unwrap_err(),
        solve_contact_gap_1d(&contact(input, 0.0)).unwrap_err(),
    ] {
        assert!(
            error.contains("spring") && error.contains("non-finite"),
            "{error}"
        );
    }
    assert!(
        solve_nonlinear_spring_1d(&spring(1.0, 1.0, 2.0))
            .unwrap()
            .converged
    );
}

#[test]
fn zero_residual_does_not_hide_overflow_in_the_assembled_tangent() {
    let mut input = spring(1e308, 0.0, 0.0);
    input.elements.push(input.elements[0].clone());
    for error in [
        solve_nonlinear_spring_1d(&input).unwrap_err(),
        solve_contact_gap_1d(&contact(input, 0.0)).unwrap_err(),
    ] {
        assert!(
            error.contains("tangent") && error.contains("non-finite"),
            "{error}"
        );
    }
}

#[test]
fn support_loads_cannot_overflow_an_equation_removed_by_constraints() {
    let mut input = spring(1.0, 0.0, 1e308);
    input.nodes[0].load_x = 1e308;
    let result = solve_nonlinear_spring_1d(&input).unwrap();
    assert!(result.converged);
    close(result.nodes[1].ux, 1e308);
    let result = solve_contact_gap_1d(&contact(input, 1e308)).unwrap();
    assert!(result.converged);
    assert!(!result.contacts[0].active);
    close(result.nodes[1].ux, 1e308);
}

#[test]
fn contact_penalty_overflow_is_rejected_during_assembly_and_final_output() {
    for iterations in [1, 64] {
        let mut input = contact(spring(1.0, 0.0, 1e100), 0.0);
        input.max_iterations = Some(iterations);
        input.contacts[0].normal_stiffness = 1e300;
        let error = solve_contact_gap_1d(&input).unwrap_err();
        assert!(
            error.contains("stop") && error.contains("non-finite"),
            "{error}"
        );
    }
    assert!(
        solve_contact_gap_1d(&contact(spring(1.0, 0.0, 2.0), 0.0))
            .unwrap()
            .converged
    );
}

#[test]
fn active_contact_cannot_hide_overflow_in_the_assembled_tangent() {
    let mut input = contact(spring(1e308, 0.0, 1e300), 0.0);
    input.contacts[0].normal_stiffness = 1e308;
    let error = solve_contact_gap_1d(&input).unwrap_err();
    assert!(
        error.contains("tangent") && error.contains("non-finite"),
        "{error}"
    );
}

#[test]
fn finite_spring_and_contact_forces_cannot_overflow_the_internal_force_sum() {
    let mut input = contact(spring(1e308, 0.0, 1e308), 0.0);
    input.contacts[0].normal_stiffness = 1e308;
    let error = solve_contact_gap_1d(&input).unwrap_err();
    assert!(
        error.contains("internal force") && error.contains("non-finite"),
        "{error}"
    );
}

#[test]
fn assembly_and_result_construction_can_cancel_and_replay_without_partial_success() {
    for contact_mode in [false, true] {
        for stage in [
            SolverStage::ElementAssembly,
            SolverStage::ResultNodes,
            SolverStage::ResultElements,
            SolverStage::ResultTotals,
        ] {
            let control = SolverControl::default();
            let cancel = control.clone();
            let result = with_solver_observer(
                &control,
                move |progress| {
                    if progress.stage == stage {
                        cancel.request_cancel();
                    }
                },
                || {
                    let input = spring(1.0, 1.0, 2.0);
                    if contact_mode {
                        solve_contact_gap_1d(&contact(input, 0.0)).map(|_| ())
                    } else {
                        solve_nonlinear_spring_1d(&input).map(|_| ())
                    }
                },
            );
            let error = result.expect_err("the numerical call must observe cancellation");
            assert!(error.contains("cancelled"), "{stage:?}: {error}");
            assert!(control.was_interrupted());
            let input = spring(1.0, 1.0, 2.0);
            let replay_converged = if contact_mode {
                solve_contact_gap_1d(&contact(input, 0.0))
                    .unwrap()
                    .converged
            } else {
                solve_nonlinear_spring_1d(&input).unwrap().converged
            };
            assert!(replay_converged);
        }
    }
}

#[test]
fn large_chain_checks_cancellation_inside_assembly_and_result_batches() {
    let mut input = spring(1.0, 0.0, 0.0);
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
    for stage in [
        SolverStage::ElementAssembly,
        SolverStage::SparseValidateMatrix,
        SolverStage::SparseResidual,
        SolverStage::PcgVectorUpdate,
        SolverStage::ResultNodes,
        SolverStage::ResultElements,
        SolverStage::ResultNodeSummary,
        SolverStage::ResultElementSummary,
    ] {
        let control = SolverControl::default();
        let cancel = control.clone();
        let result = with_solver_observer(
            &control,
            move |progress| {
                if progress.stage == stage && progress.completed_steps >= 64 {
                    cancel.request_cancel();
                }
            },
            || solve_nonlinear_spring_1d_owned(input.clone()),
        );
        assert!(result.unwrap_err().contains("cancelled"), "{stage:?}");
        assert!(control.was_interrupted());
    }
    let result = solve_nonlinear_spring_1d_owned(input).unwrap();
    assert!(result.converged);
    close(result.nodes[130].ux, 130.0);
}
