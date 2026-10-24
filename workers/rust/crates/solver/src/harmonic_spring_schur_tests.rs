use super::*;

#[test]
fn harmonic_schur_update_weak_coefficient_retains_the_square_product_tail() {
    let d = 2.0_f64.powi(-27);
    let w = 2.0_f64.powi(-20);
    let result = matrix_difference(
        Complex::real(1.0 - 2.0 * d + w),
        Complex::real(1.0 - d),
        Complex::real(1.0 - d),
        SolverStage::DenseFactor,
    )
    .unwrap();
    assert_eq!(result.re.to_bits(), (w - d * d).to_bits());
}

#[test]
fn harmonic_schur_update_both_factorizations_preserve_the_soft_mode_with_row_swaps() {
    let d = 2.0_f64.powi(-27);
    let w = 2.0_f64.powi(-20);
    let b = 1.0 - d;
    let diagonal = 1.0 - 2.0 * d + w;
    for (first, second) in [
        (Complex::real(1.0), Complex::real(1.0)),
        (Complex { re: 0.0, im: 1.0 }, Complex::real(1.0)),
        (Complex { re: 1.0, im: 1.0 }, Complex { re: 0.0, im: 1.0 }),
    ] {
        for power in [-300, 0, 300] {
            let scale = 2.0_f64.powi(power);
            let phase = |value: f64, p: Complex| Complex {
                re: value * scale * p.re,
                im: value * scale * p.im,
            };
            for swapped in [false, true] {
                let mut matrix = vec![
                    vec![phase(1.0, first), phase(-b, first)],
                    vec![phase(-b, second), phase(diagonal, second)],
                ];
                let mut rhs = vec![Complex::default(), phase(w, second)];
                if swapped {
                    matrix.swap(0, 1);
                    rhs.swap(0, 1);
                }
                let path = ComplexTridiagonalSystem {
                    diagonal: vec![matrix[0][0], matrix[1][1]],
                    lower: vec![matrix[1][0]],
                    upper: vec![matrix[0][1]],
                    rhs: rhs.clone(),
                };
                for result in [
                    solve_complex_system(matrix, rhs),
                    solve_complex_tridiagonal(path),
                ] {
                    let result = result.unwrap();
                    let expected = w / (w - d * d);
                    assert!(
                        (result[1].re / expected - 1.0).abs() <= 4.0 * f64::EPSILON,
                        "{result:?} != {expected}"
                    );
                    assert!((result[0].re / (b * expected) - 1.0).abs() <= 4.0 * f64::EPSILON);
                    assert!(
                        result
                            .iter()
                            .all(|x| x.im.abs() <= 4.0 * f64::EPSILON * expected)
                    );
                }
            }
        }
    }
}

#[test]
fn harmonic_schur_update_complex_partial_loss_uses_complete_expression_replay() {
    let d = 2.0_f64.powi(-27);
    let w = 2.0_f64.powi(-20);
    let result = matrix_difference(
        Complex {
            re: 1.0 + w,
            im: 1.0,
        },
        Complex { re: d, im: -1.0 },
        Complex {
            re: 2.0 * d,
            im: 1.0,
        },
        SolverStage::DenseFactor,
    )
    .unwrap();
    assert_eq!(result.re.to_bits(), (w - 2.0 * d * d).to_bits());
    assert_eq!(result.im, 1.0 + d);
}

#[test]
fn harmonic_schur_update_rejects_range_loss_but_keeps_exact_quantized_coefficients() {
    let stage = SolverStage::DenseFactor;
    let q = f64::from_bits(1);
    let coefficient = matrix_difference(
        Complex::real(3.0 * q),
        Complex::real(0.5),
        Complex::real(2.0 * q),
        stage,
    )
    .unwrap();
    assert_eq!(coefficient.re.to_bits(), 2);
    assert!(
        matrix_difference(
            Complex::real(3.0 * q),
            Complex::real(0.5),
            Complex::real(q),
            stage
        )
        .is_err()
    );
    assert!(
        matrix_difference(
            Complex::real(q),
            Complex::real(0.5),
            Complex::real(q),
            stage
        )
        .is_err()
    );
    assert_eq!(
        matrix_difference(
            Complex::real(q),
            Complex::real(1.0),
            Complex::real(q),
            stage
        )
        .unwrap()
        .re,
        0.0
    );
    assert!(
        matrix_difference(
            Complex::real(f64::MAX),
            Complex::real(-1.0),
            Complex::real(f64::MAX),
            stage
        )
        .is_err()
    );
    let recovered = matrix_difference(
        Complex::real(1.5e308),
        Complex { re: 0.7, im: 0.7 },
        Complex {
            re: 1.5e308,
            im: -1.5e308,
        },
        stage,
    )
    .unwrap();
    assert!((recovered.re / (-6e307) - 1.0).abs() < 1e-14);
    assert_eq!(recovered.im, 0.0);
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(
            matrix_difference(
                Complex::real(1.0),
                Complex::default(),
                Complex::real(bad),
                stage
            )
            .is_err()
        );
    }
}

#[test]
fn harmonic_schur_update_replay_cancels_at_each_component_phase_then_reuses_inputs() {
    use crate::solver_control::{SolverControl, with_solver_observer};
    use std::cell::Cell;
    let d = 2.0_f64.powi(-27);
    let w = 2.0_f64.powi(-20);
    let run = || {
        matrix_difference(
            Complex {
                re: 1.0 + w,
                im: 1.0,
            },
            Complex { re: d, im: -1.0 },
            Complex {
                re: 2.0 * d,
                im: 1.0,
            },
            SolverStage::DenseFactor,
        )
    };
    let baseline = run().unwrap();
    for (completed, occurrence) in [(0, 1), (0, 2), (3, 1), (6, 1)] {
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
            run,
        )
        .unwrap_err();
        assert!(
            control.was_interrupted() && error.contains("cancelled"),
            "{error}"
        );
        let replay = run().unwrap();
        assert_eq!(
            [replay.re.to_bits(), replay.im.to_bits()],
            [baseline.re.to_bits(), baseline.im.to_bits()]
        );
    }
}

#[test]
fn harmonic_schur_update_preserves_weighted_pivot_floor_admission_with_and_without_swaps() {
    let d = 2.0_f64.powi(-27);
    let b = 1.0 - d;
    let floor = 64.0 * f64::EPSILON;
    for (factor, admitted) in [(0.9, false), (1.1, true)] {
        for swapped in [false, true] {
            let mut matrix = vec![
                vec![Complex::real(1.0), Complex::real(-b)],
                vec![Complex::real(-b), Complex::real(b * b + factor * floor)],
            ];
            let mut rhs = vec![Complex::default(), Complex::real(factor * floor)];
            if swapped {
                matrix.swap(0, 1);
                rhs.swap(0, 1);
            }
            let path = ComplexTridiagonalSystem {
                diagonal: vec![matrix[0][0], matrix[1][1]],
                lower: vec![matrix[1][0]],
                upper: vec![matrix[0][1]],
                rhs: rhs.clone(),
            };
            for result in [
                solve_complex_system(matrix, rhs),
                solve_complex_tridiagonal(path),
            ] {
                if admitted {
                    assert!(result.is_ok(), "{result:?}");
                } else {
                    assert!(result.unwrap_err().contains("singular"));
                }
            }
        }
    }
}

#[test]
fn harmonic_schur_update_adjacent_swap_preserves_the_generated_second_superdiagonal() {
    let d = 2.0_f64.powi(-27);
    let w = 2.0_f64.powi(-20);
    let b = 1.0 - d;
    let z = Complex::default();
    let matrix = vec![
        vec![Complex::real(-b), Complex::real(1.0 - 2.0 * d + w), z],
        vec![Complex::real(1.0), Complex::real(-b), Complex::real(0.25)],
        vec![z, z, Complex::real(1.0)],
    ];
    let rhs = vec![Complex::real(w), Complex::real(0.25), Complex::real(1.0)];
    let path = ComplexTridiagonalSystem {
        diagonal: vec![matrix[0][0], matrix[1][1], matrix[2][2]],
        lower: vec![matrix[1][0], z],
        upper: vec![matrix[0][1], matrix[1][2]],
        rhs: rhs.clone(),
    };
    for result in [
        solve_complex_system(matrix, rhs),
        solve_complex_tridiagonal(path),
    ] {
        let result = result.unwrap();
        let expected = w / (w - d * d);
        assert!(
            (result[1].re / expected - 1.0).abs() <= 4.0 * f64::EPSILON,
            "{result:?}"
        );
        assert!((result[0].re / (b * expected) - 1.0).abs() <= 4.0 * f64::EPSILON);
        assert_eq!(result[2].re, 1.0);
    }
}
