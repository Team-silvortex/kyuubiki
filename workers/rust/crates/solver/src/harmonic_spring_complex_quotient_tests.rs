use super::*;

#[test]
fn harmonic_general_division_integer_storage_checks_shift_carry_borrow_and_operand_bounds() {
    let mut one = Magnitude([0; LIMBS]);
    one.0[0] = 1;
    for shift in [0, 1, 63, 64, 65, 127, CAPACITY - 1] {
        let actual = one.shifted(shift as i64).unwrap();
        assert_eq!(actual.bits(), shift + 1);
        assert_eq!(actual.0[shift / 64], 1 << (shift % 64));
        assert_eq!(actual.0.iter().filter(|&&word| word != 0).count(), 1);
    }
    assert!(one.shifted(CAPACITY as i64).is_err());
    assert!(one.shifted(-1).is_err());
    let mut full = Magnitude([u64::MAX; LIMBS]);
    assert!(full.add(&one).is_err());
    let mut zero = Magnitude([0; LIMBS]);
    assert!(zero.subtract(&one).is_err());
    let mut boundary = one.shifted(128).unwrap();
    boundary.subtract(&one).unwrap();
    assert_eq!(&boundary.0[..3], &[u64::MAX, u64::MAX, 0]);
    boundary.add(&one).unwrap();
    assert_eq!(&boundary.0[..3], &[0, 0, 1]);

    let stage = SolverStage::DenseSubstitutionRange;
    let q = f64::from_bits(1);
    let span = Pair::products([(f64::MAX, f64::MAX, 0), (q, q, 0)], stage).unwrap();
    assert_eq!(span.magnitude.bits(), 4196);
    assert_eq!(span.magnitude.0[0], 1);
    assert!(!span.negative);
    let sum = Pair::products([(f64::MAX, f64::MAX, 0); 2], stage).unwrap();
    assert_eq!(sum.magnitude.bits(), 107);
    let cancelled = Pair::products([(q, q, 0), (-q, q, 0)], stage).unwrap();
    assert_eq!(cancelled.magnitude.bits(), 0);
    for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(Pair::products([(invalid, 1.0, 0), (0.0, 0.0, 0)], stage).is_err());
    }
    assert!(std::mem::size_of::<Pair>() <= 576);
}
