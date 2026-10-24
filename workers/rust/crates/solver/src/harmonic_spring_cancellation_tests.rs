use super::*;

#[test]
fn harmonic_complex_cancellation_product_recovers_real_and_imaginary_tails() {
    for exponent in [-500, 0, 500] {
        let scale = 2.0_f64.powi(exponent);
        for delta in [2.0_f64.powi(-27), 2.0_f64.powi(-30)] {
            for sign in [-1.0, 1.0] {
                let right = Complex {
                    re: scale * (1.0 - delta),
                    im: scale,
                };
                for (left, real, imaginary) in [
                    (
                        Complex {
                            re: sign * (1.0 + delta),
                            im: sign,
                        },
                        -sign * scale * delta * delta,
                        sign * scale * 2.0,
                    ),
                    (
                        Complex {
                            re: -sign,
                            im: sign * (1.0 + delta),
                        },
                        -sign * scale * 2.0,
                        -sign * scale * delta * delta,
                    ),
                ] {
                    let value =
                        arithmetic::product(left, right, SolverStage::ResultElements).unwrap();
                    assert_eq!(value.re, real, "{left:?} * {right:?}");
                    assert_eq!(value.im, imaginary, "{left:?} * {right:?}");
                }
            }
        }
    }
}

#[test]
fn harmonic_complex_cancellation_updates_recover_normal_product_tails() {
    for exponent in [-500, 0, 500] {
        let scale = 2.0_f64.powi(exponent);
        for delta in [2.0_f64.powi(-27), 2.0_f64.powi(-30)] {
            for sign in [-1.0, 1.0] {
                for imaginary in [false, true] {
                    let component = |value| {
                        if imaginary {
                            Complex { re: 0.0, im: value }
                        } else {
                            Complex::real(value)
                        }
                    };
                    let base = component(sign * scale);
                    let left = Complex::real(1.0 - delta);
                    let right = component(sign * scale * (1.0 + delta));
                    let expected = sign * scale * delta * delta;
                    let value = difference(base, left, right, SolverStage::DenseFactor).unwrap();
                    let wide = arithmetic::WideComplex::from(base)
                        .subtract_product(left, right.into(), SolverStage::TridiagonalFactor)
                        .unwrap()
                        .finite()
                        .unwrap();
                    for actual in [value, wide] {
                        assert_eq!(if imaginary { actual.im } else { actual.re }, expected);
                        assert_eq!(if imaginary { actual.re } else { actual.im }, 0.0);
                    }
                }
            }
        }
    }
}

#[test]
fn harmonic_complex_cancellation_division_recovers_a_normal_determinant_tail() {
    for exponent in [-500, 0, 500] {
        let scale = 2.0_f64.powi(exponent);
        for delta in [2.0_f64.powi(-27), 2.0_f64.powi(-30)] {
            for sign in [-1.0, 1.0] {
                let value = divide(
                    Complex {
                        re: sign * scale,
                        im: sign * scale * (1.0 + delta),
                    },
                    Complex {
                        re: scale * (1.0 - delta),
                        im: scale,
                    },
                    SolverStage::DenseFactor,
                )
                .unwrap();
                let expected = -sign * delta * delta / ((1.0 - delta).powi(2) + 1.0);
                assert!(
                    (value.im / expected - 1.0).abs() <= 4.0 * f64::EPSILON,
                    "{value:?}"
                );
            }
        }
    }
}

#[test]
fn harmonic_complex_cancellation_replays_the_complete_row_after_an_intermediate_zero() {
    let delta = 2.0_f64.powi(-27);
    let terms = [
        (Complex::real(1.0 - delta), Complex::real(1.0 + delta)),
        (Complex::real(0.5), Complex::real(delta * delta)),
    ];
    let value = solve_row(
        Complex::real(1.0),
        terms.into_iter(),
        Complex::real(1.0),
        SolverStage::DenseSubstitutionRange,
    )
    .unwrap();
    assert_eq!(value.re, 0.5 * delta * delta);
    assert_eq!(value.im, 0.0);
    let large = 2.0_f64.powi(60);
    let value = solve_row(
        Complex { re: 1.0, im: -1.0 },
        [
            (Complex::real(large), Complex { re: 1.0, im: -1.0 }),
            (Complex::real(-large), Complex { re: 1.0, im: -1.0 }),
        ]
        .into_iter(),
        Complex::real(1.0),
        SolverStage::DenseSubstitutionRange,
    )
    .unwrap();
    assert_eq!(value.re, 1.0);
    assert_eq!(value.im, -1.0);
}

#[test]
fn harmonic_complex_cancellation_both_factorizations_preserve_tiny_unknowns() {
    let delta = 2.0_f64.powi(-27);
    let one = Complex::real(1.0);
    let zero = Complex::default();
    for forward in [false, true] {
        let (lower, upper, rhs) = if forward {
            (
                Complex::real(1.0 - delta),
                zero,
                vec![Complex::real(1.0 + delta), one],
            )
        } else {
            (
                zero,
                Complex::real(1.0 - delta),
                vec![one, Complex::real(1.0 + delta)],
            )
        };
        let dense = solve_complex_system(vec![vec![one, upper], vec![lower, one]], rhs.clone());
        let path = solve_complex_tridiagonal(ComplexTridiagonalSystem {
            diagonal: vec![one, one],
            lower: vec![lower],
            upper: vec![upper],
            rhs,
        });
        for result in [dense, path] {
            let values = result.unwrap();
            assert_eq!(values[usize::from(forward)].re, delta * delta);
            assert_eq!(values[usize::from(!forward)].re, 1.0 + delta);
            assert!(values.iter().all(|value| value.im == 0.0));
        }
    }
}

#[test]
fn harmonic_complex_cancellation_exact_zeros_remain_zero() {
    let phase = Complex { re: 1.0, im: 1.0 };
    let value = arithmetic::product(phase, phase, SolverStage::ResultElements).unwrap();
    assert_eq!(value.re, 0.0);
    assert_eq!(value.im, 2.0);
    let value = divide(phase, phase, SolverStage::DenseFactor).unwrap();
    assert_eq!(value.re, 1.0);
    assert_eq!(value.im, 0.0);
    let value = difference(phase, Complex::real(1.0), phase, SolverStage::DenseFactor).unwrap();
    assert_eq!(value.re, 0.0);
    assert_eq!(value.im, 0.0);
}

#[test]
fn harmonic_complex_cancellation_ordinary_and_empty_zero_paths_need_no_replay() {
    use crate::solver_control::{SolverControl, with_solver_observer};
    let stage = SolverStage::ResultElements;
    let control = SolverControl::default();
    let cancel = control.clone();
    with_solver_observer(
        &control,
        move |point| {
            if point.stage == stage {
                cancel.request_cancel();
            }
        },
        || {
            let a = Complex {
                re: 0.75,
                im: -0.125,
            };
            let x = Complex { re: 1.25, im: 0.5 };
            let base = Complex { re: 2.0, im: 0.25 };
            let zero = Complex::default();
            arithmetic::product(a, x, stage)?;
            difference(base, a, x, stage)?;
            divide(base, a, stage)?;
            solve_row(base, std::iter::once((a, x)), a, stage)?;
            arithmetic::product(zero, a, stage)?;
            difference(zero, zero, a, stage)?;
            divide(zero, a, stage)?;
            Ok::<_, String>(())
        },
    )
    .unwrap();
    assert!(!control.was_interrupted());
}

#[test]
fn harmonic_complex_cancellation_replay_is_cancellable_and_inputs_are_reusable() {
    use crate::solver_control::{SolverControl, with_solver_observer};
    use std::cell::Cell;
    let large = 2.0_f64.powi(60);
    let terms = (0..2050)
        .map(|index| {
            (
                Complex::real(1.0),
                Complex::real(if index % 2 == 0 { large } else { -large }),
            )
        })
        .collect::<Vec<_>>();
    let stage = SolverStage::DenseSubstitutionRange;
    let run = || {
        solve_row(
            Complex::real(1.0),
            terms.iter().copied(),
            Complex::real(1.0),
            stage,
        )
    };
    let baseline = run().unwrap();
    assert_eq!(baseline.re, 1.0);
    for (completed, phase) in [(0, 1), (1024, 0), (4101, 0), (8202, 0), (0, 2)] {
        let control = SolverControl::default();
        let cancel = control.clone();
        let entered = Cell::new(0);
        let error = with_solver_observer(
            &control,
            move |point| {
                if point.stage == stage {
                    if point.completed_steps == 0 {
                        entered.set(entered.get() + 1);
                    }
                    if point.completed_steps == completed && (phase == 0 || entered.get() == phase)
                    {
                        cancel.request_cancel();
                    }
                }
            },
            run,
        )
        .unwrap_err();
        assert!(
            control.was_interrupted() && error.contains(stage.as_str()),
            "{error}"
        );
        let value = run().unwrap();
        assert_eq!(
            (value.re.to_bits(), value.im.to_bits()),
            (baseline.re.to_bits(), baseline.im.to_bits())
        );
    }
}
