use super::*;

fn nested_row(scale: f64, sign: f64, padded: bool) -> Result<Complex, String> {
    let phase = |value| Complex {
        re: sign * value,
        im: -sign * value,
    };
    let large = scale * 2.0_f64.powi(60);
    let tiny = scale * 2.0_f64.powi(-60);
    let mut terms = [scale, tiny, -large, -scale]
        .map(|value| (Complex::real(-1.0), phase(value)))
        .to_vec();
    if padded {
        terms.resize(2050, (Complex::default(), Complex::default()));
    }
    solve_row(
        phase(large),
        terms.into_iter(),
        Complex::real(1.0),
        SolverStage::DenseSubstitutionRange,
    )
}

#[test]
fn harmonic_complex_accumulation_replay_keeps_nested_sum_tails() {
    for exponent in [-500, 0, 500] {
        let scale = 2.0_f64.powi(exponent);
        for sign in [-1.0, 1.0] {
            let value = nested_row(scale, sign, false).unwrap();
            let expected = sign * scale * 2.0_f64.powi(-60);
            assert_eq!(value.re, expected);
            assert_eq!(value.im, -expected);
        }
    }
}

#[test]
fn harmonic_complex_accumulation_replay_keeps_nested_fma_product_tails() {
    let large = 2.0_f64.powi(60);
    let delta = 2.0_f64.powi(-27);
    let tiny = 2.0_f64.powi(-100);
    let value = solve_row(
        Complex::real(large),
        [
            (Complex::real(-(1.0 + delta)), Complex::real(1.0 - delta)),
            (Complex::real(-1.0), Complex::real(tiny)),
            (Complex::real(1.0), Complex::real(large)),
            (Complex::real(1.0), Complex::real(1.0)),
            (Complex::real(-1.0), Complex::real(delta * delta)),
        ]
        .into_iter(),
        Complex::real(1.0),
        SolverStage::DenseSubstitutionRange,
    )
    .unwrap();
    assert_eq!(value.re, tiny);
    assert_eq!(value.im, 0.0);
}

#[test]
fn harmonic_complex_accumulation_two_complex_terms_preserve_independent_components() {
    let large = 2.0_f64.powi(60);
    let tiny = 2.0_f64.powi(-60);
    let value = solve_row(
        Complex::real(large),
        [
            (Complex { re: -1.0, im: 1.0 }, Complex { re: 1.0, im: tiny }),
            (
                Complex {
                    re: large,
                    im: -1.0,
                },
                Complex { re: 1.0, im: 1.0 },
            ),
        ]
        .into_iter(),
        Complex::real(1.0),
        SolverStage::TridiagonalSubstitution,
    )
    .unwrap();
    assert_eq!(value.re, tiny);
    assert_eq!(value.im, -large);
}

#[test]
fn harmonic_complex_accumulation_dense_back_substitution_preserves_nested_tails() {
    let zero = Complex::default();
    let one = Complex::real(1.0);
    let large = 2.0_f64.powi(60);
    let tiny = 2.0_f64.powi(-60);
    let mut matrix = vec![vec![zero; 6]; 6];
    for (index, row) in matrix.iter_mut().enumerate() {
        row[index] = one;
    }
    matrix[0].fill(one);
    let values = solve_complex_system(
        matrix,
        [0.0, large, 1.0, tiny, -large, -1.0]
            .map(Complex::real)
            .to_vec(),
    )
    .unwrap();
    assert_eq!(values[0].re, -tiny);
    assert!(values.iter().all(|value| value.im == 0.0));
}

#[test]
fn harmonic_complex_accumulation_integer_replay_cancels_then_reuses_inputs() {
    use crate::solver_control::{SolverControl, with_solver_observer};
    use std::cell::Cell;
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
            || nested_row(1.0, 1.0, true),
        )
        .unwrap_err();
        assert!(
            control.was_interrupted() && error.contains("cancelled"),
            "{error}"
        );
        let value = nested_row(1.0, 1.0, true).unwrap();
        assert_eq!(value.re, 2.0_f64.powi(-60));
        assert_eq!(value.im, -2.0_f64.powi(-60));
    }
}
