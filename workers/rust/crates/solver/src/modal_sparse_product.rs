use super::SparseMassNormalizedOperator;
use crate::solver_control::{SolverStage, checkpoint, checkpoint_chunk};

#[path = "modal_sparse_product_range.rs"]
mod range;

#[inline(always)]
pub(super) fn staged_factor_is_safe(value: f64) -> bool {
    // Four such factors, plus any usize-bounded row sum, stay inside normal f64 range.
    // This chooses an algorithm; it never rejects a model or cuts a weak coupling.
    value == 0.0 || (1e-60..=1e60).contains(&value.abs())
}

impl SparseMassNormalizedOperator {
    pub(crate) fn apply(&self, vector: &[f64]) -> Result<Vec<f64>, String> {
        if vector.len() != self.inverse_mass_sqrt.len() {
            return Err("sparse modal operator vector dimensions must match".into());
        }
        checkpoint(SolverStage::SparseMatvec, 0)?;
        let mut staged = self.staged_range_safe;
        let mut scaled = if staged {
            Vec::with_capacity(vector.len())
        } else {
            Vec::new()
        };
        for (block, values) in vector.chunks(64).enumerate() {
            let start = block * 64;
            for (&value, &inverse) in values.iter().zip(&self.inverse_mass_sqrt[start..]) {
                if !value.is_finite() {
                    return Err("sparse modal operator vector must remain finite".into());
                }
                staged &= staged_factor_is_safe(value);
                if staged {
                    scaled.push(value * inverse);
                }
            }
            checkpoint(SolverStage::SparseMatvec, start + values.len())?;
        }
        if !staged {
            return self.apply_balanced::<false>(vector);
        }
        let mut product = vec![0.0; vector.len()];
        self.stiffness.multiply_vector_into(&scaled, &mut product)?;
        for (block, values) in product.chunks_mut(64).enumerate() {
            let start = block * 64;
            for (value, &inverse) in values.iter_mut().zip(&self.inverse_mass_sqrt[start..]) {
                *value *= inverse;
                if !value.is_finite() {
                    return Err("sparse modal stiffness product must remain finite".into());
                }
            }
            checkpoint(SolverStage::SparseMatvec, start + values.len())?;
        }
        Ok(product)
    }

    /// Evaluate the same physical K and mass scaling with compensated products and sums.
    /// Keep this extra work on the bounded dense-spectrum refinement/validation path.
    pub(crate) fn apply_compensated(&self, vector: &[f64]) -> Result<Vec<f64>, String> {
        if vector.len() != self.inverse_mass_sqrt.len() {
            return Err("sparse modal operator vector dimensions must match".into());
        }
        checkpoint(SolverStage::SparseMatvec, 0)?;
        let mut staged = self.staged_range_safe;
        let mut scaled = if staged {
            Vec::with_capacity(vector.len())
        } else {
            Vec::new()
        };
        for (block, values) in vector.chunks(64).enumerate() {
            let start = block * 64;
            for (&value, &inverse_mass) in values.iter().zip(&self.inverse_mass_sqrt[start..]) {
                if !value.is_finite() {
                    return Err("sparse modal operator vector must remain finite".into());
                }
                staged &= staged_factor_is_safe(value);
                if staged {
                    let high = value * inverse_mass;
                    let low = value.mul_add(inverse_mass, -high);
                    scaled.push((high, low));
                }
            }
            checkpoint(SolverStage::SparseMatvec, start + values.len())?;
        }
        if staged {
            self.apply_physical_parts(&scaled)
        } else {
            self.apply_balanced::<true>(vector)
        }
    }

    /// Evaluate M^-1/2 K phi directly on the published physical coordinates.
    pub(crate) fn apply_physical_compensated(&self, shape: &[f64]) -> Result<Vec<f64>, String> {
        if shape.len() != self.inverse_mass_sqrt.len() {
            return Err("physical modal shape dimensions must match".into());
        }
        checkpoint(SolverStage::SparseMatvec, 0)?;
        for (index, &value) in shape.iter().enumerate() {
            if !value.is_finite() {
                return Err("physical modal shape must remain finite".into());
            }
            checkpoint_chunk(SolverStage::SparseMatvec, index + 1, shape.len())?;
        }
        let mut result = Vec::with_capacity(shape.len());
        for row in 0..shape.len() {
            let start = self.stiffness.row_offsets[row];
            let end = self.stiffness.row_offsets[row + 1];
            let mut sum = 0.0_f64;
            let mut correction = 0.0;
            for entry in start..end {
                let (product, low) = physical_product(
                    self.stiffness.values[entry],
                    shape[self.stiffness.columns[entry]],
                    self.inverse_mass_sqrt[row],
                );
                let next = sum + product;
                correction += if sum.abs() >= product.abs() {
                    (sum - next) + product
                } else {
                    (product - next) + sum
                };
                correction += low;
                sum = next;
                if end - start > 1024 && (entry - start + 1) % 1024 == 0 {
                    checkpoint(SolverStage::SparseMatvecRow, entry - start + 1)?;
                }
            }
            let value = sum + correction;
            if !value.is_finite() {
                return Err("physical modal stiffness product must remain finite".into());
            }
            result.push(value);
            checkpoint_chunk(SolverStage::SparseMatvec, row + 1, shape.len())?;
        }
        Ok(result)
    }

    fn apply_physical_parts(&self, scaled: &[(f64, f64)]) -> Result<Vec<f64>, String> {
        let mut result = Vec::with_capacity(scaled.len());
        for row in 0..scaled.len() {
            let start = self.stiffness.row_offsets[row];
            let end = self.stiffness.row_offsets[row + 1];
            let mut sum = 0.0_f64;
            let mut correction = 0.0;
            for entry in start..end {
                let (high, low) = scaled[self.stiffness.columns[entry]];
                let coefficient = self.stiffness.values[entry];
                let product = coefficient * high;
                let next = sum + product;
                correction += if sum.abs() >= product.abs() {
                    (sum - next) + product
                } else {
                    (product - next) + sum
                };
                correction += coefficient.mul_add(high, -product) + coefficient * low;
                sum = next;
                if end - start > 1024 && (entry - start + 1) % 1024 == 0 {
                    checkpoint(SolverStage::SparseMatvecRow, entry - start + 1)?;
                }
            }
            let inverse_mass = self.inverse_mass_sqrt[row];
            let high = sum * inverse_mass;
            let value = high + (sum.mul_add(inverse_mass, -high) + correction * inverse_mass);
            if !value.is_finite() {
                return Err("compensated modal stiffness product must remain finite".into());
            }
            result.push(value);
            checkpoint_chunk(SolverStage::SparseMatvec, row + 1, scaled.len())?;
        }
        Ok(result)
    }
}

fn physical_product(stiffness: f64, shape: f64, inverse_mass: f64) -> (f64, f64) {
    if stiffness == 0.0 || shape == 0.0 {
        return (0.0, 0.0);
    }
    // Balance the three factors before multiplying: K*phi alone may be subnormal
    // even though M^-1/2*K*phi is normal. Keep both multiplication roundoff terms.
    let mut factors = [stiffness, shape, inverse_mass];
    factors.sort_unstable_by(|a, b| a.abs().total_cmp(&b.abs()));
    let [small, middle, large] = factors;
    let first = small * large;
    let error = small.mul_add(large, -first);
    let product = first * middle;
    (product, first.mul_add(middle, -product) + error * middle)
}

#[cfg(test)]
#[path = "modal_sparse_product_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "modal_sparse_range_tests.rs"]
mod range_tests;
