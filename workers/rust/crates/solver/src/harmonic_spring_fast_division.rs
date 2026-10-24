use super::{Complex, division_product::Product};
use crate::linear_binary_scale::restore;
use std::cmp::Ordering;

const FRACTION: u64 = (1 << 52) - 1;

pub(super) fn divide(left: Complex, pivot: Complex) -> Option<Complex> {
    let denominator = SmallPair::products([(pivot.re, pivot.re), (pivot.im, pivot.im)])?;
    if denominator.units == 0 {
        return None;
    }
    let real = SmallPair::products([(left.re, pivot.re), (left.im, pivot.im)])?;
    let imaginary = SmallPair::products([(left.im, pivot.re), (-left.re, pivot.im)])?;
    Some(Complex {
        re: component(&real, &denominator)?,
        im: component(&imaginary, &denominator)?,
    })
}

struct SmallPair {
    units: u128,
    power: i64,
    negative: bool,
}

impl SmallPair {
    fn products(terms: [(f64, f64); 2]) -> Option<Self> {
        let [a, b] = terms.map(|(a, b)| Product::new((a, b, 0)).ok());
        let (a, b) = (a?, b?);
        let power = [&a, &b]
            .into_iter()
            .filter(|p| p.units != 0)
            .map(|p| p.power)
            .min()
            .unwrap_or(0);
        let aligned = |p: &Product| {
            if p.units == 0 {
                return Some(0);
            }
            let shift = u32::try_from(p.power - power).ok()?;
            if u64::from(128 - p.units.leading_zeros()) + u64::from(shift) > 126 {
                return None;
            }
            Some(p.units << shift)
        };
        let (first, second) = (aligned(&a)?, aligned(&b)?);
        let (units, negative) = if a.negative == b.negative {
            (first.checked_add(second)?, a.negative)
        } else if first >= second {
            (first - second, a.negative)
        } else {
            (second - first, b.negative)
        };
        if units.leading_zeros() < 2 {
            return None;
        }
        let peak = first.max(second);
        // Keep strongly cancelling expressions on the established observed
        // replay path, even when the small integer pair happens to fit.
        if peak != 0 && units <= peak >> 26 {
            return None;
        }
        Some(Self {
            units,
            power,
            negative,
        })
    }
}

fn component(n: &SmallPair, d: &SmallPair) -> Option<f64> {
    if n.units == 0 {
        return Some(0.0);
    }
    let split = |integer: u128| {
        let high = integer as f64;
        let low = (integer as i128 - high as i128) as f64;
        (high, low)
    };
    let (nh, nl) = split(n.units);
    let (dh, dl) = split(d.units);
    let estimate = nh / dh;
    let residual = (-estimate).mul_add(dl, (-estimate).mul_add(dh, nh) + nl);
    let refined = estimate + residual / dh;
    let candidate = restore(refined, i32::try_from(n.power - d.power).ok()?);
    // Approximation quality is not the admission rule. Compare the EXACT
    // rational with both rounding-cell midpoints, including ties-to-even.
    if !candidate.is_normal() || !in_rounding_cell(n, d, candidate)? {
        return None;
    }
    Some(if n.negative { -candidate } else { candidate })
}

fn in_rounding_cell(n: &SmallPair, d: &SmallPair, candidate: f64) -> Option<bool> {
    if !candidate.is_normal() || candidate.is_sign_negative() || d.units == 0 {
        return Some(false);
    }
    let bits = candidate.to_bits();
    let significand = (bits & FRACTION) | (1 << 52);
    let field = ((bits >> 52) & 0x7ff) as i64;
    let power = field - 1075;
    let (lower, lower_power) = if significand == 1 << 52 && field > 1 {
        (4 * significand - 1, power - 2)
    } else {
        (2 * significand - 1, power - 1)
    };
    let compare = |midpoint, midpoint_power| {
        let a = Integer192::from(n.units);
        let b = Integer192::from(d.units).times(midpoint)?;
        let shift = n.power - d.power - midpoint_power;
        Some(if shift >= 0 {
            a.shifted(shift)?.cmp(&b)
        } else {
            a.cmp(&b.shifted(-shift)?)
        })
    };
    let lo = compare(lower, lower_power)?;
    let hi = compare(2 * significand + 1, power - 1)?;
    let even = significand & 1 == 0;
    Some(
        (lo == Ordering::Greater || (lo == Ordering::Equal && even))
            && (hi == Ordering::Less || (hi == Ordering::Equal && even)),
    )
}

struct Integer192([u64; 3]);

impl From<u128> for Integer192 {
    fn from(value: u128) -> Self {
        Self([value as u64, (value >> 64) as u64, 0])
    }
}

impl Integer192 {
    fn times(&self, factor: u64) -> Option<Self> {
        let mut result = Self([0; 3]);
        let mut carry = 0_u128;
        for (word, &value) in result.0.iter_mut().zip(&self.0) {
            let next = u128::from(value) * u128::from(factor) + carry;
            *word = next as u64;
            carry = next >> 64;
        }
        (carry == 0).then_some(result)
    }

    fn shifted(&self, shift: i64) -> Option<Self> {
        let mut result = Self([0; 3]);
        let Some(top) = self.0.iter().rposition(|&word| word != 0) else {
            return Some(result);
        };
        let bits = top * 64 + 64 - self.0[top].leading_zeros() as usize;
        let shift = usize::try_from(shift).ok()?;
        if shift > 192 - bits {
            return None;
        }
        let (words, offset) = (shift / 64, shift % 64);
        for (index, &word) in self.0[..=top].iter().enumerate() {
            result.0[index + words] |= word << offset;
            if offset > 0 && word >> (64 - offset) != 0 {
                result.0[index + words + 1] |= word >> (64 - offset);
            }
        }
        Some(result)
    }

    fn cmp(&self, other: &Self) -> Ordering {
        self.0.iter().rev().cmp(other.0.iter().rev())
    }
}

#[cfg(test)]
#[path = "harmonic_spring_fast_certificate_tests.rs"]
mod tests;
