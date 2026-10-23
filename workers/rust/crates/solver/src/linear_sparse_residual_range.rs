use super::{SparseMatrix, SparseResidualValidation};
use crate::linear_binary_scale::{restore, split_finite};
use crate::solver_control::{SolverStage, checkpoint, checkpoint_chunk};

// Only an iterative candidate gate may retry Unresolved; final validation must fail.
#[derive(Debug)]
pub(in crate::linear_algebra) enum RangeError {
    Unresolved(&'static str),
    Failure(String),
}

impl From<String> for RangeError {
    fn from(error: String) -> Self {
        Self::Failure(error)
    }
}

impl From<RangeError> for String {
    fn from(error: RangeError) -> Self {
        match error {
            RangeError::Unresolved(message) => message.into(),
            RangeError::Failure(message) => message,
        }
    }
}

pub(in crate::linear_algebra) struct ScaledRow {
    pub(super) residual: f64,
    pub(super) relative: f64,
    equation_scale: f64,
    solution_roundoff: f64,
}

impl ScaledRow {
    pub(in crate::linear_algebra) fn resolved_relative(&self) -> f64 {
        let residual = self.residual.abs();
        if residual > 0.0 && residual.is_finite() && self.solution_roundoff > 0.0 {
            self.relative * ((residual - self.solution_roundoff).max(0.0) / residual)
        } else {
            self.relative
        }
    }
}

pub(super) fn relative_residual(
    matrix: &SparseMatrix,
    rhs: &[f64],
    solution: &[f64],
) -> Result<SparseResidualValidation, String> {
    let mut worst = SparseResidualValidation {
        relative: 0.0,
        row: 0,
        residual: 0.0,
        equation_scale: 0.0,
    };
    for (index, (row, &expected)) in matrix.rows.iter().zip(rhs).enumerate() {
        let value = scaled_row(row, expected, solution, SolverStage::ResidualValidateRow)?;
        let relative = value.resolved_relative();
        if relative > worst.relative {
            worst = SparseResidualValidation {
                relative,
                row: index,
                residual: value.residual.abs(),
                equation_scale: value.equation_scale,
            };
        }
        checkpoint_chunk(SolverStage::ResidualValidate, index + 1, matrix.size())?;
    }
    Ok(worst)
}

/// Two passes, constant extra row storage. Never form an unscaled product.
/// Recompute locally when direct products overflow or enter the subnormal range.
pub(super) fn scaled_row(
    row: &[(usize, f64)],
    expected: f64,
    solution: &[f64],
    stage: SolverStage,
) -> Result<ScaledRow, String> {
    scaled_entries(row.iter().copied(), row.len(), expected, solution, stage).map_err(String::from)
}

pub(in crate::linear_algebra) fn scaled_entries(
    entries: impl Iterator<Item = (usize, f64)> + Clone,
    length: usize,
    expected: f64,
    solution: &[f64],
    stage: SolverStage,
) -> Result<ScaledRow, RangeError> {
    let (rhs, rhs_exponent) = split(expected)?;
    let mut exponent = if rhs == 0.0 { i32::MIN } else { rhs_exponent };
    let mut solution_roundoff = 0.0;
    let mut has_response = false;
    checkpoint(stage, 0)?;
    for (index, (column, coefficient)) in entries.clone().enumerate() {
        let component = component(solution, column)?;
        let (high, _, power) = product(coefficient, component)?;
        has_response |= high != 0.0;
        solution_roundoff += super::subnormal_solution_roundoff(coefficient, component);
        if high != 0.0 {
            exponent = exponent.max(power);
        }
        if (index + 1) % 1024 == 0 {
            checkpoint(stage, index + 1)?;
        }
    }
    if exponent == i32::MIN {
        exponent = 0;
    }
    checkpoint(stage, length)?;

    let rhs = normalize(rhs, rhs_exponent - exponent)?;
    let mut residual = CompensatedSum::default();
    residual.add(rhs);
    let mut scale = CompensatedSum::default();
    scale.add(rhs.abs());
    for (index, (column, coefficient)) in entries.enumerate() {
        let (high, low, power) = product(coefficient, component(solution, column)?)?;
        let high = normalize(high, power - exponent)?;
        let low = normalize(low, power - exponent)?;
        residual.add(-high);
        residual.add(-low);
        scale.add(high.abs());
        scale.add(if high.is_sign_negative() { -low } else { low });
        if (index + 1) % 1024 == 0 {
            checkpoint(stage, length + index + 1)?;
        }
    }
    checkpoint(stage, length.saturating_mul(2))?;
    let residual = residual.value();
    let scale = scale.value();
    let restored = restore(residual, exponent);
    if residual != 0.0 && restored == 0.0 {
        return Err(RangeError::Unresolved(
            "sparse residual is outside the representable range",
        ));
    }
    Ok(ScaledRow {
        residual: restored,
        relative: if scale == 0.0 {
            0.0
        } else {
            residual.abs() / scale
        },
        equation_scale: restore(scale, exponent),
        solution_roundoff: super::row_solution_roundoff(expected, has_response, solution_roundoff),
    })
}

fn component(solution: &[f64], column: usize) -> Result<f64, String> {
    solution
        .get(column)
        .copied()
        .ok_or_else(|| format!("sparse residual matrix column {column} is outside the solution"))
}

fn product(left: f64, right: f64) -> Result<(f64, f64, i32), String> {
    let (left, le) = split(left)?;
    let (right, re) = split(right)?;
    let high = left * right;
    Ok((high, left.mul_add(right, -high), le + re))
}

fn split(value: f64) -> Result<(f64, i32), String> {
    if !value.is_finite() {
        return Err("sparse residual contains a non-finite operand".into());
    }
    Ok(split_finite(value))
}

fn normalize(value: f64, exponent: i32) -> Result<f64, RangeError> {
    let scaled = restore(value, exponent);
    if !scaled.is_finite() || restore(scaled, -exponent) != value {
        return Err(RangeError::Unresolved(
            "sparse residual normalization would lose numerical range",
        ));
    }
    Ok(scaled)
}

#[derive(Default)]
struct CompensatedSum {
    sum: f64,
    correction: f64,
}

impl CompensatedSum {
    fn add(&mut self, value: f64) {
        let next = self.sum + value;
        self.correction += if self.sum.abs() >= value.abs() {
            (self.sum - next) + value
        } else {
            (value - next) + self.sum
        };
        self.sum = next;
    }

    fn value(self) -> f64 {
        self.sum + self.correction
    }
}
