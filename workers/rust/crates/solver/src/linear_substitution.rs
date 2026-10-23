use crate::linear_binary_scale::{restore, split_finite};
use crate::solver_control::{SolverStage, check_cancellation, checkpoint, checkpoint_chunk};

pub(crate) enum SubnormalResult {
    RejectErasure,
    /// Reduction has no unknown-rounding budget: restoring the exponent must be reversible.
    RequireExactRange,
    /// Only for callers with an original-equation gate allowing rounded tails.
    ValidateOriginalEquations,
}

/// Compute (rhs - dot(coefficients, solved)) / pivot without restoring an
/// exceptional numerator before the division. Ordinary rows keep their order.
pub(crate) fn solve_row<I>(
    rhs: f64,
    operands: I,
    pivot: f64,
    stage: SolverStage,
) -> Result<f64, String>
where
    I: ExactSizeIterator<Item = (f64, f64)> + Clone,
{
    if !rhs.is_finite() || !pivot.is_finite() || pivot == 0.0 {
        return Err(
            "triangular substitution requires a finite RHS and nonzero finite pivot".into(),
        );
    }
    let mut sum = -0.0_f64;
    let mut needs_range = false;
    for (index, (coefficient, value)) in operands.clone().enumerate() {
        if !coefficient.is_finite() || !value.is_finite() {
            return Err("triangular substitution contains a non-finite operand".into());
        }
        let product = coefficient * value;
        needs_range |= coefficient != 0.0 && value != 0.0 && !product.is_normal();
        sum += product;
        needs_range |= !sum.is_finite() || sum.is_subnormal();
        if (index + 1) % 1024 == 0 {
            check_cancellation()?;
        }
    }
    let numerator = rhs - sum;
    let result = numerator / pivot;
    needs_range |= !numerator.is_finite()
        || numerator.is_subnormal()
        || !result.is_finite()
        || (result == 0.0 && numerator != 0.0);
    if !needs_range {
        return Ok(result);
    }
    solve_scaled_rhs_row(split_finite(rhs), operands, pivot, stage)
}

/// The RHS may have a wider exponent than f64; restore only the final quotient.
pub(crate) fn solve_scaled_rhs_row<I>(
    rhs: (f64, i32),
    operands: I,
    pivot: f64,
    stage: SolverStage,
) -> Result<f64, String>
where
    I: ExactSizeIterator<Item = (f64, f64)> + Clone,
{
    solve_scaled_rhs_row_with_policy(rhs, operands, pivot, stage, SubnormalResult::RejectErasure)
}

pub(crate) fn solve_scaled_rhs_row_with_policy<I>(
    rhs: (f64, i32),
    operands: I,
    pivot: f64,
    stage: SolverStage,
    policy: SubnormalResult,
) -> Result<f64, String>
where
    I: ExactSizeIterator<Item = (f64, f64)> + Clone,
{
    checkpoint(stage, 0)?;
    let (rhs_mantissa, rhs_exponent) = rhs;
    if !rhs_mantissa.is_finite() || !pivot.is_finite() || pivot == 0.0 {
        return Err(
            "triangular substitution requires a finite RHS mantissa and nonzero finite pivot"
                .into(),
        );
    }
    let mut exponent = (rhs_mantissa != 0.0).then_some(rhs_exponent);
    let length = operands.len();
    for (index, (coefficient, value)) in operands.clone().enumerate() {
        if !coefficient.is_finite() || !value.is_finite() {
            return Err("triangular substitution contains a non-finite operand".into());
        }
        if coefficient != 0.0 && value != 0.0 {
            let power = split_finite(coefficient).1 + split_finite(value).1;
            exponent = Some(exponent.map_or(power, |current| current.max(power)));
        }
        checkpoint_chunk(stage, index + 1, length)?;
    }
    let exponent = exponent.unwrap_or(0);
    let mut sum = CompensatedSum::default();
    sum.add(normalized_term(rhs_mantissa, rhs_exponent, exponent)?);
    for (index, (coefficient, value)) in operands.enumerate() {
        let (left, le) = split_finite(coefficient);
        let (right, re) = split_finite(value);
        let high = left * right;
        let low = left.mul_add(right, -high);
        sum.add(-normalized_term(high, le + re, exponent)?);
        sum.add(-normalized_term(low, le + re, exponent)?);
        checkpoint_chunk(stage, length + index + 1, 2 * length)?;
    }
    checkpoint(stage, 2 * length)?;
    let numerator = sum.value();
    if !numerator.is_finite() {
        return Err("triangular substitution scaled accumulation diverged".into());
    }
    let (mantissa, power) = split_finite(numerator);
    let (pivot_mantissa, pivot_power) = split_finite(pivot);
    let result_power = exponent
        .checked_add(power)
        .and_then(|value| value.checked_sub(pivot_power))
        .ok_or("triangular substitution exponent is outside the supported range")?;
    let quotient = mantissa / pivot_mantissa;
    let result = restore(quotient, result_power);
    if !result.is_finite()
        || (!matches!(policy, SubnormalResult::ValidateOriginalEquations)
            && numerator != 0.0
            && result == 0.0)
    {
        return Err("triangular substitution solution is outside the representable range".into());
    }
    if matches!(policy, SubnormalResult::RequireExactRange) {
        let inverse = result_power
            .checked_neg()
            .ok_or("triangular substitution restoration would lose numerical range")?;
        if restore(result, inverse) != quotient {
            return Err("triangular substitution restoration would lose numerical range".into());
        }
    }
    Ok(result)
}

fn normalized_term(value: f64, source: i32, target: i32) -> Result<f64, String> {
    if value == 0.0 {
        return Ok(value);
    }
    let shift = source
        .checked_sub(target)
        .ok_or("triangular substitution normalization would lose numerical range")?;
    normalize(value, shift)
}

pub(crate) fn normalize(value: f64, exponent: i32) -> Result<f64, String> {
    if value == 0.0 {
        return Ok(value);
    }
    let inverse = exponent
        .checked_neg()
        .ok_or("triangular substitution normalization would lose numerical range")?;
    let scaled = restore(value, exponent);
    if !scaled.is_finite() || restore(scaled, inverse) != value {
        return Err("triangular substitution normalization would lose numerical range".into());
    }
    Ok(scaled)
}

#[derive(Default)]
struct CompensatedSum {
    sum: f64,
    correction: f64,
}

impl CompensatedSum {
    fn add(&mut self, value: f64) {
        let next = self.sum + value;
        self.correction += if self.sum.abs() >= value.abs() {
            (self.sum - next) + value
        } else {
            (value - next) + self.sum
        };
        self.sum = next;
    }

    fn value(self) -> f64 {
        self.sum + self.correction
    }
}
