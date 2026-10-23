use super::solve_with_prescribed;
use crate::solver_control::{SolverControl, SolverStage, with_solver_observer};

#[test]
fn prescribed_range_retains_a_half_quantum_boundary_load_before_division() {
    for sign in [-1.0, 1.0] {
        let value = sign * f64::from_bits(1);
        let result =
            solve_with_prescribed(&[0.5; 2], &[-0.5], &[-0.5], &[0.0; 2], &[(0, value)]).unwrap();
        assert_eq!(result, [value; 2]);
    }
}

#[test]
fn prescribed_range_combines_two_half_quantum_boundary_contributions() {
    for sign in [-1.0, 1.0] {
        let value = sign * f64::from_bits(1);
        let result = solve_with_prescribed(
            &[0.5, 1.0, 0.5],
            &[-0.5; 2],
            &[-0.5; 2],
            &[0.0; 3],
            &[(0, value), (2, value)],
        )
        .unwrap();
        assert_eq!(result, [value; 3]);
    }
}

#[test]
fn prescribed_range_recovers_finite_response_from_overflowing_boundary_product() {
    for sign in [-1.0, 1.0] {
        let value = sign * 2.0_f64.powi(1023);
        let result =
            solve_with_prescribed(&[2.0; 2], &[-2.0], &[-2.0], &[0.0; 2], &[(0, value)]).unwrap();
        assert_eq!(result, [value; 2]);
    }
}

#[test]
fn prescribed_range_cancels_overflowing_boundaries_without_erasing_the_load() {
    let value = 2.0_f64.powi(1023);
    let result = solve_with_prescribed(
        &[1.0; 3],
        &[2.0, 0.0],
        &[0.0, -2.0],
        &[0.0, 1.0, 0.0],
        &[(0, value), (2, value)],
    )
    .unwrap();
    assert_eq!(result, [value, 1.0, value]);
}

#[test]
fn prescribed_range_is_independent_of_boundary_side_sign_and_coefficient_scale() {
    for coefficient in [2.0_f64.powi(-600), 0.5, 2.0_f64.powi(600)] {
        for bits in [1, 3, 7, 511] {
            for sign in [-1.0, 1.0] {
                for index in [0, 1] {
                    let value = sign * f64::from_bits(bits);
                    let result = solve_with_prescribed(
                        &[coefficient; 2],
                        &[-coefficient],
                        &[-coefficient],
                        &[0.0; 2],
                        &[(index, value)],
                    )
                    .unwrap();
                    assert_eq!(result, [value; 2]);
                }
            }
        }
    }
}

#[test]
fn prescribed_range_uses_scaled_rows_with_pivoting_and_multiple_free_unknowns() {
    for sign in [-1.0, 1.0] {
        let value = sign * 2.0_f64.powi(1023);
        let result = solve_with_prescribed(
            &[2.0, 4.0, 2.0],
            &[-2.0; 2],
            &[-2.0; 2],
            &[0.0; 3],
            &[(0, value)],
        )
        .unwrap();
        assert_eq!(result, [value; 3]);
        let result = solve_with_prescribed(
            &[1.0, 0.0, 2.0],
            &[-2.0, 2.0],
            &[0.0, 2.0],
            &[0.0; 3],
            &[(0, value)],
        )
        .unwrap();
        assert_eq!(result, [value, -value, value]);
    }
}

#[test]
fn prescribed_range_does_not_rescale_away_an_independent_quantum_load() {
    let size = 514;
    let quantum = f64::from_bits(1);
    let mut diagonal = vec![1.0; size];
    diagonal[1] = 0.5;
    let mut lower = vec![0.0; size - 1];
    lower[0] = -0.5;
    let upper = vec![0.0; size - 1];
    let mut rhs = vec![0.0; size];
    rhs[size - 1] = quantum;
    let solved = with_solver_observer(
        &SolverControl::default(),
        |point| assert_ne!(point.stage, SolverStage::DenseFactor),
        || solve_with_prescribed(&diagonal, &lower, &upper, &rhs, &[(0, quantum)]),
    )
    .unwrap();
    assert_eq!(solved[0], quantum);
    assert_eq!(solved[1], quantum);
    assert!(solved[2..size - 1].iter().all(|&value| value == 0.0));
    assert_eq!(solved[size - 1], quantum);
}

#[test]
fn prescribed_range_keeps_ordinary_subtraction_order_and_bits() {
    for sign in [-1.0, 1.0] {
        for load in [0.0, 0.3, 1.0, 10.0] {
            let left = sign * 0.7;
            let right = sign * 1.1;
            let expected = ((load - 0.3 * left) - (-0.7 * right)) / 2.0;
            let result = solve_with_prescribed(
                &[1.0, 2.0, 1.0],
                &[0.3, 0.0],
                &[0.0, -0.7],
                &[0.0, load, 0.0],
                &[(0, left), (2, right)],
            )
            .unwrap();
            assert_eq!(result[1].to_bits(), expected.to_bits());
        }
    }
}

#[test]
fn prescribed_range_does_not_give_fixed_inputs_a_solution_rounding_allowance() {
    let quantum = f64::from_bits(1);
    let operands = [(0.0, 1.0), (0.5, 0.0), (-0.5, quantum), (0.0, 0.0)];
    assert_eq!(
        super::residual::row_backward_error(operands, [true; 3]).unwrap(),
        0.0
    );
    assert_eq!(
        super::residual::row_backward_error(operands, [true, false, false]).unwrap(),
        0.5
    );
    // Fixed equations can have reactions; only free equations must balance.
    assert_eq!(
        solve_with_prescribed(
            &[1.0; 2],
            &[0.0],
            &[0.0],
            &[0.0; 2],
            &[(0, quantum), (1, 7.0)],
        )
        .unwrap(),
        [quantum, 7.0]
    );
}

#[test]
fn prescribed_range_rejects_true_overflow_and_irreversible_scaling_then_replays() {
    let large = 2.0_f64.powi(1023);
    let error =
        solve_with_prescribed(&[1.0; 2], &[-4.0], &[0.0], &[0.0; 2], &[(0, large)]).unwrap_err();
    assert!(error.contains("representable range"), "{error}");
    assert_eq!(
        solve_with_prescribed(&[1.0; 2], &[-4.0], &[0.0], &[0.0; 2], &[(0, 1.0)],).unwrap(),
        [1.0, 4.0]
    );
    let error = solve_with_prescribed(
        &[1.0, large, 1.0],
        &[-large, 0.0],
        &[0.0, f64::from_bits(1)],
        &[0.0; 3],
        &[(0, 2.0)],
    )
    .unwrap_err();
    assert!(error.contains("normalization"), "{error}");
    assert_eq!(
        solve_with_prescribed(
            &[1.0; 3],
            &[-large, 0.0],
            &[0.0, f64::from_bits(1)],
            &[0.0; 3],
            &[(0, 1.0)],
        )
        .unwrap(),
        [1.0, large, 0.0]
    );
}

#[test]
fn prescribed_range_preserves_valid_quantized_responses_and_rejects_erased_reduced_loads() {
    let quantum = f64::from_bits(1);
    for sign in [-1.0, 1.0] {
        let value = sign * quantum;
        assert_eq!(
            solve_with_prescribed(&[1.0, 3.0], &[-4.0], &[0.0], &[0.0; 2], &[(0, value)],).unwrap(),
            [value, value]
        );
        let error = solve_with_prescribed(&[1.0, 2.0], &[-0.5], &[0.0], &[0.0; 2], &[(0, value)])
            .unwrap_err();
        assert!(error.contains("representable range"), "{error}");
    }
}

#[test]
fn prescribed_range_cancellation_keeps_inputs_reusable_and_never_publishes_a_partial_result() {
    let quantum = f64::from_bits(1);
    let boundaries = [(0, quantum), (2, quantum)];
    for (stage, step) in [
        (SolverStage::ConstraintIndex, 0),
        (SolverStage::ConstraintIndex, 2),
        (SolverStage::ConstraintReduce, 0),
        (SolverStage::ConstraintReduce, 2),
        (SolverStage::ConstraintReduce, 4),
        (SolverStage::ResidualValidate, 0),
        (SolverStage::ResidualValidate, 1),
        (SolverStage::ResultPrescribed, 3),
    ] {
        let control = SolverControl::default();
        let cancel = control.clone();
        let result = with_solver_observer(
            &control,
            move |point| {
                if point.stage == stage && point.completed_steps == step {
                    cancel.request_cancel();
                }
            },
            || {
                solve_with_prescribed(
                    &[0.5, 1.0, 0.5],
                    &[-0.5; 2],
                    &[-0.5; 2],
                    &[0.0; 3],
                    &boundaries,
                )
            },
        );
        assert!(result.unwrap_err().contains("cancelled"));
        assert!(control.was_interrupted());
        assert_eq!(
            solve_with_prescribed(
                &[0.5, 1.0, 0.5],
                &[-0.5; 2],
                &[-0.5; 2],
                &[0.0; 3],
                &boundaries,
            )
            .unwrap(),
            [quantum; 3]
        );
    }
}

#[test]
fn prescribed_range_cancels_long_constraint_scans_and_reuses_the_same_inputs() {
    let size = 513;
    let diagonal = vec![1.0; size];
    let off = vec![0.0; size - 1];
    let rhs = vec![0.0; size];
    let boundaries = (0..size)
        .map(|index| (index, index as f64))
        .collect::<Vec<_>>();
    for stage in [SolverStage::ConstraintIndex, SolverStage::ConstraintMap] {
        for step in [64, 512, size] {
            let control = SolverControl::default();
            let cancel = control.clone();
            let result = with_solver_observer(
                &control,
                move |point| {
                    if point.stage == stage && point.completed_steps == step as u64 {
                        cancel.request_cancel();
                    }
                },
                || solve_with_prescribed(&diagonal, &off, &off, &rhs, &boundaries),
            );
            assert!(result.unwrap_err().contains("cancelled"));
            assert_eq!(
                solve_with_prescribed(&diagonal, &off, &off, &rhs, &boundaries).unwrap(),
                boundaries
                    .iter()
                    .map(|&(_, value)| value)
                    .collect::<Vec<_>>()
            );
        }
    }
}
