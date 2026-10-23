use super::SparseMatrix;
use super::product::visit_row_chunks;
use crate::solver_control::{SolverStage, checkpoint, checkpoint_chunk};

#[path = "linear_sparse_residual_range.rs"]
pub(super) mod range;

#[cfg(test)]
#[path = "linear_sparse_residual_range_tests.rs"]
mod range_tests;

#[cfg(test)]
#[path = "linear_sparse_residual_underflow_tests.rs"]
mod underflow_tests;

#[cfg(test)]
#[path = "linear_sparse_erased_load_tests.rs"]
mod erased_load_tests;

// A subnormal displacement has a fixed representational spacing. Bound its
// half-ulp contribution locally; never borrow another equation's force or units.
pub(super) fn subnormal_solution_roundoff(coefficient: f64, value: f64) -> f64 {
    if value.abs() < f64::MIN_POSITIVE {
        (coefficient.abs() * f64::from_bits(1)) * 0.5
    } else {
        0.0
    }
}

pub(super) fn row_solution_roundoff(expected: f64, has_response: bool, roundoff: f64) -> f64 {
    // A zero response cannot spend its representation budget to erase a nonzero load.
    if expected != 0.0 && !has_response {
        0.0
    } else {
        roundoff
    }
}

#[inline]
pub(super) fn product_needs_range_check(coefficient: f64, value: f64, product: f64) -> bool {
    // A finite zero or subnormal product can have lost an entire equation's signal.
    coefficient != 0.0 && value != 0.0 && product.abs() < f64::MIN_POSITIVE
}

fn residual_row(row: &[(usize, f64)], expected: f64, solution: &[f64]) -> Result<f64, String> {
    // Match Iterator::sum::<f64>'s identity and original entry order.
    let mut actual = -0.0;
    let mut needs_range_check = false;
    visit_row_chunks(row, SolverStage::SparseResidualRow, |_, entries| {
        for &(column, value) in entries {
            let component = solution.get(column).copied().unwrap_or(f64::NAN);
            let product = value * component;
            needs_range_check |= product_needs_range_check(value, component, product);
            actual += product;
        }
    })?;
    let residual = expected - actual;
    if residual.is_finite() && !needs_range_check {
        Ok(residual)
    } else {
        range::scaled_row(row, expected, solution, SolverStage::SparseResidualRow)
            .map(|scaled| scaled.residual)
    }
}

pub(crate) fn sparse_residual_norm(
    matrix: &SparseMatrix,
    rhs: &[f64],
    solution: &[f64],
) -> Result<f64, String> {
    checkpoint(SolverStage::SparseResidual, 0)?;
    validate_dimensions(matrix, rhs, solution)?;
    let mut norm = ScaledNorm::new();
    let size = matrix.size();
    for (index, (row, &expected)) in matrix.rows.iter().zip(rhs).enumerate() {
        let value = residual_row(row, expected, solution)?;
        checkpoint_chunk(SolverStage::SparseResidual, index + 1, size)?;
        if let Some(non_finite) = norm.add(value) {
            return Ok(non_finite);
        }
    }
    Ok(norm.finish())
}

pub(crate) fn sparse_residual_vector(
    matrix: &SparseMatrix,
    rhs: &[f64],
    solution: &[f64],
) -> Result<Vec<f64>, String> {
    checkpoint(SolverStage::SparseResidual, 0)?;
    validate_dimensions(matrix, rhs, solution)?;
    let size = matrix.size();
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

pub(crate) fn cancellable_l2_norm(
    values: impl ExactSizeIterator<Item = f64>,
    stage: SolverStage,
) -> Result<f64, String> {
    checkpoint(stage, 0)?;
    let size = values.len();
    // Retain the original accumulator and term order across cancellation boundaries.
    let mut norm = ScaledNorm::new();
    for (index, value) in values.enumerate() {
        let non_finite = norm.add(value);
        checkpoint_chunk(stage, index + 1, size)?;
        if let Some(non_finite) = non_finite {
            return Ok(non_finite);
        }
    }
    Ok(norm.finish())
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
    validate_dimensions(matrix, rhs, solution)?;
    if !tolerance.is_finite() || tolerance <= 0.0 {
        return Err("sparse residual tolerance must be finite and positive".into());
    }
    let size = matrix.size();
    let mut rows = Vec::with_capacity(matrix.size());
    for (index, (row, expected)) in matrix.rows.iter().zip(rhs).enumerate() {
        let mut actual = 0.0;
        let mut equation_scale = expected.abs();
        let mut solution_roundoff = 0.0;
        let mut has_response = false;
        let mut needs_range_check = false;
        visit_row_chunks(row, SolverStage::ResidualValidateRow, |_, entries| {
            for &(column, value) in entries {
                let component = solution.get(column).copied().unwrap_or(f64::NAN);
                let term = value * component;
                has_response |= term != 0.0;
                needs_range_check |= product_needs_range_check(value, component, term);
                actual += term;
                equation_scale += term.abs();
                solution_roundoff += subnormal_solution_roundoff(value, component);
            }
        })?;
        let residual = (expected - actual).abs();
        if !residual.is_finite() || !equation_scale.is_finite() || needs_range_check {
            drop(rows);
            return range::relative_residual(matrix, rhs, solution);
        }
        rows.push((
            residual,
            equation_scale,
            row_solution_roundoff(*expected, has_response, solution_roundoff),
        ));
        checkpoint_chunk(SolverStage::ResidualValidate, index + 1, size)?;
    }

    let mut worst = SparseResidualValidation {
        relative: 0.0,
        row: 0,
        residual: 0.0,
        equation_scale: 0.0,
    };
    for (row_index, (residual, equation_scale, solution_roundoff)) in rows.into_iter().enumerate() {
        // A different equation's units or load must not relax this row's error.
        let effective_scale = equation_scale;
        let resolved_residual = (residual - solution_roundoff).max(0.0);
        let relative = if effective_scale == 0.0 {
            if resolved_residual == 0.0 {
                0.0
            } else {
                f64::INFINITY
            }
        } else {
            resolved_residual / effective_scale
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

fn validate_dimensions(matrix: &SparseMatrix, rhs: &[f64], solution: &[f64]) -> Result<(), String> {
    if rhs.len() != matrix.size() || solution.len() != matrix.size() {
        return Err("sparse residual dimensions must match the complete system".into());
    }
    Ok(())
}
