/// Split a finite value without losing subnormal mantissa bits.
pub(crate) fn split_finite(value: f64) -> (f64, i32) {
    debug_assert!(value.is_finite());
    if value == 0.0 {
        return (value, 0);
    }
    if value.abs() < f64::MIN_POSITIVE {
        let (mantissa, exponent) = split_finite(value * 4_503_599_627_370_496.0);
        return (mantissa, exponent - 52);
    }
    let bits = value.to_bits();
    (
        f64::from_bits((bits & 0x800f_ffff_ffff_ffff) | (1023_u64 << 52)),
        ((bits >> 52) & 0x7ff) as i32 - 1023,
    )
}

/// Restore only after combining exponents; avoid an overflowing power-of-two factor.
pub(crate) fn restore(value: f64, exponent: i32) -> f64 {
    if value == 0.0 || !value.is_finite() {
        return value;
    }
    let (mantissa, power) = split_finite(value);
    let exponent = exponent.saturating_add(power);
    if exponent > 1023 {
        f64::INFINITY.copysign(value)
    } else if exponent >= -1022 {
        mantissa * power_of_two(exponent)
    } else if exponent >= -1075 {
        (mantissa * power_of_two(exponent + 1022)) * f64::MIN_POSITIVE
    } else {
        0.0_f64.copysign(value)
    }
}

fn power_of_two(exponent: i32) -> f64 {
    debug_assert!((-1022..=1023).contains(&exponent));
    f64::from_bits(((exponent + 1023) as u64) << 52)
}
