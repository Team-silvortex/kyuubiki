use crate::linear_algebra::stable_l2_norm;
use crate::modal_sparse::SparseEigenpair;
use crate::solver_control::{SolverStage, checkpoint, checkpoint_chunk};

pub(crate) fn smallest_tridiagonal_eigenpair(
    diagonal: &[f64],
    off_diagonal: &[f64],
    tolerance: f64,
) -> Result<SparseEigenpair, String> {
    checkpoint(SolverStage::ModalValidation, 0)?;
    if diagonal.is_empty() || off_diagonal.len() != diagonal.len() - 1 {
        return Err("tridiagonal modal dimensions must match and be non-empty".into());
    }
    if !tolerance.is_finite() || tolerance <= 0.0 {
        return Err("tridiagonal modal tolerance must be finite and positive".into());
    }
    let mut scale = 0.0_f64;
    let total = diagonal.len() + off_diagonal.len();
    for (index, &value) in diagonal.iter().chain(off_diagonal).enumerate() {
        if !value.is_finite() {
            return Err("tridiagonal modal coefficients must be finite".into());
        }
        scale = scale.max(value.abs());
        checkpoint_chunk(SolverStage::ModalValidation, index + 1, total)?;
    }
    if scale == 0.0 {
        return Err("tridiagonal modal operator has zero scale".into());
    }
    let normalize = |values: &[f64]| -> Result<Vec<f64>, String> {
        let mut normalized = Vec::with_capacity(values.len());
        for (index, &value) in values.iter().enumerate() {
            let entry = value / scale;
            if value != 0.0 && entry == 0.0 {
                return Err("tridiagonal modal scaling lost a nonzero coefficient".into());
            }
            normalized.push(entry);
            checkpoint_chunk(SolverStage::ModalValidation, index + 1, values.len())?;
        }
        Ok(normalized)
    };
    let diagonal = normalize(diagonal)?;
    let off_diagonal = normalize(off_diagonal)?;
    if off_diagonal.contains(&0.0) {
        return Err("tridiagonal modal operator has a disconnected mode".into());
    }
    let (eigenvalue, iterations) = lowest_positive_value(&diagonal, &off_diagonal)?;
    let vector = recover_vector(&diagonal, &off_diagonal, eigenvalue)?;
    let residual_norm = checked_residual(&diagonal, &off_diagonal, eigenvalue, &vector, tolerance)?;
    let eigenvalue = eigenvalue * scale;
    let residual_norm = residual_norm * scale;
    if !eigenvalue.is_finite() || eigenvalue <= 0.0 || !residual_norm.is_finite() {
        return Err("tridiagonal modal eigenpair is not positive and representable".into());
    }
    Ok(SparseEigenpair {
        eigenvalue,
        iterations,
        residual_norm,
        vector,
    })
}

fn lowest_positive_value(diagonal: &[f64], off: &[f64]) -> Result<(f64, usize), String> {
    if sturm_count(diagonal, off, 0.0)? != 0 {
        return Err("tridiagonal modal spectrum contains a nonpositive eigenvalue".into());
    }
    // The smallest diagonal is a Rayleigh upper bound, even for a very soft local mode.
    let mut lower = 0.0;
    let mut upper = diagonal.iter().copied().fold(f64::INFINITY, f64::min);
    if upper <= 0.0 || sturm_count(diagonal, off, upper)? == 0 {
        return Err("tridiagonal modal eigenvalue could not be bracketed".into());
    }
    // Cover the normalized f64 exponent span plus its mantissa, not a fixed 96 halvings.
    for iteration in 0..1152 {
        checkpoint(SolverStage::ModalIteration, iteration)?;
        let middle = lower + (upper - lower) * 0.5;
        if middle == lower || middle == upper || (upper - lower) / upper <= 2.0 * f64::EPSILON {
            return Ok((if middle > 0.0 { middle } else { upper }, iteration));
        }
        if sturm_count(diagonal, off, middle)? == 0 {
            lower = middle;
        } else {
            upper = middle;
        }
    }
    Err("tridiagonal modal eigenvalue bisection did not converge".into())
}

// Only exact zero needs a one-sided convention; never replace a small positive pivot.
fn nonzero_pivot(pivot: f64) -> f64 {
    if pivot == 0.0 {
        -f64::from_bits(1)
    } else {
        pivot
    }
}

fn square_over(value: f64, pivot: f64) -> f64 {
    let pivot = nonzero_pivot(pivot);
    let ratio = value / pivot;
    if ratio.is_finite() {
        ratio * value
    } else {
        // Normalized |value| <= 1; balance the division without first squaring it.
        let balance = f64::from_bits((1023_u64 + 512) << 52);
        (value / (pivot * balance)) * (value * balance)
    }
}

fn sturm_count(diagonal: &[f64], off: &[f64], shift: f64) -> Result<usize, String> {
    let mut count = 0;
    let mut pivot = diagonal[0] - shift;
    for index in 0..diagonal.len() {
        if index > 0 {
            pivot = diagonal[index] - shift - square_over(off[index - 1], pivot);
        }
        if pivot.is_nan() {
            return Err("tridiagonal modal Sturm sequence is not representable".into());
        }
        if pivot <= 0.0 {
            count += 1;
        }
        checkpoint_chunk(SolverStage::ModalSweep, index + 1, diagonal.len())?;
    }
    Ok(count)
}

fn recover_vector(diagonal: &[f64], off: &[f64], value: f64) -> Result<Vec<f64>, String> {
    let size = diagonal.len();
    let mut left = vec![0.0; size];
    let mut right = vec![0.0; size];
    for index in 0..size {
        left[index] = diagonal[index]
            - value
            - if index == 0 {
                0.0
            } else {
                square_over(off[index - 1], left[index - 1])
            };
        let row = size - 1 - index;
        right[row] = diagonal[row]
            - value
            - if row + 1 == size {
                0.0
            } else {
                square_over(off[row], right[row + 1])
            };
        checkpoint_chunk(SolverStage::ResultFreeDofs, index + 1, size)?;
    }
    // Join two eliminations where the reciprocal inverse diagonal is smallest.
    // This avoids propagating a tiny endpoint component across the whole chain.
    let mut anchor = 0;
    let mut minimum = f64::INFINITY;
    for index in 0..size {
        let gamma = diagonal[index]
            - value
            - if index == 0 {
                0.0
            } else {
                square_over(off[index - 1], left[index - 1])
            }
            - if index + 1 == size {
                0.0
            } else {
                square_over(off[index], right[index + 1])
            };
        if gamma.abs() < minimum {
            minimum = gamma.abs();
            anchor = index;
        }
        checkpoint_chunk(SolverStage::ResultFreeDofs, index + 1, size)?;
    }
    if !minimum.is_finite() {
        return Err("tridiagonal modal eigenvector anchor is not representable".into());
    }
    let mut vector = vec![0.0; size];
    vector[anchor] = 1.0;
    for index in (0..anchor).rev() {
        vector[index] = -(off[index] / nonzero_pivot(left[index])) * vector[index + 1];
        checkpoint_chunk(SolverStage::ResultNodes, anchor - index, size)?;
    }
    for index in anchor + 1..size {
        vector[index] = -(off[index - 1] / nonzero_pivot(right[index])) * vector[index - 1];
        checkpoint_chunk(SolverStage::ResultNodes, index, size)?;
    }
    checkpoint(SolverStage::ResultNodes, size)?;
    let mut norm = 0.0_f64;
    for (index, chunk) in vector.chunks(64).enumerate() {
        norm = norm.hypot(stable_l2_norm(chunk.iter().copied()));
        checkpoint(SolverStage::ResultNodeSummary, ((index + 1) * 64).min(size))?;
    }
    if !norm.is_finite() || norm <= 0.0 {
        return Err("tridiagonal modal eigenvector is zero or not representable".into());
    }
    for (index, component) in vector.iter_mut().enumerate() {
        *component /= norm;
        checkpoint_chunk(SolverStage::ResultTotals, index + 1, size)?;
    }
    Ok(vector)
}

fn checked_residual(
    diagonal: &[f64],
    off: &[f64],
    value: f64,
    vector: &[f64],
    tolerance: f64,
) -> Result<f64, String> {
    let mut residual = 0.0_f64;
    let mut applied = 0.0_f64;
    for start in (0..diagonal.len()).step_by(64) {
        let end = (start + 64).min(diagonal.len());
        let mut products = [0.0; 64];
        let mut differences = [0.0; 64];
        for index in start..end {
            let mut product = diagonal[index] * vector[index];
            if index > 0 {
                product += off[index - 1] * vector[index - 1];
            }
            if index + 1 < diagonal.len() {
                product += off[index] * vector[index + 1];
            }
            products[index - start] = product;
            differences[index - start] = product - value * vector[index];
        }
        applied = applied.hypot(stable_l2_norm(products.into_iter()));
        residual = residual.hypot(stable_l2_norm(differences.into_iter()));
        checkpoint(SolverStage::ModalValidation, end)?;
    }
    let scale = applied.max(value.abs());
    if !residual.is_finite() || !scale.is_finite() || scale <= 0.0 || residual / scale > tolerance {
        return Err(format!(
            "tridiagonal modal residual is too large ({residual:.6e})"
        ));
    }
    Ok(residual)
}

#[cfg(test)]
#[path = "modal_tridiagonal_tests.rs"]
mod tests;
