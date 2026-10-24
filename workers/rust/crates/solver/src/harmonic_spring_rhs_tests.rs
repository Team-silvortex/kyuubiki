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
