use super::*;

fn point(value: f64) -> Interval {
    Interval {
        lower: value,
        upper: value,
    }
}

fn grid_bounds(value: f64, quantum: i32) -> (i128, i128) {
    assert!(value.is_finite());
    let bits = value.abs().to_bits();
    let exponent = (bits >> 52) as i32;
    let integer =
        ((bits & ((1_u64 << 52) - 1)) | if exponent == 0 { 0 } else { 1_u64 << 52 }) as u128;
    let shift = if exponent == 0 {
        -1074
    } else {
        exponent - 1075
    } - quantum;
    let (floor, ceil) = if shift >= 0 {
        assert!(shift <= 70);
        let exact = (integer << shift) as i128;
        (exact, exact)
    } else {
        let right = -shift as u32;
        let (floor, remainder) = if right >= 128 {
            (0, integer != 0)
        } else {
            (integer >> right, integer & ((1_u128 << right) - 1) != 0)
        };
        (floor as i128, floor as i128 + i128::from(remainder))
    };
    if value.is_sign_negative() {
        (-ceil, -floor)
    } else {
        (floor, ceil)
    }
}

#[test]
fn harmonic_residual_many_term_sums_enclose_independent_integer_endpoint_references() {
    let mut seed = 0xb723_528d_4873_1cb9_u64;
    for batch in 0..128 {
        let mut values = Vec::new();
        let mut lower = 0_i128;
        let mut upper = 0_i128;
        for _ in 0..1024 {
            seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
            let (power, coefficient) = if batch % 2 == 0 {
                (((seed >> 32) % 101) as i32, (seed % 7) as i128 - 3)
            } else {
                let integer = ((1_u64 << 52) | (seed & ((1_u64 << 52) - 1))) as i128;
                (
                    ((seed >> 32) % 51) as i32,
                    if seed & 1 == 0 { integer } else { -integer },
                )
            };
            lower += coefficient << power;
            upper += (coefficient + 1) << power;
            let scale = 2.0_f64.powi(power - 100);
            values.push(Interval {
                lower: coefficient as f64 * scale,
                upper: (coefficient + 1) as f64 * scale,
            });
        }
        for reverse in [false, true] {
            if reverse {
                values.reverse();
            }
            let mut sum = IntervalSum::default();
            for &value in &values {
                sum.add(value).unwrap();
            }
            let bound = sum.interval().unwrap();
            assert!(grid_bounds(bound.lower, -100).1 <= lower);
            assert!(grid_bounds(bound.upper, -100).0 >= upper);
        }
    }
}

#[test]
fn harmonic_residual_many_term_sums_keep_parallel_cancellation_bounds_tight() {
    let count = 1 << 16;
    for reverse in [false, true] {
        let mut sum = IntervalSum::default();
        if !reverse {
            sum.add(point(0.5)).unwrap();
        }
        for _ in 0..count {
            sum.add(point(-0.4 * 2.0_f64.powi(-16))).unwrap();
            sum.add(point(-0.1 * 2.0_f64.powi(-16))).unwrap();
        }
        if reverse {
            sum.add(point(0.5)).unwrap();
        }
        let bound = sum.interval().unwrap();
        // The represented .4 and .1 add to exactly .5 + 2^-55.
        let reference = -2.0_f64.powi(-55);
        assert!(
            bound.lower <= reference && bound.upper >= reference,
            "{bound:?}"
        );
        assert!(bound.upper - bound.lower <= 8.0 * f64::EPSILON, "{bound:?}");
    }
    assert!(std::mem::size_of::<IntervalSum>() <= 48);
}

#[test]
fn harmonic_residual_many_term_sums_bound_the_compensation_tail_instead_of_erasing_it() {
    for sign in [-1.0, 1.0] {
        for power in [-500, 0, 500] {
            let scale = 2.0_f64.powi(power);
            let mut sum = IntervalSum::default();
            for value in [
                1.0,
                2.0_f64.powi(-53),
                2.0_f64.powi(-106),
                -1.0,
                -2.0_f64.powi(-53),
            ] {
                sum.add(point(sign * value * scale)).unwrap();
            }
            assert!(sum.lower.radius > 0.0);
            let bound = sum.interval().unwrap();
            let reference = sign * 2.0_f64.powi(power - 106);
            assert!(
                bound.lower <= reference && bound.upper >= reference,
                "{bound:?}"
            );
            assert!(bound.maximum_abs() > 0.0);
        }
    }
    let q = f64::from_bits(1);
    let mut sum = IntervalSum::default();
    for value in [1.0, q, -1.0, f64::MIN_POSITIVE, -f64::MIN_POSITIVE] {
        sum.add(point(value)).unwrap();
    }
    let bound = sum.interval().unwrap();
    assert_eq!((bound.lower, bound.upper), (q, q));
}

#[test]
fn harmonic_residual_many_term_sums_reject_invalid_or_overflowed_updates_without_publication() {
    let mut sum = IntervalSum::default();
    sum.add(point(1.0)).unwrap();
    for invalid in [
        point(f64::NAN),
        point(f64::INFINITY),
        point(f64::NEG_INFINITY),
        Interval {
            lower: 2.0,
            upper: 1.0,
        },
    ] {
        assert!(sum.add(invalid).is_err());
        let bound = sum.interval().unwrap();
        assert_eq!((bound.lower, bound.upper), (1.0, 1.0));
    }
    sum = IntervalSum::default();
    sum.add(Interval {
        lower: 0.0,
        upper: f64::MAX,
    })
    .unwrap();
    assert!(
        sum.add(Interval {
            lower: 1.0,
            upper: f64::MAX
        })
        .is_err()
    );
    let bound = sum.interval().unwrap();
    assert_eq!((bound.lower, bound.upper), (0.0, f64::MAX));
    for invalid in [f64::NAN, f64::INFINITY, -1.0] {
        let mut bad = Sum {
            high: 1.0,
            low: 0.0,
            radius: invalid,
        };
        assert!(bad.add(0.0).is_err());
        assert!(bad.interval().is_err());
    }
    assert!(add_radius(f64::MAX, f64::MAX).is_err());
    assert!(two_sum(f64::MAX, f64::MAX).is_err());
}
