use crate::linear_binary_scale::{restore as scale, split_finite as split};
use crate::solver_control::{SolverStage, checkpoint};

#[derive(Clone, Copy)]
pub(super) struct PositiveDot {
    mantissa: f64,
    exponent: i32,
}

impl PositiveDot {
    pub(super) fn ratio(self, denominator: Self) -> f64 {
        scale(
            self.mantissa / denominator.mantissa,
            self.exponent - denominator.exponent,
        )
    }

    fn from_positive(value: f64, exponent: i32) -> Option<Self> {
        if !value.is_finite() || value <= 0.0 {
            return None;
        }
        let (mantissa, power) = split(value);
        Some(Self {
            mantissa,
            exponent: exponent + power,
        })
    }
}

/// Retain the exponent until PCG forms alpha/beta, rather than rounding a tiny
/// positive inner product to zero. Ordinary dots keep the existing single pass.
pub(super) fn positive_dot(lhs: &[f64], rhs: &[f64]) -> Result<Option<PositiveDot>, String> {
    if lhs.len() != rhs.len() {
        return Err("PCG inner product dimensions must match".into());
    }
    let direct = super::vectors::dot(lhs, rhs)?;
    if direct.is_normal() && direct > 0.0 {
        return Ok(PositiveDot::from_positive(direct, 0));
    }

    let mut exponent = i32::MIN;
    checkpoint(SolverStage::PcgDot, 0)?;
    for (block, values) in lhs.chunks(1024).enumerate() {
        let start = block * 1024;
        for (&left, &right) in values.iter().zip(&rhs[start..]) {
            if !left.is_finite() || !right.is_finite() {
                return Ok(None);
            }
            if left != 0.0 && right != 0.0 {
                exponent = exponent.max(split(left).1 + split(right).1);
            }
        }
        checkpoint(SolverStage::PcgDot, start + values.len())?;
    }
    if exponent == i32::MIN {
        return Ok(None);
    }

    let mut sum = 0.0;
    let mut correction = 0.0;
    checkpoint(SolverStage::PcgDot, 0)?;
    for (block, values) in lhs.chunks(1024).enumerate() {
        let start = block * 1024;
        for (&left, &right) in values.iter().zip(&rhs[start..]) {
            let (left, le) = split(left);
            let (right, re) = split(right);
            let high = left * right;
            let low = left.mul_add(right, -high);
            for term in [high, low] {
                let term = scale(term, le + re - exponent);
                let next = sum + term;
                correction += if sum.abs() >= term.abs() {
                    (sum - next) + term
                } else {
                    (term - next) + sum
                };
                sum = next;
            }
        }
        checkpoint(SolverStage::PcgDot, start + values.len())?;
    }
    // This is still finite-precision iteration, not an equilibrium certificate.
    // Original-equation validation remains mandatory after restoring the solution.
    Ok(PositiveDot::from_positive(sum + correction, exponent))
}

#[cfg(test)]
#[path = "linear_spd_inner_product_tests.rs"]
mod tests;
