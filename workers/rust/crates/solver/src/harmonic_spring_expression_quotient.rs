use super::super::{Accumulated, Complex};
use super::{CAPACITY_ERROR, LIMBS, Magnitude, Pair, Product};
use crate::solver_control::{SolverStage, checkpoint};
use std::cmp::Ordering;

// Only replayed substitution rows enter here. Preserve their admitted exact
// integer sum until quotient rounding, rather than rounding a scaled numerator.
pub(in super::super) fn divide_accumulated(
    real: Accumulated,
    imaginary: Accumulated,
    pivot: Complex,
    stage: SolverStage,
) -> Result<Complex, String> {
    let real = Pair::accumulated(real)?;
    let imaginary = Pair::accumulated(imaginary)?;
    let denominator = Pair::products([(pivot.re, pivot.re, 0), (pivot.im, pivot.im, 0)], stage)?;
    let nr = real
        .multiplied(pivot.re, stage)?
        .combined(imaginary.multiplied(pivot.im, stage)?)?;
    let ni = imaginary
        .multiplied(pivot.re, stage)?
        .combined(real.multiplied(-pivot.im, stage)?)?;
    Ok(Complex {
        re: nr.quotient(&denominator, stage)?,
        im: ni.quotient(&denominator, stage)?,
    })
}

impl Pair {
    fn accumulated((sum, exponent): Accumulated) -> Result<Self, String> {
        let (words, negative) = sum.into_magnitude()?;
        let mut magnitude = Magnitude([0; LIMBS]);
        magnitude.0[..words.len()].copy_from_slice(&words);
        Ok(Self {
            magnitude,
            power: i64::from(exponent) - 1074,
            negative,
        })
    }

    fn multiplied(&self, scalar: f64, stage: SolverStage) -> Result<Self, String> {
        checkpoint(stage, 0)?;
        let scalar = Product::new((scalar, 1.0, 0))?;
        if scalar.units == 0 || self.magnitude.bits() == 0 {
            return Ok(Self {
                magnitude: Magnitude([0; LIMBS]),
                power: 0,
                negative: false,
            });
        }
        // Remove the exact one's significand and all remaining binary zeros.
        let shift = scalar.units.trailing_zeros();
        let units = u64::try_from(scalar.units >> shift).map_err(|_| CAPACITY_ERROR)?;
        Ok(Self {
            magnitude: self.magnitude.multiplied(units)?,
            power: self.power + scalar.power + i64::from(shift),
            negative: self.negative ^ scalar.negative,
        })
    }

    fn combined(self, other: Self) -> Result<Self, String> {
        if self.magnitude.bits() == 0 {
            return Ok(other);
        }
        if other.magnitude.bits() == 0 {
            return Ok(self);
        }
        let power = self.power.min(other.power);
        let mut left = self.magnitude.shifted(self.power - power)?;
        let right = other.magnitude.shifted(other.power - power)?;
        let negative = if self.negative == other.negative {
            left.add(&right)?;
            self.negative
        } else if left.cmp(&right) != Ordering::Less {
            left.subtract(&right)?;
            self.negative
        } else {
            let mut swapped = right;
            swapped.subtract(&left)?;
            left = swapped;
            other.negative
        };
        Ok(Self {
            magnitude: left,
            power,
            negative,
        })
    }
}

impl Magnitude {
    fn multiplied(&self, scalar: u64) -> Result<Self, String> {
        let mut result = Self([0; LIMBS]);
        let mut carry = 0_u128;
        for (out, &word) in result.0.iter_mut().zip(&self.0) {
            let product = u128::from(word) * u128::from(scalar) + carry;
            *out = product as u64;
            carry = product >> 64;
        }
        if carry != 0 {
            return Err(CAPACITY_ERROR.into());
        }
        Ok(result)
    }
}

#[cfg(test)]
#[path = "harmonic_spring_expression_quotient_tests.rs"]
mod tests;
