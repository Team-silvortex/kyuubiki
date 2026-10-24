use super::*;

#[test]
fn harmonic_factor_compensation_updates_coefficients_and_rhs_with_one_shared_tail() {
    let w = 2.0_f64.powi(-20);
    let base = Complex::real(1.0 / 3.0 + w);
    let expected = (3.0 * w - 2.0_f64.powi(-54)) / 3.0;
    for (pivot, other) in [
        (Complex::real(3.0), Complex::real(1.0)),
        (Complex { re: 0.0, im: 3.0 }, Complex { re: 0.0, im: 1.0 }),
        (Complex { re: 3.0, im: 3.0 }, Complex { re: 1.0, im: 1.0 }),
    ] {
        for sign in [-1.0, 1.0] {
            let factor =
                EliminationFactor::new(Complex::real(sign), pivot, SolverStage::DenseFactor)
                    .unwrap();
            assert!(factor.correction.component_scale() > 0.0);
            let other = Complex {
                re: sign * other.re,
                im: sign * other.im,
            };
            let matrix = factor
                .update_matrix(base, other, SolverStage::DenseFactor)
                .unwrap();
            let rhs = factor
                .update_rhs(base.into(), other.into(), SolverStage::DenseFactor)
                .unwrap()
                .finite()
                .unwrap();
            for result in [matrix, rhs] {
                assert!(
                    (result.re / expected - 1.0).abs() <= 4.0 * f64::EPSILON,
                    "{result:?}"
                );
                assert!(result.im.abs() <= 4.0 * f64::EPSILON * expected);
            }
        }
    }
    let factor = EliminationFactor::new(
        Complex::real(1.0),
        Complex::real(3.0),
        SolverStage::DenseFactor,
    )
    .unwrap();
    assert_eq!(factor.value.re.to_bits(), (1.0_f64 / 3.0).to_bits());
    assert_eq!(
        factor.correction.re.to_bits(),
        (2.0_f64.powi(-54) / 3.0).to_bits()
    );
    assert!(std::mem::size_of::<EliminationFactor>() <= 32);
}

#[test]
fn harmonic_factor_compensation_bounded_eligibility_preserves_range_and_operand_guards() {
    use super::super::Scaled;
    let stage = SolverStage::DenseFactor;
    for value in [
        0.0,
        -0.0,
        MIN_COMPONENT,
        -MIN_COMPONENT,
        MAX_COMPONENT,
        -MAX_COMPONENT,
    ] {
        assert!(moderate(Complex::real(value)));
    }
    for value in [
        MIN_COMPONENT.next_down(),
        MAX_COMPONENT.next_up(),
        f64::NAN,
        f64::INFINITY,
    ] {
        assert!(!moderate(Complex::real(value)));
    }
    let factor = EliminationFactor::new(Complex::real(1.0), Complex::real(3.0), stage).unwrap();
    for mode in 0..3 {
        let mut gated =
            EliminationFactor::new(Complex::real(1.0), Complex::real(3.0), stage).unwrap();
        let update = if mode != 0 {
            (Complex::real(1.0), Complex::real(1.0))
        } else {
            (Complex::real(1e100), Complex::real(1.0))
        };
        let rhs = if mode == 1 {
            WideComplex {
                real: Scaled {
                    value: 1.0,
                    exponent: 1500,
                },
                imaginary: Scaled::from(0.0),
            }
        } else if mode == 2 {
            Complex::real(1e100).into()
        } else {
            Complex::real(1.0).into()
        };
        gated
            .restrict(
                [(Complex::real(1.0), Complex::real(1.0)), update].into_iter(),
                rhs,
                Complex::real(1.0).into(),
            )
            .unwrap();
        assert_eq!(gated.correction.component_scale(), 0.0);
        let base = Complex::real(1.0 / 3.0 + 2.0_f64.powi(-20));
        let matrix = gated
            .update_matrix(base, Complex::real(1.0), stage)
            .unwrap();
        let rhs = gated
            .update_rhs(base.into(), Complex::real(1.0).into(), stage)
            .unwrap()
            .finite()
            .unwrap();
        assert_eq!(matrix.re.to_bits(), 2.0_f64.powi(-20).to_bits());
        assert_eq!(rhs.re.to_bits(), matrix.re.to_bits());
    }
    let outside =
        EliminationFactor::new(Complex::real(2.0_f64.powi(-101)), Complex::real(3.0), stage)
            .unwrap();
    assert_eq!(outside.correction.component_scale(), 0.0);
    let exact = EliminationFactor::new(Complex::real(1.0), Complex::real(2.0), stage).unwrap();
    assert_eq!(exact.correction.component_scale(), 0.0);
    let base = Complex::real(1e100);
    let reference = matrix_difference(base, factor.value, Complex::real(1.0), stage).unwrap();
    assert_eq!(
        factor
            .update_matrix(base, Complex::real(1.0), stage)
            .unwrap()
            .re
            .to_bits(),
        reference.re.to_bits()
    );
    let wide = |exponent| WideComplex {
        real: Scaled {
            value: 1.0,
            exponent,
        },
        imaginary: Scaled::from(0.0),
    };
    let base = wide(1500);
    let other = wide(1490);
    let divisor = Complex::real(2.0_f64.powi(500));
    let reference = base
        .subtract_product(factor.value, other, stage)
        .unwrap()
        .divide(divisor, stage)
        .unwrap();
    let result = factor
        .update_rhs(base, other, stage)
        .unwrap()
        .divide(divisor, stage)
        .unwrap();
    assert_eq!(
        [result.re.to_bits(), result.im.to_bits()],
        [reference.re.to_bits(), reference.im.to_bits()]
    );
    let q = f64::from_bits(1);
    assert_eq!(
        exact
            .update_matrix(Complex::real(3.0 * q), Complex::real(2.0 * q), stage)
            .unwrap()
            .re
            .to_bits(),
        2
    );
    assert!(
        exact
            .update_matrix(Complex::real(3.0 * q), Complex::real(q), stage)
            .is_err()
    );
    assert!(EliminationFactor::new(Complex::real(q), Complex::real(f64::MAX), stage).is_err());
    for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(EliminationFactor::new(Complex::real(invalid), Complex::real(1.0), stage).is_err());
        assert!(
            factor
                .update_matrix(Complex::real(1.0), Complex::real(invalid), stage)
                .is_err()
        );
        assert!(
            factor
                .update_rhs(
                    Complex::real(1.0).into(),
                    Complex::real(invalid).into(),
                    stage
                )
                .is_err()
        );
    }
    assert!(EliminationFactor::new(Complex::real(1.0), Complex::default(), stage).is_err());
    let negative = EliminationFactor::new(Complex::real(-1.0), Complex::real(1.0), stage).unwrap();
    assert!(
        negative
            .update_matrix(Complex::real(f64::MAX), Complex::real(f64::MAX), stage)
            .is_err()
    );
}

#[test]
fn harmonic_factor_compensation_cancelled_residual_prevents_factor_publication_then_replays() {
    use crate::solver_control::{SolverControl, with_solver_observer};
    use std::cell::Cell;
    let pivot = Complex { re: 3.0, im: 3.0 };
    assert!(super::super::fast_division::divide(Complex::real(1.0), pivot).is_some());
    for occurrence in [1, 2] {
        let control = SolverControl::default();
        let cancel = control.clone();
        let seen = Cell::new(0);
        let error = with_solver_observer(
            &control,
            move |p| {
                if p.stage == SolverStage::DenseFactor && p.completed_steps == 0 {
                    seen.set(seen.get() + 1);
                    if seen.get() == occurrence {
                        cancel.request_cancel();
                    }
                }
            },
            || EliminationFactor::new(Complex::real(1.0), pivot, SolverStage::DenseFactor),
        )
        .err()
        .expect("cancelled factor cannot be published");
        assert!(
            control.was_interrupted() && error.contains("cancelled"),
            "{error}"
        );
        let replay =
            EliminationFactor::new(Complex::real(1.0), pivot, SolverStage::DenseFactor).unwrap();
        assert!(replay.correction.component_scale() > 0.0);
    }
    let mut factor = EliminationFactor::new(
        Complex::real(1.0),
        Complex::real(3.0),
        SolverStage::DenseFactor,
    )
    .unwrap();
    let control = SolverControl::default();
    control.request_cancel();
    let error = with_solver_observer(
        &control,
        |_| {},
        || {
            factor.restrict(
                std::iter::repeat_n((Complex::real(1.0), Complex::real(1.0)), 2050),
                Complex::real(1.0).into(),
                Complex::real(1.0).into(),
            )
        },
    )
    .unwrap_err();
    assert!(control.was_interrupted() && error.contains("cancelled"));
    let mut fresh = EliminationFactor::new(
        Complex::real(1.0),
        Complex::real(3.0),
        SolverStage::DenseFactor,
    )
    .unwrap();
    fresh
        .restrict(
            std::iter::once((Complex::real(1.0), Complex::real(1.0))),
            Complex::real(1.0).into(),
            Complex::real(1.0).into(),
        )
        .unwrap();
    assert!(fresh.correction.component_scale() > 0.0);
}

#[test]
fn harmonic_factor_compensation_cancelled_two_term_replay_discards_partial_updates_then_reuses() {
    use crate::solver_control::{SolverControl, with_solver_observer};
    use std::cell::Cell;
    let factor = EliminationFactor::new(
        Complex::real(1.0),
        Complex { re: 3.0, im: 3.0 },
        SolverStage::DenseFactor,
    )
    .unwrap();
    let w = 2.0_f64.powi(-20);
    let base = Complex {
        re: 1.0 / 3.0 + w,
        im: 1.0,
    };
    let other = Complex { re: 1.0, im: 1.0 };
    let run = |rhs| {
        if rhs {
            factor
                .update_rhs(base.into(), other.into(), SolverStage::DenseFactor)
                .map(|value| value.finite().unwrap())
        } else {
            factor.update_matrix(base, other, SolverStage::DenseFactor)
        }
    };
    for rhs in [false, true] {
        for (completed, occurrence) in [(0, 1), (0, 2), (5, 1), (10, 1)] {
            let control = SolverControl::default();
            let cancel = control.clone();
            let seen = Cell::new(0);
            let error = with_solver_observer(
                &control,
                move |p| {
                    if p.stage == SolverStage::DenseFactor && p.completed_steps == completed {
                        seen.set(seen.get() + 1);
                        if seen.get() == occurrence {
                            cancel.request_cancel();
                        }
                    }
                },
                || run(rhs),
            )
            .unwrap_err();
            assert!(
                control.was_interrupted() && error.contains("cancelled"),
                "{error}"
            );
            let recovered = run(rhs).unwrap();
            let expected = (3.0 * w - 2.0_f64.powi(-54)) / 3.0;
            assert!((recovered.re / expected - 1.0).abs() <= 4.0 * f64::EPSILON);
            assert_eq!(recovered.im, 1.0);
        }
    }
}
