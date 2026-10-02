use super::SparseMassNormalizedOperator;
use crate::solver_control::{SolverStage, checkpoint, checkpoint_chunk};

impl SparseMassNormalizedOperator {
    pub(super) fn apply_balanced<const COMPENSATED: bool>(
        &self,
        vector: &[f64],
    ) -> Result<Vec<f64>, String> {
        let mut result = Vec::with_capacity(vector.len());
        for row in 0..vector.len() {
            let start = self.stiffness.row_offsets[row];
            let end = self.stiffness.row_offsets[row + 1];
            let mut sum = 0.0_f64;
            let mut correction = 0.0;
            for entry in start..end {
                let column = self.stiffness.columns[entry];
                let (product, low) = balanced_product::<COMPENSATED>([
                    self.stiffness.values[entry],
                    vector[column],
                    self.inverse_mass_sqrt[column],
                    self.inverse_mass_sqrt[row],
                ]);
                if !product.is_finite() || !low.is_finite() {
                    return Err("sparse modal stiffness product must remain finite".into());
                }
                let next = sum + product;
                if COMPENSATED {
                    correction += if sum.abs() >= product.abs() {
                        (sum - next) + product
                    } else {
                        (product - next) + sum
                    };
                    correction += low;
                }
                sum = next;
                if end - start > 1024 && (entry - start + 1) % 1024 == 0 {
                    checkpoint(SolverStage::SparseMatvecRow, entry - start + 1)?;
                }
            }
            let value = sum + correction;
            if !value.is_finite() {
                return Err("sparse modal stiffness product must remain finite".into());
            }
            result.push(value);
            checkpoint_chunk(SolverStage::SparseMatvec, row + 1, vector.len())?;
        }
        Ok(result)
    }
}

fn balanced_product<const COMPENSATED: bool>(factors: [f64; 4]) -> (f64, f64) {
    if factors.contains(&0.0) {
        return (0.0, 0.0);
    }
    let negative = factors
        .iter()
        .filter(|value| value.is_sign_negative())
        .count()
        % 2
        != 0;
    let mut product = 1.0;
    let mut correction = 0.0;
    let mut exponent = 0;
    // Multiply bounded mantissas of the original physical factors. Restore their
    // combined exponent only once: even a balanced order can round subnormals early.
    for factor in factors {
        let (mantissa, power) = normalized_factor(factor.abs());
        exponent += power;
        let next = product * mantissa;
        if COMPENSATED {
            correction = product.mul_add(mantissa, -next) + correction * mantissa;
        }
        product = next;
    }
    let (product, correction) = restore_scale(product, correction, exponent);
    if negative {
        (-product, -correction)
    } else {
        (product, correction)
    }
}

fn normalized_factor(mut value: f64) -> (f64, i32) {
    let mut adjustment = 0;
    if value < f64::MIN_POSITIVE {
        value *= 2.0_f64.powi(54);
        adjustment = -54;
    }
    let bits = value.to_bits();
    let exponent = ((bits >> 52) & 0x7ff) as i32 - 1023 + adjustment;
    let mantissa = f64::from_bits((bits & ((1_u64 << 52) - 1)) | (1023_u64 << 52));
    (mantissa, exponent)
}

fn power_of_two(exponent: i32) -> f64 {
    debug_assert!((-1022..=1023).contains(&exponent));
    f64::from_bits(((exponent + 1023) as u64) << 52)
}

fn restore_scale(product: f64, correction: f64, exponent: i32) -> (f64, f64) {
    if exponent > 1023 {
        return (f64::INFINITY, 0.0);
    }
    if exponent >= -1022 {
        let scale = power_of_two(exponent);
        return (product * scale, correction * scale);
    }
    if exponent < -1078 {
        return (0.0, 0.0);
    }
    let value = ((product + correction) * f64::MIN_POSITIVE) * power_of_two(exponent + 1022);
    (value, 0.0)
}
