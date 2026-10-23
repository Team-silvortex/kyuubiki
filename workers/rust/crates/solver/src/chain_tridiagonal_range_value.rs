use crate::linear_binary_scale::{restore, split_finite};
use crate::linear_substitution::normalize;

fn needs_underflow_check(coefficient: f64, previous: f64, result: f64) -> bool {
    result.abs() <= f64::MIN_POSITIVE && coefficient != 0.0 && previous != 0.0
}

/// Compare tiny updates in exponent space before trusting a rounded work entry.
/// Exact cancellation and exactly represented subnormals keep the ordinary path.
pub(super) fn update_needs_range(
    rhs: f64,
    coefficient: f64,
    previous: f64,
    result: f64,
) -> Result<bool, String> {
    if !result.is_finite() {
        return Ok(true);
    }
    if !needs_underflow_check(coefficient, previous, result) {
        return Ok(false);
    }
    let extended =
        WorkValue::finite(rhs)?.subtract_product(coefficient, WorkValue::finite(previous)?);
    // A finite probe cannot improve a tail too distant to align reversibly.
    // Retain that ordinary FMA and its original-equation admission. Actual
    // extended replay still rejects alignment loss rather than dropping terms.
    Ok(extended.is_ok_and(|work| work.represented().is_none_or(|value| value != result)))
}

#[derive(Clone, Copy)]
pub(super) struct WorkValue {
    pub(super) mantissa: f64,
    pub(super) exponent: i32,
}

impl WorkValue {
    pub(super) fn finite(value: f64) -> Result<Self, String> {
        if !value.is_finite() {
            return Err("tridiagonal forward recovery contains a non-finite value".into());
        }
        let (mantissa, exponent) = split_finite(value);
        Ok(Self { mantissa, exponent })
    }

    pub(super) fn represented(self) -> Option<f64> {
        let inverse = self.exponent.checked_neg()?;
        let value = restore(self.mantissa, self.exponent);
        (value.is_finite() && restore(value, inverse) == self.mantissa).then_some(value)
    }

    pub(super) fn subtract_product(self, coefficient: f64, previous: Self) -> Result<Self, String> {
        if let (Some(rhs), Some(value)) = (self.represented(), previous.represented()) {
            let result = (-coefficient).mul_add(value, rhs);
            if result.is_finite() && !needs_underflow_check(coefficient, value, result) {
                return Self::finite(result);
            }
        }
        // A disconnected weak row must not share the exponent of a strong row.
        // Use the mantissa only to retain signed-zero multiplication semantics.
        if coefficient == 0.0 || previous.mantissa == 0.0 {
            return match self.represented() {
                Some(rhs) => Self::finite((-coefficient).mul_add(previous.mantissa, rhs)),
                None => Ok(self),
            };
        }
        let (factor, power) = split_finite(coefficient);
        let product_power = previous
            .exponent
            .checked_add(power)
            .ok_or("tridiagonal forward recovery exponent overflow")?;
        let exponent = if self.mantissa == 0.0 {
            product_power
        } else {
            self.exponent.max(product_power)
        };
        let rhs_shift = self
            .exponent
            .checked_sub(exponent)
            .ok_or("tridiagonal forward recovery exponent overflow")?;
        let product_shift = product_power
            .checked_sub(exponent)
            .ok_or("tridiagonal forward recovery exponent overflow")?;
        let rhs = normalize(self.mantissa, rhs_shift)?;
        let factor = normalize(factor, product_shift)?;
        let mut result = Self::finite((-factor).mul_add(previous.mantissa, rhs))?;
        if result.mantissa != 0.0 {
            result.exponent = result
                .exponent
                .checked_add(exponent)
                .ok_or("tridiagonal forward recovery exponent overflow")?;
        }
        Ok(result)
    }
}
