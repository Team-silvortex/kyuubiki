use super::PreparedTridiagonal;
use crate::solver_control::{SolverStage, checkpoint_chunk};

pub(super) struct RowEquilibration {
    exponents: Vec<i32>,
    pub(super) upper: Vec<f64>,
}

pub(super) fn needed(scales: &[f64]) -> bool {
    // Leave headroom for products and elimination while keeping ordinary
    // systems on the existing allocation and arithmetic path.
    scales
        .iter()
        .any(|&scale| !(-500..=500).contains(&binary_exponent(scale)))
}

impl PreparedTridiagonal {
    pub(super) fn factor_equilibrated(
        diagonal: &[f64],
        lower: &[f64],
        upper: &[f64],
        scales: &[f64],
    ) -> Result<Self, String> {
        let size = diagonal.len();
        let mut exponents = Vec::with_capacity(size);
        let mut scaled_diagonal = Vec::with_capacity(size);
        let mut scaled_lower = Vec::with_capacity(lower.len());
        let mut scaled_upper = Vec::with_capacity(upper.len());
        for row in 0..size {
            let exponent = -binary_exponent(scales[row]);
            exponents.push(exponent);
            scaled_diagonal.push(exact_scale(diagonal[row], exponent, row, "coefficient")?);
            if row > 0 {
                scaled_lower.push(exact_scale(lower[row - 1], exponent, row, "coefficient")?);
            }
            if row + 1 < size {
                scaled_upper.push(exact_scale(upper[row], exponent, row, "coefficient")?);
            }
            checkpoint_chunk(SolverStage::TridiagonalFactor, row + 1, size)?;
        }
        // Each normalized row has maximum magnitude in [1, 2); this cannot
        // re-enter equilibration. Reuse the same factorization/pivot contracts.
        let mut result = Self::factor(&scaled_diagonal, &scaled_lower, &scaled_upper)?;
        let upper = if result.pivoted.is_none() {
            std::mem::replace(&mut result.upper, upper.to_vec())
        } else {
            result.upper.copy_from_slice(upper);
            Vec::new()
        };
        result.diagonal.copy_from_slice(diagonal);
        result.lower.copy_from_slice(lower);
        result.equilibration = Some(Box::new(RowEquilibration { exponents, upper }));
        Ok(result)
    }
}

impl RowEquilibration {
    pub(super) fn scale_rhs(&self, rhs: &[f64]) -> Result<Vec<f64>, String> {
        rhs.iter()
            .zip(&self.exponents)
            .enumerate()
            .map(|(row, (&value, &exponent))| {
                let scaled = exact_scale(value, exponent, row, "right-hand side")?;
                checkpoint_chunk(SolverStage::TridiagonalSubstitution, row + 1, rhs.len())?;
                Ok(scaled)
            })
            .collect()
    }
}

fn exact_scale(value: f64, exponent: i32, row: usize, kind: &str) -> Result<f64, String> {
    let scaled = scale_by_power(value, exponent);
    if !scaled.is_finite() || scale_by_power(scaled, -exponent) != value {
        return Err(format!(
            "tridiagonal row {row} {kind} cannot be scaled without range loss"
        ));
    }
    Ok(scaled)
}

fn binary_exponent(value: f64) -> i32 {
    if value < f64::MIN_POSITIVE {
        return binary_exponent(value * 4_503_599_627_370_496.0) - 52;
    }
    ((value.to_bits() >> 52) & 0x7ff) as i32 - 1023
}

fn scale_by_power(value: f64, exponent: i32) -> f64 {
    if exponent > 1023 {
        (value * power_of_two(1023)) * power_of_two(exponent - 1023)
    } else if exponent < -1022 {
        (value * power_of_two(-1022)) * power_of_two(exponent + 1022)
    } else {
        value * power_of_two(exponent)
    }
}

fn power_of_two(exponent: i32) -> f64 {
    debug_assert!((-1022..=1023).contains(&exponent));
    f64::from_bits(((exponent + 1023) as u64) << 52)
}
