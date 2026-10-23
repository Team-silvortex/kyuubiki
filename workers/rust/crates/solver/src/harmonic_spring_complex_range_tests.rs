use super::*;

#[test]
fn harmonic_complex_range_division_combines_two_half_quanta() {
    let quantum = f64::from_bits(1);
    let value = divide(
        Complex {
            re: quantum,
            im: quantum,
        },
        Complex { re: 1.0, im: 1.0 },
        SolverStage::LinearPrepare,
    )
    .unwrap();
    assert_eq!(value.re, quantum);
    assert_eq!(value.im, 0.0);
}

#[test]
fn harmonic_complex_range_product_combines_two_half_quanta() {
    let value = arithmetic::product(
        Complex {
            re: 2.0_f64.powi(-600),
            im: 2.0_f64.powi(-600),
        },
        Complex {
            re: 2.0_f64.powi(-475),
            im: -2.0_f64.powi(-475),
        },
        SolverStage::ResultElements,
    )
    .unwrap();
    assert_eq!(value.re, f64::from_bits(1));
    assert_eq!(value.im, 0.0);
}

#[test]
fn harmonic_complex_range_dense_back_substitution_replays_the_complete_row() {
    for phase in [Complex::real(1.0), Complex { re: 1.0, im: 1.0 }] {
        let zero = Complex::default();
        let matrix = vec![
            vec![phase, phase, phase],
            vec![zero, phase, zero],
            vec![zero, zero, phase],
        ];
        let rhs = [1e308, -1e308, 1e308]
            .map(|value| Complex {
                re: phase.re * value,
                im: phase.im * value,
            })
            .to_vec();
        let result = solve_complex_system(matrix, rhs).unwrap();
        for (actual, expected) in result.iter().zip([1e308, -1e308, 1e308]) {
            assert!((actual.re / expected - 1.0).abs() < 1e-14, "{actual:?}");
            assert!(actual.im.abs() < 1e294, "{actual:?}");
        }
    }
}

#[test]
fn harmonic_complex_range_pivoted_path_replays_both_upper_terms() {
    let one = Complex::real(1.0);
    let zero = Complex::default();
    let result = solve_complex_tridiagonal(ComplexTridiagonalSystem {
        diagonal: vec![zero, one, one],
        lower: vec![one, zero],
        upper: vec![one, one],
        rhs: vec![
            Complex::real(-1e308),
            Complex::real(1e308),
            Complex::real(1e308),
        ],
    })
    .unwrap();
    for (actual, expected) in result.iter().zip([1e308, -1e308, 1e308]) {
        assert!((actual.re / expected - 1.0).abs() < 1e-14, "{actual:?}");
        assert_eq!(actual.im, 0.0);
    }
}

#[test]
fn harmonic_complex_range_update_recovers_canceled_overflowing_products() {
    let value = difference(
        Complex::real(1.5e308),
        Complex { re: 0.7, im: 0.7 },
        Complex {
            re: 1.5e308,
            im: -1.5e308,
        },
        SolverStage::DenseFactor,
    )
    .unwrap();
    assert!((value.re / -6e307 - 1.0).abs() < 1e-14);
    assert_eq!(value.im, 0.0);
}

#[test]
fn harmonic_complex_range_forward_updates_reach_both_factorizations() {
    let one = Complex::real(1.0);
    let zero = Complex::default();
    let lower = Complex { re: 0.7, im: 0.7 };
    let rhs = vec![
        Complex {
            re: 1.5e308,
            im: -1.5e308,
        },
        Complex::real(1.5e308),
    ];
    let dense = solve_complex_system(vec![vec![one, zero], vec![lower, one]], rhs.clone()).unwrap();
    let path = solve_complex_tridiagonal(ComplexTridiagonalSystem {
        diagonal: vec![one, one],
        lower: vec![lower],
        upper: vec![zero],
        rhs,
    })
    .unwrap();
    for result in [dense, path] {
        assert!((result[0].re / 1.5e308 - 1.0).abs() < 1e-14);
        assert!((result[0].im / -1.5e308 - 1.0).abs() < 1e-14);
        assert!((result[1].re / -6e307 - 1.0).abs() < 1e-14);
        assert_eq!(result[1].im, 0.0);
    }
}

#[test]
fn harmonic_complex_range_divides_before_restoring_a_wide_numerator() {
    let value = solve_row(
        Complex {
            re: 1.5e308,
            im: 1.5e308,
        },
        std::iter::once((Complex { re: -0.5, im: -0.5 }, Complex::real(1.5e308))),
        Complex { re: 1.5, im: 1.5 },
        SolverStage::DenseSubstitutionRange,
    )
    .unwrap();
    assert!((value.re / 1.5e308 - 1.0).abs() < 1e-14);
    assert_eq!(value.im, 0.0);
}

#[test]
fn harmonic_complex_range_preserves_ordinary_arithmetic_bits() {
    let a = Complex {
        re: 0.75,
        im: -0.125,
    };
    let x = Complex { re: 1.25, im: 0.5 };
    let base = Complex { re: 2.0, im: 0.25 };
    let expected = Complex {
        re: a.re * x.re - a.im * x.im,
        im: a.re * x.im + a.im * x.re,
    };
    let product = arithmetic::product(a, x, SolverStage::ResultElements).unwrap();
    assert_eq!(
        (product.re.to_bits(), product.im.to_bits()),
        (expected.re.to_bits(), expected.im.to_bits())
    );
    let value = difference(base, a, x, SolverStage::DenseFactor).unwrap();
    assert_eq!(
        (value.re.to_bits(), value.im.to_bits()),
        (
            (base.re - expected.re).to_bits(),
            (base.im - expected.im).to_bits()
        )
    );
    let scale = a.component_scale();
    let ar = a.re / scale;
    let ai = a.im / scale;
    let den = ar * ar + ai * ai;
    let expected = Complex {
        re: (base.re / scale) * (ar / den) + (base.im / scale) * (ai / den),
        im: (base.im / scale) * (ar / den) - (base.re / scale) * (ai / den),
    };
    let value = divide(base, a, SolverStage::DenseFactor).unwrap();
    assert_eq!(
        (value.re.to_bits(), value.im.to_bits()),
        (expected.re.to_bits(), expected.im.to_bits())
    );
}

#[test]
fn harmonic_complex_range_rejects_invalid_operands_and_unrepresentable_results() {
    let stage = SolverStage::DenseSubstitutionRange;
    let zero = Complex::default();
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let bad = Complex { re: 0.0, im: bad };
        assert!(arithmetic::product(zero, bad, stage).is_err());
        assert!(divide(zero, bad, stage).is_err());
        assert!(difference(zero, zero, bad, stage).is_err());
    }
    assert!(divide(Complex::real(1.0), zero, stage).is_err());
    assert!(arithmetic::product(Complex::real(2.0), Complex::real(f64::MAX), stage).is_err());
    assert!(
        arithmetic::product(Complex::real(0.25), Complex::real(f64::from_bits(1)), stage).is_err()
    );
    assert!(divide(Complex::real(f64::from_bits(1)), Complex::real(2.0), stage).is_err());
    assert!(divide(Complex::real(f64::MAX), Complex::real(0.5), stage).is_err());
    let quantized = divide(
        Complex::real(1.0),
        Complex {
            re: 1.3e308,
            im: 1.3e308,
        },
        stage,
    )
    .unwrap();
    assert!(quantized.re.is_subnormal() && quantized.im.is_subnormal());
}

#[test]
fn harmonic_complex_range_wide_replay_and_division_cancel_then_replay() {
    use crate::solver_control::{SolverControl, with_solver_observer};
    use std::cell::Cell;
    let terms = (0..2050)
        .map(|index| {
            (
                Complex::real(1.0),
                Complex::real(if index % 2 == 0 { -1e308 } else { 1e308 }),
            )
        })
        .collect::<Vec<_>>();
    let stage = SolverStage::DenseSubstitutionRange;
    let run = || {
        solve_row(
            Complex::real(1e308),
            terms.iter().copied(),
            Complex::real(1.0),
            stage,
        )
    };
    let baseline = run().unwrap();
    assert_eq!(baseline.re, 1e308);
    for (completed, phase) in [
        (0, 1),
        (1024, 0),
        (4101, 0),
        (8202, 0),
        (0, 2),
        (0, 3),
        (0, 4),
        (0, 5),
    ] {
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
        let replay = run().unwrap();
        assert_eq!(
            (replay.re.to_bits(), replay.im.to_bits()),
            (baseline.re.to_bits(), baseline.im.to_bits())
        );
    }
}
