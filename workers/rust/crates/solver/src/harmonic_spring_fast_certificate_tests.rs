use super::*;

fn pair(units: u128, power: i64) -> SmallPair {
    SmallPair {
        units,
        power,
        negative: false,
    }
}

#[test]
fn harmonic_fast_division_certificate_checks_exact_midpoints_ties_and_binade_edges() {
    let d = pair(1 << 55, 0);
    for (units, expected) in [
        ((1 << 55) - 3, false),
        ((1 << 55) - 2, true),
        ((1 << 55) - 1, true),
        ((1 << 55) + 3, true),
        ((1 << 55) + 4, true),
        ((1 << 55) + 5, false),
    ] {
        assert_eq!(in_rounding_cell(&pair(units, 0), &d, 1.0), Some(expected));
    }
    assert_eq!(
        in_rounding_cell(&pair((1 << 54) - 1, 0), &pair(1 << 53, 0), 2.0),
        Some(true)
    );
    let odd = f64::from_bits(1.0_f64.to_bits() + 1);
    assert_eq!(
        in_rounding_cell(&pair((1 << 53) + 1, 0), &pair(1 << 53, 0), odd),
        Some(false)
    );
    assert_eq!(
        in_rounding_cell(&pair((1 << 53) - 1, -1075), &pair(1, 0), f64::MIN_POSITIVE),
        Some(true)
    );
    assert_eq!(
        in_rounding_cell(&pair((1 << 54) - 1, 970), &pair(1, 0), f64::MAX),
        Some(false)
    );
    assert_eq!(
        in_rounding_cell(&pair((1 << 55) - 3, 969), &pair(1, 0), f64::MAX),
        Some(true)
    );
    for invalid in [0.0, -1.0, f64::from_bits(1), f64::NAN, f64::INFINITY] {
        assert_eq!(
            in_rounding_cell(&pair(1, 0), &pair(1, 0), invalid),
            Some(false)
        );
    }
}

#[test]
fn harmonic_fast_division_certificate_rejects_forged_neighbor_candidates_and_capacity_loss() {
    for (n, d) in [(23, 41), (2, 41), (2, 13), (3, 13)] {
        let expected = n as f64 / d as f64;
        assert_eq!(
            in_rounding_cell(&pair(n, 0), &pair(d, 0), expected),
            Some(true)
        );
        for bits in [expected.to_bits() - 1, expected.to_bits() + 1] {
            assert_eq!(
                in_rounding_cell(&pair(n, 0), &pair(d, 0), f64::from_bits(bits)),
                Some(false)
            );
        }
    }
    assert!(in_rounding_cell(&pair(1, i64::from(i32::MAX)), &pair(1, 0), 1.0).is_none());
    assert!(
        SmallPair::products([(f64::MAX, f64::MAX), (f64::from_bits(1), f64::from_bits(1))])
            .is_none()
    );
    assert!(SmallPair::products([(1.0, 1.0), (-1.0, 1.0)]).is_none());
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(SmallPair::products([(bad, 1.0), (0.0, 0.0)]).is_none());
    }
    assert!(std::mem::size_of::<SmallPair>() <= 48);
    assert_eq!(std::mem::size_of::<Integer192>(), 24);
}

#[test]
fn harmonic_fast_division_certificate_integer_multiply_and_shift_are_checked() {
    let wide = Integer192::from(u128::MAX).times(u64::MAX).unwrap();
    assert_eq!(wide.0, [1, u64::MAX, u64::MAX - 1]);
    assert!(wide.times(2).is_none());
    assert!(wide.shifted(1).is_none());
    for shift in [0, 1, 63, 64, 65, 127, 128, 191] {
        let result = Integer192::from(1).shifted(shift).unwrap();
        let mut expected = [0; 3];
        expected[shift as usize / 64] = 1 << (shift as usize % 64);
        assert_eq!(result.0, expected);
    }
    assert!(Integer192::from(1).shifted(-1).is_none());
    assert!(Integer192::from(1).shifted(192).is_none());
    assert_eq!(Integer192::from(0).shifted(1000).unwrap().0, [0; 3]);
}
