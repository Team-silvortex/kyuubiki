use super::PreparedTridiagonal;
use crate::solver_control::{SolverControl, SolverStage, with_solver_observer};

#[test]
fn forward_underflow_recovers_erased_and_rounded_work_entries() {
    let quantum = f64::from_bits(1);
    let factor = PreparedTridiagonal::factor(&[1.0, 0.5], &[0.5], &[0.0]).unwrap();
    assert!(factor.pivoted.is_none());
    for units in [1.0, 3.0, 5.0, 7.0] {
        for sign in [-1.0, 1.0] {
            let load = sign * units * quantum;
            assert_eq!(factor.solve(&[load, 0.0]).unwrap(), [load, -load]);
        }
    }
}

#[test]
fn forward_underflow_recovers_after_adjacent_swap() {
    let quantum = f64::from_bits(1);
    let factor = PreparedTridiagonal::factor(&[0.5, 0.0], &[1.0], &[0.5]).unwrap();
    assert!(factor.pivoted.is_some());
    for units in [1.0, 3.0, 5.0, 7.0] {
        for sign in [-1.0, 1.0] {
            let load = sign * units * quantum;
            assert_eq!(factor.solve(&[0.0, load]).unwrap(), [load, -load]);
        }
    }
}

#[test]
fn forward_underflow_propagates_below_the_smallest_float() {
    let quantum = f64::from_bits(1);
    let factor = PreparedTridiagonal::factor(&[1.0, 0.5, 0.25], &[0.5, 0.25], &[0.0; 2]).unwrap();
    for sign in [-1.0, 1.0] {
        let load = sign * quantum;
        assert_eq!(
            factor.solve(&[load, 0.0, 0.0]).unwrap(),
            [load, -load, load]
        );
    }
}

#[test]
fn forward_underflow_keeps_fused_cancellation_with_nonzero_rhs() {
    let quantum = f64::from_bits(1);
    let delta = 2.0_f64.powi(-27);
    for (coefficient, pivot, rhs, expected) in [
        (0.5, 0.5, [3.0 * quantum, quantum], -quantum),
        (
            1.0 - delta,
            0.25,
            [(1.0 + delta) * f64::MIN_POSITIVE, f64::MIN_POSITIVE],
            quantum,
        ),
    ] {
        let factor = PreparedTridiagonal::factor(&[1.0, pivot], &[coefficient], &[0.0]).unwrap();
        for sign in [-1.0, 1.0] {
            assert_eq!(
                factor.solve(&rhs.map(|value| sign * value)).unwrap(),
                [sign * rhs[0], sign * expected]
            );
        }
    }
}

#[test]
fn forward_underflow_checks_values_rounded_up_to_minimum_normal() {
    let quantum = f64::from_bits(1);
    let coefficient = 1.0 - 0.5 * f64::EPSILON;
    let factor = PreparedTridiagonal::factor(&[1.0, 0.5], &[coefficient], &[0.0]).unwrap();
    for sign in [-1.0, 1.0] {
        assert_eq!(
            factor.solve(&[sign * f64::MIN_POSITIVE, 0.0]).unwrap(),
            [
                sign * f64::MIN_POSITIVE,
                sign * (-2.0 * f64::MIN_POSITIVE + quantum)
            ]
        );
    }
}

#[test]
fn forward_underflow_probe_keeps_exact_zeros_and_representable_subnormals() {
    let quantum = f64::from_bits(1);
    let factor = PreparedTridiagonal::factor(&[1.0; 2], &[0.5], &[0.0]).unwrap();
    with_solver_observer(
        &SolverControl::default(),
        |point| assert_ne!(point.stage, SolverStage::TridiagonalForwardRange),
        || {
            for rhs in [[0.0, -0.0], [2.0, 1.0], [2.0 * quantum, 0.0]] {
                let expected = (-0.5_f64).mul_add(rhs[0], rhs[1]);
                let actual = factor.solve(&rhs)?;
                assert_eq!(actual[1].to_bits(), expected.to_bits());
            }
            Ok::<_, String>(())
        },
    )
    .unwrap();
}

#[test]
fn forward_underflow_probe_does_not_reject_irrelevant_quantized_tails() {
    let quantum = f64::from_bits(1);
    let rhs = f64::from_bits(f64::MIN_POSITIVE.to_bits() - 1);
    let factor = PreparedTridiagonal::factor(&[1.0; 2], &[quantum], &[0.0]).unwrap();
    with_solver_observer(
        &SolverControl::default(),
        |point| assert_ne!(point.stage, SolverStage::TridiagonalForwardRange),
        || {
            assert_eq!(factor.solve(&[quantum, rhs])?, [quantum, rhs]);
            Ok::<_, String>(())
        },
    )
    .unwrap();
}

#[test]
fn forward_underflow_keeps_final_quantized_tail_without_erasing_independent_load() {
    let size = 2048;
    let factor =
        PreparedTridiagonal::factor(&vec![1.0; size], &vec![0.5; size - 1], &vec![0.0; size - 1])
            .unwrap();
    for sign in [-1.0, 1.0] {
        let mut rhs = vec![0.0; size];
        rhs[0] = sign;
        let solution = factor.solve(&rhs).unwrap();
        for (row, &value) in solution.iter().enumerate() {
            let expected = crate::linear_binary_scale::restore(
                if row % 2 == 0 { sign } else { -sign },
                -(row as i32),
            );
            assert_eq!(value, expected, "row {row}");
        }
    }

    let quantum = f64::from_bits(1);
    let factor = PreparedTridiagonal::factor(&[1.0, 0.5, 4.0], &[0.5, 0.0], &[0.0; 2]).unwrap();
    assert!(
        factor
            .solve(&[quantum, 0.0, quantum])
            .unwrap_err()
            .contains("backward-error")
    );
    assert_eq!(
        factor.solve(&[quantum, 0.0, 4.0 * quantum]).unwrap(),
        [quantum, -quantum, quantum]
    );
}

#[test]
fn forward_underflow_and_overflow_share_replay_without_global_scaling() {
    let high = 2.0_f64.powi(1023);
    let quantum = f64::from_bits(1);
    for (diagonal, lower, upper, rhs, expected) in [
        (
            [8.0, 16.0, 1.0, 0.5],
            [-8.0, 0.0, 0.5],
            [-8.0, 0.0, 0.0],
            [high, high, quantum, 0.0],
            [0.375 * high, 0.25 * high, quantum, -quantum],
        ),
        (
            [1.0, 0.5, 8.0, 16.0],
            [0.5, 0.0, -8.0],
            [0.0, 0.0, -8.0],
            [quantum, 0.0, high, high],
            [quantum, -quantum, 0.375 * high, 0.25 * high],
        ),
    ] {
        let factor = PreparedTridiagonal::factor(&diagonal, &lower, &upper).unwrap();
        for sign in [-1.0, 1.0] {
            assert_eq!(
                factor.solve(&rhs.map(|value| sign * value)).unwrap(),
                expected.map(|value| sign * value)
            );
        }
    }
}

#[test]
fn forward_underflow_cancels_each_recovery_phase_and_reuses_factors() {
    let size = 514;
    let quantum = f64::from_bits(3);
    for pivoted in [false, true] {
        let diagonal = (0..size)
            .map(|row| match (pivoted, row % 2) {
                (true, 1) => 0.0,
                (false, 0) => 1.0,
                _ => 0.5,
            })
            .collect::<Vec<_>>();
        let lower = (0..size - 1)
            .map(|row| {
                if row % 2 == 1 {
                    0.0
                } else if pivoted {
                    1.0
                } else {
                    0.5
                }
            })
            .collect::<Vec<_>>();
        let upper = (0..size - 1)
            .map(|row| if pivoted && row % 2 == 0 { 0.5 } else { 0.0 })
            .collect::<Vec<_>>();
        let rhs = (0..size)
            .map(|row| {
                if (row % 2 == 1) == pivoted {
                    quantum
                } else {
                    0.0
                }
            })
            .collect::<Vec<_>>();
        let expected = (0..size)
            .map(|row| if row % 2 == 0 { quantum } else { -quantum })
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
            assert_eq!(
                control.last_checkpoint().unwrap().completed_steps,
                step as u64
            );
            assert_eq!(factor.solve(&rhs).unwrap(), expected);
        }
    }
}

#[test]
fn forward_underflow_still_validates_original_equations_before_replay() {
    let quantum = f64::from_bits(7);
    for unit in [1.0, 2.0_f64.powi(600)] {
        let mut factor = PreparedTridiagonal::factor(
            &[unit, 0.5 * unit, unit],
            &[0.5 * unit, 0.0],
            &[0.0, unit],
        )
        .unwrap();
        assert_eq!(factor.equilibration.is_some(), unit != 1.0);
        let rhs = [quantum * unit, 0.0, 0.0];
        factor.diagonal[1] = unit;
        assert!(factor.solve(&rhs).unwrap_err().contains("backward-error"));
        factor.diagonal[1] = 0.5 * unit;
        assert_eq!(factor.solve(&rhs).unwrap(), [quantum, -quantum, 0.0]);
    }
}

#[test]
fn forward_underflow_matches_normal_load_and_dense_references() {
    for size in [2, 3, 17, 65] {
        let mut diagonal = vec![0.5; size];
        diagonal[0] = 1.0;
        let lower = vec![0.5; size - 1];
        let factor = PreparedTridiagonal::factor(&diagonal, &lower, &vec![0.0; size - 1]).unwrap();
        let mut dense = vec![vec![0.0; size]; size];
        for (row, entries) in dense.iter_mut().enumerate() {
            entries[row] = diagonal[row];
            if row > 0 {
                entries[row - 1] = 0.5;
            }
        }
        let reference = crate::linear_dense::DenseLu::factor(dense).unwrap();
        for bits in [1, 3, 7, 15, 63, 511] {
            for sign in [-1.0, 1.0] {
                let mut rhs = vec![0.0; size];
                rhs[0] = sign;
                let normal = factor.solve(&rhs).unwrap();
                assert_eq!(normal, reference.solve(&rhs).unwrap());
                let load = f64::from_bits(bits);
                rhs[0] *= load;
                let expected = normal.iter().map(|value| value * load).collect::<Vec<_>>();
                assert_eq!(factor.solve(&rhs).unwrap(), expected);
            }
        }
    }
}
