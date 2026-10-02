use crate::linear_algebra::stable_l2_norm;
use crate::modal_sparse::ReducedSparseModalSystem;
use crate::solver_control::{SolverStage, checkpoint, checkpoint_chunk};

const MAX_REFINEMENT_STEPS: usize = 4;

pub(super) fn refine_dense_modes(
    system: &ReducedSparseModalSystem,
    pairs: &mut [(f64, Vec<f64>)],
    tolerance: f64,
) -> Result<(), String> {
    refine_with_budget(system, pairs, tolerance, MAX_REFINEMENT_STEPS)
}

fn refine_with_budget(
    system: &ReducedSparseModalSystem,
    pairs: &mut [(f64, Vec<f64>)],
    tolerance: f64,
    max_steps: usize,
) -> Result<(), String> {
    if !tolerance.is_finite() || tolerance <= 0.0 {
        return Err("modal dense refinement tolerance must be finite and positive".into());
    }
    let mut inverse = None;
    let mut changed = false;
    for index in 0..pairs.len() {
        let (previous, pending) = pairs.split_at_mut(index);
        let (eigenvalue, vector) = &mut pending[0];
        let original = *eigenvalue;
        let applied = system.operator.apply_compensated(vector)?;
        let initial_relative = relative_residual(original, vector, &applied)?;
        if initial_relative <= tolerance && !changed {
            continue;
        }
        if changed {
            orthogonalize(vector, previous)?;
            *eigenvalue = rayleigh(vector, &system.operator.apply_compensated(vector)?);
        }
        for step in 0..=max_steps {
            checkpoint(SolverStage::ModalIteration, step)?;
            let applied = system.operator.apply_compensated(vector)?;
            let relative = relative_residual(*eigenvalue, vector, &applied)?;
            if relative <= tolerance {
                break;
            }
            if step == max_steps {
                return Err(format!(
                    "modal dense mode {index} refinement did not converge within {max_steps} steps (relative={relative:.6e})"
                ));
            }
            if inverse.is_none() {
                inverse = Some(system.prepare_normalized_inverse()?);
            }
            // Correct the physical-operator residual, rather than replacing the seed
            // with an eigenvector of the separately rounded normalized factor matrix.
            let residual: Vec<_> = applied
                .iter()
                .zip(vector.iter())
                .map(|(value, mode)| (-*eigenvalue).mul_add(*mode, *value))
                .collect();
            let correction = inverse.as_ref().unwrap().solve(&residual)?;
            for (value, correction) in vector.iter_mut().zip(correction) {
                *value -= correction;
            }
            orthogonalize(vector, previous)?;
            let applied = system.operator.apply_compensated(vector)?;
            *eigenvalue = rayleigh(vector, &applied);
            smooth_residual(system, eigenvalue, vector, &applied, previous, tolerance)?;
            changed = true;
        }
        // Reject root drift beyond the seed residual scale; this is not a spectral-index certificate.
        if !eigenvalue.is_finite()
            || *eigenvalue <= 0.0
            || (*eigenvalue / original - 1.0).abs() > 4.0 * initial_relative.max(tolerance)
        {
            return Err(format!(
                "modal dense mode {index} refinement left its residual-bounded spectral neighborhood"
            ));
        }
    }
    pairs.sort_by(|left, right| left.0.total_cmp(&right.0));
    Ok(())
}

fn smooth_residual(
    system: &ReducedSparseModalSystem,
    eigenvalue: &mut f64,
    vector: &mut Vec<f64>,
    applied: &[f64],
    previous: &[(f64, Vec<f64>)],
    tolerance: f64,
) -> Result<(), String> {
    let initial = relative_residual(*eigenvalue, vector, applied)?;
    if initial <= tolerance {
        return Ok(());
    }
    // One residual-minimizing Richardson candidate per inverse correction, never a retry loop.
    checkpoint(SolverStage::ModalValidation, 0)?;
    let mut direction: Vec<_> = applied
        .iter()
        .zip(vector.iter())
        .map(|(a, v)| (-*eigenvalue).mul_add(*v, *a))
        .collect();
    let norm = stable_l2_norm(direction.iter().copied());
    if !norm.is_finite() || norm == 0.0 {
        return Err("modal residual smoothing requires a finite nonzero residual".into());
    }
    direction.iter_mut().for_each(|v| *v /= norm);
    let mut projected = system.operator.apply_compensated(&direction)?;
    for (a, v) in projected.iter_mut().zip(&direction) {
        *a = (-*eigenvalue).mul_add(*v, *a);
    }
    let scale = projected.iter().map(|v| v.abs()).fold(0.0_f64, f64::max);
    if projected.iter().any(|v| !v.is_finite()) {
        return Err("modal residual smoothing produced a non-finite direction".into());
    }
    if scale == 0.0 {
        return Ok(());
    }
    projected.iter_mut().for_each(|v| *v /= scale);
    let step = (norm / scale) * (dot(&direction, &projected) / dot(&projected, &projected));
    if !step.is_finite() || step == 0.0 {
        return Ok(());
    }
    let mut candidate: Vec<_> = vector
        .iter()
        .zip(direction)
        .map(|(v, d)| (-step).mul_add(d, *v))
        .collect();
    orthogonalize(&mut candidate, previous)?;
    let applied = system.operator.apply_compensated(&candidate)?;
    let value = rayleigh(&candidate, &applied);
    if relative_residual(value, &candidate, &applied)? < initial {
        *vector = candidate;
        *eigenvalue = value;
    }
    Ok(())
}

fn relative_residual(value: f64, vector: &[f64], applied: &[f64]) -> Result<f64, String> {
    if vector.is_empty()
        || vector.len() != applied.len()
        || !value.is_finite()
        || value <= 0.0
        || vector.iter().chain(applied).any(|value| !value.is_finite())
    {
        return Err(
            "modal dense refinement requires finite positive eigenpairs with matching dimensions"
                .into(),
        );
    }
    let norm = stable_l2_norm(vector.iter().copied());
    let scale = stable_l2_norm(applied.iter().copied()).max(value.abs() * norm);
    let residual = stable_l2_norm(applied.iter().zip(vector).map(|(a, v)| a - value * v));
    let relative = residual / scale;
    if !relative.is_finite() || !scale.is_finite() || !norm.is_finite() || norm <= 0.0 {
        return Err("modal dense refinement produced an invalid residual or vector".into());
    }
    Ok(relative)
}

fn orthogonalize(vector: &mut [f64], previous: &[(f64, Vec<f64>)]) -> Result<(), String> {
    let norm = stable_l2_norm(vector.iter().copied());
    if !norm.is_finite() || norm <= 0.0 {
        return Err("modal dense refinement produced a zero or non-finite vector".into());
    }
    let initial = scale_binary(vector, norm)?;
    for _ in 0..2 {
        for (index, (_, basis)) in previous.iter().enumerate() {
            let denominator = dot(basis, basis);
            if basis.len() != vector.len() || !denominator.is_finite() || denominator <= 0.0 {
                return Err(
                    "modal refinement basis must have matching finite nonzero directions".into(),
                );
            }
            let overlap = dot(vector, basis) / denominator;
            for (value, basis) in vector.iter_mut().zip(basis) {
                *value = (-overlap).mul_add(*basis, *value);
            }
            checkpoint_chunk(SolverStage::ModalValidation, index + 1, previous.len())?;
        }
    }
    let norm = stable_l2_norm(vector.iter().copied());
    if !norm.is_finite() || norm < 0.5 * initial {
        return Err("modal dense refinement lost an independent direction".into());
    }
    scale_binary(vector, norm)?;
    Ok(())
}

// Internal modes need bounded size, not unit length. A power-of-two scale does
// not re-round every component and undo an accepted small residual correction.
fn scale_binary(vector: &mut [f64], norm: f64) -> Result<f64, String> {
    if !norm.is_finite() || norm <= 0.0 {
        return Err("modal binary scaling requires a finite positive norm".into());
    }
    let bits = norm.to_bits();
    let exponent = bits & (0x7ff_u64 << 52);
    let scale = f64::from_bits(if exponent == 0 {
        1_u64 << (63 - bits.leading_zeros())
    } else {
        exponent
    });
    // Keep norms <= 1 so Rayleigh products do not overflow for representable large roots.
    let bounded = norm / scale;
    let adjustment = if bounded > 1.0 { 0.5 } else { 1.0 };
    for value in vector {
        let scaled = (*value / scale) * adjustment;
        if !scaled.is_finite() || (*value != 0.0 && scaled == 0.0) {
            return Err("modal binary scaling cannot discard a nonzero component".into());
        }
        *value = scaled;
    }
    Ok(bounded * adjustment)
}

fn rayleigh(vector: &[f64], applied: &[f64]) -> f64 {
    dot(vector, applied) / dot(vector, vector)
}

fn dot(left: &[f64], right: &[f64]) -> f64 {
    let mut sum = 0.0;
    let mut correction = 0.0;
    for (left, right) in left.iter().zip(right) {
        let product = left * right;
        let next = sum + product;
        correction += if sum.abs() >= product.abs() {
            (sum - next) + product
        } else {
            (product - next) + sum
        };
        sum = next;
    }
    sum + correction
}

#[cfg(test)]
#[path = "modal_frame_refinement_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "modal_frame_polish_tests.rs"]
mod polish_tests;
