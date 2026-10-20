use crate::solver_control::{SolverStage, checkpoint};

/// Overflow fallback: normalize the four row terms by their largest binary
/// exponent, without ever forming an unscaled coefficient/displacement product.
/// Use row-local error so an overflowing large row cannot mask another equation.
pub(super) fn scaled_backward_error(
    diagonal: &[f64],
    lower: &[f64],
    upper: &[f64],
    rhs: &[f64],
    solution: &[f64],
) -> Result<f64, String> {
    checkpoint(SolverStage::SparseResidual, 0)?;
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
        let mut terms = [(0.0_f64, 0_i32); 4];
        for (term, (left, right)) in terms.iter_mut().zip(operands) {
            let (Some((lm, le)), Some((rm, re))) = (split(left), split(right)) else {
                return Ok(f64::INFINITY);
            };
            *term = (lm * rm, le + re);
        }
        let exponent = terms
            .iter()
            .filter(|(m, _)| *m != 0.0)
            .map(|(_, e)| *e)
            .max()
            .unwrap_or(0);
        let values = terms.map(|(mantissa, e)| {
            if mantissa == 0.0 {
                0.0
            } else {
                mantissa * power_of_two(e - exponent)
            }
        });
        let scale: f64 = values.iter().map(|value| value.abs()).sum();
        if scale > 0.0 {
            let residual = (values[0] - values[1..].iter().sum::<f64>()).abs();
            maximum = maximum.max(residual / scale);
        }
        if (row + 1) % 256 == 0 {
            checkpoint(SolverStage::SparseResidual, row + 1)?;
        }
    }
    checkpoint(SolverStage::SparseResidual, diagonal.len())?;
    Ok(maximum)
}

fn split(value: f64) -> Option<(f64, i32)> {
    if !value.is_finite() {
        return None;
    }
    if value == 0.0 {
        return Some((0.0, 0));
    }
    let bits = value.to_bits();
    let exponent = ((bits >> 52) & 0x7ff) as i32;
    if exponent == 0 {
        let (mantissa, exponent) = split(value * 4_503_599_627_370_496.0)?;
        return Some((mantissa, exponent - 52));
    }
    let mantissa = f64::from_bits((bits & 0x800f_ffff_ffff_ffff) | (1023_u64 << 52));
    Some((mantissa, exponent - 1023))
}

fn power_of_two(exponent: i32) -> f64 {
    if exponent < -1074 {
        0.0
    } else if exponent < -1022 {
        f64::from_bits(1_u64 << (exponent + 1074))
    } else {
        // Nonzero terms are scaled relative to the maximum exponent.
        f64::from_bits(((exponent + 1023) as u64) << 52)
    }
}
