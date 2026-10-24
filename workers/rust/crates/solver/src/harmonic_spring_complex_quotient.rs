use super::division_product::Product;
use super::{Complex, Scaled};
use crate::solver_control::{SolverStage, checkpoint};
use std::cmp::Ordering;

// Finite binary64 products span at most 4,197 bits including their sum carry.
// Reserve additional space for the 53-bit quotient alignment, without a heap.
const LIMBS: usize = 70;
const CAPACITY: usize = LIMBS * 64;
const FRACTION: u64 = (1 << 52) - 1;
const CAPACITY_ERROR: &str = "complex exact division capacity exceeded";
const RANGE_ERROR: &str = "complex solution is outside the representable range";

pub(super) fn divide(
    real: Scaled,
    imaginary: Scaled,
    pivot: Complex,
    stage: SolverStage,
) -> Result<Complex, String> {
    let denominator = Pair::products([(pivot.re, pivot.re, 0), (pivot.im, pivot.im, 0)], stage)?;
    let nr = Pair::products(
        [
            (real.value, pivot.re, real.exponent),
            (imaginary.value, pivot.im, imaginary.exponent),
        ],
        stage,
    )?;
    let ni = Pair::products(
        [
            (imaginary.value, pivot.re, imaginary.exponent),
            (-real.value, pivot.im, real.exponent),
        ],
        stage,
    )?;
    let re = nr.quotient(&denominator, stage)?;
    let im = ni.quotient(&denominator, stage)?;
    Ok(Complex { re, im })
}

struct Pair {
    magnitude: Magnitude,
    power: i64,
    negative: bool,
}

impl Pair {
    fn products(terms: [(f64, f64, i32); 2], stage: SolverStage) -> Result<Self, String> {
        checkpoint(stage, 0)?;
        let [a, b] = terms.map(Product::new);
        let (a, b) = (a?, b?);
        let power = [&a, &b]
            .into_iter()
            .filter(|p| p.units != 0)
            .map(|p| p.power)
            .min()
            .unwrap_or(0);
        let mut first = Magnitude::product(&a, power)?;
        let second = Magnitude::product(&b, power)?;
        let negative = if a.negative == b.negative {
            first.add(&second)?;
            a.negative
        } else if first.cmp(&second) != Ordering::Less {
            first.subtract(&second)?;
            a.negative
        } else {
            let mut swapped = second;
            swapped.subtract(&first)?;
            first = swapped;
            b.negative
        };
        Ok(Self {
            magnitude: first,
            power,
            negative,
        })
    }

    fn quotient(&self, denominator: &Self, stage: SolverStage) -> Result<f64, String> {
        checkpoint(stage, 1)?;
        let (n, d) = (&self.magnitude, &denominator.magnitude);
        if d.bits() == 0 || denominator.negative {
            return Err("complex exact division requires a positive denominator".into());
        }
        if n.bits() == 0 {
            return Ok(0.0);
        }
        let difference = n.bits() as i64 - d.bits() as i64;
        let ordering = if difference >= 0 {
            n.cmp(&d.shifted(difference)?)
        } else {
            n.shifted(-difference)?.cmp(d)
        };
        let exponent =
            self.power - denominator.power + difference - i64::from(ordering == Ordering::Less);
        if !(-1075..=1023).contains(&exponent) {
            return Err(RANGE_ERROR.into());
        }
        let quantum = (exponent - 52).max(-1074);
        let shift = self.power - denominator.power - quantum;
        let (mut remainder, divisor) = if shift >= 0 {
            (n.shifted(shift)?, d.clone())
        } else {
            (n.clone(), d.shifted(-shift)?)
        };
        let mut integer = 0_u64;
        // Each subtraction chooses one of at most 53 output bits. No rounded
        // floating numerator, squared denominator or intermediate reciprocal.
        for bit in (0..=52).rev() {
            if divisor.bits() + bit <= remainder.bits() + 1 {
                let trial = divisor.shifted(bit as i64)?;
                if remainder.cmp(&trial) != Ordering::Less {
                    remainder.subtract(&trial)?;
                    integer |= 1 << bit;
                }
            }
            if bit % 16 == 0 {
                checkpoint(stage, 64 - bit)?;
            }
        }
        let halfway = remainder.shifted(1)?.cmp(&divisor);
        if halfway == Ordering::Greater || (halfway == Ordering::Equal && integer & 1 != 0) {
            integer += 1;
        }
        let mut exponent = exponent;
        let bits = if quantum == -1074 {
            integer
        } else {
            if integer == 1 << 53 {
                integer >>= 1;
                exponent += 1;
            }
            if exponent > 1023 {
                return Err(RANGE_ERROR.into());
            }
            (((exponent + 1023) as u64) << 52) | (integer & FRACTION)
        };
        if bits == 0 {
            return Err(RANGE_ERROR.into());
        }
        checkpoint(stage, 65)?;
        Ok(f64::from_bits(bits | (u64::from(self.negative) << 63)))
    }
}

#[derive(Clone)]
struct Magnitude([u64; LIMBS]);

impl Magnitude {
    fn product(product: &Product, base: i64) -> Result<Self, String> {
        let mut value = Self([0; LIMBS]);
        if product.units == 0 {
            return Ok(value);
        }
        value.0[0] = product.units as u64;
        value.0[1] = (product.units >> 64) as u64;
        value.shifted(product.power - base)
    }

    fn bits(&self) -> usize {
        self.0
            .iter()
            .rposition(|&word| word != 0)
            .map_or(0, |i| 64 * i + 64 - self.0[i].leading_zeros() as usize)
    }

    fn cmp(&self, other: &Self) -> Ordering {
        self.0.iter().rev().cmp(other.0.iter().rev())
    }

    fn shifted(&self, shift: i64) -> Result<Self, String> {
        let mut result = Self([0; LIMBS]);
        let bits = self.bits();
        if bits == 0 {
            return Ok(result);
        }
        let shift = usize::try_from(shift).map_err(|_| CAPACITY_ERROR)?;
        if shift > CAPACITY - bits {
            return Err(CAPACITY_ERROR.into());
        }
        let (words, offset) = (shift / 64, shift % 64);
        for (index, &word) in self.0[..bits.div_ceil(64)].iter().enumerate() {
            result.0[index + words] |= word << offset;
            if offset > 0 && word >> (64 - offset) != 0 {
                result.0[index + words + 1] |= word >> (64 - offset);
            }
        }
        Ok(result)
    }

    fn add(&mut self, other: &Self) -> Result<(), String> {
        let mut carry = false;
        for (word, &rhs) in self.0.iter_mut().zip(&other.0) {
            let (next, a) = word.overflowing_add(rhs);
            let (next, b) = next.overflowing_add(u64::from(carry));
            *word = next;
            carry = a || b;
        }
        if carry {
            Err(CAPACITY_ERROR.into())
        } else {
            Ok(())
        }
    }

    fn subtract(&mut self, other: &Self) -> Result<(), String> {
        let mut borrow = false;
        for (word, &rhs) in self.0.iter_mut().zip(&other.0) {
            let (next, a) = word.overflowing_sub(rhs);
            let (next, b) = next.overflowing_sub(u64::from(borrow));
            *word = next;
            borrow = a || b;
        }
        if borrow {
            Err("complex exact division subtraction underflowed".into())
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
#[path = "harmonic_spring_complex_quotient_tests.rs"]
mod tests;
