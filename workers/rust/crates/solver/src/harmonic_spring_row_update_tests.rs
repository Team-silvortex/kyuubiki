use super::*;

#[test]
fn harmonic_row_update_weak_numerator_retains_product_tails_before_final_division() {
    let delta = 2.0_f64.powi(-27);
    let weak = 2.0_f64.powi(-20);
    for power in [-500, 0, 500] {
        let scale = 2.0_f64.powi(power);
        for sign in [-1.0, 1.0] {
            for imaginary in [false, true] {
                let phase = |value| {
                    if imaginary {
                        Complex { re: 0.0, im: value }
                    } else {
                        Complex::real(value)
                    }
                };
                let result = solve_row(
                    phase(sign * scale * (1.0 + weak)),
                    std::iter::once((
                        Complex::real(scale * (1.0 + delta)),
                        phase(sign * (1.0 - delta)),
                    )),
                    Complex::real(3.0 * scale),
                    SolverStage::DenseSubstitutionRange,
                )
                .unwrap();
                let expected = sign * (weak + delta * delta) / 3.0;
                let value = if imaginary { result.im } else { result.re };
                assert!(
                    (value / expected - 1.0).abs() <= 4.0 * f64::EPSILON,
                    "{result:?} != {expected}"
                );
                assert_eq!(if imaginary { result.re } else { result.im }, 0.0);
            }
        }
    }
}

#[test]
fn harmonic_row_update_rhs_subtraction_keeps_a_weak_finite_component() {
    let delta = 2.0_f64.powi(-27);
    let weak = 2.0_f64.powi(-20);
    let result = WideComplex::from(Complex::real(1.0 + weak))
        .subtract_product(
            Complex::real(1.0 + delta),
            Complex::real(1.0 - delta).into(),
            SolverStage::TridiagonalFactor,
        )
        .unwrap()
        .finite()
        .unwrap();
    assert_eq!(result.re.to_bits(), (weak + delta * delta).to_bits());
}

#[test]
fn harmonic_row_update_rounding_budget_replays_complex_partial_cancellation() {
    let delta = 2.0_f64.powi(-27);
    let weak = 2.0_f64.powi(-20);
    let rhs = Complex {
        re: 1.0 + weak,
        im: 1.0,
    };
    let term = (
        Complex {
            re: delta,
            im: -1.0,
        },
        Complex { re: delta, im: 1.0 },
    );
    assert!(
        row_update::numerator(rhs, std::iter::once(term))
            .unwrap()
            .is_none()
    );
    let result = solve_row(
        rhs,
        std::iter::once(term),
        Complex::real(1.0),
        SolverStage::DenseSubstitutionRange,
    )
    .unwrap();
    assert_eq!(result.re.to_bits(), (weak - delta * delta).to_bits());
    assert_eq!(result.im, 1.0);
}

#[test]
fn harmonic_row_update_rounding_budget_catches_earlier_loss_and_preserves_order_controls() {
    let delta = 2.0_f64.powi(-27);
    let weak = 2.0_f64.powi(-20);
    let small = (Complex::real(delta), Complex::real(delta));
    let large = (Complex::real(1.0), Complex::real(1.0));
    for (terms, should_replay) in [([small, large], true), ([large, small], false)] {
        let rhs = Complex::real(1.0 + weak);
        assert_eq!(
            row_update::numerator(rhs, terms.into_iter())
                .unwrap()
                .is_none(),
            should_replay
        );
        let result = solve_row(
            rhs,
            terms.into_iter(),
            Complex::real(1.0),
            SolverStage::DenseSubstitutionRange,
        )
        .unwrap();
        assert_eq!(result.re.to_bits(), (weak - delta * delta).to_bits());
    }
}

#[test]
fn harmonic_row_update_replay_and_long_ordinary_scan_cancel_then_reuse() {
    use crate::solver_control::{SolverControl, with_solver_observer};
    let delta = 2.0_f64.powi(-27);
    let weak = 2.0_f64.powi(-20);
    let mut terms = vec![
        (Complex::real(delta), Complex::real(delta)),
        (Complex::real(1.0), Complex::real(1.0)),
    ];
    terms.resize(2050, (Complex::default(), Complex::default()));
    let run = || {
        solve_row(
            Complex::real(1.0 + weak),
            terms.iter().copied(),
            Complex::real(1.0),
            SolverStage::DenseSubstitutionRange,
        )
    };
    let control = SolverControl::default();
    let cancel = control.clone();
    let error = with_solver_observer(
        &control,
        move |p| {
            if p.stage == SolverStage::DenseSubstitutionRange && p.completed_steps == 1024 {
                cancel.request_cancel();
            }
        },
        run,
    )
    .unwrap_err();
    assert!(control.was_interrupted() && error.contains("cancelled"));
    assert_eq!(
        run().unwrap().re.to_bits(),
        (weak - delta * delta).to_bits()
    );
    let control = SolverControl::default();
    control.request_cancel();
    let error = with_solver_observer(
        &control,
        |_| {},
        || {
            row_update::numerator(
                Complex::real(1.0),
                std::iter::repeat_n((Complex::default(), Complex::default()), 2050),
            )
        },
    )
    .unwrap_err();
    assert!(control.was_interrupted() && error.contains("cancelled"));
    assert_eq!(
        row_update::numerator(Complex::real(1.0), std::iter::empty())
            .unwrap()
            .unwrap()
            .re,
        1.0
    );
}

#[test]
fn harmonic_row_update_both_factorizations_preserve_the_signed_weak_response() {
    use super::super::{ComplexTridiagonalSystem, solve_complex_system, solve_complex_tridiagonal};
    let delta = 2.0_f64.powi(-27);
    let weak = 2.0_f64.powi(-20);
    for sign in [-1.0, 1.0] {
        let rhs = vec![
            Complex::real(sign * (1.0 + weak)),
            Complex::real(sign * (1.0 - delta)),
        ];
        for result in [
            solve_complex_system(
                vec![
                    vec![Complex::real(3.0), Complex::real(1.0 + delta)],
                    vec![Complex::default(), Complex::real(1.0)],
                ],
                rhs.clone(),
            ),
            solve_complex_tridiagonal(ComplexTridiagonalSystem {
                diagonal: vec![Complex::real(3.0), Complex::real(1.0)],
                lower: vec![Complex::default()],
                upper: vec![Complex::real(1.0 + delta)],
                rhs,
            }),
        ] {
            let result = result.unwrap();
            let expected = sign * (weak + delta * delta) / 3.0;
            assert!((result[0].re / expected - 1.0).abs() <= 4.0 * f64::EPSILON);
            assert_eq!(result[1].re, sign * (1.0 - delta));
        }
    }
}

#[test]
fn harmonic_row_update_invalid_later_operands_and_range_failures_cannot_be_hidden() {
    let d = 2.0_f64.powi(-27);
    let rhs = Complex::real(1.0 + 2.0_f64.powi(-20));
    let terms = [
        (Complex::real(d), Complex::real(d)),
        (Complex::real(1.0), Complex::real(1.0)),
        (Complex::real(f64::NAN), Complex::default()),
    ];
    assert!(
        row_update::numerator(rhs, terms.into_iter())
            .unwrap_err()
            .contains("non-finite")
    );
    assert!(
        solve_row(
            Complex::real(f64::MAX),
            std::iter::once((Complex::real(-1.0), Complex::real(f64::MAX))),
            Complex::real(1.0),
            SolverStage::DenseSubstitutionRange
        )
        .is_err()
    );
    let recovered = solve_row(
        rhs,
        std::iter::once((Complex::real(1.0 + d), Complex::real(1.0 - d))),
        Complex::real(1.0),
        SolverStage::DenseSubstitutionRange,
    )
    .unwrap();
    assert_eq!(
        recovered.re.to_bits(),
        (2.0_f64.powi(-20) + d * d).to_bits()
    );
}
