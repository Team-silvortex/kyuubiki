use super::PreparedTridiagonal;
use crate::linear_binary_scale::restore;
use crate::solver_control::{SolverControl, SolverStage, with_solver_observer};
use std::cell::Cell;
use std::rc::Rc;

#[test]
fn back_underflow_recovers_erased_and_rounded_numerators() {
    let factor = PreparedTridiagonal::factor(&[0.5, 1.0], &[0.0], &[0.5]).unwrap();
    assert!(factor.pivoted.is_none());
    for bits in [1, 3, 5, 7] {
        for sign in [-1.0, 1.0] {
            let load = sign * f64::from_bits(bits);
            assert_eq!(factor.solve(&[0.0, load]).unwrap(), [-load, load]);
        }
    }
}

#[test]
fn back_underflow_recovers_both_pivoted_superdiagonal_products() {
    let quantum = f64::from_bits(1);
    for unit in [1.0, 2.0_f64.powi(600)] {
        let factor =
            PreparedTridiagonal::factor(&[0.0, 0.25 * unit, unit], &[unit, 0.0], &[0.5 * unit; 2])
                .unwrap();
        assert!(factor.pivoted.is_some());
        assert_eq!(factor.equilibration.is_some(), unit != 1.0);
        for sign in [-1.0, 1.0] {
            let load = sign * quantum;
            assert_eq!(
                factor.solve(&[load * unit, 0.0, load * unit]).unwrap(),
                [-load, 2.0 * load, load]
            );
        }
    }
}

#[test]
fn back_underflow_keeps_early_loss_when_the_last_update_is_normal() {
    let quantum = f64::from_bits(1);
    let factor = PreparedTridiagonal::factor(&[0.0, 0.25, 1.0], &[0.5, 0.0], &[0.5, -1.0]).unwrap();
    assert!(factor.pivoted.is_some());
    for sign in [-1.0, 1.0] {
        assert_eq!(
            factor
                .solve(&[sign * quantum, 0.0, sign * f64::MIN_POSITIVE])
                .unwrap(),
            [
                sign * (2.0 * f64::MIN_POSITIVE - quantum),
                sign * 2.0 * quantum,
                sign * f64::MIN_POSITIVE
            ]
        );
    }
}

#[test]
fn back_underflow_retains_fused_cancellation_and_minimum_normal_boundary() {
    let quantum = f64::from_bits(1);
    let delta = 2.0_f64.powi(-27);
    for (pivot, upper, rhs, expected) in [
        (
            0.25,
            1.0 - delta,
            [f64::MIN_POSITIVE, (1.0 + delta) * f64::MIN_POSITIVE],
            quantum,
        ),
        (
            0.5,
            1.0 - 0.5 * f64::EPSILON,
            [0.0, f64::MIN_POSITIVE],
            -2.0 * f64::MIN_POSITIVE + quantum,
        ),
    ] {
        let factor = PreparedTridiagonal::factor(&[pivot, 1.0], &[0.0], &[upper]).unwrap();
        for sign in [-1.0, 1.0] {
            assert_eq!(
                factor.solve(&rhs.map(|x| x * sign)).unwrap(),
                [sign * expected, sign * rhs[1]]
            );
        }
    }
}

#[test]
fn back_underflow_preserves_exact_fma_bits_without_range_entry() {
    let quantum = f64::from_bits(1);
    for coefficient in [0.5, quantum] {
        let factor = PreparedTridiagonal::factor(&[1.0; 2], &[0.0], &[coefficient]).unwrap();
        let exact_value = if coefficient == quantum {
            2.0
        } else {
            2.0 * quantum
        };
        let tail_value = if coefficient == quantum {
            quantum
        } else {
            2.0 * quantum
        };
        with_solver_observer(
            &SolverControl::default(),
            |point| {
                assert!(!matches!(
                    point.stage,
                    SolverStage::TridiagonalSubstitutionRange
                        | SolverStage::TridiagonalForwardRange
                ))
            },
            || {
                for rhs in [
                    [0.0; 2],
                    [-0.0; 2],
                    [1.0, 2.0],
                    [0.0, exact_value],
                    [f64::from_bits(f64::MIN_POSITIVE.to_bits() - 1), tail_value],
                ] {
                    // Keep the forward zero-factor FMA: it affects signed-zero bits.
                    let last = (-0.0_f64).mul_add(rhs[0], rhs[1]);
                    let expected = (-coefficient).mul_add(last, rhs[0]);
                    let result = factor.solve(&rhs)?;
                    assert_eq!(result[0].to_bits(), expected.to_bits());
                }
                Ok::<_, String>(())
            },
        )
        .unwrap();
    }
}

#[test]
fn back_underflow_keeps_final_quantized_tails_and_rejects_erased_loads() {
    let size = 2048;
    let factor =
        PreparedTridiagonal::factor(&vec![1.0; size], &vec![0.0; size - 1], &vec![0.5; size - 1])
            .unwrap();
    for sign in [-1.0, 1.0] {
        let mut rhs = vec![0.0; size];
        rhs[size - 1] = sign;
        for (distance, &value) in factor.solve(&rhs).unwrap().iter().rev().enumerate() {
            assert_eq!(
                value,
                restore(
                    if distance % 2 == 0 { sign } else { -sign },
                    -(distance as i32)
                ),
                "distance {distance}"
            );
        }
    }
    let quantum = f64::from_bits(1);
    let factor = PreparedTridiagonal::factor(&[0.5, 1.0, 4.0], &[0.0; 2], &[0.5, 0.0]).unwrap();
    assert!(
        factor
            .solve(&[0.0, quantum, quantum])
            .unwrap_err()
            .contains("backward-error")
    );
    assert_eq!(
        factor.solve(&[0.0, quantum, 4.0 * quantum]).unwrap(),
        [-quantum, quantum, quantum]
    );
}

#[test]
fn back_underflow_is_checked_inside_forward_overflow_replay() {
    let high = 2.0_f64.powi(1023);
    let quantum = f64::from_bits(1);
    let factor =
        PreparedTridiagonal::factor(&[8.0, 16.0, 0.5, 1.0], &[-8.0, 0.0, 0.0], &[-8.0, 0.0, 0.5])
            .unwrap();
    for sign in [-1.0, 1.0] {
        assert_eq!(
            factor
                .solve(&[sign * high, sign * high, 0.0, sign * quantum])
                .unwrap(),
            [
                sign * 0.375 * high,
                sign * 0.25 * high,
                -sign * quantum,
                sign * quantum
            ]
        );
    }
}

#[test]
fn back_underflow_cancels_row_recovery_before_publication_then_replays() {
    let quantum = f64::from_bits(1);
    for (factor, rhs, expected, end) in [
        (
            PreparedTridiagonal::factor(&[0.5, 1.0], &[0.0], &[0.5]).unwrap(),
            vec![0.0, quantum],
            vec![-quantum, quantum],
            2,
        ),
        (
            PreparedTridiagonal::factor(&[0.0, 0.25, 1.0], &[1.0, 0.0], &[0.5; 2]).unwrap(),
            vec![quantum, 0.0, quantum],
            vec![-quantum, 2.0 * quantum, quantum],
            4,
        ),
    ] {
        for step in [0, end / 2, end] {
            let control = SolverControl::default();
            let cancel = control.clone();
            let result = with_solver_observer(
                &control,
                move |point| {
                    assert_ne!(point.stage, SolverStage::DenseFactor);
                    if point.stage == SolverStage::TridiagonalSubstitutionRange
                        && point.completed_steps == step
                    {
                        cancel.request_cancel();
                    }
                },
                || factor.solve(&rhs),
            );
            assert!(
                result
                    .unwrap_err()
                    .contains("tridiagonal_substitution_range")
            );
            assert!(control.was_interrupted());
            assert_eq!(control.last_checkpoint().unwrap().completed_steps, step);
            assert_eq!(factor.solve(&rhs).unwrap(), expected);
        }
    }
}

#[test]
fn back_underflow_late_block_cancellation_keeps_factors_immutable() {
    let size = 514;
    let quantum = f64::from_bits(3);
    let diagonal = (0..size)
        .map(|row| if row % 2 == 0 { 0.5 } else { 1.0 })
        .collect::<Vec<_>>();
    let upper = (0..size - 1)
        .map(|row| if row % 2 == 0 { 0.5 } else { 0.0 })
        .collect::<Vec<_>>();
    let factor = PreparedTridiagonal::factor(&diagonal, &vec![0.0; size - 1], &upper).unwrap();
    let rhs = (0..size)
        .map(|row| if row % 2 == 0 { 0.0 } else { quantum })
        .collect::<Vec<_>>();
    let entries = Rc::new(Cell::new(0));
    let observed = entries.clone();
    let control = SolverControl::default();
    let cancel = control.clone();
    let result = with_solver_observer(
        &control,
        move |point| {
            assert_ne!(point.stage, SolverStage::DenseFactor);
            if point.stage == SolverStage::TridiagonalSubstitutionRange
                && point.completed_steps == 0
            {
                observed.set(observed.get() + 1);
                if observed.get() == size / 2 {
                    cancel.request_cancel();
                }
            }
        },
        || factor.solve(&rhs),
    );
    assert!(result.unwrap_err().contains("cancelled"));
    assert_eq!(entries.get(), size / 2);
    assert_eq!(
        factor.solve(&rhs).unwrap(),
        (0..size)
            .map(|row| if row % 2 == 0 { -quantum } else { quantum })
            .collect::<Vec<_>>()
    );
}

#[test]
fn back_underflow_requires_original_equations_and_rejects_true_overflow() {
    let quantum = f64::from_bits(7);
    let mut factor = PreparedTridiagonal::factor(&[0.5, 1.0], &[0.0], &[0.5]).unwrap();
    factor.diagonal[0] = 1.0;
    assert!(
        factor
            .solve(&[0.0, quantum])
            .unwrap_err()
            .contains("backward-error")
    );
    factor.diagonal[0] = 0.5;
    assert_eq!(factor.solve(&[0.0, quantum]).unwrap(), [-quantum, quantum]);
    let factor = PreparedTridiagonal::factor(&[0.25, 1.0], &[0.0], &[1.0]).unwrap();
    assert!(factor.solve(&[0.0, f64::MAX]).is_err());
    assert_eq!(
        factor.solve(&[0.0, quantum]).unwrap(),
        [-4.0 * quantum, quantum]
    );
}

#[test]
fn back_underflow_matches_normal_load_and_dense_reference() {
    for size in [2, 3, 17, 65] {
        let mut diagonal = vec![0.5; size];
        diagonal[size - 1] = 1.0;
        let factor =
            PreparedTridiagonal::factor(&diagonal, &vec![0.0; size - 1], &vec![0.5; size - 1])
                .unwrap();
        let mut dense = vec![vec![0.0; size]; size];
        for (row, entries) in dense.iter_mut().enumerate() {
            entries[row] = diagonal[row];
            if row + 1 < size {
                entries[row + 1] = 0.5;
            }
        }
        let reference = crate::linear_dense::DenseLu::factor(dense).unwrap();
        for sign in [-1.0, 1.0] {
            let mut rhs = vec![0.0; size];
            rhs[size - 1] = sign;
            let normal = factor.solve(&rhs).unwrap();
            assert_eq!(normal, reference.solve(&rhs).unwrap());
            for bits in [1, 3, 7, 63, 511] {
                let quantum = f64::from_bits(bits);
                rhs[size - 1] = sign * quantum;
                assert_eq!(
                    factor.solve(&rhs).unwrap(),
                    normal.iter().map(|x| x * quantum).collect::<Vec<_>>()
                );
            }
        }
    }
}

#[test]
fn back_underflow_rejects_irreversible_recovery_alignment_then_replays() {
    let quantum = f64::from_bits(1);
    let factor = PreparedTridiagonal::factor(&[0.0, 0.25, 1.0], &[0.5, 0.0], &[0.5, -1.0]).unwrap();
    assert!(
        factor
            .solve(&[quantum, 0.0, 1.0])
            .unwrap_err()
            .contains("normalization would lose numerical range")
    );
    assert_eq!(
        factor.solve(&[quantum, 0.0, f64::MIN_POSITIVE]).unwrap(),
        [
            2.0 * f64::MIN_POSITIVE - quantum,
            2.0 * quantum,
            f64::MIN_POSITIVE
        ]
    );
}
