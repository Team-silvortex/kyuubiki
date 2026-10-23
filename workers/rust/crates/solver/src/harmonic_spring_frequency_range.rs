use crate::linear_binary_scale::{restore, split_finite};

pub(super) fn product(frequency: f64, value: f64) -> Result<f64, String> {
    validate(frequency, value)?;
    let result = frequency * value;
    if result.is_normal() || frequency == 0.0 || value == 0.0 {
        return Ok(result);
    }
    let (frequency, fe) = split_finite(frequency);
    let (value, ve) = split_finite(value);
    restore_checked(frequency * value, fe + ve)
}

pub(super) fn square_product(frequency: f64, value: f64) -> Result<f64, String> {
    validate(frequency, value)?;
    if frequency == 0.0 || value == 0.0 {
        return Ok(0.0);
    }
    let square = frequency * frequency;
    let result = square * value;
    if square.is_normal() && result.is_normal() {
        return Ok(result);
    }
    // A square outside f64 may still yield an ordinary coefficient/response.
    // Only restore after the value's exponent has joined both frequency factors.
    let (frequency, fe) = split_finite(frequency);
    let (value, ve) = split_finite(value);
    restore_checked((frequency * frequency) * value, 2 * fe + ve)
}

fn validate(frequency: f64, value: f64) -> Result<(), String> {
    if !frequency.is_finite() || !value.is_finite() {
        return Err("contains a non-finite frequency/product operand".into());
    }
    Ok(())
}

fn restore_checked(mantissa: f64, exponent: i32) -> Result<f64, String> {
    let result = restore(mantissa, exponent);
    if !result.is_finite() {
        return Err("exceeds the finite numeric range".into());
    }
    if restore(result, -exponent) != mantissa {
        return Err("would lose numerical range during product restoration".into());
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn harmonic_frequency_product_keeps_ordinary_arithmetic_bits() {
        for frequency in [0.1, 0.75, 2.75, 1e-100, 1e100] {
            for value in [0.25, 1.25, -1.25, 1e-50, 1e50] {
                assert_eq!(
                    product(frequency, value).unwrap().to_bits(),
                    (frequency * value).to_bits()
                );
                assert_eq!(
                    square_product(frequency, value).unwrap().to_bits(),
                    ((frequency * frequency) * value).to_bits()
                );
            }
        }
    }

    #[test]
    fn harmonic_frequency_product_recovers_squares_before_range_loss() {
        for power in [-600, 600] {
            assert_eq!(
                square_product(2.0_f64.powi(power), 2.0_f64.powi(-3 * power / 2)).unwrap(),
                2.0_f64.powi(power / 2)
            );
            assert_eq!(
                square_product(2.0_f64.powi(power), 2.0_f64.powi(-power)).unwrap(),
                2.0_f64.powi(power)
            );
        }
        assert_eq!(
            square_product(1.5 * 2.0_f64.powi(-537), 2.0_f64.powi(800)).unwrap(),
            2.25 * 2.0_f64.powi(-274)
        );
    }

    #[test]
    fn harmonic_frequency_product_requires_exact_subnormal_restoration() {
        let quantum = f64::from_bits(1);
        assert_eq!(
            product(2.0_f64.powi(-100), 2.0_f64.powi(-974)).unwrap(),
            quantum
        );
        assert_eq!(square_product(0.5, 4.0 * quantum).unwrap(), quantum);
        for factor in [0.25, 0.75, 1.5] {
            assert!(
                product(factor, quantum)
                    .unwrap_err()
                    .contains("lose numerical range")
            );
            assert!(square_product(0.5, 4.0 * factor * quantum).is_err());
        }
        assert!(
            square_product(2.0_f64.powi(-600), 1.0)
                .unwrap_err()
                .contains("lose numerical range")
        );
        assert!(
            square_product(2.0_f64.powi(600), 1.0)
                .unwrap_err()
                .contains("finite numeric range")
        );
    }

    #[test]
    fn harmonic_frequency_product_keeps_true_zeros_but_rejects_nonfinite_operands() {
        assert_eq!(square_product(2.0_f64.powi(600), 0.0).unwrap(), 0.0);
        assert_eq!(square_product(0.0, f64::MAX).unwrap(), 0.0);
        assert_eq!(product(0.0, f64::MAX).unwrap(), 0.0);
        for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert!(square_product(0.0, value).is_err());
            assert!(product(value, 0.0).is_err());
        }
    }
}
