use super::PreparedTridiagonal;
use crate::linear_dense::DenseLu;
use crate::solver_control::{SolverControl, SolverStage, with_solver_observer};
use std::cell::Cell;
use std::rc::Rc;

#[test]
fn forward_range_recovers_finite_solution_from_overflowing_work_vector() {
    let high = 2.0_f64.powi(1023);
    let factor = PreparedTridiagonal::factor(&[8.0, 16.0], &[-8.0], &[-8.0]).unwrap();
    assert!(factor.pivoted.is_none());
    for sign in [-1.0, 1.0] {
        assert_eq!(
            factor.solve(&[sign * high; 2]).unwrap(),
            [sign * 0.375 * high, sign * 0.25 * high]
        );
    }
}

#[test]
fn forward_range_recovers_swapped_rhs_without_dense_fallback() {
    let high = 2.0_f64.powi(1023);
    let factor = PreparedTridiagonal::factor(&[4.0, 8.0], &[8.0], &[-4.0]).unwrap();
    assert!(factor.pivoted.is_some());
    for sign in [-1.0, 1.0] {
        assert_eq!(
            factor
                .solve(&[sign * 1.5 * high, -sign * 1.5 * high])
                .unwrap(),
            [sign * 0.09375 * high, -sign * 0.28125 * high]
        );
    }
}

#[test]
fn forward_range_propagates_and_cancels_extended_intermediates() {
    let high = 2.0_f64.powi(1023);
    for (diagonal, lower, upper, rhs, expected) in [
        (
            [8.0, 16.0, 16.0],
            [-8.0; 2],
            [-8.0; 2],
            [high; 3],
            [0.75 * high, 0.625 * high, 0.375 * high],
        ),
        (
            [8.0; 3],
            [-8.0, 8.0],
            [0.0; 2],
            [high, high, 1.5 * high],
            [0.125 * high, 0.25 * high, -0.0625 * high],
        ),
    ] {
        let factor = PreparedTridiagonal::factor(&diagonal, &lower, &upper).unwrap();
        for sign in [-1.0, 1.0] {
            assert_eq!(
                factor.solve(&rhs.map(|x| sign * x)).unwrap(),
                expected.map(|x| sign * x)
            );
        }
    }
}

#[test]
fn forward_range_does_not_rescale_away_an_independent_minimum_load() {
    let high = 2.0_f64.powi(1023);
    let quantum = f64::from_bits(1);
    let factor =
        PreparedTridiagonal::factor(&[8.0, 16.0, 1.0], &[-8.0, 0.0], &[-8.0, 0.0]).unwrap();
    for sign in [-1.0, 1.0] {
        assert_eq!(
            factor
                .solve(&[sign * high, sign * high, sign * quantum])
                .unwrap(),
            [sign * 0.375 * high, sign * 0.25 * high, sign * quantum]
        );
    }
}

#[test]
fn forward_range_uses_equilibrated_factors_but_validates_original_equations() {
    let high = 2.0_f64.powi(1023);
    let unit = 2.0_f64.powi(-600);
    let mut factor = PreparedTridiagonal::factor(&[unit; 2], &[-unit], &[unit]).unwrap();
    assert!(factor.equilibration.is_some());
    let entered = Rc::new(Cell::new(false));
    let observed = entered.clone();
    let result = with_solver_observer(
        &SolverControl::default(),
        move |point| {
            assert_ne!(point.stage, SolverStage::DenseFactor);
            if point.stage == SolverStage::TridiagonalForwardRange {
                observed.set(true);
            }
        },
        || factor.solve(&[high * unit; 2]),
    )
    .unwrap();
    assert!(entered.get());
    assert_eq!(result, [0.0, high]);
    factor.diagonal[1] *= 2.0;
    assert!(
        factor
            .solve(&[high * unit; 2])
            .unwrap_err()
            .contains("backward-error")
    );
    factor.diagonal[1] = unit;
    assert_eq!(factor.solve(&[-high * unit; 2]).unwrap(), [0.0, -high]);
}

#[test]
fn forward_range_matches_dense_reference_and_normal_load_scaling() {
    let high = 2.0_f64.powi(1023);
    for size in [2, 3, 17, 65] {
        let diagonal = vec![24.0; size];
        let offdiagonal = vec![-8.0; size - 1];
        let factor = PreparedTridiagonal::factor(&diagonal, &offdiagonal, &offdiagonal).unwrap();
        let mut dense = vec![vec![0.0; size]; size];
        for (row, entries) in dense.iter_mut().enumerate() {
            entries[row] = 24.0;
            if row > 0 {
                entries[row - 1] = -8.0;
            }
            if row + 1 < size {
                entries[row + 1] = -8.0;
            }
        }
        let reference = DenseLu::factor(dense)
            .unwrap()
            .solve(&vec![1.5; size])
            .unwrap();
        let normal = factor.solve(&vec![1.5; size]).unwrap();
        for sign in [-1.0, 1.0] {
            let result = factor.solve(&vec![sign * 1.5 * high; size]).unwrap();
            for row in 0..size {
                assert!((result[row] / high - sign * reference[row]).abs() < 1e-13);
                assert!((result[row] / high - sign * normal[row]).abs() < 1e-13);
            }
        }
    }
}

#[test]
fn forward_range_cancels_conversion_forward_and_backward_then_replays() {
    let size = 514;
    let high = 2.0_f64.powi(1023);
    for pivoted in [false, true] {
        let diagonal = (0..size)
            .map(|row| match (pivoted, row % 2) {
                (false, 0) | (true, 1) => 8.0,
                (false, _) => 16.0,
                (true, _) => 4.0,
            })
            .collect::<Vec<_>>();
        let lower = (0..size - 1)
            .map(|row| {
                if row % 2 != 0 {
                    0.0
                } else if pivoted {
                    8.0
                } else {
                    -8.0
                }
            })
            .collect::<Vec<_>>();
        let upper = lower
            .iter()
            .map(|x| if pivoted { -0.5 * x } else { *x })
            .collect::<Vec<_>>();
        let rhs = (0..size)
            .map(|row| {
                if !pivoted {
                    high
                } else if row % 2 == 0 {
                    1.5 * high
                } else {
                    -1.5 * high
                }
            })
            .collect::<Vec<_>>();
        let expected = (0..size)
            .map(|row| {
                high * match (pivoted, row % 2) {
                    (false, 0) => 0.375,
                    (false, _) => 0.25,
                    (true, 0) => 0.09375,
                    (true, _) => -0.28125,
                }
            })
            .collect::<Vec<_>>();
        let factor = PreparedTridiagonal::factor(&diagonal, &lower, &upper).unwrap();
        assert_eq!(factor.pivoted.is_some(), pivoted);
        for step in [0, 64, size, 576, 2 * size, 1088, 3 * size] {
            let control = SolverControl::default();
            let cancel = control.clone();
            let result = with_solver_observer(
                &control,
                move |point| {
                    assert_ne!(point.stage, SolverStage::DenseFactor);
                    if point.stage == SolverStage::TridiagonalForwardRange
                        && point.completed_steps == step as u64
                    {
                        cancel.request_cancel();
                    }
                },
                || factor.solve(&rhs),
            );
            assert!(result.unwrap_err().contains("tridiagonal_forward_range"));
            assert!(control.was_interrupted());
            let point = control.last_checkpoint().unwrap();
            assert_eq!(point.stage, SolverStage::TridiagonalForwardRange);
            assert_eq!(point.completed_steps, step as u64);
            assert_eq!(factor.solve(&rhs).unwrap(), expected);
        }
    }
}

#[test]
fn forward_range_rejects_alignment_loss_and_erased_load_before_clean_replay() {
    let high = 2.0_f64.powi(1023);
    let quantum = f64::from_bits(1);
    let coupled = PreparedTridiagonal::factor(&[8.0, 16.0, 16.0], &[-8.0; 2], &[-8.0; 2]).unwrap();
    for sign in [-1.0, 1.0] {
        let error = coupled
            .solve(&[sign * high, sign * high, sign * quantum])
            .unwrap_err();
        assert!(
            error.contains("normalization would lose numerical range"),
            "{error}"
        );
        assert_eq!(
            coupled.solve(&[sign * high; 3]).unwrap(),
            [sign * 0.75 * high, sign * 0.625 * high, sign * 0.375 * high]
        );
    }
    let separated =
        PreparedTridiagonal::factor(&[8.0, 16.0, 4.0], &[-8.0, 0.0], &[-8.0, 0.0]).unwrap();
    assert!(
        separated
            .solve(&[high, high, quantum])
            .unwrap_err()
            .contains("backward-error")
    );
    assert_eq!(
        separated.solve(&[high, high, 4.0 * quantum]).unwrap(),
        [0.375 * high, 0.25 * high, quantum]
    );
}

#[test]
fn forward_range_rejects_true_solution_overflow_without_poisoning_factors() {
    let high = 2.0_f64.powi(1023);
    let factor = PreparedTridiagonal::factor(&[1.0, 2.0], &[-1.0], &[-1.0]).unwrap();
    for sign in [-1.0, 1.0] {
        assert!(
            factor
                .solve(&[sign * high; 2])
                .unwrap_err()
                .contains("outside the representable range")
        );
        assert_eq!(factor.solve(&[sign; 2]).unwrap(), [3.0 * sign, 2.0 * sign]);
    }
}

#[test]
fn forward_range_adapter_rejects_invalid_internal_shapes_and_operands() {
    use super::forward_range::solve;
    for (diagonal, upper, second, factors, swapped, rhs) in [
        (vec![1.0], vec![], vec![], vec![], vec![], vec![]),
        (
            vec![1.0; 2],
            vec![],
            vec![],
            vec![0.0],
            vec![],
            vec![1.0; 2],
        ),
        (
            vec![1.0; 2],
            vec![0.0],
            vec![],
            vec![],
            vec![],
            vec![1.0; 2],
        ),
        (
            vec![1.0; 2],
            vec![0.0],
            vec![],
            vec![0.0],
            vec![false; 2],
            vec![1.0; 2],
        ),
        (
            vec![1.0; 3],
            vec![0.0; 2],
            vec![],
            vec![0.0; 2],
            vec![false; 2],
            vec![1.0; 3],
        ),
        (
            vec![1.0; 3],
            vec![0.0; 2],
            vec![0.0],
            vec![0.0; 2],
            vec![],
            vec![1.0; 3],
        ),
    ] {
        assert!(solve(&diagonal, &upper, &second, &factors, &swapped, &rhs).is_err());
    }
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        for column in 0..5 {
            let mut values = [1.0, 0.0, 0.0, 0.0, 1.0];
            values[column] = bad;
            assert!(
                solve(
                    &[values[0]; 3],
                    &[values[1]; 2],
                    &[values[2]],
                    &[values[3]; 2],
                    &[false; 2],
                    &[values[4]; 3]
                )
                .is_err()
            );
        }
    }
    assert!(solve(&[0.0], &[], &[], &[], &[], &[1.0]).is_err());
    assert_eq!(
        solve(&[], &[], &[], &[], &[], &[]).unwrap(),
        Vec::<f64>::new()
    );
}

#[test]
fn extended_rhs_checks_exponents_and_operands_without_integer_overflow() {
    use crate::linear_binary_scale::restore;
    use crate::linear_substitution::solve_scaled_rhs_row;
    let stage = SolverStage::TridiagonalSubstitutionRange;
    let empty = std::iter::empty::<(f64, f64)>();
    assert_eq!(
        solve_scaled_rhs_row((1.0, 1024), empty.clone(), 8.0, stage).unwrap(),
        2.0_f64.powi(1021)
    );
    assert_eq!(
        solve_scaled_rhs_row((1.0, -1076), empty.clone(), 0.25, stage).unwrap(),
        f64::from_bits(1)
    );
    for exponent in [i32::MIN, i32::MIN + 1, i32::MAX - 1, i32::MAX] {
        for pivot in [0.5, 1.0, 2.0] {
            assert!(solve_scaled_rhs_row((1.0, exponent), empty.clone(), pivot, stage).is_err());
            assert!(
                solve_scaled_rhs_row((1.0, exponent), [(1.0, 1.0)].into_iter(), pivot, stage)
                    .is_err()
            );
        }
        assert_eq!(
            solve_scaled_rhs_row((0.0, exponent), empty.clone(), 1.0, stage).unwrap(),
            0.0
        );
    }
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(solve_scaled_rhs_row((bad, 0), empty.clone(), 1.0, stage).is_err());
        assert!(solve_scaled_rhs_row((1.0, 1024), empty.clone(), bad, stage).is_err());
        assert!(solve_scaled_rhs_row((1.0, 1024), [(bad, 1.0)].into_iter(), 1.0, stage).is_err());
        assert!(solve_scaled_rhs_row((1.0, 1024), [(1.0, bad)].into_iter(), 1.0, stage).is_err());
    }
    assert!(solve_scaled_rhs_row((1.0, 1024), empty, 0.0, stage).is_err());
    assert_eq!(restore(2.0, i32::MAX), f64::INFINITY);
    assert_eq!(restore(-0.5, i32::MIN).to_bits(), (-0.0_f64).to_bits());
}
