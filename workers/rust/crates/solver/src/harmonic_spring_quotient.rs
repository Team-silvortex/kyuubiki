use crate::linear_binary_scale::{restore, split_finite};

const RANGE_ERROR: &str = "complex solution is outside the representable range";
const FRACTION: u64 = (1 << 52) - 1;

pub(super) fn scaled_quotient(
    numerator: f64,
    numerator_power: i32,
    divisor: f64,
    divisor_power: i32,
) -> Result<f64, String> {
    if !numerator.is_finite() || !divisor.is_finite() || divisor == 0.0 {
        return Err("complex scaled division requires finite values and a nonzero divisor".into());
    }
    if numerator == 0.0 {
        return Ok(numerator / divisor);
    }
    let (n, ne) = split_finite(numerator);
    let (d, de) = split_finite(divisor);
    let power =
        i64::from(numerator_power) + i64::from(ne) - i64::from(divisor_power) - i64::from(de);
    if !(-1075..=1024).contains(&power) {
        return Err(RANGE_ERROR.into());
    }
    let value = if power <= -1022 {
        // The normalized ratio lies strictly between 1/2 and 2. Convert directly
        // to quantum units instead of rounding the ratio and then the subnormal.
        let mut n_units = u128::from((n.to_bits() & FRACTION) | (1 << 52));
        let mut d_units = u128::from((d.to_bits() & FRACTION) | (1 << 52));
        let shift = power + 1074;
        if shift >= 0 {
            n_units <<= shift;
        } else {
            d_units <<= -shift;
        }
        let mut integer = n_units / d_units;
        let remainder = n_units % d_units;
        if 2 * remainder > d_units || (2 * remainder == d_units && integer & 1 != 0) {
            integer += 1;
        }
        let bits = u64::try_from(integer).map_err(|_| RANGE_ERROR)?;
        let sign = (n.to_bits() ^ d.to_bits()) & (1 << 63);
        f64::from_bits(sign | bits)
    } else {
        restore(n / d, power as i32)
    };
    // Nonzero unknowns may round to a nonzero quantum, never silently to zero.
    if !value.is_finite() || value == 0.0 {
        return Err(RANGE_ERROR.into());
    }
    Ok(value)
}
