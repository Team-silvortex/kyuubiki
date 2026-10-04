use super::{SparseEigenpair, SparseMassNormalizedOperator, approximately_equal, l2_norm};
use crate::linear_algebra::stable_l2_norm;
use crate::modal_tridiagonal::smallest_tridiagonal_eigenpair;
use crate::solver_control::{SolverStage, checkpoint, checkpoint_chunk};

struct TridiagonalStiffness {
    diagonal: Vec<f64>,
    upper: Vec<f64>,
}

impl SparseMassNormalizedOperator {
    /// Decline unsupported topology, but propagate range errors and cancellation.
    pub(crate) fn smallest_tridiagonal_eigenpair(
        &self,
        tolerance: f64,
    ) -> Option<Result<SparseEigenpair, String>> {
        self.prepare_tridiagonal_eigenpair(tolerance).transpose()
    }

    fn prepare_tridiagonal_eigenpair(
        &self,
        tolerance: f64,
    ) -> Result<Option<SparseEigenpair>, String> {
        if !tolerance.is_finite() || tolerance <= 0.0 {
            return Err("tridiagonal modal tolerance must be finite and positive".into());
        }
        let Some(coefficients) = self.tridiagonal_stiffness()? else {
            return Ok(None);
        };
        if let Some(pair) = self.uniform_chain_from(&coefficients, tolerance)? {
            return Ok(Some(pair));
        }
        let TridiagonalStiffness {
            mut diagonal,
            mut upper,
        } = coefficients;
        let size = diagonal.len();
        checkpoint(SolverStage::SparseMatrixScale, 0)?;
        for row in 0..size {
            diagonal[row] = self.normalized_entry(row, row, diagonal[row])?;
            if row + 1 < size {
                upper[row] = self.normalized_entry(row, row + 1, upper[row])?;
            }
            checkpoint_chunk(SolverStage::SparseMatrixScale, row + 1, size)?;
        }
        smallest_tridiagonal_eigenpair(&diagonal, &upper, tolerance).map(Some)
    }

    fn tridiagonal_stiffness(&self) -> Result<Option<TridiagonalStiffness>, String> {
        let size = self.inverse_mass_sqrt.len();
        if size < 2 {
            return Ok(None);
        }
        checkpoint(SolverStage::SparseValidateMatrix, 0)?;
        let mut diagonal = vec![0.0; size];
        let mut upper = vec![0.0; size - 1];
        let mut lower = vec![0.0; size - 1];
        for row in 0..size {
            for entry in self.stiffness.row_offsets[row]..self.stiffness.row_offsets[row + 1] {
                let column = self.stiffness.columns[entry];
                let value = self.stiffness.values[entry];
                if column == row {
                    diagonal[row] = value;
                } else if column == row + 1 {
                    upper[row] = value;
                } else if column + 1 == row {
                    lower[column] = value;
                } else {
                    return Ok(None);
                }
            }
            checkpoint_chunk(SolverStage::SparseValidateMatrix, row + 1, size)?;
        }
        let mut connected = true;
        for (index, (&upper, &lower)) in upper.iter().zip(&lower).enumerate() {
            if upper != lower {
                return Err("tridiagonal modal stiffness must be symmetric".into());
            }
            connected &= upper != 0.0;
            checkpoint_chunk(SolverStage::SparseValidateMatrix, index + 1, size - 1)?;
        }
        drop(lower);
        Ok(connected.then_some(TridiagonalStiffness { diagonal, upper }))
    }

    #[cfg(test)]
    pub(super) fn uniform_axial_chain_eigenpair(
        &self,
        tolerance: f64,
    ) -> Option<Result<SparseEigenpair, String>> {
        self.tridiagonal_stiffness()
            .and_then(|coefficients| match coefficients {
                Some(coefficients) => self.uniform_chain_from(&coefficients, tolerance),
                None => Ok(None),
            })
            .transpose()
    }

    /// The fixed-free uniform chain is recognized in physical, not normalized, units.
    fn uniform_chain_from(
        &self,
        coefficients: &TridiagonalStiffness,
        tolerance: f64,
    ) -> Result<Option<SparseEigenpair>, String> {
        let size = coefficients.diagonal.len();
        let stiffness = -coefficients.upper[0];
        if !stiffness.is_finite() || stiffness <= 0.0 {
            return Ok(None);
        }
        let relative_tolerance = 1.0e-10;
        for row in 0..size {
            let expected = if row + 1 == size {
                stiffness
            } else {
                2.0 * stiffness
            };
            if !approximately_equal(coefficients.diagonal[row], expected, relative_tolerance)
                || (row + 1 < size
                    && !approximately_equal(
                        coefficients.upper[row],
                        -stiffness,
                        relative_tolerance,
                    ))
            {
                return Ok(None);
            }
            checkpoint_chunk(SolverStage::SparseDiagonalScale, row + 1, size)?;
        }
        let mut masses = Vec::with_capacity(size);
        let interior_mass = self.inverse_mass_sqrt[0].recip().powi(2);
        if !interior_mass.is_finite() || interior_mass <= 0.0 {
            return Ok(None);
        }
        for (index, inverse) in self.inverse_mass_sqrt.iter().enumerate() {
            let mass = inverse.recip().powi(2);
            let expected = if index + 1 == size {
                0.5 * interior_mass
            } else {
                interior_mass
            };
            if !approximately_equal(mass, expected, relative_tolerance) {
                return Ok(None);
            }
            masses.push(mass);
            checkpoint_chunk(SolverStage::SparseDiagonalMagnitude, index + 1, size)?;
        }
        let theta = std::f64::consts::PI / (2 * size) as f64;
        // For size >= 2 the dimensionless factor is below one; do not form 4k/m first.
        let eigenvalue = (stiffness / interior_mass) * (4.0 * (theta * 0.5).sin().powi(2));
        if !eigenvalue.is_finite() || eigenvalue <= 0.0 {
            return Err("uniform axial modal eigenvalue is not positive and representable".into());
        }
        let mut vector = Vec::with_capacity(size);
        for (index, mass) in masses.iter().enumerate() {
            vector.push(mass.sqrt() * ((index + 1) as f64 * theta).sin());
            checkpoint_chunk(SolverStage::ResultFreeDofs, index + 1, size)?;
        }
        drop(masses);
        let norm = cancellable_norm(&vector)?;
        if !norm.is_finite() || norm <= 0.0 {
            return Err("uniform axial modal eigenvector is zero or non-finite".into());
        }
        for (index, value) in vector.iter_mut().enumerate() {
            *value /= norm;
            checkpoint_chunk(SolverStage::ResultTotals, index + 1, size)?;
        }
        let applied = self.apply(&vector)?;
        let mut residual_norm = 0.0_f64;
        let mut applied_norm = 0.0_f64;
        for (block, (values, modes)) in applied.chunks(64).zip(vector.chunks(64)).enumerate() {
            residual_norm = residual_norm.hypot(stable_l2_norm(
                values
                    .iter()
                    .zip(modes)
                    .map(|(value, mode)| value - eigenvalue * mode),
            ));
            applied_norm = applied_norm.hypot(l2_norm(values));
            checkpoint(SolverStage::ResidualValidate, block * 64 + values.len())?;
        }
        let residual_scale = applied_norm.max(eigenvalue.abs());
        // Preserve the existing bounded f64 floor for cancellation in long uniform chains.
        let relative_floor = 2.0e-4;
        if !(residual_norm.is_finite() && residual_scale.is_finite() && residual_scale > 0.0)
            || residual_norm > tolerance.max(relative_floor) * residual_scale
        {
            return Err(format!(
                "uniform axial modal residual is too large ({residual_norm:.6e})"
            ));
        }
        Ok(Some(SparseEigenpair {
            eigenvalue,
            iterations: 0,
            residual_norm,
            vector,
        }))
    }
}

fn cancellable_norm(values: &[f64]) -> Result<f64, String> {
    let mut norm = 0.0_f64;
    for (block, chunk) in values.chunks(64).enumerate() {
        norm = norm.hypot(l2_norm(chunk));
        checkpoint(SolverStage::ResultNodeSummary, block * 64 + chunk.len())?;
    }
    Ok(norm)
}
