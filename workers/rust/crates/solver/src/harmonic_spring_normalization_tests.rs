use super::*;

#[test]
fn harmonic_normalization_precision_preserves_signed_weak_responses_before_cancellation() {
    for exponent in [-500, 0, 500] {
        let scale = 2.0_f64.powi(exponent);
        for multiple in [1.0, 3.0, 5.0, 7.0] {
            let delta = multiple * 2.0_f64.powi(-27);
            for sign in [-1.0, 1.0] {
                for imaginary in [false, true] {
                    let rhs = |value| {
                        if imaginary {
                            Complex { re: 0.0, im: value }
                        } else {
                            Complex::real(value)
                        }
                    };
                    for forward in [false, true] {
                        let diagonal = if forward {
                            vec![Complex::real(scale), Complex::real(3.0 * scale)]
                        } else {
                            vec![Complex::real(3.0 * scale), Complex::real(scale)]
                        };
                        let coupling = Complex::real(scale * (1.0 - delta));
                        let (lower, upper, rhs) = if forward {
                            (
                                coupling,
                                Complex::default(),
                                vec![rhs(sign * scale * (1.0 + delta)), rhs(sign * scale)],
                            )
                        } else {
                            (
                                Complex::default(),
                                coupling,
                                vec![rhs(sign * scale), rhs(sign * scale * (1.0 + delta))],
                            )
                        };
                        let dense = solve_complex_system(
                            vec![vec![diagonal[0], upper], vec![lower, diagonal[1]]],
                            rhs.clone(),
                        );
                        let path = solve_complex_tridiagonal(ComplexTridiagonalSystem {
                            diagonal,
                            lower: vec![lower],
                            upper: vec![upper],
                            rhs,
                        });
                        for result in [dense, path] {
                            let values = result.unwrap();
                            let value = values[usize::from(forward)];
                            let actual = if imaginary { value.im } else { value.re };
                            let expected = sign * delta * delta / 3.0;
                            assert!(
                                (actual / expected - 1.0).abs() <= 4.0 * f64::EPSILON,
                                "{value:?}, expected {expected}"
                            );
                            assert_eq!(if imaginary { value.re } else { value.im }, 0.0);
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn harmonic_normalization_precision_keeps_weighted_pivot_floor_before_and_after_swaps() {
    let floor = 32.0 * f64::EPSILON * 2.0;
    for swap in [false, true] {
        for ratio in [0.9, 1.1] {
            let weak = Complex::real(3.0 * ratio * floor);
            let (diagonal, lower, upper, rhs) = if swap {
                (
                    vec![Complex::default(), Complex::real(3.0)],
                    weak,
                    Complex::real(5.0),
                    vec![Complex::real(5.0), Complex::real(3.0)],
                )
            } else {
                (
                    vec![weak, Complex::real(1.0)],
                    Complex::default(),
                    Complex::real(3.0),
                    vec![Complex::real(3.0), Complex::real(1.0)],
                )
            };
            let dense = solve_complex_system(
                vec![vec![diagonal[0], upper], vec![lower, diagonal[1]]],
                rhs.clone(),
            );
            let path = solve_complex_tridiagonal(ComplexTridiagonalSystem {
                diagonal,
                lower: vec![lower],
                upper: vec![upper],
                rhs,
            });
            for result in [dense, path] {
                if ratio < 1.0 {
                    assert!(result.unwrap_err().contains("singular"));
                } else {
                    let values = result.unwrap();
                    assert_eq!(values[0].re, 0.0);
                    assert_eq!(values[1].re, 1.0);
                }
            }
        }
    }
}

#[test]
fn harmonic_normalization_precision_prefers_exact_binary_scaling_with_explicit_weights() {
    let values = [
        Complex::real(3.0),
        Complex {
            re: -0.75,
            im: 0.125,
        },
    ];
    let normalization = RowNormalization::choose(3.0, values.into_iter()).unwrap();
    assert_eq!(normalization.divisor, 2.0);
    assert_eq!(normalization.weight, 1.5);
    for value in values {
        let scaled = value.scaled(normalization.divisor).unwrap();
        assert_eq!(scaled.re * normalization.divisor, value.re);
        assert_eq!(scaled.im * normalization.divisor, value.im);
    }
    assert_eq!(pivot_score(Complex::real(3.0), normalization.weight), 2.0);
}

#[test]
fn harmonic_normalization_precision_preserves_legacy_reversible_subnormal_couplings() {
    let quantum = f64::from_bits(1);
    let values = [
        Complex::real(5.0),
        Complex {
            re: 5.0 * quantum,
            im: -5.0 * quantum,
        },
    ];
    let normalization = RowNormalization::choose(5.0, values.into_iter()).unwrap();
    assert_eq!(normalization.divisor, 5.0);
    assert_eq!(normalization.weight, 1.0);
    let value = values[1].scaled(normalization.divisor).unwrap();
    assert_eq!(value.re, quantum);
    assert_eq!(value.im, -quantum);
}

#[test]
fn harmonic_normalization_precision_invalid_maxima_are_rejected() {
    for maximum in [0.0, -0.0, -1.0, f64::NAN, f64::INFINITY] {
        let error = RowNormalization::choose(maximum, std::iter::once(Complex::real(1.0)))
            .err()
            .unwrap();
        assert!(error.contains("finite positive maximum"), "{error}");
    }
}

#[test]
fn harmonic_normalization_precision_preflight_is_bounded_and_cleanly_reusable() {
    use crate::solver_control::{SolverControl, with_solver_observer};
    let values = vec![Complex { re: 3.0, im: 0.75 }; 2050];
    for completed in [64, 1024, 2050] {
        let control = SolverControl::default();
        let cancel = control.clone();
        let error = with_solver_observer(
            &control,
            move |point| {
                if point.stage == SolverStage::LinearPrepare && point.completed_steps == completed {
                    cancel.request_cancel();
                }
            },
            || RowNormalization::choose(3.0, values.iter().copied()),
        )
        .err()
        .unwrap();
        assert!(
            control.was_interrupted() && error.contains("linear_prepare"),
            "{error}"
        );
        let normalization = RowNormalization::choose(3.0, values.iter().copied()).unwrap();
        assert_eq!(normalization.divisor, 2.0);
        assert_eq!(normalization.weight, 1.5);
    }
}

#[test]
fn harmonic_normalization_precision_extreme_binary_scales_preserve_single_unknowns() {
    for scale in [f64::from_bits(1), f64::MIN_POSITIVE, 1.0, f64::MAX] {
        for sign in [-1.0, 1.0] {
            let coefficient = Complex {
                re: scale,
                im: scale,
            };
            let rhs = Complex {
                re: sign * scale,
                im: sign * scale,
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
                assert_eq!(value.re, sign);
                assert_eq!(value.im, 0.0);
            }
        }
    }
}

#[test]
fn harmonic_normalization_precision_phase_rows_keep_weights_through_adjacent_pivots() {
    let expected = [1.0_f64, -2.0, 3.0, -4.0, 5.0];
    let diagonal = [1.0, 3.0, 5.0, 7.0, 11.0];
    let lower = [5.0, 7.0, 3.0, 5.0];
    let upper = [3.0, 5.0, 11.0, 7.0];
    let phases = [(1.0, 0.0), (1.0, 0.5), (0.0, 1.0), (0.5, -1.0), (1.0, -1.0)];
    let mut matrix = vec![vec![Complex::default(); 5]; 5];
    for (row, values) in matrix.iter_mut().enumerate() {
        let coefficient = |value| Complex {
            re: phases[row].0 * value,
            im: phases[row].1 * value,
        };
        values[row] = coefficient(diagonal[row]);
        if row > 0 {
            values[row - 1] = coefficient(lower[row - 1]);
        }
        if row < 4 {
            values[row + 1] = coefficient(upper[row]);
        }
    }
    // Integer and half-integer products/sums here are exact, independent of preparation.
    let rhs = matrix
        .iter()
        .map(|row| Complex {
            re: row
                .iter()
                .zip(expected)
                .map(|(value, x)| value.re * x)
                .sum(),
            im: row
                .iter()
                .zip(expected)
                .map(|(value, x)| value.im * x)
                .sum(),
        })
        .collect::<Vec<_>>();
    let path = solve_complex_tridiagonal(ComplexTridiagonalSystem {
        diagonal: (0..5).map(|row| matrix[row][row]).collect(),
        lower: (0..4).map(|row| matrix[row + 1][row]).collect(),
        upper: (0..4).map(|row| matrix[row][row + 1]).collect(),
        rhs: rhs.clone(),
    });
    let dense = solve_complex_system(matrix, rhs);
    for result in [dense, path] {
        for (actual, expected) in result.unwrap().into_iter().zip(expected) {
            assert!(
                (actual.re - expected).abs() < 1e-12 * expected.abs().max(1.0),
                "{actual:?}"
            );
            assert!(actual.im.abs() < 1e-12, "{actual:?}");
        }
    }
}
