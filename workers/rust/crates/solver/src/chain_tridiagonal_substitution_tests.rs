use super::PreparedTridiagonal;
use crate::solver_control::{SolverControl, SolverStage, with_solver_observer};
use std::cell::Cell;
use std::rc::Rc;

#[test]
fn chain_back_substitution_recovers_finite_response_after_numerator_overflow() {
    let high = 2.0_f64.powi(1023);
    for scale in [1.0, 2.0_f64.powi(-600)] {
        let factor =
            PreparedTridiagonal::factor(&[8.0 * scale; 2], &[-7.0 * scale], &[-7.0 * scale])
                .unwrap();
        assert!(factor.pivoted.is_none());
        assert_eq!(factor.equilibration.is_some(), scale != 1.0);
        for sign in [-1.0, 1.0] {
            assert_eq!(
                factor.solve(&[sign * high * scale; 2]).unwrap(),
                vec![sign * high; 2]
            );
        }
    }
}

#[test]
fn pivoted_chain_recovers_two_superdiagonal_overflow_before_division() {
    let high = 2.0_f64.powi(1023);
    let factor =
        PreparedTridiagonal::factor(&[0.0, -7.0, 8.0], &[8.0, -7.0], &[1.0, -7.0]).unwrap();
    assert!(factor.pivoted.is_some());
    for sign in [-1.0, 1.0] {
        assert_eq!(
            factor.solve(&[sign * high, 0.0, sign * high]).unwrap(),
            vec![sign * 1.75 * high, sign * high, sign * high]
        );
    }
}

#[test]
fn pivoted_chain_overflow_cancellation_retains_fused_product_tail() {
    let high = 2.0_f64.powi(1023);
    let delta = 2.0_f64.powi(-27);
    let factor =
        PreparedTridiagonal::factor(&[0.0, 2.0 * (1.0 + delta), 1.0], &[1.0, 0.0], &[1.0, -2.0])
            .unwrap();
    assert!(factor.pivoted.is_some());
    for sign in [-1.0, 1.0] {
        let rhs = [sign * high * (1.0 - delta), 0.0, sign * high];
        assert_eq!(
            factor.solve(&rhs).unwrap(),
            [sign * 2.0_f64.powi(970), rhs[0], rhs[2]]
        );
    }
}

#[test]
fn chain_range_recovery_cancels_before_publication_and_reuses_factors() {
    let high = 2.0_f64.powi(1023);
    for (factor, rhs, expected, steps) in [
        (
            PreparedTridiagonal::factor(&[8.0; 2], &[-7.0], &[-7.0]).unwrap(),
            vec![high; 2],
            vec![high; 2],
            [0, 1, 2],
        ),
        (
            PreparedTridiagonal::factor(&[0.0, -7.0, 8.0], &[8.0, -7.0], &[1.0, -7.0]).unwrap(),
            vec![high, 0.0, high],
            vec![1.75 * high, high, high],
            [0, 2, 4],
        ),
    ] {
        for step in steps {
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
            let point = control.last_checkpoint().unwrap();
            assert_eq!(point.stage, SolverStage::TridiagonalSubstitutionRange);
            assert_eq!(point.completed_steps, step);
            assert_eq!(factor.solve(&rhs).unwrap(), expected);
        }
    }
}

fn original_unpivoted(factor: &PreparedTridiagonal, rhs: &[f64]) -> Vec<f64> {
    let mut values = rhs.to_vec();
    for row in 1..values.len() {
        values[row] = (-factor.elimination_factors[row - 1]).mul_add(values[row - 1], values[row]);
    }
    let last = values.len() - 1;
    values[last] /= factor.factored_diagonal[last];
    for row in (0..last).rev() {
        values[row] = (-factor.upper[row]).mul_add(values[row + 1], values[row])
            / factor.factored_diagonal[row];
    }
    values
}

fn original_single_swap(rhs: [f64; 3]) -> Vec<f64> {
    // Retain zero-factor FMAs too: eliding them changes negative-zero bits.
    let forward = (-0.0_f64).mul_add(rhs[1], rhs[0]);
    let last = (-0.0_f64).mul_add(forward, rhs[2]);
    let middle = (-0.0_f64).mul_add(last, forward);
    let first = 2.0_f64.mul_add(last, (-2.25_f64).mul_add(middle, rhs[1]));
    vec![first, middle, last]
}

#[test]
fn ordinary_chain_substitution_preserves_fma_bits_and_skips_range_passes() {
    let ordinary =
        PreparedTridiagonal::factor(&[4.0, 5.0, 6.0], &[1.0, -0.75], &[1.0, 0.5]).unwrap();
    assert!(ordinary.pivoted.is_none());
    let pivoted =
        PreparedTridiagonal::factor(&[0.0, 2.25, 1.0], &[1.0, 0.0], &[1.0, -2.0]).unwrap();
    assert!(pivoted.pivoted.is_some());
    with_solver_observer(
        &SolverControl::default(),
        |point| {
            assert!(!matches!(
                point.stage,
                SolverStage::TridiagonalSubstitutionRange | SolverStage::TridiagonalForwardRange
            ));
        },
        || {
            for rhs in [[1.0, -0.5, 3.0], [-2.0, 4.0, 1.5], [0.0; 3], [-0.0; 3]] {
                for (factor, expected) in [
                    (&ordinary, original_unpivoted(&ordinary, &rhs)),
                    (&pivoted, original_single_swap(rhs)),
                ] {
                    let actual = factor.solve(&rhs)?;
                    assert_eq!(
                        actual
                            .iter()
                            .map(|value| value.to_bits())
                            .collect::<Vec<_>>(),
                        expected
                            .iter()
                            .map(|value| value.to_bits())
                            .collect::<Vec<_>>()
                    );
                }
            }
            Ok::<_, String>(())
        },
    )
    .unwrap();
}

#[test]
fn chain_range_recovery_does_not_bypass_original_equations_or_true_overflow() {
    let high = 2.0_f64.powi(1023);
    let mut factor = PreparedTridiagonal::factor(&[8.0; 2], &[-7.0], &[-7.0]).unwrap();
    factor.diagonal[0] = 16.0;
    assert!(
        factor
            .solve(&[high; 2])
            .unwrap_err()
            .contains("backward-error")
    );
    factor.diagonal[0] = 8.0;
    assert_eq!(factor.solve(&[high; 2]).unwrap(), [high; 2]);

    let factor = PreparedTridiagonal::factor(&[0.25], &[], &[]).unwrap();
    for rhs in [
        vec![],
        vec![1.0; 2],
        vec![f64::NAN],
        vec![f64::INFINITY],
        vec![f64::MAX],
        vec![-f64::MAX],
    ] {
        assert!(factor.solve(&rhs).is_err());
        assert_eq!(factor.solve(&[0.25]).unwrap(), [1.0]);
    }
}

#[test]
fn large_block_chain_range_recovery_stays_sparse_and_cancellable() {
    let size = 514;
    let high = 2.0_f64.powi(1023);
    let offdiagonal = (0..size - 1)
        .map(|index| if index % 2 == 0 { -7.0 } else { 0.0 })
        .collect::<Vec<_>>();
    let factor = PreparedTridiagonal::factor(&vec![8.0; size], &offdiagonal, &offdiagonal).unwrap();
    let rhs = vec![high; size];
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
    let solution = with_solver_observer(
        &SolverControl::default(),
        |point| assert_ne!(point.stage, SolverStage::DenseFactor),
        || factor.solve(&rhs),
    )
    .unwrap();
    assert_eq!(solution, rhs);
}
