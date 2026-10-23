use std::borrow::Cow;
use std::ops::Range;

use super::{TridiagonalSystem, backward_error_tolerance, solve_tridiagonal};
use crate::linear_binary_scale::{restore, split_finite};
use crate::linear_substitution::{normalize, solve_scaled_rhs_row};
use crate::solver_control::{SolverStage, checkpoint, checkpoint_chunk};

pub(super) fn solve_with_prescribed(
    diagonal: &[f64],
    lower: &[f64],
    upper: &[f64],
    rhs: &[f64],
    prescribed: &[(usize, f64)],
) -> Result<Vec<f64>, String> {
    checkpoint(SolverStage::ConstraintIndex, 0)?;
    super::validate_dimensions(diagonal, lower, upper, rhs)
        .map_err(|error| format!("1d chain solver failed: {error}"))?;
    super::validate_finite(diagonal, lower, upper, rhs)
        .map_err(|error| format!("1d chain solver failed: {error}"))?;
    let node_count = diagonal.len();
    if node_count == 0 {
        return Err("1d chain solver received inconsistent tridiagonal dimensions".into());
    }

    let mut values = vec![0.0; node_count];
    let mut fixed = vec![false; node_count];
    for (step, &(index, value)) in prescribed.iter().enumerate() {
        if index >= node_count {
            return Err("1d chain solver received an out-of-range prescribed value".into());
        }
        if !value.is_finite() {
            return Err("1d chain solver received a non-finite prescribed value".into());
        }
        if fixed[index] && values[index] != value {
            return Err("1d chain solver received conflicting prescribed values".into());
        }
        fixed[index] = true;
        values[index] = value;
        checkpoint_chunk(SolverStage::ConstraintIndex, step + 1, prescribed.len())?;
    }

    let system = TridiagonalSystem {
        diagonal,
        lower,
        upper,
        rhs,
    };
    let mut cursor = 0;
    while cursor < node_count {
        if fixed[cursor] {
            cursor += 1;
            checkpoint_chunk(SolverStage::ConstraintMap, cursor, node_count)?;
            continue;
        }
        let start = cursor;
        while cursor < node_count && !fixed[cursor] {
            cursor += 1;
            checkpoint_chunk(SolverStage::ConstraintMap, cursor, node_count)?;
        }
        solve_free_segment(start..cursor, &system, &mut values)?;
    }
    checkpoint(SolverStage::ResultPrescribed, node_count)?;
    Ok(values)
}

fn solve_free_segment(
    range: Range<usize>,
    system: &TridiagonalSystem<'_>,
    values: &mut [f64],
) -> Result<(), String> {
    let start = range.start;
    let end = range.end;
    let count = end - start;
    let mut diagonal = Cow::Borrowed(&system.diagonal[start..end]);
    let mut lower = Cow::Borrowed(&system.lower[start..end - 1]);
    let mut upper = Cow::Borrowed(&system.upper[start..end - 1]);
    let mut rhs = system.rhs[start..end].to_vec();
    let boundary_rows = [start, end - 1];
    let boundary_count = if count == 1 { 1 } else { 2 };
    for &row in &boundary_rows[..boundary_count] {
        let mut operands = [(0.0, 0.0); 2];
        let mut length = 0;
        if row == start && start > 0 {
            operands[length] = (system.lower[start - 1], values[start - 1]);
            length += 1;
        }
        if row == end - 1 && end < values.len() {
            operands[length] = (system.upper[end - 1], values[end]);
            length += 1;
        }
        if length == 0 {
            continue;
        }
        let local = row - start;
        let mut reduced = rhs[local];
        let mut needs_range = false;
        for &(coefficient, value) in &operands[..length] {
            let product = coefficient * value;
            reduced -= product;
            needs_range |= (coefficient != 0.0 && value != 0.0 && !product.is_normal())
                || !reduced.is_finite()
                || reduced.is_subnormal()
                // Include both boundaries even if only an intermediate update became zero.
                || (reduced == 0.0 && product != 0.0);
        }
        if needs_range {
            // Normalize only the affected free equation, never an independent load.
            // Coefficients must scale reversibly before the reduced RHS is accepted.
            let mut scale = diagonal[local].abs();
            if local > 0 {
                scale = scale.max(lower[local - 1].abs());
            }
            if local + 1 < count {
                scale = scale.max(upper[local].abs());
            }
            if scale == 0.0 {
                return Err(
                    "1d chain solver failed: tridiagonal system contains a singular row".into(),
                );
            }
            let exponent = split_finite(scale).1;
            diagonal.to_mut()[local] = normalize(diagonal[local], -exponent)?;
            if local > 0 {
                lower.to_mut()[local - 1] = normalize(lower[local - 1], -exponent)?;
            }
            if local + 1 < count {
                upper.to_mut()[local] = normalize(upper[local], -exponent)?;
            }
            reduced = solve_scaled_rhs_row(
                split_finite(system.rhs[row]),
                operands[..length].iter().copied(),
                restore(1.0, exponent),
                SolverStage::ConstraintReduce,
            )
            .map_err(|error| format!("1d chain boundary row {row}: {error}"))?;
        }
        rhs[local] = reduced;
    }

    let solved = solve_tridiagonal(&diagonal, &lower, &upper, &rhs)
        .map_err(|error| format!("1d chain solver failed: {error}"))?;
    checkpoint(SolverStage::ResidualValidate, 0)?;
    for (step, &row) in boundary_rows[..boundary_count].iter().enumerate() {
        validate_boundary_row(row, &range, system, values, &solved)?;
        checkpoint(SolverStage::ResidualValidate, step + 1)?;
    }
    values[start..end].copy_from_slice(&solved);
    Ok(())
}

fn validate_boundary_row(
    row: usize,
    range: &Range<usize>,
    system: &TridiagonalSystem<'_>,
    prescribed: &[f64],
    solved: &[f64],
) -> Result<(), String> {
    let value = |index| {
        if range.contains(&index) {
            solved[index - range.start]
        } else {
            prescribed[index]
        }
    };
    let operands = [
        (system.rhs[row], 1.0),
        (system.diagonal[row], value(row)),
        if row > 0 {
            (system.lower[row - 1], value(row - 1))
        } else {
            (0.0, 0.0)
        },
        if row + 1 < prescribed.len() {
            (system.upper[row], value(row + 1))
        } else {
            (0.0, 0.0)
        },
    ];
    // Prescribed inputs are exact data, not rounded solution unknowns.
    let error = super::residual::row_backward_error(
        operands,
        [true, row > range.start, row + 1 < range.end],
    )?;
    if !error.is_finite() || error > backward_error_tolerance(solved.len()) {
        return Err(format!(
            "1d chain boundary row {row} failed original-equation validation ({error:.6e})"
        ));
    }
    Ok(())
}
