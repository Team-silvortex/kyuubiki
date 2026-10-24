use super::*;

#[test]
fn harmonic_coefficient_range_erased_upper_coupling_cannot_return_a_wrong_candidate() {
    let zero = Complex::default();
    let diagonal = vec![Complex::real(1e308), Complex::real(1.0)];
    let rhs = vec![zero, Complex::real(1e308)];
    for weak in [
        Complex::real(1e-308),
        Complex {
            re: 0.0,
            im: 1e-308,
        },
    ] {
        let dense = solve_complex_system(
            vec![vec![diagonal[0], weak], vec![zero, diagonal[1]]],
            rhs.clone(),
        );
        let path = solve_complex_tridiagonal(ComplexTridiagonalSystem {
            diagonal: diagonal.clone(),
            lower: vec![zero],
            upper: vec![weak],
            rhs: rhs.clone(),
        });
        for result in [dense, path] {
            let error = result
                .expect_err("normalization erased a coupling and returned the wrong zero unknown");
            assert!(
                error.contains("row 0")
                    && error.contains("column 1")
                    && error.contains("numerical range"),
                "{error}"
            );
        }
    }
}

#[test]
fn harmonic_coefficient_range_partial_subnormal_rounding_is_not_admitted_as_a_new_matrix() {
    let quantum = f64::from_bits(1);
    let zero = Complex::default();
    for weak in [
        Complex::real(5.0 * quantum),
        Complex {
            re: 0.0,
            im: 5.0 * quantum,
        },
    ] {
        let dense = solve_complex_system(
            vec![
                vec![Complex::real(2.0), weak],
                vec![zero, Complex::real(1.0)],
            ],
            vec![zero, zero],
        );
        let path = solve_complex_tridiagonal(ComplexTridiagonalSystem {
            diagonal: vec![Complex::real(2.0), Complex::real(1.0)],
            lower: vec![zero],
            upper: vec![weak],
            rhs: vec![zero, zero],
        });
        for result in [dense, path] {
            let error = result
                .expect_err("five quanta became four after row normalization and restoration");
            assert!(
                error.contains("row 0")
                    && error.contains("column 1")
                    && error.contains("numerical range"),
                "{error}"
            );
        }
    }
}

#[test]
fn harmonic_coefficient_range_diagonal_lower_and_upper_errors_keep_component_identity() {
    let quantum = f64::from_bits(1);
    let zero = Complex::default();
    for (row, column, weak, component) in [
        (
            0,
            0,
            Complex {
                re: 2.0,
                im: quantum,
            },
            "imaginary",
        ),
        (0, 1, Complex::real(5.0 * quantum), "real"),
        (
            1,
            0,
            Complex {
                re: 0.0,
                im: 5.0 * quantum,
            },
            "imaginary",
        ),
    ] {
        let mut matrix = vec![
            vec![Complex::real(2.0), zero],
            vec![zero, Complex::real(2.0)],
        ];
        matrix[row][column] = weak;
        let path = solve_complex_tridiagonal(ComplexTridiagonalSystem {
            diagonal: vec![matrix[0][0], matrix[1][1]],
            lower: vec![matrix[1][0]],
            upper: vec![matrix[0][1]],
            rhs: vec![zero, zero],
        });
        let dense = solve_complex_system(matrix, vec![zero, zero]);
        for result in [dense, path] {
            let error = result.unwrap_err();
            assert!(
                error.contains(&format!("row {row} column {column}"))
                    && error.contains(component)
                    && error.contains("normalization would lose numerical range"),
                "{error}"
            );
        }
    }
}

#[test]
fn harmonic_coefficient_range_exact_subnormal_couplings_remain_supported() {
    let quantum = f64::from_bits(1);
    let zero = Complex::default();
    for (scale, multiple) in [(2.0, 4.0), (4.0, 4.0), (5.0, 5.0)] {
        for sign in [-1.0, 1.0] {
            let weak = Complex {
                re: sign * multiple * quantum,
                im: -sign * multiple * quantum,
            };
            let normalized = weak.scaled(scale).unwrap();
            assert_eq!(normalized.re * scale, weak.re);
            assert_eq!(normalized.im * scale, weak.im);
            assert!(normalized.re != 0.0 && normalized.im != 0.0);
            let dense = solve_complex_system(
                vec![
                    vec![Complex::real(scale), weak],
                    vec![zero, Complex::real(1.0)],
                ],
                vec![zero, Complex::real(1.0)],
            )
            .unwrap();
            let path = solve_complex_tridiagonal(ComplexTridiagonalSystem {
                diagonal: vec![Complex::real(scale), Complex::real(1.0)],
                lower: vec![zero],
                upper: vec![weak],
                rhs: vec![zero, Complex::real(1.0)],
            })
            .unwrap();
            for values in [dense, path] {
                assert_eq!(values[0].re.to_bits(), (-normalized.re).to_bits());
                assert_eq!(values[0].im.to_bits(), (-normalized.im).to_bits());
                assert_eq!(values[1].re, 1.0);
                assert_eq!(values[1].im, 0.0);
            }
        }
    }
}

#[test]
fn harmonic_coefficient_range_normal_division_keeps_existing_bits_and_signed_zeros() {
    for value in [
        Complex {
            re: 0.75,
            im: -0.125,
        },
        Complex { re: -0.0, im: 0.0 },
    ] {
        for scale in [0.5, 1.0, 1.25, 3.0] {
            let normalized = value.scaled(scale).unwrap();
            assert_eq!(normalized.re.to_bits(), (value.re / scale).to_bits());
            assert_eq!(normalized.im.to_bits(), (value.im / scale).to_bits());
        }
    }
}

#[test]
fn harmonic_coefficient_range_invalid_scales_and_nonfinite_components_remain_errors() {
    for scale in [0.0, -0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(Complex::real(1.0).scaled(scale).is_err());
    }
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        for value in [Complex { re: bad, im: 0.0 }, Complex { re: 0.0, im: bad }] {
            assert!(value.scaled(1.0).is_err());
        }
    }
}

#[test]
fn harmonic_coefficient_range_preparation_cancels_before_late_loss_and_clean_repair_replays() {
    use crate::solver_control::{SolverControl, with_solver_observer};
    use std::{cell::Cell, rc::Rc};
    for dense in [false, true] {
        let size = if dense { 32 } else { 2050 };
        let bad = size - 2;
        let run = |repaired| {
            let mut upper = vec![Complex::real(0.25); size - 1];
            upper[bad] = Complex::real(f64::from_bits(if repaired { 4 } else { 5 }));
            if dense {
                let mut matrix = vec![vec![Complex::default(); size]; size];
                for row in 0..size {
                    matrix[row][row] = Complex::real(2.0);
                    if row + 1 < size {
                        matrix[row][row + 1] = upper[row];
                    }
                }
                solve_complex_system(matrix, vec![Complex::default(); size])
            } else {
                solve_complex_tridiagonal(ComplexTridiagonalSystem {
                    diagonal: vec![Complex::real(2.0); size],
                    lower: vec![Complex::default(); size - 1],
                    upper,
                    rhs: vec![Complex::default(); size],
                })
            }
        };
        let expected_error = run(false).unwrap_err();
        let control = SolverControl::default();
        let cancel = control.clone();
        let observed = Rc::new(Cell::new(0));
        let counts = Rc::clone(&observed);
        let error = with_solver_observer(
            &control,
            move |point| {
                let completed = if dense { 8 } else { 1024 };
                if point.stage == SolverStage::LinearPrepare && point.completed_steps == completed {
                    counts.set(counts.get() + 1);
                    if counts.get() == if dense { 1 } else { 2 } {
                        cancel.request_cancel();
                    }
                }
            },
            || run(false),
        )
        .unwrap_err();
        assert!(
            control.was_interrupted() && observed.get() > 0 && error.contains("linear_prepare"),
            "{error}"
        );
        assert_eq!(run(false).unwrap_err(), expected_error);
        assert_eq!(run(true).unwrap().len(), size);
    }
}
