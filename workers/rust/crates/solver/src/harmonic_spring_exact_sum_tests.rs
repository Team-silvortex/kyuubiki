use super::*;

fn sum(values: impl IntoIterator<Item = f64>) -> f64 {
    let mut accumulator = ExactSum::default();
    for value in values {
        accumulator.add(value).unwrap();
    }
    accumulator.finish().unwrap()
}

#[test]
fn harmonic_complex_accumulation_integer_lattice_matches_an_independent_i128_oracle() {
    let mut seed = 0x9e37_79b9_7f4a_7c15_u64;
    for _ in 0..128 {
        let mut values = Vec::new();
        let mut reference = 0_i128;
        for _ in 0..64 {
            seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
            let shift = (seed >> 32) % 101;
            let coefficient = (seed % 7) as i128 - 3;
            reference += coefficient << shift;
            values.push(coefficient as f64 * 2.0_f64.powi(shift as i32 - 100));
        }
        let expected = reference as f64 * 2.0_f64.powi(-100);
        assert_eq!(sum(values.iter().copied()).to_bits(), expected.to_bits());
        assert_eq!(
            sum(values.iter().rev().copied()).to_bits(),
            expected.to_bits()
        );
    }
}

#[test]
fn harmonic_complex_accumulation_rounds_ties_sticky_bits_and_significand_carry_once() {
    let half = 2.0_f64.powi(-53);
    let tiny = 2.0_f64.powi(-100);
    let quantum = f64::from_bits(1);
    for sign in [-1.0, 1.0] {
        for (parts, expected) in [
            ([1.0, half, 0.0], 1.0),
            ([1.0, half, tiny], f64::from_bits(1.0_f64.to_bits() + 1)),
            ([1.0, half, -tiny], 1.0),
            ([1.0, half, quantum], f64::from_bits(1.0_f64.to_bits() + 1)),
            ([1.0, half, -quantum], 1.0),
            (
                [1.0, 3.0 * half, 0.0],
                f64::from_bits(1.0_f64.to_bits() + 2),
            ),
            ([2.0 - 2.0_f64.powi(-52), half, 0.0], 2.0),
        ] {
            assert_eq!(
                sum(parts.map(|value| sign * value)).to_bits(),
                (sign * expected).to_bits()
            );
        }
    }
}

#[test]
fn harmonic_complex_accumulation_preserves_subnormal_lattice_borrows_and_exact_zeros() {
    let quantum = f64::from_bits(1);
    let subnormal = f64::from_bits((1 << 52) - 1);
    for sign in [-1.0, 1.0] {
        assert_eq!(
            sum([sign * f64::MIN_POSITIVE, -sign * quantum]),
            sign * subnormal
        );
        assert_eq!(
            sum([sign * f64::MIN_POSITIVE, -sign * subnormal]),
            sign * quantum
        );
        assert_eq!(
            sum([sign * subnormal, sign * quantum]),
            sign * f64::MIN_POSITIVE
        );
        assert_eq!(
            sum([sign * 4.0, sign * quantum, -sign * 4.0]),
            sign * quantum
        );
    }
    assert_eq!(sum([0.0, -0.0, 4.0, -4.0]).to_bits(), 0);
}

#[test]
fn harmonic_complex_accumulation_nested_tails_are_order_independent_with_fixed_storage() {
    let tiny = 2.0_f64.powi(-100);
    let mut values = [
        4.0,
        -4.0,
        1.0,
        -1.0,
        2.0_f64.powi(-60),
        tiny,
        -2.0_f64.powi(-60),
    ];
    for index in 0..128 {
        values.swap(index % 7, (index * 5 + 3) % 7);
        assert_eq!(sum(values), tiny);
    }
    assert!(std::mem::size_of::<ExactSum>() <= 320);
}

#[test]
fn harmonic_complex_accumulation_invalid_parts_and_capacity_errors_cannot_publish_wrapped_sums() {
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 4.5, -4.5] {
        let mut accumulator = ExactSum::default();
        assert!(
            accumulator
                .add(bad)
                .unwrap_err()
                .contains("normalized part")
        );
        assert!(accumulator.add(1.0).is_err());
        assert!(accumulator.finish().is_err());
    }
    for negative in [false, true] {
        let mut accumulator = ExactSum::default();
        if negative {
            accumulator.negative.fill(u64::MAX);
        } else {
            accumulator.positive.fill(u64::MAX);
        }
        let value = if negative {
            -f64::from_bits(1)
        } else {
            f64::from_bits(1)
        };
        assert!(
            accumulator
                .add(value)
                .unwrap_err()
                .contains("capacity exceeded")
        );
        assert!(accumulator.finish().is_err());
    }
    assert_eq!(sum([1.0, -1.0, 2.0]), 2.0);
}

#[test]
fn harmonic_assembly_full_range_accumulator_capacity_failure_remains_poisoned() {
    for negative in [false, true] {
        let mut sum = CoefficientSum::default();
        if negative {
            sum.0.negative.fill(u64::MAX);
        } else {
            sum.0.positive.fill(u64::MAX);
        }
        let quantum = f64::from_bits(1);
        assert!(
            sum.add(if negative { -quantum } else { quantum })
                .unwrap_err()
                .contains("capacity")
        );
        assert!(sum.add(1.0).is_err());
        assert!(sum.finish().is_err());
    }
}

#[test]
fn harmonic_inertia_precision_scaled_lattice_rounding_range_and_poisoning_are_checked() {
    for sign in [-1.0, 1.0] {
        let mut sum = InertiaSum::default();
        sum.add_scaled(sign * f64::MAX, 0).unwrap();
        sum.add_scaled(sign, 970).unwrap();
        sum.add_scaled(-sign, -3222).unwrap();
        assert_eq!(sum.finish().unwrap(), sign * f64::MAX);
        let mut sum = InertiaSum::default();
        sum.add_scaled(sign * f64::MIN_POSITIVE, 0).unwrap();
        sum.add_scaled(-sign, -1075).unwrap();
        assert_eq!(sum.finish().unwrap(), sign * f64::MIN_POSITIVE);
        let mut sum = InertiaSum::default();
        sum.add_scaled(sign * 0.75, -1074).unwrap();
        assert!(sum.finish().unwrap_err().contains("lose numerical range"));
    }
    let mut sum = InertiaSum::default();
    sum.0.positive.fill(u64::MAX);
    assert!(sum.add_scaled(1.0, -3222).unwrap_err().contains("capacity"));
    assert!(sum.add_scaled(1.0, 0).is_err());
    assert!(sum.finish().is_err());
    let mut sum = InertiaSum::default();
    assert!(
        sum.add_scaled(1.0, -3223)
            .unwrap_err()
            .contains("lose numerical range")
    );
    assert!(sum.finish().is_err());
    let mut sum = InertiaSum::default();
    sum.add_scaled(1.0, -3222).unwrap();
    sum.add_scaled(-1.0, -3222).unwrap();
    assert_eq!(sum.finish().unwrap().to_bits(), 0);
    assert!(std::mem::size_of::<InertiaSum>() <= 1120);
}
