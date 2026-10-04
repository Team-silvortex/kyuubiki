use super::{MAX_GRID_RADIUS, QrFit, apply, dot};
use crate::solver_control::{SolverStage, checkpoint};

impl QrFit {
    // Round in original column units before using the decision upstream in R.
    pub(crate) fn quantized_solve(&self, rhs: &[f64], radius: usize) -> Result<Vec<f64>, String> {
        let rows = self.columns[0].len();
        if rhs.len() != rows
            || rhs.iter().any(|v| !v.is_finite())
            || !(1..=MAX_GRID_RADIUS).contains(&radius)
        {
            return Err(
                "quantized QR requires matching finite data and a bounded positive radius".into(),
            );
        }
        checkpoint(SolverStage::DenseSubstitution, 0)?;
        let mut transformed = rhs.to_vec();
        for (index, (direction, tau)) in self.reflectors.iter().enumerate() {
            apply(direction, *tau, &mut transformed[index..])?;
            checkpoint(SolverStage::DenseSubstitution, index + 1)?;
        }
        let count = self.columns.len();
        let mut scaled = vec![0.0; count];
        let mut integers = vec![0.0; count];
        for row in (0..count).rev() {
            let coefficients: Vec<_> = self.columns[row + 1..].iter().map(|c| c[row]).collect();
            let accumulated = dot(&coefficients, &scaled[row + 1..])?;
            let (scale, adjustment) = self.scales[row];
            let coefficient =
                ((transformed[row] - accumulated) / self.columns[row][row]) * adjustment / scale;
            if !coefficient.is_finite() {
                return Err("quantized QR coefficient is not representable".into());
            }
            integers[row] = coefficient.round().clamp(-(radius as f64), radius as f64);
            scaled[row] = integers[row] * scale / adjustment;
            if !scaled[row].is_finite() {
                return Err("quantized QR decision exceeds its scaled range".into());
            }
            checkpoint(SolverStage::DenseSubstitution, count + count - row)?;
        }
        Ok(integers)
    }
}
