use super::*;

#[test]
fn harmonic_quotient_rounding_scaled_division_rounds_once_on_the_subnormal_lattice() {
    let epsilon = f64::EPSILON;
    for (numerator, power, expected) in [
        (1.5 + epsilon, -1074, 1_u64),
        (1.0 + 4.0 * epsilon, -1023, (1_u64 << 51) + 1),
    ] {
        for sign in [-1.0, 1.0] {
            let value = Scaled {
                value: sign * numerator,
                exponent: power,
            }
            .quotient(Scaled::from(1.0 + epsilon))
            .unwrap();
            assert_eq!(value.to_bits(), (sign * f64::from_bits(expected)).to_bits());
        }
    }
}

#[test]
fn harmonic_quotient_rounding_axial_division_preserves_independent_quantized_components() {
    let numerator = f64::from_bits((1023_u64 << 52) | 4);
    let divisor = f64::from_bits((2046_u64 << 52) | 1);
    let expected = f64::from_bits((1_u64 << 51) + 1);
    for sign in [-1.0, 1.0] {
        for imaginary_pivot in [false, true] {
            let pivot = if imaginary_pivot {
                Complex {
                    re: 0.0,
                    im: divisor,
                }
            } else {
                Complex::real(divisor)
            };
            let value = divide(
                Complex {
                    re: sign * numerator,
                    im: -sign * numerator,
                },
                pivot,
                SolverStage::TridiagonalSubstitutionRange,
            )
            .unwrap();
            assert_eq!(
                value.re.to_bits(),
                ((if imaginary_pivot { -sign } else { sign }) * expected).to_bits()
            );
            assert_eq!(value.im.to_bits(), (-sign * expected).to_bits());
        }
    }
}

#[test]
fn harmonic_quotient_rounding_matches_direct_hardware_division_on_exactly_scaled_inputs() {
    let mut seed = 0x517c_c1b7_2722_0a95_u64;
    for power in -1075..=-1022 {
        for _ in 0..32 {
            seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
            let n = f64::from_bits((1023_u64 << 52) | (seed & ((1 << 52) - 1)));
            seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
            let d = f64::from_bits((1023_u64 << 52) | (seed & ((1 << 52) - 1)));
            for sign in [-1.0, 1.0] {
                // Both scaled operands are normal and exactly represented. This
                // reference performs one hardware division, with no scaled restore.
                let expected = (sign * n * f64::MIN_POSITIVE) / (d * 2.0_f64.powi(-1022 - power));
                let actual = Scaled {
                    value: sign * n,
                    exponent: power,
                }
                .quotient(Scaled::from(d));
                if expected == 0.0 {
                    assert!(actual.is_err());
                } else {
                    assert_eq!(actual.unwrap().to_bits(), expected.to_bits());
                }
            }
        }
    }
}

#[test]
fn harmonic_quotient_rounding_keeps_ties_range_guards_normal_bits_and_signed_zeros() {
    for (n, power, expected) in [
        (1.5, -1074, 2_u64),
        (1.25, -1073, 2),
        (1.75, -1073, 4),
        (1.0 - 2.0_f64.powi(-53), -1022, 1 << 52),
        (1.0 - f64::EPSILON, -1022, (1 << 52) - 1),
    ] {
        for sign in [-1.0, 1.0] {
            let value = Scaled {
                value: sign * n,
                exponent: power,
            }
            .quotient(Scaled::from(1.0))
            .unwrap();
            assert_eq!(value.to_bits(), (sign * f64::from_bits(expected)).to_bits());
        }
    }
    for n in [0.0, -0.0, 0.75, -1.25] {
        for d in [-1.5, 1.25] {
            for power in [-500, 0, 500] {
                let expected = restore(n / d, power);
                let value = Scaled {
                    value: n,
                    exponent: power,
                }
                .quotient(Scaled::from(d))
                .unwrap();
                assert_eq!(value.to_bits(), expected.to_bits());
            }
        }
    }
    assert_eq!(
        Scaled::from(f64::MAX).quotient(Scaled::from(1.0)).unwrap(),
        f64::MAX
    );
    for power in [-1075, i32::MIN, i32::MAX] {
        assert!(
            Scaled {
                value: 1.0,
                exponent: power
            }
            .quotient(Scaled::from(1.0))
            .is_err()
        );
    }
    assert!(Scaled::from(f64::MAX).quotient(Scaled::from(0.5)).is_err());
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(Scaled::from(bad).quotient(Scaled::from(1.0)).is_err());
        assert!(Scaled::from(1.0).quotient(Scaled::from(bad)).is_err());
    }
    assert!(Scaled::from(1.0).quotient(Scaled::from(0.0)).is_err());
}

#[test]
fn harmonic_quotient_rounding_axial_division_cancels_at_each_component_boundary_then_replays() {
    use crate::solver_control::{SolverControl, with_solver_observer};
    let left = Complex {
        re: 1.0 + 4.0 * f64::EPSILON,
        im: -1.0 - 4.0 * f64::EPSILON,
    };
    let divisor = f64::from_bits((2046_u64 << 52) | 1);
    for pivot in [
        Complex::real(divisor),
        Complex {
            re: 0.0,
            im: -divisor,
        },
    ] {
        for completed in 0..=2 {
            let control = SolverControl::default();
            let cancel = control.clone();
            let error = with_solver_observer(
                &control,
                move |point| {
                    if point.stage == SolverStage::TridiagonalSubstitutionRange
                        && point.completed_steps == completed
                    {
                        cancel.request_cancel();
                    }
                },
                || divide(left, pivot, SolverStage::TridiagonalSubstitutionRange),
            )
            .unwrap_err();
            assert!(
                control.was_interrupted() && error.contains("cancelled"),
                "{error}"
            );
            let replay = divide(left, pivot, SolverStage::TridiagonalSubstitutionRange).unwrap();
            assert_eq!(replay.re.abs().to_bits(), (1_u64 << 51) + 1);
            assert_eq!(replay.im.abs().to_bits(), (1_u64 << 51) + 1);
        }
    }
}

#[test]
fn harmonic_quotient_rounding_both_factorizations_preserve_quantized_axial_unknowns() {
    use super::super::{ComplexTridiagonalSystem, solve_complex_system, solve_complex_tridiagonal};
    let divisor = f64::from_bits((2046_u64 << 52) | 1);
    let rhs = Complex::real(1.0 + 4.0 * f64::EPSILON);
    for imaginary in [false, true] {
        let coefficient = if imaginary {
            Complex {
                re: 0.0,
                im: divisor,
            }
        } else {
            Complex::real(divisor)
        };
        let dense = solve_complex_system(vec![vec![coefficient]], vec![rhs]);
        let path = solve_complex_tridiagonal(ComplexTridiagonalSystem {
            diagonal: vec![coefficient],
            lower: vec![],
            upper: vec![],
            rhs: vec![rhs],
        });
        for result in [dense, path] {
            let value = result.unwrap()[0];
            let component = if imaginary { value.im } else { value.re };
            assert_eq!(component.abs().to_bits(), (1_u64 << 51) + 1);
            assert_eq!(if imaginary { value.re } else { value.im }, 0.0);
        }
    }
}
