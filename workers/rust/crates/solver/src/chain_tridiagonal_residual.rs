use crate::linear_binary_scale::{restore, split_finite};
use crate::solver_control::{SolverStage, checkpoint};

/// Every equation owns its error scale, including ordinary unscaled systems.
/// Keep normal rows on direct arithmetic and normalize exceptional rows only.
pub(super) fn maximum_backward_error(
    diagonal: &[f64],
    lower: &[f64],
    upper: &[f64],
    rhs: &[f64],
    solution: &[f64],
    tolerance: f64,
) -> Result<f64, String> {
    checkpoint(SolverStage::SparseResidual, 0)?;
    super::validate_dimensions(diagonal, lower, upper, rhs)?;
    if solution.len() != diagonal.len() {
        return Err("tridiagonal residual dimensions must match".into());
    }
    if !tolerance.is_finite() || tolerance <= 0.0 {
        return Err("tridiagonal residual tolerance must be finite and positive".into());
    }
    let mut maximum = 0.0_f64;
    for row in 0..diagonal.len() {
        let operands = [
            (rhs[row], 1.0),
            (diagonal[row], solution[row]),
            if row > 0 {
                (lower[row - 1], solution[row - 1])
            } else {
                (0.0, 0.0)
            },
            if row + 1 < diagonal.len() {
                (upper[row], solution[row + 1])
            } else {
                (0.0, 0.0)
            },
        ];
        maximum = maximum.max(row_backward_error(operands, [true; 3])?);
        if (row + 1) % 256 == 0 {
            checkpoint(SolverStage::SparseResidual, row + 1)?;
        }
    }
    checkpoint(SolverStage::SparseResidual, diagonal.len())?;
    Ok(maximum)
}

pub(super) fn row_backward_error(
    operands: [(f64, f64); 4],
    roundable: [bool; 3],
) -> Result<f64, String> {
    let mut values = [0.0; 4];
    let mut needs_range = false;
    for (value, (left, right)) in values.iter_mut().zip(operands) {
        if !left.is_finite() || !right.is_finite() {
            return Err("tridiagonal residual contains a non-finite operand".into());
        }
        *value = left * right;
        needs_range |= left != 0.0 && right != 0.0 && !value.is_normal();
    }
    let residual = (values[0] - (values[1] + values[2] + values[3])).abs();
    let scale = values.iter().map(|value| value.abs()).sum::<f64>();
    if !needs_range && residual.is_finite() && scale.is_finite() && !residual.is_subnormal() {
        let roundoff = operands[1..]
            .iter()
            .zip(roundable)
            .map(|(&(coefficient, value), allowed)| {
                if allowed && value.abs() < f64::MIN_POSITIVE {
                    (coefficient.abs() * f64::from_bits(1)) * 0.5
                } else {
                    0.0
                }
            })
            .sum();
        return Ok(relative_error(values, scale, residual, roundoff));
    }

    // Fixed-size row storage avoids an O(n) validation buffer or a whole-system restart.
    let terms = operands.map(|(left, right)| {
        let (lm, le) = split_finite(left);
        let (rm, re) = split_finite(right);
        (lm * rm, le + re)
    });
    let exponent = terms
        .iter()
        .filter(|(m, _)| *m != 0.0)
        .map(|(_, e)| *e)
        .max()
        .unwrap_or(0);
    let values = terms.map(|(mantissa, power)| restore(mantissa, power - exponent));
    let scale = values.iter().map(|value| value.abs()).sum::<f64>();
    let residual = (values[0] - (values[1] + values[2] + values[3])).abs();
    let roundoff = operands[1..]
        .iter()
        .zip(roundable)
        .map(|(&(coefficient, value), allowed)| {
            if allowed && value.abs() < f64::MIN_POSITIVE {
                let (mantissa, power) = split_finite(coefficient.abs());
                restore(mantissa, power - 1075 - exponent)
            } else {
                0.0
            }
        })
        .sum();
    Ok(relative_error(values, scale, residual, roundoff))
}

fn relative_error(values: [f64; 4], scale: f64, residual: f64, roundoff: f64) -> f64 {
    if scale == 0.0 {
        return 0.0;
    }
    // Subnormal unknowns carry at most half a quantum of rounding each. Keep
    // that allowance row-local, and never certify a completely erased load.
    let erased_load = values[0] != 0.0 && values[1..].iter().all(|value| *value == 0.0);
    let resolved = if erased_load {
        residual
    } else {
        (residual - roundoff).max(0.0)
    };
    resolved / scale
}
