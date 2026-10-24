use std::cmp::Ordering;

const LIMBS: usize = 18;
const FRACTION: u64 = (1 << 52) - 1;

// Parts aligned by accumulate_products have magnitude <= 4. Each is an integer multiple
// of 2^-1074; 1,152 bits leave room for sums below 2^78, with checked carries.
pub(super) type ExactSum = Accumulator<LIMBS>;

pub(super) struct Accumulator<const N: usize> {
    positive: [u64; N],
    negative: [u64; N],
    failed: bool,
}

impl<const N: usize> Default for Accumulator<N> {
    fn default() -> Self {
        Self {
            positive: [0; N],
            negative: [0; N],
            failed: false,
        }
    }
}

// A finite binary64 value needs 2,098 lattice bits. 34 limbs also cover
// the carry from a usize-bounded number of assembly contributions.
#[derive(Default)]
pub(crate) struct CoefficientSum(Accumulator<34>);

// Triple products have a 2^-3222 lattice. Include finite coefficient range
// and a usize-bounded sum carry without allocating an accumulator per cell.
#[derive(Default)]
pub(crate) struct InertiaSum(Accumulator<68>);

impl InertiaSum {
    pub(crate) fn add_scaled(&mut self, value: f64, exponent: i32) -> Result<(), String> {
        if self.0.failed {
            return Err("complex exact accumulation is invalid".into());
        }
        let result = self.0.add_scaled_part(value, exponent, -3222);
        self.0.failed = result.is_err();
        result
    }

    pub(crate) fn finish(self) -> Result<f64, String> {
        let (magnitude, negative) = self.0.into_magnitude()?;
        if magnitude.iter().all(|&word| word == 0) {
            return Ok(0.0);
        }
        rounded(&magnitude, negative, 2148)
    }
}

impl CoefficientSum {
    pub(crate) fn add(&mut self, value: f64) -> Result<(), String> {
        self.0.add_finite(value)
    }

    pub(crate) fn finish(self) -> Result<f64, String> {
        self.0.finish()
    }
}

impl<const N: usize> Accumulator<N> {
    pub(super) fn add(&mut self, value: f64) -> Result<(), String> {
        if !value.is_finite() || value.abs() > 4.0 {
            self.failed = true;
            return Err("complex exact accumulation requires a finite normalized part".into());
        }
        self.add_finite(value)
    }

    fn add_finite(&mut self, value: f64) -> Result<(), String> {
        if self.failed {
            return Err("complex exact accumulation is invalid".into());
        }
        let result = self.add_part(value);
        self.failed = result.is_err();
        result
    }

    fn add_part(&mut self, value: f64) -> Result<(), String> {
        self.add_scaled_part(value, 0, -1074)
    }

    fn add_scaled_part(&mut self, value: f64, extra: i32, base: i64) -> Result<(), String> {
        if !value.is_finite() {
            return Err("complex exact accumulation requires a finite part".into());
        }
        let bits = value.to_bits();
        let exponent = ((bits >> 52) & 0x7ff) as usize;
        let mut significand = (bits & FRACTION) | if exponent == 0 { 0 } else { 1 << 52 };
        if significand == 0 {
            return Ok(());
        }
        let power = if exponent == 0 {
            -1074
        } else {
            exponent as i64 - 1075
        };
        let mut shift = power + i64::from(extra) - base;
        if shift < 0 {
            let right = -shift;
            if right >= 64 || i64::from(significand.trailing_zeros()) < right {
                return Err("complex exact accumulation would lose numerical range".into());
            }
            significand >>= right;
            shift = 0;
        }
        let shift =
            usize::try_from(shift).map_err(|_| "complex exact accumulation capacity exceeded")?;
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

    pub(super) fn finish(self) -> Result<f64, String> {
        let (magnitude, negative) = self.into_magnitude()?;
        if magnitude.iter().all(|&word| word == 0) {
            return Ok(0.0);
        }
        rounded(&magnitude, negative, 0)
    }

    pub(super) fn into_magnitude(mut self) -> Result<([u64; N], bool), String> {
        if self.failed {
            return Err("complex exact accumulation is invalid".into());
        }
        let negative = match self.positive.iter().rev().cmp(self.negative.iter().rev()) {
            Ordering::Equal => return Ok(([0; N], false)),
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
        Ok((*magnitude, negative))
    }
}

fn add_word<const N: usize>(
    limbs: &mut [u64; N],
    mut index: usize,
    mut value: u64,
) -> Result<(), String> {
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

fn rounded<const N: usize>(
    magnitude: &[u64; N],
    negative: bool,
    quantum: usize,
) -> Result<f64, String> {
    let word = magnitude
        .iter()
        .rposition(|&value| value != 0)
        .ok_or("complex exact accumulation lost its magnitude")?;
    let mut top = word * 64 + 63 - magnitude[word].leading_zeros() as usize;
    let sign = u64::from(negative) << 63;
    if top < quantum {
        return Err("complex exact accumulation would lose numerical range".into());
    }
    let drop = top.saturating_sub(52).max(quantum);
    let word = drop / 64;
    let offset = drop % 64;
    let mut significand = magnitude[word] >> offset;
    if offset > 0 && word + 1 < N {
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
    if top < quantum + 52 {
        if significand < 1 << 52 && any_below(magnitude, drop) {
            return Err("complex exact accumulation would lose numerical range".into());
        }
        return Ok(f64::from_bits(sign | significand));
    }
    let exponent = top - quantum - 51;
    if exponent >= 0x7ff {
        return Err("complex exact accumulation result is outside the representable range".into());
    }
    Ok(f64::from_bits(
        sign | ((exponent as u64) << 52) | (significand & FRACTION),
    ))
}

fn bit<const N: usize>(magnitude: &[u64; N], index: usize) -> bool {
    magnitude[index / 64] & (1 << (index % 64)) != 0
}

fn any_below<const N: usize>(magnitude: &[u64; N], end: usize) -> bool {
    let word = end / 64;
    let offset = end % 64;
    magnitude[..word].iter().any(|&value| value != 0)
        || (offset > 0 && magnitude[word] & ((1 << offset) - 1) != 0)
}

#[cfg(test)]
#[path = "harmonic_spring_exact_sum_tests.rs"]
mod tests;
