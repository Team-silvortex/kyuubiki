use kyuubiki_protocol::{
    ContactGap1dContactInput, NonlinearSpring1dElementInput, NonlinearSpring1dNodeInput,
    SolveContactGap1dRequest,
};
use kyuubiki_solver::solver_control::{SolverControl, SolverStage, with_solver_observer};
use kyuubiki_solver::{solve_contact_gap_1d, solve_contact_gap_1d_owned};
use std::cell::Cell;

fn model(scale: f64, reverse: bool) -> SolveContactGap1dRequest {
    let mut nodes = vec![
        NonlinearSpring1dNodeInput {
            id: "base".into(),
            x: 0.0,
            fix_x: true,
            load_x: 0.0,
        },
        NonlinearSpring1dNodeInput {
            id: "loaded".into(),
            x: 1.0,
            fix_x: false,
            load_x: 2.0 * scale,
        },
        NonlinearSpring1dNodeInput {
            id: "tail".into(),
            x: 2.0,
            fix_x: false,
            load_x: 0.0,
        },
    ];
    let (loaded, tail) = if reverse {
        nodes.swap(1, 2);
        (2, 1)
    } else {
        (1, 2)
    };
    SolveContactGap1dRequest {
        nodes,
        elements: vec![
            NonlinearSpring1dElementInput {
                id: "softening".into(),
                node_i: 0,
                node_j: loaded,
                stiffness: 2.0 * scale,
                cubic_stiffness: -scale,
            },
            NonlinearSpring1dElementInput {
                id: "tail".into(),
                node_i: loaded,
                node_j: tail,
                stiffness: scale,
                cubic_stiffness: 0.0,
            },
        ],
        contacts: vec![ContactGap1dContactInput {
            id: "stop".into(),
            node: loaded,
            gap: 1.0,
            normal_stiffness: 5.8125 * scale,
        }],
        load_steps: Some(1),
        max_iterations: Some(32),
        tolerance: Some(1e-12 * scale),
    }
}

#[test]
fn contact_switch_crosses_invertible_zero_pivot_independent_of_node_order() {
    // First Newton state: both free displacements are 1, contact just touches,
    // and K/scale = [[0, -1], [-1, 1]]. Its determinant is -1, not zero.
    // Final state u = 1.25 has spring tangent -2.6875 and contact stiffness 5.8125.
    for scale in [2.0_f64.powi(-200), 1.0, 2.0_f64.powi(200)] {
        for reverse in [true, false] {
            let input = model(scale, reverse);
            for result in [
                solve_contact_gap_1d(&input)
                    .unwrap_or_else(|error| panic!("scale={scale}, reverse={reverse}: {error}")),
                solve_contact_gap_1d_owned(input).unwrap(),
            ] {
                assert!(result.converged, "{:?}", result.steps);
                assert_eq!(result.achieved_load_factor, Some(1.0));
                for node in &result.nodes[1..] {
                    assert!((node.ux - 1.25).abs() < 1e-10, "{node:?}");
                }
                assert!(result.contacts[0].active);
                assert!((result.contacts[0].force / scale - 1.453125).abs() < 1e-10);
                assert!((result.elements[0].force / scale - 0.546875).abs() < 1e-10);
                assert!(result.residual_norm <= result.input.tolerance.unwrap());
            }
        }
    }
}

#[test]
fn pivoted_newton_budget_exhaustion_does_not_commit_a_failed_trial() {
    let mut input = model(1.0, false);
    input.max_iterations = Some(2);
    let failed = solve_contact_gap_1d(&input).unwrap();
    assert!(!failed.converged);
    assert_eq!(failed.achieved_load_factor, Some(0.0));
    assert_eq!(failed.steps[0].iterations, 2);
    assert!(failed.steps[0].residual_norm > input.tolerance.unwrap());
    assert!(failed.steps[0].residual_norm.is_finite());
    assert!(failed.nodes.iter().all(|node| node.ux == 0.0));
    assert!(!failed.contacts[0].active);
    assert_eq!(failed.residual_norm, 0.0);
    input.max_iterations = Some(32);
    assert!(solve_contact_gap_1d_owned(input).unwrap().converged);
}

#[test]
fn cancellation_at_pivoted_factorization_stays_an_error_and_replays_cleanly() {
    let control = SolverControl::default();
    let cancel = control.clone();
    let starts = Cell::new(0);
    let result = with_solver_observer(
        &control,
        move |point| {
            assert_ne!(point.stage, SolverStage::DenseFactor);
            if point.stage == SolverStage::TridiagonalFactor && point.completed_steps == 0 {
                starts.set(starts.get() + 1);
                // First correction uses Thomas; second enters the pivoted fallback.
                if starts.get() == 3 {
                    cancel.request_cancel();
                }
            }
        },
        || solve_contact_gap_1d(&model(1.0, false)),
    );
    assert!(result.unwrap_err().contains("cancelled"));
    assert!(control.was_interrupted());
    assert!(
        solve_contact_gap_1d_owned(model(1.0, false))
            .unwrap()
            .converged
    );
}
