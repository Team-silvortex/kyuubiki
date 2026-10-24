use super::super::super::ExactSum;
use super::*;

#[test]
fn harmonic_row_quotient_integer_multiply_and_signed_alignment_check_capacity() {
    let mut one = Magnitude([0; LIMBS]);
    one.0[0] = 1;
    let product = one.multiplied(u64::MAX).unwrap();
    assert_eq!(product.0[0], u64::MAX);
    assert_eq!(product.bits(), 64);
    let shifted = product.shifted(64).unwrap().multiplied(3).unwrap();
    assert_eq!(&shifted.0[..3], &[0, u64::MAX - 2, 2]);
    assert_eq!(one.multiplied(0).unwrap().bits(), 0);
    assert!(Magnitude([u64::MAX; LIMBS]).multiplied(2).is_err());
    let pair = |power, negative| Pair {
        magnitude: one.clone(),
        power,
        negative,
    };
    let sum = pair(1, false).combined(pair(0, true)).unwrap();
    assert_eq!(sum.magnitude.0[0], 1);
    assert!(!sum.negative);
    let negative = pair(0, false).combined(pair(1, true)).unwrap();
    assert_eq!(negative.magnitude.0[0], 1);
    assert!(negative.negative);
    assert_eq!(
        pair(0, false)
            .combined(pair(0, true))
            .unwrap()
            .magnitude
            .bits(),
        0
    );
    assert!(
        pair(0, false)
            .combined(pair((LIMBS * 64) as i64, false))
            .is_err()
    );
    let stage = SolverStage::DenseSubstitutionRange;
    assert!(pair(0, false).multiplied(f64::NAN, stage).is_err());
    assert!(pair(0, false).multiplied(f64::INFINITY, stage).is_err());
    assert!(std::mem::size_of::<Pair>() <= 576);
}

#[test]
fn harmonic_row_quotient_accumulation_handoff_preserves_unrounded_parts_and_poisoning() {
    let mut sum = ExactSum::default();
    sum.add(1.0).unwrap();
    sum.add(2.0_f64.powi(-54)).unwrap();
    let actual = divide_accumulated(
        (sum, 0),
        (ExactSum::default(), 0),
        Complex::real(3.0),
        SolverStage::DenseSubstitutionRange,
    )
    .unwrap();
    // (1+2^-54)/3 rounds upward; rounding its numerator first would round down.
    assert_eq!(actual.re.to_bits(), (1.0_f64 / 3.0).next_up().to_bits());
    assert_eq!(actual.im, 0.0);
    let mut poisoned = ExactSum::default();
    assert!(poisoned.add(f64::NAN).is_err());
    assert!(
        divide_accumulated(
            (poisoned, 0),
            (ExactSum::default(), 0),
            Complex::real(1.0),
            SolverStage::DenseSubstitutionRange
        )
        .is_err()
    );
}
