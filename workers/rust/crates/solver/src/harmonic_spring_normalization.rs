use super::Complex;
use crate::{
    linear_binary_scale::{restore, split_finite},
    solver_control::{SolverStage, checkpoint_chunk},
};

pub(super) struct RowNormalization {
    pub(super) divisor: f64,
    pub(super) weight: f64,
}

impl RowNormalization {
    pub(super) fn choose(
        maximum: f64,
        entries: impl ExactSizeIterator<Item = Complex>,
    ) -> Result<Self, String> {
        if !maximum.is_finite() || maximum <= 0.0 {
            return Err("complex row normalization requires a finite positive maximum".into());
        }
        let (_, exponent) = split_finite(maximum);
        let binary = restore(1.0, exponent);
        let length = entries.len();
        let mut exact = true;
        for (index, value) in entries.enumerate() {
            for component in [value.re, value.im] {
                let scaled = component / binary;
                exact &= scaled.is_finite() && scaled * binary == component;
            }
            checkpoint_chunk(SolverStage::LinearPrepare, index + 1, length)?;
        }
        // A binary shift may lose a tiny coefficient that the legacy divisor preserved.
        let divisor = if exact { binary } else { maximum };
        Ok(Self {
            divisor,
            weight: maximum / divisor,
        })
    }
}

pub(super) fn pivot_score(value: Complex, weight: f64) -> f64 {
    // Compare on the original max-normalized scale, including after row swaps.
    value.amplitude() / weight
}
