use kyuubiki_protocol::{
    NonlinearSpring1dElementInput, NonlinearSpring1dNodeInput, SolveNonlinearSpring1dRequest,
};
use kyuubiki_solver::solver_control::{SolverControl, SolverStage, with_solver_observer};
use kyuubiki_solver::{solve_nonlinear_spring_1d, solve_nonlinear_spring_1d_owned};

fn model(unit: f64) -> SolveNonlinearSpring1dRequest {
    SolveNonlinearSpring1dRequest {
        nodes: (0..4)
            .map(|i| NonlinearSpring1dNodeInput {
                id: format!("n{i}"),
                x: i as f64,
                fix_x: i == 0 || i == 3,
                load_x: if i == 1 || i == 2 { 1.5 * unit } else { 0.0 },
            })
            .collect(),
        elements: (0..3)
            .map(|i| NonlinearSpring1dElementInput {
                id: format!("e{i}"),
                node_i: i,
                node_j: i + 1,
                stiffness: 0.5 * unit,
                cubic_stiffness: 0.0,
            })
            .collect(),
        load_steps: Some(1),
        max_iterations: Some(4),
        tolerance: Some(unit * 1e-12),
    }
}

#[test]
fn finite_two_support_chain_is_invariant_under_extreme_force_units() {
    for unit in [2.0_f64.powi(1023), 1.0, 2.0_f64.powi(-1000)] {
        let input = model(unit);
        for result in [
            solve_nonlinear_spring_1d(&input).unwrap(),
            solve_nonlinear_spring_1d_owned(input).unwrap(),
        ] {
            assert!(result.converged, "{:?}", result.steps);
            assert_eq!(result.achieved_load_factor, Some(1.0));
            for (node, expected) in result.nodes.iter().zip([0.0, 3.0, 3.0, 0.0]) {
                assert!((node.ux - expected).abs() < 1e-12, "{node:?}");
            }
            for (element, expected) in result.elements.iter().zip([1.5, 0.0, -1.5]) {
                assert!(
                    (element.force / unit - expected).abs() < 1e-12,
                    "{element:?}"
                );
            }
            assert!(result.residual_norm <= result.input.tolerance.unwrap());
        }
    }
}

#[test]
fn cancellation_during_equation_scaling_remains_an_error_and_replays() {
    let input = model(2.0_f64.powi(1023));
    let control = SolverControl::default();
    let cancel = control.clone();
    let result = with_solver_observer(
        &control,
        move |point| {
            assert_ne!(point.stage, SolverStage::DenseFactor);
            if point.stage == SolverStage::TridiagonalFactor && point.completed_steps > 0 {
                cancel.request_cancel();
            }
        },
        || solve_nonlinear_spring_1d(&input),
    );
    assert!(result.unwrap_err().contains("cancelled"));
    assert!(control.was_interrupted());
    let replay = solve_nonlinear_spring_1d_owned(input).unwrap();
    assert!(replay.converged);
    assert_eq!(replay.achieved_load_factor, Some(1.0));
}
