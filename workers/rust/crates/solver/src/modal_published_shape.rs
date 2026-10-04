use super::SparseMassNormalizedOperator;
use crate::linear_algebra::cancellable_l2_norm;
use crate::modal_frame_assembly::positive_product;
use crate::solver_control::{SolverStage, checkpoint, checkpoint_chunk};

const MAX_SHAPE_POLISH_STEPS: usize = 4;

impl SparseMassNormalizedOperator {
    #[cfg(test)]
    pub(crate) fn polish_published_shape(
        &self,
        value: f64,
        shape: &mut Vec<f64>,
        mass: &[f64],
        tolerance: f64,
    ) -> Result<(), String> {
        self.polish_shape_with_budget(value, shape, mass, tolerance, MAX_SHAPE_POLISH_STEPS)
    }

    pub(crate) fn polish_published_shape_with_recovery(
        &self,
        value: f64,
        shape: &mut Vec<f64>,
        mass: &[f64],
        tolerance: f64,
        allow_roundoff: bool,
    ) -> Result<(), String> {
        self.polish_shape_internal(
            value,
            shape,
            mass,
            tolerance,
            MAX_SHAPE_POLISH_STEPS,
            allow_roundoff,
        )
    }

    #[cfg(test)]
    fn polish_shape_with_budget(
        &self,
        value: f64,
        shape: &mut Vec<f64>,
        mass: &[f64],
        tolerance: f64,
        max_steps: usize,
    ) -> Result<(), String> {
        self.polish_shape_internal(value, shape, mass, tolerance, max_steps, false)
    }

    fn polish_shape_internal(
        &self,
        value: f64,
        shape: &mut Vec<f64>,
        mass: &[f64],
        tolerance: f64,
        max_steps: usize,
        allow_roundoff: bool,
    ) -> Result<(), String> {
        if !tolerance.is_finite() || tolerance <= 0.0 {
            return Err("published modal shape tolerance must be finite and positive".into());
        }
        let applied = self.apply_physical_compensated(shape)?;
        let (relative, residual) = self.physical_residual(value, shape, mass, &applied)?;
        if relative <= tolerance {
            return Ok(());
        }
        let (mut candidate, relative) = self.ordinary_shape_proposal(
            value,
            shape,
            mass,
            tolerance,
            max_steps,
            (relative, residual),
        )?;
        if relative <= tolerance {
            *shape = candidate;
            return Ok(());
        }
        if allow_roundoff && crate::modal_frame_spectrum::roundoff::eligible(shape.len()) {
            // Use the best private candidate, then commit only a fresh certificate.
            candidate = crate::modal_frame_spectrum::roundoff::physical(
                self, value, &candidate, mass, tolerance,
            )?;
            *shape = candidate;
            return Ok(());
        }
        Err(format!(
            "published modal shape failed its relative residual check within {max_steps} corrections (relative={relative:.6e}, tolerance={tolerance:.6e})"
        ))
    }

    fn ordinary_shape_proposal(
        &self,
        value: f64,
        shape: &[f64],
        mass: &[f64],
        tolerance: f64,
        max_steps: usize,
        (mut relative, mut residual): (f64, Vec<f64>),
    ) -> Result<(Vec<f64>, f64), String> {
        // Keep all candidates private: failure or cancellation must not alter the caller's shape.
        let mut candidate = shape.to_vec();
        for step in 0..max_steps {
            checkpoint(SolverStage::ModalValidation, step)?;
            let mut trial = candidate.clone();
            self.coordinate_sweep(value, &mut trial, mass, &mut residual)?;
            let applied = self.apply_physical_compensated(&trial)?;
            let (next_relative, next_residual) =
                self.physical_residual(value, &trial, mass, &applied)?;
            if next_relative >= relative {
                break;
            }
            relative = next_relative;
            residual = next_residual;
            candidate = trial;
            if relative <= tolerance {
                break;
            }
        }
        Ok((candidate, relative))
    }

    #[cfg(test)]
    pub(crate) fn roundoff_comparison_seed(
        &self,
        value: f64,
        shape: &[f64],
        mass: &[f64],
    ) -> Result<Vec<f64>, String> {
        let applied = self.apply_physical_compensated(shape)?;
        let checked = self.physical_residual(value, shape, mass, &applied)?;
        self.ordinary_shape_proposal(value, shape, mass, 1e-8, MAX_SHAPE_POLISH_STEPS, checked)
            .map(|(candidate, _)| candidate)
    }

    fn coordinate_sweep(
        &self,
        value: f64,
        shape: &mut [f64],
        mass: &[f64],
        residual: &mut [f64],
    ) -> Result<(), String> {
        checkpoint(SolverStage::ResidualValidate, 0)?;
        let size = shape.len();
        for (position, column) in (0..size).chain((0..size).rev()).enumerate() {
            let start = self.stiffness.row_offsets[column];
            let end = self.stiffness.row_offsets[column + 1];
            let mut scale = 0.0_f64;
            for entry in start..end {
                scale = scale.max(self.shifted_column(value, mass, column, entry).abs());
                check_wide_column(entry, start, end)?;
            }
            if scale.is_finite() && scale > 0.0 {
                let (mut numerator, mut error, mut denominator) = (0.0, 0.0, 0.0);
                for entry in start..end {
                    let row = self.stiffness.columns[entry];
                    let a = self.shifted_column(value, mass, column, entry) / scale;
                    add_compensated(&mut numerator, &mut error, a * residual[row]);
                    denominator = a.mul_add(a, denominator);
                    check_wide_column(entry, start, end)?;
                }
                let correction = ((numerator + error) / denominator) / scale;
                let updated = shape[column] - correction;
                if updated.is_finite() {
                    let change = shape[column] - updated;
                    shape[column] = updated;
                    for entry in start..end {
                        let row = self.stiffness.columns[entry];
                        let a = self.shifted_column(value, mass, column, entry);
                        residual[row] = (-change).mul_add(a, residual[row]);
                        check_wide_column(entry, start, end)?;
                    }
                }
            }
            checkpoint_chunk(SolverStage::ResidualValidate, position + 1, 2 * size)?;
        }
        Ok(())
    }

    fn shifted_column(&self, value: f64, mass: &[f64], column: usize, entry: usize) -> f64 {
        let row = self.stiffness.columns[entry];
        let stiffness = self.stiffness.values[entry];
        // Symmetric stiffness rows supply column directions without a new matrix/factor.
        // Acceptance always recomputes the true row product, including assembly roundoff.
        let shifted = if row == column {
            (-value).mul_add(mass[row], stiffness)
        } else {
            stiffness
        };
        shifted * self.inverse_mass_sqrt[row]
    }

    pub(crate) fn physical_residual(
        &self,
        value: f64,
        shape: &[f64],
        mass: &[f64],
        applied: &[f64],
    ) -> Result<(f64, Vec<f64>), String> {
        if shape.is_empty()
            || shape.len() != applied.len()
            || shape.len() != mass.len()
            || shape.len() != self.inverse_mass_sqrt.len()
            || !value.is_finite()
            || value <= 0.0
        {
            return Err(
                "published modal shape requires matching finite positive eigenpairs".into(),
            );
        }
        checkpoint(SolverStage::ModalShapeScan, 0)?;
        for index in 0..shape.len() {
            if !mass[index].is_finite()
                || mass[index] <= 0.0
                || !shape[index].is_finite()
                || !applied[index].is_finite()
            {
                return Err(
                    "published modal shape requires matching finite positive eigenpairs".into(),
                );
            }
            checkpoint_chunk(SolverStage::ModalShapeScan, index + 1, shape.len())?;
        }
        let mut target = Vec::with_capacity(shape.len());
        let mut residual = Vec::with_capacity(shape.len());
        for (index, (&mode, &a)) in shape.iter().zip(applied).enumerate() {
            let b = positive_product([
                value,
                mass[index],
                self.inverse_mass_sqrt[index],
                mode.abs(),
            ])
            .copysign(mode);
            if !b.is_finite() {
                return Err("published modal mass product must remain finite".into());
            }
            target.push(b);
            residual.push(a - b);
            checkpoint_chunk(SolverStage::ResidualValidate, index + 1, shape.len())?;
        }
        let scale = cancellable_l2_norm(applied.iter().copied(), SolverStage::ModalShapeNorm)?.max(
            cancellable_l2_norm(target.iter().copied(), SolverStage::ModalShapeNorm)?,
        );
        let relative =
            cancellable_l2_norm(residual.iter().copied(), SolverStage::ModalShapeNorm)? / scale;
        if !scale.is_finite() || scale <= 0.0 || !relative.is_finite() {
            return Err("published modal shape residual is not finite or representable".into());
        }
        Ok((relative, residual))
    }
}

fn add_compensated(sum: &mut f64, error: &mut f64, value: f64) {
    let next = *sum + value;
    *error += if sum.abs() >= value.abs() {
        (*sum - next) + value
    } else {
        (value - next) + *sum
    };
    *sum = next;
}

fn check_wide_column(entry: usize, start: usize, end: usize) -> Result<(), String> {
    if end - start > 1024 && (entry - start + 1) % 1024 == 0 {
        checkpoint(SolverStage::ResidualValidateRow, entry - start + 1)?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "modal_published_shape_tests.rs"]
mod tests;
