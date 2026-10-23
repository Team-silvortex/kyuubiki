use crate::linear_binary_scale::{restore, split_finite};
use crate::solver_control::{SolverStage, check_cancellation, checkpoint, checkpoint_chunk};

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
    scaled_row(rhs, operands, pivot, stage)
}

fn scaled_row<I>(rhs: f64, operands: I, pivot: f64, stage: SolverStage) -> Result<f64, String>
where
    I: ExactSizeIterator<Item = (f64, f64)> + Clone,
{
    checkpoint(stage, 0)?;
    let (rhs_mantissa, rhs_exponent) = split_finite(rhs);
    let mut exponent = if rhs == 0.0 { i32::MIN } else { rhs_exponent };
    let length = operands.len();
    for (index, (coefficient, value)) in operands.clone().enumerate() {
        if coefficient != 0.0 && value != 0.0 {
            exponent = exponent.max(split_finite(coefficient).1 + split_finite(value).1);
        }
        checkpoint_chunk(stage, index + 1, length)?;
    }
    if exponent == i32::MIN {
        exponent = 0;
    }
    let mut sum = CompensatedSum::default();
    sum.add(normalize(rhs_mantissa, rhs_exponent - exponent)?);
    for (index, (coefficient, value)) in operands.enumerate() {
        let (left, le) = split_finite(coefficient);
        let (right, re) = split_finite(value);
        let high = left * right;
        let low = left.mul_add(right, -high);
        sum.add(-normalize(high, le + re - exponent)?);
        sum.add(-normalize(low, le + re - exponent)?);
        checkpoint_chunk(stage, length + index + 1, 2 * length)?;
    }
    checkpoint(stage, 2 * length)?;
    let numerator = sum.value();
    if !numerator.is_finite() {
        return Err("triangular substitution scaled accumulation diverged".into());
    }
    let (mantissa, power) = split_finite(numerator);
    let (pivot_mantissa, pivot_power) = split_finite(pivot);
    let result = restore(mantissa / pivot_mantissa, exponent + power - pivot_power);
    if !result.is_finite() || (numerator != 0.0 && result == 0.0) {
        return Err("triangular substitution solution is outside the representable range".into());
    }
    Ok(result)
}

fn normalize(value: f64, exponent: i32) -> Result<f64, String> {
    let scaled = restore(value, exponent);
    if !scaled.is_finite() || restore(scaled, -exponent) != value {
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
