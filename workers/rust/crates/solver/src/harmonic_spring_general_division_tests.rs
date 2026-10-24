use super::*;

#[test]
fn harmonic_general_division_extreme_component_identity_is_not_an_intermediate_range_error() {
    for large in [1.0, f64::MAX] {
        for sign in [-1.0, 1.0] {
            let pivot = Complex {
                re: large,
                im: sign * f64::from_bits(1),
            };
            let value = divide(pivot, pivot, SolverStage::DenseSubstitutionRange).unwrap();
            assert_eq!(value.re, 1.0);
            assert_eq!(value.im, 0.0);
        }
    }
}

#[test]
fn harmonic_general_division_rounds_the_ratio_of_exact_products_at_the_quantum_boundary() {
    let pivot = Complex {
        re: f64::from_bits((2046_u64 << 52) | 5),
        im: f64::from_bits((2046_u64 << 52) | 2),
    };
    for sign in [-1.0, 1.0] {
        let value = divide(
            Complex::real(sign),
            pivot,
            SolverStage::TridiagonalSubstitutionRange,
        )
        .unwrap();
        let expected = f64::from_bits((1_u64 << 50) - 1);
        assert_eq!(value.re.to_bits(), (sign * expected).to_bits());
        assert_eq!(value.im.to_bits(), (-sign * expected).to_bits());
    }
}

#[test]
fn harmonic_general_division_matches_independent_integer_ratios_across_binary_scales() {
    let mut seed = 0x721a_5f4c_193e_0081_u64;
    let mut next = || {
        seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
        ((seed >> 32) % 63) as i64 - 31
    };
    for _ in 0..256 {
        let (a, b, c, mut d) = (next(), next(), next(), next());
        if c == 0 && d == 0 {
            d = 1;
        }
        let denominator = c * c + d * d;
        let numerators = [a * c + b * d, b * c - a * d];
        for power in [-1075, -1023, -1022, -1000, -500, 0, 500, 1000] {
            let left_scale = 2.0_f64.powi(power / 2);
            let right_scale = 2.0_f64.powi(power / 2 - power);
            // Integer products are exact and small. For subnormal answers,
            // exactly scale two normal operands before ONE hardware division.
            let expected = numerators.map(|n| {
                if power <= -1022 {
                    (n as f64 * f64::MIN_POSITIVE)
                        / (denominator as f64 * 2.0_f64.powi(-1022 - power))
                } else {
                    (n as f64 / denominator as f64) * 2.0_f64.powi(power)
                }
            });
            let actual = complex_quotient::divide(
                Scaled::from(a as f64 * left_scale),
                Scaled::from(b as f64 * left_scale),
                Complex {
                    re: c as f64 * right_scale,
                    im: d as f64 * right_scale,
                },
                SolverStage::DenseSubstitutionRange,
            );
            if expected
                .iter()
                .zip(numerators)
                .any(|(&x, n)| x == 0.0 && n != 0)
            {
                assert!(actual.is_err());
            } else {
                let actual = actual.unwrap();
                for (value, reference) in [actual.re, actual.im].into_iter().zip(expected) {
                    if reference == 0.0 {
                        assert_eq!(value, 0.0);
                    } else {
                        assert_eq!(value.to_bits(), reference.to_bits());
                    }
                }
            }
        }
    }
}

#[test]
fn harmonic_general_division_preserves_ties_true_range_failures_and_wide_exponents() {
    let stage = SolverStage::DenseSubstitutionRange;
    let pivot = Complex { re: 1.0, im: 1.0 };
    for (units, expected) in [(3, 2), (5, 2), (7, 4)] {
        let value = divide(Complex::real(f64::from_bits(units)), pivot, stage).unwrap();
        assert_eq!(value.re.to_bits(), expected);
        assert_eq!(value.im.to_bits(), expected | (1 << 63));
    }
    for (a, b, expected) in [
        (1.0, 1.0 + f64::EPSILON, 1.0),
        (2.0 - f64::EPSILON, 2.0, 2.0),
    ] {
        let result =
            complex_quotient::divide(Scaled::from(a), Scaled::from(b), pivot, stage).unwrap();
        assert_eq!(result.re, expected);
        assert_eq!(result.im, (b - a) / 2.0);
    }
    let maximum =
        complex_quotient::divide(Scaled::from(f64::MAX), Scaled::from(f64::MAX), pivot, stage)
            .unwrap();
    assert_eq!(maximum.re, f64::MAX);
    assert_eq!(maximum.im, 0.0);
    assert!(divide(Complex::real(f64::from_bits(1)), pivot, stage).is_err());
    assert!(
        divide(
            Complex {
                re: f64::MAX,
                im: f64::MAX
            },
            Complex { re: 0.5, im: 0.5 },
            stage,
        )
        .is_err()
    );
    for power in [i32::MIN, i32::MAX] {
        assert!(
            complex_quotient::divide(
                Scaled {
                    value: 1.0,
                    exponent: power
                },
                Scaled::from(0.0),
                pivot,
                stage,
            )
            .unwrap_err()
            .contains("representable range")
        );
    }
    assert!(
        complex_quotient::divide(
            Scaled {
                value: 1.0,
                exponent: i32::MAX
            },
            Scaled {
                value: 1.0,
                exponent: i32::MIN
            },
            pivot,
            stage,
        )
        .unwrap_err()
        .contains("capacity")
    );
    let wide = complex_quotient::divide(
        Scaled {
            value: 1.0,
            exponent: 1500,
        },
        Scaled {
            value: 1.0,
            exponent: 1500,
        },
        Complex {
            re: 2.0_f64.powi(500),
            im: 2.0_f64.powi(500),
        },
        stage,
    )
    .unwrap();
    assert_eq!(wide.re, 2.0_f64.powi(1000));
    assert_eq!(wide.im, 0.0);
}

#[test]
fn harmonic_general_division_cancels_each_exact_phase_and_output_boundary_then_replays() {
    use crate::solver_control::{SolverControl, with_solver_observer};
    use std::cell::Cell;
    let stage = SolverStage::TridiagonalSubstitutionRange;
    let run = || {
        divide(
            Complex::real(1.0),
            Complex {
                re: f64::from_bits((2046_u64 << 52) | 2),
                im: f64::from_bits((2046_u64 << 52) | 7),
            },
            stage,
        )
    };
    let baseline = run().unwrap();
    for (completed, occurrence) in [
        (0, 1),
        (0, 2),
        (0, 3),
        (1, 1),
        (1, 2),
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
            move |point| {
                if point.stage == stage && point.completed_steps == completed {
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
            "{error}"
        );
        let replay = run().unwrap();
        assert_eq!(
            (replay.re.to_bits(), replay.im.to_bits()),
            (baseline.re.to_bits(), baseline.im.to_bits())
        );
    }
}

#[test]
fn harmonic_general_division_both_factorizations_retain_the_damped_quantum_components() {
    use super::super::{ComplexTridiagonalSystem, solve_complex_system, solve_complex_tridiagonal};
    let coefficient = Complex {
        re: f64::from_bits((2046_u64 << 52) | 2),
        im: f64::from_bits((2046_u64 << 52) | 7),
    };
    for result in [
        solve_complex_system(vec![vec![coefficient]], vec![Complex::real(1.0)]),
        solve_complex_tridiagonal(ComplexTridiagonalSystem {
            diagonal: vec![coefficient],
            lower: vec![],
            upper: vec![],
            rhs: vec![Complex::real(1.0)],
        }),
    ] {
        let value = result.unwrap()[0];
        assert_eq!(value.re.to_bits(), (1_u64 << 50) - 2);
        assert_eq!(value.im.to_bits(), ((1_u64 << 50) - 1) | (1 << 63));
    }
}
