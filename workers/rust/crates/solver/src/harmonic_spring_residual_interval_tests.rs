use super::*;

fn exact(value: f64) -> Interval {
    Interval {
        lower: value,
        upper: value,
    }
}
fn product(value: f64, exponent: i32) -> Product {
    Product {
        high: value,
        low: 0.0,
        exponent,
    }
}

fn summed(value: Interval) -> IntervalSum {
    let mut result = IntervalSum::default();
    result.add(value).unwrap();
    result
}

fn boundary_grid(value: f64) -> i128 {
    let bits = value.abs().to_bits();
    let fraction = bits & ((1_u64 << 52) - 1);
    let exponent = (bits >> 52) as i32;
    let (integer, shift) = if exponent == 0 {
        (fraction, 54)
    } else {
        ((1_u64 << 52) | fraction, exponent - 1023 - 52 + 1128)
    };
    assert!((0..=55).contains(&shift));
    let scaled = (integer as i128) << shift;
    if value.is_sign_negative() {
        -scaled
    } else {
        scaled
    }
}

#[test]
fn harmonic_residual_enclosure_binary_scaling_bounds_half_quantum_and_nonzero_underflow() {
    let q = f64::from_bits(1);
    for sign in [-1.0, 1.0] {
        let half = normalized(&product(sign, -1075), 0).unwrap();
        assert!(2.0 * half.lower <= sign * q && 2.0 * half.upper >= sign * q);
        let tiny = normalized(&product(sign, -2000), 0).unwrap();
        assert!(tiny.lower <= 0.0 && tiny.upper >= 0.0);
        assert!(tiny.maximum_abs() > 0.0);
        assert_eq!(tiny.minimum_abs(), 0.0);
        let ordinary = normalized(&product(sign * 1.5, -20), -20).unwrap();
        assert_eq!((ordinary.lower, ordinary.upper), (sign * 1.5, sign * 1.5));
    }
    let mut seed = 0x9712_5368_7adb_0821_u64;
    for sample in 0..128 {
        seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
        let integer = match sample {
            0 => 1_i128 << 52,
            1 => (1_i128 << 53) - 1,
            _ => (1_i128 << 52) + (seed & ((1_u64 << 52) - 1)) as i128,
        };
        for power in [-1075, -1074, -1073, -1023, -1022] {
            for sign in [-1_i128, 1] {
                for tail in [-1_i128, 0, 1] {
                    let source = Product {
                        high: (sign * integer) as f64 * 2.0_f64.powi(-52),
                        low: tail as f64 * 2.0_f64.powi(-53),
                        exponent: power,
                    };
                    let bound = normalized(&source, 0).unwrap();
                    // Exact dyadics on a 2^-1128 grid, without underflowing
                    // either the independent reference or the endpoints.
                    let reference = (2 * sign * integer + tail) << (power + 1075);
                    assert!(boundary_grid(bound.lower) <= reference);
                    assert!(boundary_grid(bound.upper) >= reference);
                }
            }
        }
    }
}

#[test]
fn harmonic_residual_enclosure_interval_sums_contain_an_independent_integer_lattice() {
    let mut seed = 0x7843_882a_5f67_212b_u64;
    for _ in 0..128 {
        let mut values = Vec::new();
        let mut reference = 0_i128;
        for _ in 0..64 {
            seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
            let shift = (seed >> 32) % 61;
            let coefficient = (seed % 7) as i128 - 3;
            reference += coefficient << shift;
            values.push(coefficient as f64 * 2.0_f64.powi(shift as i32 - 60));
        }
        for reverse in [false, true] {
            if reverse {
                values.reverse();
            }
            let mut sum = Interval::default();
            for &value in &values {
                sum.add(exact(value)).unwrap();
            }
            let grid = 2.0_f64.powi(60);
            assert!((sum.lower * grid).ceil() as i128 <= reference);
            assert!((sum.upper * grid).floor() as i128 >= reference);
            let negative = sum.negated();
            assert!((negative.lower * grid).ceil() as i128 <= -reference);
            assert!((negative.upper * grid).floor() as i128 >= -reference);
        }
    }
}

#[test]
fn harmonic_residual_enclosure_comparison_is_outward_and_uncertainty_is_not_success() {
    let tol = 1e-12_f64;
    let mut row = Enclosure::new(0);
    row.scale_lower = 1.0;
    row.real = summed(exact(tol.next_down().next_down()));
    row.check(3, tol).unwrap();
    row.real = summed(exact(tol));
    assert!(row.check(3, tol).unwrap_err().contains("free row 3"));
    row.real = summed(Interval {
        lower: -tol,
        upper: tol.next_up(),
    });
    assert!(row.check(3, tol).is_err());
    row.real = IntervalSum::default();
    row.check(3, tol).unwrap();
    for bad in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        row.scale_lower = bad;
        assert!(row.check(3, tol).is_err());
    }
    row.scale_lower = 1.0;
    for bad in [-1.0, f64::NAN, f64::INFINITY] {
        assert!(row.check(3, bad).is_err());
    }
}

#[test]
fn harmonic_residual_enclosure_uses_component_norm_lower_bounds_and_original_signs() {
    assert!(std::mem::size_of::<Enclosure>() <= 112);
    let mut row = Enclosure::new(0);
    row.subtract([
        product(1.0, 0),
        product(2.0, 0),
        product(3.0, 0),
        product(4.0, 0),
    ])
    .unwrap();
    let real = row.real.interval().unwrap();
    let imaginary = row.imaginary.interval().unwrap();
    assert!(real.lower <= 1.0 && real.upper >= 1.0);
    assert!(imaginary.lower <= -7.0 && imaginary.upper >= -7.0);
    assert_eq!(row.scale_lower, 4.0);
    assert!(row.scale_lower <= (30.0_f64).sqrt());
    assert_eq!(exact(-2.0).minimum_abs(), 2.0);
    assert_eq!(
        Interval {
            lower: -1.0,
            upper: 2.0
        }
        .minimum_abs(),
        0.0
    );
}

#[test]
fn harmonic_residual_enclosure_invalid_bounds_and_exponent_ranges_do_not_panic_or_publish() {
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(normalized(&product(bad, 0), 0).is_err());
        assert!(exact(bad).checked().is_err());
        assert!(Product::scaled(bad, 0, 1.0).is_err());
    }
    assert!(normalized(&product(1.0, i32::MIN), 0).is_err());
    assert!(normalized(&product(1.0, i32::MAX), i32::MIN).is_err());
    assert!(Product::scaled(2.0, i32::MAX, 2.0).is_err());
    assert!(Product::scaled(0.5, i32::MIN, 0.5).is_err());
    let mut value = exact(1.0);
    assert!(
        value
            .add(Interval {
                lower: 2.0,
                upper: 1.0
            })
            .is_err()
    );
    assert_eq!((value.lower, value.upper), (1.0, 1.0));
    value = exact(f64::MAX);
    assert!(value.add(exact(f64::MAX)).is_err());
    assert_eq!(value.lower, f64::MAX);
    let mut row = Enclosure::new(0);
    row.scale_lower = f64::MAX;
    assert!(row.add_scale(f64::MAX).is_err());
    assert_eq!(row.scale_lower, f64::MAX);
    assert!(entry(&mut [(1, Enclosure::new(0))], 2).is_err());
}
