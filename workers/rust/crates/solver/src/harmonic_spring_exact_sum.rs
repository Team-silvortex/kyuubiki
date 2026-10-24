use std::cmp::Ordering;

const LIMBS: usize = 18;
const FRACTION: u64 = (1 << 52) - 1;

// Parts aligned by sum_products have magnitude <= 4. Each is an integer multiple
// of 2^-1074; 1,152 bits leave room for sums below 2^78, with checked carries.
#[derive(Default)]
pub(super) struct ExactSum {
    positive: [u64; LIMBS],
    negative: [u64; LIMBS],
    failed: bool,
}

impl ExactSum {
    pub(super) fn add(&mut self, value: f64) -> Result<(), String> {
        if self.failed {
            return Err("complex exact accumulation is invalid".into());
        }
        let result = self.add_part(value);
        self.failed = result.is_err();
        result
    }

    fn add_part(&mut self, value: f64) -> Result<(), String> {
        if !value.is_finite() || value.abs() > 4.0 {
            return Err("complex exact accumulation requires a finite normalized part".into());
        }
        let bits = value.to_bits();
        let exponent = ((bits >> 52) & 0x7ff) as usize;
        let significand = (bits & FRACTION) | if exponent == 0 { 0 } else { 1 << 52 };
        if significand == 0 {
            return Ok(());
        }
        let shift = exponent.saturating_sub(1);
        let word = shift / 64;
        let offset = shift % 64;
        let limbs = if value.is_sign_negative() {
            &mut self.negative
        } else {
            &mut self.positive
        };
        add_word(limbs, word, significand << offset)?;
        if offset > 0 {
            add_word(limbs, word + 1, significand >> (64 - offset))?;
        }
        Ok(())
    }

    pub(super) fn finish(mut self) -> Result<f64, String> {
        if self.failed {
            return Err("complex exact accumulation is invalid".into());
        }
        let negative = match self.positive.iter().rev().cmp(self.negative.iter().rev()) {
            Ordering::Equal => return Ok(0.0),
            Ordering::Less => true,
            Ordering::Greater => false,
        };
        let (magnitude, smaller) = if negative {
            (&mut self.negative, &self.positive)
        } else {
            (&mut self.positive, &self.negative)
        };
        let mut borrow = false;
        for (value, &subtrahend) in magnitude.iter_mut().zip(smaller) {
            let (next, first) = value.overflowing_sub(subtrahend);
            let (next, second) = next.overflowing_sub(u64::from(borrow));
            *value = next;
            borrow = first || second;
        }
        if borrow {
            return Err("complex exact accumulation subtraction underflowed".into());
        }
        rounded(magnitude, negative)
    }
}

fn add_word(limbs: &mut [u64; LIMBS], mut index: usize, mut value: u64) -> Result<(), String> {
    while value != 0 {
        let Some(word) = limbs.get_mut(index) else {
            return Err("complex exact accumulation capacity exceeded".into());
        };
        let (next, carry) = word.overflowing_add(value);
        *word = next;
        value = u64::from(carry);
        index += 1;
    }
    Ok(())
}

fn rounded(magnitude: &[u64; LIMBS], negative: bool) -> Result<f64, String> {
    let word = magnitude
        .iter()
        .rposition(|&value| value != 0)
        .ok_or("complex exact accumulation lost its magnitude")?;
    let mut top = word * 64 + 63 - magnitude[word].leading_zeros() as usize;
    let sign = u64::from(negative) << 63;
    if top < 52 {
        return Ok(f64::from_bits(sign | magnitude[0]));
    }
    let drop = top - 52;
    let word = drop / 64;
    let offset = drop % 64;
    let mut significand = magnitude[word] >> offset;
    if offset > 0 && word + 1 < LIMBS {
        significand |= magnitude[word + 1] << (64 - offset);
    }
    // Round the integer lattice once, retaining both the half bit and all sticky bits.
    if drop > 0
        && bit(magnitude, drop - 1)
        && (any_below(magnitude, drop - 1) || significand & 1 != 0)
    {
        significand += 1;
        if significand == 1 << 53 {
            significand >>= 1;
            top += 1;
        }
    }
    let exponent = top - 51;
    if exponent >= 0x7ff {
        return Err("complex exact accumulation result is outside the representable range".into());
    }
    Ok(f64::from_bits(
        sign | ((exponent as u64) << 52) | (significand & FRACTION),
    ))
}

fn bit(magnitude: &[u64; LIMBS], index: usize) -> bool {
    magnitude[index / 64] & (1 << (index % 64)) != 0
}

fn any_below(magnitude: &[u64; LIMBS], end: usize) -> bool {
    let word = end / 64;
    let offset = end % 64;
    magnitude[..word].iter().any(|&value| value != 0)
        || (offset > 0 && magnitude[word] & ((1 << offset) - 1) != 0)
}

#[cfg(test)]
#[path = "harmonic_spring_exact_sum_tests.rs"]
mod tests;
