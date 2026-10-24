use super::*;

fn sum(values: impl IntoIterator<Item = f64>) -> f64 {
    let mut value = Complex::default();
    let mut corrections = Corrections::default();
    for term in values {
        corrections.add(0, &mut value, Complex::real(term)).unwrap();
    }
    corrections.finish(std::iter::once(&mut value), 0).unwrap();
    value.re
}

#[test]
fn harmonic_assembly_corrections_keep_full_range_tails_and_exact_zero() {
    let quantum = f64::from_bits(1);
    for sign in [-1.0, 1.0] {
        for large in [4.0, 2.0_f64.powi(900), f64::MAX] {
            assert_eq!(
                sum([-sign * large, sign * quantum, sign * large]),
                sign * quantum
            );
            assert_eq!(
                sum([sign * large, sign * large, -sign * large]),
                sign * large
            );
        }
    }
    assert_eq!(sum([f64::MAX, quantum, -f64::MAX, -quantum]).to_bits(), 0);
    for sign in [-1.0, 1.0] {
        assert_eq!(
            sum([sign * f64::MAX, sign * 2.0_f64.powi(970), -sign * quantum]),
            sign * f64::MAX
        );
    }
    assert!(std::mem::size_of::<CoefficientSum>() <= 576);
}

#[test]
fn harmonic_assembly_corrections_round_midpoints_sticky_bits_and_carries_once() {
    let half = 2.0_f64.powi(-53);
    let quantum = f64::from_bits(1);
    for sign in [-1.0, 1.0] {
        for (parts, expected) in [
            ([1.0, half, 0.0], 1.0),
            ([1.0, half, quantum], 1.0_f64.next_up()),
            ([1.0, half, -quantum], 1.0),
            (
                [1.0, 3.0 * half, 0.0],
                f64::from_bits(1.0_f64.to_bits() + 2),
            ),
            ([2.0 - 2.0_f64.powi(-52), half, 0.0], 2.0),
            (
                [f64::MIN_POSITIVE, -quantum, 0.0],
                f64::MIN_POSITIVE.next_down(),
            ),
        ] {
            assert_eq!(
                sum(parts.map(|value| sign * value)).to_bits(),
                (sign * expected).to_bits()
            );
        }
    }
}

#[test]
fn harmonic_assembly_corrections_match_an_independent_signed_integer_lattice() {
    let mut seed = 0x78f7_d329_8132_1249_u64;
    for _ in 0..128 {
        let mut parts = vec![2.0_f64.powi(900)];
        let mut exact = 0_i128;
        for _ in 0..64 {
            seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
            let shift = ((seed >> 32) % 101) as u32;
            let coefficient = (seed % 7) as i128 - 3;
            exact += coefficient << shift;
            parts.push(coefficient as f64 * 2.0_f64.powi(shift as i32 - 100));
        }
        parts.push(-2.0_f64.powi(900));
        let expected = exact as f64 * 2.0_f64.powi(-100);
        assert_eq!(sum(parts.iter().copied()).to_bits(), expected.to_bits());
        assert_eq!(sum(parts.into_iter().rev()).to_bits(), expected.to_bits());
    }
}

#[test]
fn harmonic_assembly_corrections_group_components_and_skip_exact_addition_storage() {
    let mut cells = vec![Complex::real(1.0), Complex::real(1.0)];
    let mut corrections = Corrections::default();
    for index in (0..2).rev() {
        corrections
            .add(
                index,
                &mut cells[index],
                Complex {
                    re: 2.0_f64.powi(-53),
                    im: 1.0,
                },
            )
            .unwrap();
        corrections
            .add(
                index,
                &mut cells[index],
                Complex {
                    re: f64::from_bits(1),
                    im: 2.0_f64.powi(-53),
                },
            )
            .unwrap();
        corrections
            .add(
                index,
                &mut cells[index],
                Complex {
                    re: 0.0,
                    im: f64::from_bits(1),
                },
            )
            .unwrap();
    }
    corrections.finish(cells.iter_mut(), 0).unwrap();
    for value in cells {
        assert_eq!(value.re, 1.0_f64.next_up());
        assert_eq!(value.im, 1.0_f64.next_up());
    }
    let mut ordinary = Complex::default();
    let mut corrections = Corrections::default();
    for _ in 0..100_000 {
        corrections
            .add(0, &mut ordinary, Complex::real(1.0))
            .unwrap();
    }
    assert_eq!(corrections.tails.capacity(), 0);
    corrections
        .finish(std::iter::once(&mut ordinary), 0)
        .unwrap();
    assert_eq!(ordinary.re, 100_000.0);
}

#[test]
fn harmonic_assembly_corrections_reject_nonfinite_capacity_and_true_overflow() {
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let mut value = Complex::real(1.0);
        let mut corrections = Corrections::default();
        assert!(
            corrections
                .add(0, &mut value, Complex { re: 1.0, im: bad })
                .is_err()
        );
        assert!(corrections.add(0, &mut value, Complex::real(1.0)).is_err());
        assert!(corrections.finish(std::iter::once(&mut value), 0).is_err());
        let mut sum = CoefficientSum::default();
        assert!(sum.add(bad).is_err());
        assert!(sum.finish().is_err());
    }
    let mut value = Complex::real(f64::MAX);
    let mut corrections = Corrections::default();
    corrections
        .add(0, &mut value, Complex::real(f64::MAX))
        .unwrap();
    assert!(
        corrections
            .finish(std::iter::once(&mut value), 0)
            .unwrap_err()
            .contains("range")
    );
    let mut corrections = Corrections::default();
    value = Complex::real(1.0);
    corrections
        .add(2, &mut value, Complex::real(2.0_f64.powi(-53)))
        .unwrap();
    assert!(
        corrections
            .finish(std::iter::once(&mut value), 0)
            .unwrap_err()
            .contains("outside the matrix")
    );
    assert_eq!(sum([f64::MAX, -f64::MAX, 1.0]), 1.0);
}

#[test]
fn harmonic_assembly_corrections_cancel_before_after_grouping_and_during_replay_then_reuse() {
    use crate::solver_control::{SolverControl, with_solver_observer};
    for completed in [41, 42, 64, 1025, 43] {
        let control = SolverControl::default();
        let cancel = control.clone();
        let error = with_solver_observer(
            &control,
            move |p| {
                if p.stage == SolverStage::ElementAssembly && p.completed_steps == completed {
                    cancel.request_cancel();
                }
            },
            || {
                let mut value = Complex::real(1.0);
                let mut corrections = Corrections::default();
                for _ in 0..1025 {
                    corrections.add(0, &mut value, Complex::real(f64::from_bits(1)))?;
                }
                corrections.finish(std::iter::once(&mut value), 40)
            },
        )
        .unwrap_err();
        assert!(
            control.was_interrupted() && error.contains("cancelled"),
            "{error}"
        );
        assert_eq!(
            sum([1.0, 2.0_f64.powi(-53), f64::from_bits(1)]),
            1.0_f64.next_up()
        );
    }
}

#[test]
fn harmonic_inertia_precision_scaled_collection_preserves_sticky_bits_below_float_quantum() {
    let mut value = Complex::real(1.0);
    let mut corrections = Corrections::default();
    corrections
        .add_scaled_real(
            0,
            &mut value,
            [2.0_f64.powi(947), 2.0_f64.powi(-100), 0.0, 0.0],
            -1000,
        )
        .unwrap();
    assert!(!corrections.scaled.is_empty());
    corrections.finish(std::iter::once(&mut value), 0).unwrap();
    assert_eq!(value.re, 1.0_f64.next_up());
    let mut value = Complex::default();
    let mut corrections = Corrections::default();
    corrections
        .add_scaled_real(0, &mut value, [0.75, 0.0, 0.0, 0.0], -1074)
        .unwrap();
    assert!(
        corrections
            .finish(std::iter::once(&mut value), 0)
            .unwrap_err()
            .contains("lose numerical range")
    );
    let mut corrections = Corrections::default();
    assert!(
        corrections
            .add_scaled_real(0, &mut value, [1.0; 4], i32::MIN)
            .is_err()
    );
    assert!(corrections.finish(std::iter::once(&mut value), 0).is_err());
}

#[test]
fn harmonic_inertia_precision_scaled_replay_cancels_without_a_successful_partial_sum() {
    use crate::solver_control::{SolverControl, with_solver_observer};
    let control = SolverControl::default();
    let cancel = control.clone();
    let error = with_solver_observer(
        &control,
        move |p| {
            if p.stage == SolverStage::ElementAssembly && p.completed_steps == 64 {
                cancel.request_cancel();
            }
        },
        || {
            let mut value = Complex::real(1.0);
            let mut corrections = Corrections::default();
            for _ in 0..128 {
                corrections.add_scaled_real(0, &mut value, [1.0, 0.0, 0.0, 0.0], -1100)?;
            }
            corrections.finish(std::iter::once(&mut value), 0)
        },
    )
    .unwrap_err();
    assert!(control.was_interrupted() && error.contains("cancelled"));
    let mut value = Complex::real(1.0);
    let mut corrections = Corrections::default();
    corrections
        .add_scaled_real(0, &mut value, [1.0, 0.0, 0.0, 0.0], -1100)
        .unwrap();
    corrections.finish(std::iter::once(&mut value), 0).unwrap();
    assert_eq!(value.re, 1.0);
}
