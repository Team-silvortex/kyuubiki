use super::*;

#[test]
fn harmonic_complex_near_cancellation_product_retains_nonzero_component_tails() {
    for exponent in [-500, 0, 500] {
        let scale = 2.0_f64.powi(exponent);
        for multiple in [3.0, 5.0, 7.0] {
            let delta = multiple * 2.0_f64.powi(-27);
            for sign in [-1.0, 1.0] {
                let right = Complex {
                    re: scale * (1.0 - delta),
                    im: scale,
                };
                for (left, expected) in [
                    (
                        Complex {
                            re: sign * (1.0 + delta),
                            im: sign,
                        },
                        Complex {
                            re: -sign * scale * delta * delta,
                            im: sign * scale * 2.0,
                        },
                    ),
                    (
                        Complex {
                            re: -sign,
                            im: sign * (1.0 + delta),
                        },
                        Complex {
                            re: -sign * scale * 2.0,
                            im: -sign * scale * delta * delta,
                        },
                    ),
                ] {
                    let value =
                        arithmetic::product(left, right, SolverStage::ResultElements).unwrap();
                    assert_eq!(value.re, expected.re);
                    assert_eq!(value.im, expected.im);
                }
            }
        }
    }
}

#[test]
fn harmonic_complex_near_cancellation_updates_keep_nonzero_product_tails() {
    for exponent in [-500, 0, 500] {
        let scale = 2.0_f64.powi(exponent);
        for multiple in [3.0, 5.0, 7.0] {
            let delta = multiple * 2.0_f64.powi(-27);
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
                    let a = Complex::real(1.0 - delta);
                    let x = component(sign * scale * (1.0 + delta));
                    let expected = sign * scale * delta * delta;
                    let direct = difference(base, a, x, SolverStage::DenseFactor).unwrap();
                    let rhs = arithmetic::WideComplex::from(base)
                        .subtract_product(a, x.into(), SolverStage::TridiagonalFactor)
                        .unwrap()
                        .finite()
                        .unwrap();
                    for value in [direct, rhs] {
                        assert_eq!(if imaginary { value.im } else { value.re }, expected);
                        assert_eq!(if imaginary { value.re } else { value.im }, 0.0);
                    }
                }
            }
        }
    }
}

#[test]
fn harmonic_complex_near_cancellation_division_keeps_nonzero_determinant_tails() {
    for exponent in [-500, 0, 500] {
        let scale = 2.0_f64.powi(exponent);
        for multiple in [3.0, 5.0, 7.0] {
            let delta = multiple * 2.0_f64.powi(-27);
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
                    "{value:?}, expected {expected}"
                );
            }
        }
    }
}

fn gradual_terms() -> Vec<(Complex, Complex)> {
    let one = Complex::real(1.0);
    let phase = |value| Complex {
        re: value,
        im: -value,
    };
    let mut terms = vec![(one, phase(1.0))];
    terms.extend((0..1023).map(|_| (one, phase(-1.0 / 1024.0))));
    terms.push((one, phase(-1.0 / 1024.0 - 2.0_f64.powi(-30))));
    terms.resize(2050, (Complex::default(), Complex::default()));
    terms
}

fn gradual_row(terms: &[(Complex, Complex)]) -> Result<Complex, String> {
    let tiny = 2.0_f64.powi(-54);
    solve_row(
        Complex {
            re: tiny,
            im: -tiny,
        },
        terms.iter().copied(),
        Complex::real(1.0),
        SolverStage::DenseSubstitutionRange,
    )
}

#[test]
fn harmonic_complex_near_cancellation_replays_earlier_addition_loss_after_gradual_shrinkage() {
    let value = gradual_row(&gradual_terms()).unwrap();
    let expected = 2.0_f64.powi(-30) + 2.0_f64.powi(-54);
    assert_eq!(value.re, expected);
    assert_eq!(value.im, -expected);
}

#[test]
fn harmonic_complex_near_cancellation_both_factorizations_preserve_nonzero_unknown_tails() {
    let one = Complex::real(1.0);
    let zero = Complex::default();
    for multiple in [3.0, 5.0, 7.0] {
        let delta = multiple * 2.0_f64.powi(-27);
        for forward in [false, true] {
            for sign in [-1.0, 1.0] {
                let rhs = if forward {
                    vec![Complex::real(sign * (1.0 + delta)), Complex::real(sign)]
                } else {
                    vec![Complex::real(sign), Complex::real(sign * (1.0 + delta))]
                };
                let (lower, upper) = if forward {
                    (Complex::real(1.0 - delta), zero)
                } else {
                    (zero, Complex::real(1.0 - delta))
                };
                let dense =
                    solve_complex_system(vec![vec![one, upper], vec![lower, one]], rhs.clone());
                let path = solve_complex_tridiagonal(ComplexTridiagonalSystem {
                    diagonal: vec![one, one],
                    lower: vec![lower],
                    upper: vec![upper],
                    rhs,
                });
                for result in [dense, path] {
                    let values = result.unwrap();
                    assert_eq!(values[usize::from(forward)].re, sign * delta * delta);
                    assert_eq!(values[usize::from(!forward)].re, sign * (1.0 + delta));
                    assert!(values.iter().all(|value| value.im == 0.0));
                }
            }
        }
    }
}

#[test]
fn harmonic_complex_near_cancellation_gradual_replay_cancels_then_reuses_inputs() {
    use crate::solver_control::{SolverControl, with_solver_observer};
    use std::cell::Cell;
    let terms = gradual_terms();
    let expected = 2.0_f64.powi(-30) + 2.0_f64.powi(-54);
    for (completed, phase) in [(0, 1), (1024, 0), (4101, 0), (8202, 0), (0, 2)] {
        let control = SolverControl::default();
        let cancel = control.clone();
        let entered = Cell::new(0);
        let error = with_solver_observer(
            &control,
            move |point| {
                if point.stage == SolverStage::DenseSubstitutionRange {
                    if point.completed_steps == 0 {
                        entered.set(entered.get() + 1);
                    }
                    if point.completed_steps == completed && (phase == 0 || entered.get() == phase)
                    {
                        cancel.request_cancel();
                    }
                }
            },
            || gradual_row(&terms),
        )
        .unwrap_err();
        assert!(
            control.was_interrupted() && error.contains("cancelled"),
            "{error}"
        );
        let value = gradual_row(&terms).unwrap();
        assert_eq!(value.re, expected);
        assert_eq!(value.im, -expected);
    }
}

#[test]
fn harmonic_complex_near_cancellation_routing_is_relative_and_keeps_exact_values() {
    use crate::solver_control::{SolverControl, with_solver_observer};
    use std::{cell::Cell, rc::Rc};
    for exponent in [-500, 0, 500] {
        let scale = 2.0_f64.powi(exponent);
        for (power, replay) in [(-27, true), (-26, true), (-25, false)] {
            for sign in [-1.0, 1.0] {
                let residual = 2.0_f64.powi(power);
                let entered = Rc::new(Cell::new(false));
                let observed = Rc::clone(&entered);
                let value = with_solver_observer(
                    &SolverControl::default(),
                    move |point| {
                        if point.stage == SolverStage::DenseFactor {
                            observed.set(true);
                        }
                    },
                    || {
                        difference(
                            Complex::real(sign * scale),
                            Complex::real(1.0),
                            Complex::real(sign * scale * (1.0 - residual)),
                            SolverStage::DenseFactor,
                        )
                    },
                )
                .unwrap();
                assert_eq!(value.re.to_bits(), (sign * scale * residual).to_bits());
                assert_eq!(value.im, 0.0);
                assert_eq!(
                    entered.get(),
                    replay,
                    "scale 2^{exponent}, residual 2^{power}"
                );
            }
        }
    }
}

#[test]
fn harmonic_complex_near_cancellation_does_not_hide_invalid_later_terms_or_poison_replay() {
    let mut terms = gradual_terms();
    terms[2049].1.im = f64::NAN;
    let error = gradual_row(&terms).unwrap_err();
    assert!(error.contains("non-finite"), "{error}");
    terms[2049].1.im = 0.0;
    let value = gradual_row(&terms).unwrap();
    assert_eq!(value.re, 2.0_f64.powi(-30) + 2.0_f64.powi(-54));
}
