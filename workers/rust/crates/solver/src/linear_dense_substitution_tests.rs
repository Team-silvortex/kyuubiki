use super::DenseLu;
use super::substitution::solve_row;
use crate::linear_algebra::{PreparedSpdSolver, SparseMatrix, add_at, solve_spd_system_profile};
use crate::solver_control::{SolverControl, SolverStage, with_solver_observer};
use std::cell::Cell;
use std::rc::Rc;

#[test]
fn ordinary_matrix_with_large_rhs_does_not_overflow_before_pivot_division() {
    let high = 2.0_f64.powi(1023);
    let factor = DenseLu::factor(vec![vec![2.0, -1.0], vec![-1.0, 2.0]]).unwrap();
    assert_eq!(factor.solve(&[high, high]).unwrap(), [high, high]);
}

#[test]
fn back_substitution_recovers_cancelling_overflowed_products() {
    let high = 2.0_f64.powi(1023);
    let factor = DenseLu::factor(vec![
        vec![1.0, 2.0, -2.0],
        vec![0.0, 1.0, 0.0],
        vec![0.0, 0.0, 1.0],
    ])
    .unwrap();
    assert_eq!(factor.solve(&[1.0, high, high]).unwrap(), [1.0, high, high]);
}

#[test]
fn forward_substitution_recovers_cancelling_overflowed_products() {
    let high = 2.0_f64.powi(1023);
    let factor = DenseLu::factor(vec![
        vec![1.0, 0.0, 0.0],
        vec![0.0, 1.0, 0.0],
        vec![2.0, -2.0, 4.0],
    ])
    .unwrap();
    assert_eq!(factor.solve(&[high, high, 4.0]).unwrap(), [high, high, 1.0]);
}

#[test]
fn substitution_preserves_fused_product_tails_after_overflow_cancellation() {
    let high = 2.0_f64.powi(1023);
    let delta = 2.0_f64.powi(-27);
    let factor = DenseLu::factor(vec![
        vec![4.0, 2.0 * (1.0 + delta), -2.0],
        vec![0.0, 1.0, 0.0],
        vec![0.0, 0.0, 1.0],
    ])
    .unwrap();
    // 2(1+d) * 2^1023(1-d) - 2 * 2^1023 = -2^970 exactly.
    let result = factor.solve(&[0.0, high * (1.0 - delta), high]).unwrap();
    assert_eq!(result, [2.0_f64.powi(968), high * (1.0 - delta), high]);
}

#[test]
fn substitution_recovers_erased_and_rounded_subnormal_products_before_division() {
    let quantum = f64::from_bits(1);
    for sign in [1.0, -1.0] {
        let value = sign * quantum;
        let erased = DenseLu::factor(vec![
            vec![1.0, 0.5, 0.5],
            vec![0.0, 1.0, 0.0],
            vec![0.0, 0.0, 1.0],
        ])
        .unwrap();
        assert_eq!(
            erased.solve(&[value, value, value]).unwrap(),
            [0.0, value, value]
        );
        let rounded = DenseLu::factor(vec![vec![0.25, 0.75], vec![0.0, 1.0]]).unwrap();
        assert_eq!(rounded.solve(&[value, value]).unwrap(), [value, value]);
    }
}

#[test]
fn direct_and_prepared_spd_recover_large_rhs_without_relaxing_original_balance() {
    let mut matrix = SparseMatrix::new(3);
    for (row, entries) in [[2.0, -1.0, -0.25], [-1.0, 2.0, -1.0], [-0.25, -1.0, 2.0]]
        .iter()
        .enumerate()
    {
        for (column, value) in entries.iter().enumerate() {
            add_at(&mut matrix, row, column, *value);
        }
    }
    let prepared = PreparedSpdSolver::factor(matrix.clone()).unwrap();
    for high in [2.0_f64.powi(1023), -2.0_f64.powi(1023), 0.0] {
        let rhs = [0.75 * high, 0.0, 0.75 * high];
        let direct = solve_spd_system_profile(&matrix, &rhs).unwrap();
        assert_eq!(direct.solution, [high; 3]);
        assert_eq!(direct.residual_norm, 0.0);
        assert_eq!(prepared.solve(&rhs).unwrap(), direct.solution);
    }
}

#[test]
fn substitution_rejects_unresolved_range_and_invalid_operands_before_replay() {
    let high = 2.0_f64.powi(1023);
    let quantum = f64::from_bits(1);
    assert!(
        solve_row(quantum, &[2.0, -2.0], &[high, high], 1.0)
            .unwrap_err()
            .contains("normalization would lose numerical range")
    );
    for (rhs, pivot) in [(f64::MAX, 0.5), (quantum, 2.0)] {
        assert!(
            solve_row(rhs, &[], &[], pivot)
                .unwrap_err()
                .contains("solution is outside the representable range")
        );
    }
    for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(solve_row(invalid, &[], &[], 1.0).is_err());
        assert!(solve_row(1.0, &[], &[], invalid).is_err());
        // An early overflow cannot hide an invalid later operand.
        assert!(
            solve_row(1.0, &[2.0, invalid], &[high, 1.0], 1.0)
                .unwrap_err()
                .contains("non-finite")
        );
        assert!(
            solve_row(1.0, &[2.0, 0.0], &[high, invalid], 1.0)
                .unwrap_err()
                .contains("non-finite")
        );
    }
    assert!(solve_row(1.0, &[], &[], 0.0).is_err());
    assert!(solve_row(1.0, &[1.0], &[], 1.0).is_err());
    assert_eq!(
        solve_row(1.0, &[2.0, -2.0], &[high, high], 1.0).unwrap(),
        1.0
    );
}

#[test]
fn ordinary_substitution_keeps_original_bits_and_avoids_range_passes() {
    let controls = SolverControl::default();
    let range_visits = Rc::new(Cell::new(0));
    let observed = range_visits.clone();
    with_solver_observer(
        &controls,
        move |point| {
            if point.stage == SolverStage::DenseSubstitutionRange {
                observed.set(observed.get() + 1);
            }
        },
        || {
            for (rhs, pivot, coefficients, values) in [
                (1.0, 3.0, vec![0.1, -0.2, 0.3], vec![1.0, 2.0, 3.0]),
                (-0.0, -1.0, vec![], vec![]),
                (0.0, 1.0, vec![-0.0, 0.0], vec![1.0, -2.0]),
                (-2.0, 7.0, vec![3.0, -4.0, 2.0], vec![0.25, -0.5, 1.0]),
            ] {
                let direct: f64 = (rhs
                    - coefficients
                        .iter()
                        .zip(&values)
                        .map(|(a, x)| a * x)
                        .sum::<f64>())
                    / pivot;
                assert_eq!(
                    solve_row(rhs, &coefficients, &values, pivot)
                        .unwrap()
                        .to_bits(),
                    direct.to_bits()
                );
            }
            Ok::<_, String>(())
        },
    )
    .unwrap();
    assert_eq!(range_visits.get(), 0);
}

fn cancel_at<T>(step: u64, run: impl FnOnce() -> Result<T, String>) {
    let control = SolverControl::default();
    let cancel = control.clone();
    let result = with_solver_observer(
        &control,
        move |point| {
            if point.stage == SolverStage::DenseSubstitutionRange && point.completed_steps == step {
                cancel.request_cancel();
            }
        },
        run,
    );
    assert!(result.is_err());
    assert!(control.was_interrupted());
    assert_eq!(control.last_checkpoint().unwrap().completed_steps, step);
}

#[test]
fn substitution_range_passes_cancel_and_prepared_factors_replay_cleanly() {
    let high = 2.0_f64.powi(1023);
    let mut coefficients = vec![0.0; 2049];
    coefficients[..2].copy_from_slice(&[2.0, -2.0]);
    let values = vec![high; coefficients.len()];
    for step in [0, 1024, 2049, 3072, 4098] {
        cancel_at(step, || solve_row(1.0, &coefficients, &values, 1.0));
        assert_eq!(solve_row(1.0, &coefficients, &values, 1.0).unwrap(), 1.0);
    }
    let factor = DenseLu::factor(vec![vec![2.0, -1.0], vec![-1.0, 2.0]]).unwrap();
    cancel_at(0, || factor.solve(&[high, high]));
    assert_eq!(factor.solve(&[high, high]).unwrap(), [high, high]);
    let underflow = DenseLu::factor(vec![vec![2.0]]).unwrap();
    assert!(underflow.solve(&[f64::from_bits(1)]).is_err());
    assert_eq!(underflow.solve(&[2.0]).unwrap(), [1.0]);
}
