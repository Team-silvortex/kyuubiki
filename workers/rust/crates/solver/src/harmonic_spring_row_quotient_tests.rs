use super::*;

fn fixture() -> (Complex, Complex, Complex) {
    (
        Complex::real(1.0 + 2.0_f64.powi(-30)),
        Complex::real(f64::from_bits(0x3ff0000000021abf)),
        Complex::real(f64::from_bits(0x3feffffffff91180)),
    )
}

#[test]
fn harmonic_row_quotient_replay_divides_the_unrounded_expression() {
    let (rhs, a, x) = fixture();
    assert!(
        row_update::numerator(rhs, std::iter::once((a, x)))
            .unwrap()
            .is_none()
    );
    // Independent rational numerator: 301425993429140949475 / 2^98.
    for (pivot, bits) in [
        (Complex::real(51.0), [0x3db48169b9b9de56, 0]),
        (
            Complex { re: 51.0, im: 4.0 },
            [0x3db461519a11f21b, 0xbd79933939d03ee1],
        ),
    ] {
        for power in [-500, 0, 500] {
            let scale = 2.0_f64.powi(power);
            for phase in 0..4 {
                let expected = rotate(
                    Complex {
                        re: f64::from_bits(bits[0]),
                        im: f64::from_bits(bits[1]),
                    },
                    phase,
                );
                for terms in [
                    [
                        (scaled(a, scale), rotate(x, phase)),
                        (Complex::default(), Complex::default()),
                        (Complex::default(), Complex::default()),
                    ],
                    [
                        (Complex::real(-scale), rotate(Complex::real(1.0), phase)),
                        (scaled(a, scale), rotate(x, phase)),
                        (Complex::real(scale), rotate(Complex::real(1.0), phase)),
                    ],
                ] {
                    let actual = solve_row(
                        rotate(scaled(rhs, scale), phase),
                        terms.into_iter(),
                        scaled(pivot, scale),
                        SolverStage::DenseSubstitutionRange,
                    )
                    .unwrap();
                    assert_components(actual, expected);
                }
            }
        }
    }
}

#[test]
fn harmonic_row_quotient_replay_reaches_both_factorizations() {
    use super::super::{ComplexTridiagonalSystem, solve_complex_system, solve_complex_tridiagonal};
    let (rhs, a, x) = fixture();
    for (pivot, expected) in [
        (
            Complex::real(51.0),
            Complex::real(f64::from_bits(0x3db48169b9b9de56)),
        ),
        (
            Complex { re: 51.0, im: 4.0 },
            Complex {
                re: f64::from_bits(0x3db461519a11f21b),
                im: f64::from_bits(0xbd79933939d03ee1),
            },
        ),
        (
            Complex { re: 0.0, im: 51.0 },
            Complex {
                re: 0.0,
                im: -f64::from_bits(0x3db48169b9b9de56),
            },
        ),
    ] {
        for swapped in [false, true] {
            let (diagonal, lower, upper, rhs) = if swapped {
                (
                    vec![Complex::default(), a],
                    vec![pivot],
                    vec![Complex::real(1.0)],
                    vec![x, rhs],
                )
            } else {
                (
                    vec![pivot, Complex::real(1.0)],
                    vec![Complex::default()],
                    vec![a],
                    vec![rhs, x],
                )
            };
            for result in [
                solve_complex_system(
                    vec![vec![diagonal[0], upper[0]], vec![lower[0], diagonal[1]]],
                    rhs.clone(),
                ),
                solve_complex_tridiagonal(ComplexTridiagonalSystem {
                    diagonal,
                    lower,
                    upper,
                    rhs,
                }),
            ] {
                let actual = result.unwrap();
                assert_components(actual[0], expected);
                assert_eq!(actual[1].re.to_bits(), x.re.to_bits());
            }
        }
    }
}

fn scaled(value: Complex, scale: f64) -> Complex {
    Complex {
        re: value.re * scale,
        im: value.im * scale,
    }
}

fn rotate(value: Complex, phase: usize) -> Complex {
    match phase {
        0 => value,
        1 => Complex {
            re: -value.im,
            im: value.re,
        },
        2 => Complex {
            re: -value.re,
            im: -value.im,
        },
        _ => Complex {
            re: value.im,
            im: -value.re,
        },
    }
}

fn assert_components(actual: Complex, expected: Complex) {
    for (a, e) in [(actual.re, expected.re), (actual.im, expected.im)] {
        if e == 0.0 {
            assert_eq!(a, 0.0);
        } else {
            assert_eq!(a.to_bits(), e.to_bits());
        }
    }
}

#[test]
fn harmonic_row_quotient_replay_rounds_once_at_the_subnormal_midpoint() {
    let stage = SolverStage::DenseSubstitutionRange;
    for sign in [-1.0, 1.0] {
        for (units, perturbation, expected) in [
            (3.0, 1.0, Some(1)),
            (3.0, 0.0, Some(2)),
            (3.0, -1.0, Some(2)),
            (5.0, -1.0, Some(3)),
            (1.0, -1.0, Some(1)),
            (1.0, 0.0, None),
            (1.0, 1.0, None),
        ] {
            let terms = [
                (Complex::real(-sign), Complex::real(1.0)),
                (Complex::real(sign), Complex::real(1.0)),
                (
                    Complex::real(-sign * units * 2.0_f64.powi(-52)),
                    Complex::real(1.0),
                ),
                (
                    Complex::real(sign * perturbation * 2.0_f64.powi(-53)),
                    Complex::real(2.0_f64.powi(-53)),
                ),
            ];
            for pivot in [
                Complex::real(2.0_f64.powi(1023)),
                Complex {
                    re: 0.0,
                    im: 2.0_f64.powi(1023),
                },
            ] {
                let actual = solve_row(Complex::default(), terms.into_iter(), pivot, stage);
                if let Some(bits) = expected {
                    let value = sign * f64::from_bits(bits);
                    assert_components(
                        actual.unwrap(),
                        if pivot.im == 0.0 {
                            Complex::real(value)
                        } else {
                            Complex {
                                re: 0.0,
                                im: -value,
                            }
                        },
                    );
                } else {
                    assert!(actual.unwrap_err().contains("representable range"));
                }
            }
        }
    }
}

#[test]
fn harmonic_row_quotient_retains_alignment_and_final_range_guards() {
    let stage = SolverStage::DenseSubstitutionRange;
    for pivot in [Complex::real(1.0), Complex { re: 0.5, im: 0.5 }] {
        assert!(
            solve_row(
                Complex::real(f64::MAX),
                std::iter::once((Complex::real(-1.0), Complex::real(f64::MAX))),
                pivot,
                stage
            )
            .is_err()
        );
    }
    let terms = [
        (Complex::real(1.0), Complex::real(f64::MAX)),
        (Complex::real(f64::from_bits(1)), Complex::real(1.0)),
    ];
    assert!(
        solve_row(
            Complex::real(f64::MAX),
            terms.into_iter(),
            Complex::real(1.0),
            stage
        )
        .unwrap_err()
        .contains("normalization would lose numerical range")
    );
    let (rhs, a, x) = fixture();
    let bad = [(a, x), (Complex::real(f64::NAN), Complex::default())];
    assert!(
        solve_row(rhs, bad.into_iter(), Complex::real(51.0), stage)
            .unwrap_err()
            .contains("non-finite")
    );
    assert_eq!(
        solve_row(rhs, std::iter::once((a, x)), Complex::real(51.0), stage)
            .unwrap()
            .re
            .to_bits(),
        0x3db48169b9b9de56
    );
}

#[test]
fn harmonic_row_quotient_new_integer_phases_cancel_then_reuse_without_partial_results() {
    use crate::solver_control::{SolverControl, with_solver_observer};
    use std::cell::Cell;
    let (rhs, a, x) = fixture();
    let stage = SolverStage::DenseSubstitutionRange;
    let run = || {
        solve_row(
            rhs,
            std::iter::once((a, x)),
            Complex { re: 51.0, im: 4.0 },
            stage,
        )
    };
    for (completed, occurrence) in [
        (0, 1),
        (0, 2),
        (0, 3),
        (0, 4),
        (0, 5),
        (0, 6),
        (0, 7),
        (1, 1),
        (16, 1),
        (32, 1),
        (48, 1),
        (64, 1),
        (65, 1),
        (65, 2),
    ] {
        let control = SolverControl::default();
        let cancel = control.clone();
        let seen = Cell::new(0);
        let error = with_solver_observer(
            &control,
            move |p| {
                if p.stage == stage && p.completed_steps == completed {
                    seen.set(seen.get() + 1);
                    if seen.get() == occurrence {
                        cancel.request_cancel();
                    }
                }
            },
            run,
        )
        .unwrap_err();
        assert!(
            control.was_interrupted() && error.contains("cancelled"),
            "{completed}/{occurrence}: {error}"
        );
        assert_components(
            run().unwrap(),
            Complex {
                re: f64::from_bits(0x3db461519a11f21b),
                im: f64::from_bits(0xbd79933939d03ee1),
            },
        );
    }
}

#[test]
fn harmonic_row_quotient_ordinary_positive_control_needs_no_integer_replay() {
    use crate::solver_control::{SolverControl, with_solver_observer};
    use std::{cell::Cell, rc::Rc};
    let events = Rc::new(Cell::new(0));
    let observed = Rc::clone(&events);
    let result = with_solver_observer(
        &SolverControl::default(),
        move |_| observed.set(observed.get() + 1),
        || {
            solve_row(
                Complex::real(5.0),
                std::iter::once((Complex::real(1.0), Complex::real(1.0))),
                Complex::real(2.0),
                SolverStage::DenseSubstitutionRange,
            )
        },
    )
    .unwrap();
    assert_components(result, Complex::real(2.0));
    assert_eq!(events.get(), 0);
}

#[test]
fn harmonic_row_quotient_wide_rhs_cancels_before_exact_complex_division() {
    let large = 2.0_f64.powi(500);
    let value = solve_row(
        WideComplex {
            real: Scaled {
                value: 1.0,
                exponent: 1500,
            },
            imaginary: Scaled {
                value: 1.0,
                exponent: 1500,
            },
        },
        [
            (
                Complex::real(large),
                Complex {
                    re: 2.0_f64.powi(1000),
                    im: 2.0_f64.powi(1000),
                },
            ),
            (Complex::real(-3.0 * large), Complex::real(1.0)),
        ]
        .into_iter(),
        Complex {
            re: large,
            im: large,
        },
        SolverStage::DenseSubstitutionRange,
    )
    .unwrap();
    assert_components(value, Complex { re: 1.5, im: -1.5 });
}
