use super::SparseMatrix;
use super::product::visit_row_chunks;
use crate::solver_control::{SolverStage, checkpoint, checkpoint_chunk};

fn residual_row(row: &[(usize, f64)], expected: f64, solution: &[f64]) -> Result<f64, String> {
    // Match Iterator::sum::<f64>'s identity and original entry order.
    let mut actual = -0.0;
    visit_row_chunks(row, SolverStage::SparseResidualRow, |_, entries| {
        for &(column, value) in entries {
            actual += value * solution[column];
        }
    })?;
    Ok(expected - actual)
}

pub(crate) fn sparse_residual_norm(
    matrix: &SparseMatrix,
    rhs: &[f64],
    solution: &[f64],
) -> Result<f64, String> {
    checkpoint(SolverStage::SparseResidual, 0)?;
    let mut norm = ScaledNorm::new();
    let size = matrix.size().min(rhs.len());
    for (index, (row, &expected)) in matrix.rows.iter().zip(rhs).enumerate() {
        let value = residual_row(row, expected, solution)?;
        checkpoint_chunk(SolverStage::SparseResidual, index + 1, size)?;
        if let Some(non_finite) = norm.add(value) {
            return Ok(non_finite);
        }
    }
    Ok(norm.finish())
}

pub(super) fn sparse_residual_vector(
    matrix: &SparseMatrix,
    rhs: &[f64],
    solution: &[f64],
) -> Result<Vec<f64>, String> {
    checkpoint(SolverStage::SparseResidual, 0)?;
    let size = matrix.size().min(rhs.len());
    let mut residuals = Vec::with_capacity(size);
    for (index, (row, &expected)) in matrix.rows.iter().zip(rhs).enumerate() {
        residuals.push(residual_row(row, expected, solution)?);
        checkpoint_chunk(SolverStage::SparseResidual, index + 1, size)?;
    }
    Ok(residuals)
}

struct ScaledNorm {
    scale: f64,
    sum_squares: f64,
}

impl ScaledNorm {
    fn new() -> Self {
        Self {
            scale: 0.0,
            sum_squares: 1.0,
        }
    }

    fn add(&mut self, value: f64) -> Option<f64> {
        let value = value.abs();
        if !value.is_finite() {
            return Some(value);
        }
        if value != 0.0 {
            if self.scale < value {
                self.sum_squares = 1.0 + self.sum_squares * (self.scale / value).powi(2);
                self.scale = value;
            } else {
                self.sum_squares += (value / self.scale).powi(2);
            }
        }
        None
    }

    fn finish(self) -> f64 {
        if self.scale == 0.0 {
            0.0
        } else {
            self.scale * self.sum_squares.sqrt()
        }
    }
}

pub(crate) fn stable_l2_norm(values: impl IntoIterator<Item = f64>) -> f64 {
    let mut norm = ScaledNorm::new();
    for value in values {
        if let Some(non_finite) = norm.add(value) {
            return non_finite;
        }
    }
    norm.finish()
}

pub(super) struct SparseResidualValidation {
    pub(super) relative: f64,
    pub(super) row: usize,
    pub(super) residual: f64,
    pub(super) equation_scale: f64,
}

pub(super) fn sparse_relative_residual(
    matrix: &SparseMatrix,
    rhs: &[f64],
    solution: &[f64],
    tolerance: f64,
) -> Result<SparseResidualValidation, String> {
    checkpoint(SolverStage::ResidualValidate, 0)?;
    let size = matrix.size().min(rhs.len());
    let mut rows = Vec::with_capacity(matrix.size());
    let mut global_scale = 0.0_f64;
    for (index, (row, expected)) in matrix.rows.iter().zip(rhs).enumerate() {
        let mut actual = 0.0;
        let mut equation_scale = expected.abs();
        visit_row_chunks(row, SolverStage::ResidualValidateRow, |_, entries| {
            for &(column, value) in entries {
                let term = value * solution[column];
                actual += term;
                equation_scale += term.abs();
            }
        })?;
        let residual = (expected - actual).abs();
        global_scale = global_scale.max(equation_scale);
        rows.push((residual, equation_scale));
        checkpoint_chunk(SolverStage::ResidualValidate, index + 1, size)?;
    }

    let roundoff_floor = global_scale * f64::EPSILON / tolerance;
    let mut worst = SparseResidualValidation {
        relative: 0.0,
        row: 0,
        residual: 0.0,
        equation_scale: 0.0,
    };
    for (row_index, (residual, equation_scale)) in rows.into_iter().enumerate() {
        let effective_scale = equation_scale.max(roundoff_floor);
        let relative = if effective_scale == 0.0 {
            if residual == 0.0 { 0.0 } else { f64::INFINITY }
        } else {
            residual / effective_scale
        };
        if relative > worst.relative {
            worst = SparseResidualValidation {
                relative,
                row: row_index,
                residual,
                equation_scale: effective_scale,
            };
        }
        // Cumulative row counts distinguish the scale and worst-row passes.
        checkpoint_chunk(
            SolverStage::ResidualValidate,
            size + row_index + 1,
            size * 2,
        )?;
    }
    Ok(worst)
}
