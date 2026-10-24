use super::*;
use crate::solver_control::{SolverControl, with_solver_observer};
use std::{cell::Cell, rc::Rc};

#[test]
fn harmonic_complex_rhs_range_ordinary_storage_and_arithmetic_keep_the_original_allocation() {
    let input = vec![Complex { re: 3.0, im: -0.5 }, Complex::real(2.0)];
    let pointer = input.as_ptr();
    let mut rhs = ComplexRhs::new(input);
    rhs.normalize(0, 2.0).unwrap();
    let updated = rhs
        .get(0)
        .subtract_product(Complex::real(0.25), rhs.get(1), SolverStage::DenseFactor)
        .unwrap();
    rhs.set(0, updated, SolverStage::DenseFactor).unwrap();
    assert!(matches!(rhs, ComplexRhs::Finite(_)));
    let values = rhs.into_solutions().unwrap();
    assert_eq!(values.as_ptr(), pointer);
    assert_eq!(values[0].re.to_bits(), 1.0_f64.to_bits());
    assert_eq!(values[0].im.to_bits(), (-0.25_f64).to_bits());
}

#[test]
fn harmonic_complex_rhs_range_promotion_preserves_other_rows_and_swaps() {
    let quantum = f64::from_bits(1);
    let mut rhs = ComplexRhs::new(vec![
        Complex::real(2.0),
        Complex {
            re: quantum,
            im: -quantum,
        },
    ]);
    rhs.normalize(0, 1e-308).unwrap();
    assert!(matches!(rhs, ComplexRhs::Wide(_)));
    rhs.swap(0, 1);
    let small = rhs.solution(0).unwrap();
    assert_eq!(
        (small.re.to_bits(), small.im.to_bits()),
        (quantum.to_bits(), (-quantum).to_bits())
    );
    assert!(rhs.solution(1).is_err());
    let solved = rhs
        .get(1)
        .divide(
            Complex { re: 1.0, im: 1.0 },
            SolverStage::DenseSubstitutionRange,
        )
        .unwrap();
    rhs.set(1, solved.into(), SolverStage::DenseSubstitutionRange)
        .unwrap();
    assert!(rhs.into_solutions().is_ok());
}

#[test]
fn harmonic_complex_rhs_range_subquantum_temporaries_survive_until_division() {
    let quantum = f64::from_bits(1);
    let original = WideComplex::from(Complex::real(quantum));
    for value in [
        original.normalized(4.0).unwrap(),
        original
            .subtract_product(Complex::real(0.75), original, SolverStage::DenseFactor)
            .unwrap(),
    ] {
        assert!(value.finite().is_none());
        let solved = value
            .divide(Complex::real(0.25), SolverStage::DenseSubstitutionRange)
            .unwrap();
        assert_eq!(solved.re, quantum);
        assert_eq!(solved.im, 0.0);
    }
}

#[test]
fn harmonic_complex_rhs_range_invalid_normalization_and_true_solution_overflow_stay_errors() {
    let rhs = WideComplex::from(Complex::real(f64::MAX));
    for scale in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(rhs.normalized(scale).is_err());
    }
    assert!(
        WideComplex::from(Complex::real(f64::INFINITY))
            .normalized(1.0)
            .is_err()
    );
    let wide = rhs.normalized(0.5).unwrap();
    let error = wide
        .divide(Complex::real(1.0), SolverStage::DenseSubstitutionRange)
        .unwrap_err();
    assert!(error.contains("representable range"), "{error}");
}

#[test]
fn harmonic_complex_rhs_range_promotion_and_solution_collection_cancel_then_replay() {
    for stage in [
        SolverStage::LinearPrepare,
        SolverStage::TridiagonalSubstitution,
    ] {
        for completed in [1024, 2050] {
            let control = SolverControl::default();
            let cancel = control.clone();
            let observed = Rc::new(Cell::new(false));
            let seen = Rc::clone(&observed);
            let run = || {
                let mut rhs = ComplexRhs::new(vec![Complex::real(2.0); 2050]);
                rhs.normalize(0, 1e-308)?;
                let solved = rhs.get(0).divide(
                    Complex { re: 1.0, im: 1.0 },
                    SolverStage::DenseSubstitutionRange,
                )?;
                rhs.set(0, solved.into(), SolverStage::DenseSubstitutionRange)?;
                rhs.into_solutions()
            };
            let error = with_solver_observer(
                &control,
                move |point| {
                    if point.stage == stage && point.completed_steps == completed {
                        seen.set(true);
                        cancel.request_cancel();
                    }
                },
                run,
            )
            .unwrap_err();
            assert!(
                observed.get() && control.was_interrupted() && error.contains(stage.as_str()),
                "{error}"
            );
            assert_eq!(run().unwrap().len(), 2050);
        }
    }
}

fn promotion_source(size: usize) -> Vec<Complex> {
    (0..size)
        .map(|i| Complex {
            re: if i == 2 {
                f64::from_bits(1)
            } else {
                i as f64 + 2.0
            },
            im: if i == 1 { -0.0 } else { -(i as f64 + 0.5) },
        })
        .collect()
}

fn assert_finite_source(rhs: &ComplexRhs, expected: &[Complex], pointer: *const Complex) {
    let ComplexRhs::Finite(values) = rhs else {
        panic!("cancelled promotion changed representation");
    };
    assert_eq!(
        values.len(),
        expected.len(),
        "cancelled promotion lost rows"
    );
    assert_eq!(
        values.as_ptr(),
        pointer,
        "cancelled promotion replaced allocation"
    );
    for (actual, expected) in values.iter().zip(expected) {
        assert_eq!(
            [actual.re.to_bits(), actual.im.to_bits()],
            [expected.re.to_bits(), expected.im.to_bits()]
        );
    }
}

fn assert_same_rhs(actual: &ComplexRhs, expected: &ComplexRhs, size: usize) {
    for row in 0..size {
        let a = actual.get(row);
        let b = expected.get(row);
        let (a, b) = match (a.finite(), b.finite()) {
            (Some(a), Some(b)) => (a, b),
            (None, None) => (
                a.divide(Complex::real(1e308), SolverStage::DenseSubstitutionRange)
                    .unwrap(),
                b.divide(Complex::real(1e308), SolverStage::DenseSubstitutionRange)
                    .unwrap(),
            ),
            _ => panic!("RHS representation changed numerical value at row {row}"),
        };
        assert_eq!(
            [a.re.to_bits(), a.im.to_bits()],
            [b.re.to_bits(), b.im.to_bits()]
        );
    }
}

#[test]
fn harmonic_complex_rhs_range_cancelled_promotion_preserves_same_object_and_allocation() {
    for stage in [
        SolverStage::LinearPrepare,
        SolverStage::DenseFactor,
        SolverStage::TridiagonalFactor,
    ] {
        for row in [0, 1025, 2049] {
            for completed in [0, 1024, 2048, 2050] {
                for normalize in [false, true] {
                    let original = promotion_source(2050);
                    let mut rhs = ComplexRhs::new(original.clone());
                    let pointer = match &rhs {
                        ComplexRhs::Finite(v) => v.as_ptr(),
                        _ => unreachable!(),
                    };
                    let incoming = WideComplex::from(Complex::real(2.0))
                        .normalized(1e-308)
                        .unwrap();
                    let run = |rhs: &mut ComplexRhs| {
                        if normalize {
                            rhs.normalize(row, 1e-308)
                        } else {
                            rhs.set(row, incoming, stage)
                        }
                    };
                    let observed_stage = if normalize {
                        SolverStage::LinearPrepare
                    } else {
                        stage
                    };
                    let control = SolverControl::default();
                    let cancel = control.clone();
                    let error = with_solver_observer(
                        &control,
                        move |p| {
                            if p.stage == observed_stage && p.completed_steps == completed {
                                cancel.request_cancel();
                            }
                        },
                        || run(&mut rhs),
                    )
                    .unwrap_err();
                    assert!(control.was_interrupted() && error.contains("cancelled"));
                    assert_finite_source(&rhs, &original, pointer);
                    run(&mut rhs).unwrap();
                    let mut clean = ComplexRhs::new(original);
                    run(&mut clean).unwrap();
                    assert_same_rhs(&rhs, &clean, 2050);
                    rhs.swap(0, row);
                    clean.swap(0, row);
                    assert_same_rhs(&rhs, &clean, 2050);
                }
            }
        }
    }
}

#[test]
fn harmonic_complex_rhs_range_repeated_cancel_and_short_final_boundary_keep_retryable_source() {
    for size in [1, 63, 64, 65, 2050] {
        let original = promotion_source(size);
        let mut rhs = ComplexRhs::new(original.clone());
        let pointer = match &rhs {
            ComplexRhs::Finite(v) => v.as_ptr(),
            _ => unreachable!(),
        };
        for completed in [0, size.min(64), size, size] {
            let control = SolverControl::default();
            let cancel = control.clone();
            let error = with_solver_observer(
                &control,
                move |p| {
                    if p.stage == SolverStage::LinearPrepare
                        && p.completed_steps == completed as u64
                    {
                        cancel.request_cancel();
                    }
                },
                || rhs.normalize(0, 1e-308),
            )
            .unwrap_err();
            assert!(control.was_interrupted() && error.contains("cancelled"));
            assert_finite_source(&rhs, &original, pointer);
        }
        rhs.normalize(0, 1e-308).unwrap();
        let mut clean = ComplexRhs::new(original);
        clean.normalize(0, 1e-308).unwrap();
        assert_same_rhs(&rhs, &clean, size);
    }
}

#[test]
fn harmonic_complex_rhs_range_failed_normalization_preserves_finite_and_wide_rows() {
    let original = promotion_source(65);
    let mut rhs = ComplexRhs::new(original.clone());
    let pointer = match &rhs {
        ComplexRhs::Finite(v) => v.as_ptr(),
        _ => unreachable!(),
    };
    for bad in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(rhs.normalize(0, bad).is_err());
        assert_finite_source(&rhs, &original, pointer);
    }
    rhs.normalize(0, 1e-308).unwrap();
    let mut clean = ComplexRhs::new(original);
    clean.normalize(0, 1e-308).unwrap();
    let wide_pointer = match &rhs {
        ComplexRhs::Wide(v) => v.as_ptr(),
        _ => unreachable!(),
    };
    for bad in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(rhs.normalize(0, bad).is_err());
        assert_same_rhs(&rhs, &clean, 65);
        assert!(matches!(&rhs, ComplexRhs::Wide(v) if v.as_ptr() == wide_pointer));
    }
}
