use crate::linear_algebra::cancellable_l2_norm;
#[cfg(test)]
use crate::linear_algebra::stable_l2_norm;
use crate::modal_sparse::ReducedSparseModalSystem;
use crate::solver_control::{SolverStage, checkpoint, checkpoint_chunk};

const MAX_REFINEMENT_STEPS: usize = 4;

#[path = "modal_roundoff.rs"]
pub(crate) mod roundoff;

#[derive(Debug)]
struct CheckedResidual {
    applied: Vec<f64>,
    relative: f64,
}

pub(super) fn refine_dense_modes(
    system: &ReducedSparseModalSystem,
    pairs: &mut [(f64, Vec<f64>)],
    tolerance: f64,
) -> Result<(), String> {
    let allow_roundoff = pairs.len() == 1 && roundoff::eligible(system.free_dofs.len());
    refine_internal(
        system,
        pairs,
        tolerance,
        MAX_REFINEMENT_STEPS,
        allow_roundoff,
    )
}

#[cfg(test)]
fn refine_with_budget(
    system: &ReducedSparseModalSystem,
    pairs: &mut [(f64, Vec<f64>)],
    tolerance: f64,
    max_steps: usize,
) -> Result<(), String> {
    refine_internal(system, pairs, tolerance, max_steps, false)
}

fn refine_internal(
    system: &ReducedSparseModalSystem,
    pairs: &mut [(f64, Vec<f64>)],
    tolerance: f64,
    max_steps: usize,
    allow_roundoff: bool,
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
        let mut search = vector.clone();
        let mut search_value = original;
        let mut best_relative = initial_relative;
        let mut checked = CheckedResidual {
            applied,
            relative: initial_relative,
        };
        if changed {
            orthogonalize(&mut search, previous)?;
            checked.applied = system.operator.apply_compensated(&search)?;
            search_value = rayleigh(&search, &checked.applied)?;
            checked.relative = relative_residual(search_value, &search, &checked.applied)?;
            // Compare candidates in the same deflated subspace, not to an unprojected seed.
            best_relative = f64::INFINITY;
        }
        for step in 0..=max_steps {
            checkpoint(SolverStage::ModalIteration, step)?;
            let relative = checked.relative;
            if relative < best_relative {
                *vector = search.clone();
                *eigenvalue = search_value;
                best_relative = relative;
            }
            if relative <= tolerance {
                break;
            }
            if step == max_steps {
                if allow_roundoff {
                    *vector = roundoff::normalized(system, *eigenvalue, vector, tolerance)?;
                    break;
                }
                return Err(format!(
                    "modal dense mode {index} refinement did not converge within {max_steps} steps (relative={best_relative:.6e})"
                ));
            }
            if inverse.is_none() {
                inverse = Some(system.prepare_normalized_inverse()?);
            }
            // Correct the physical-operator residual, rather than replacing the seed
            // with an eigenvector of the separately rounded normalized factor matrix.
            let residual = residual_vector(search_value, &search, &checked.applied)?;
            let correction = inverse.as_ref().unwrap().solve(&residual)?;
            checkpoint(SolverStage::ModalVectorUpdate, 0)?;
            let size = search.len();
            let mut candidate = Vec::with_capacity(size);
            for (index, (value, correction)) in search.iter().zip(correction).enumerate() {
                candidate.push(value - correction);
                checkpoint_chunk(SolverStage::ModalVectorUpdate, index + 1, size)?;
            }
            orthogonalize(&mut candidate, previous)?;
            let applied = system.operator.apply_compensated(&candidate)?;
            let mut value = rayleigh(&candidate, &applied)?;
            // Carry the exact checked product with its private iterate; recomputing
            // it on loop entry adds work, not an independent validation boundary.
            checked = smooth_residual(
                system,
                &mut value,
                &mut candidate,
                applied,
                previous,
                tolerance,
            )?;
            // Keep the best checked pair, but permit private uphill search steps:
            // rounding can make a later correction improve after a temporary regression.
            if checked.relative < best_relative {
                *vector = candidate.clone();
                *eigenvalue = value;
                best_relative = checked.relative;
            }
            search = candidate;
            search_value = value;
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
    applied: Vec<f64>,
    previous: &[(f64, Vec<f64>)],
    tolerance: f64,
) -> Result<CheckedResidual, String> {
    let initial = relative_residual(*eigenvalue, vector, &applied)?;
    if initial <= tolerance {
        return Ok(CheckedResidual {
            applied,
            relative: initial,
        });
    }
    // One residual-minimizing Richardson candidate per inverse correction, never a retry loop.
    checkpoint(SolverStage::ModalValidation, 0)?;
    let mut direction = residual_vector(*eigenvalue, vector, &applied)?;
    let norm = vector_norm(direction.iter().copied())?;
    if !norm.is_finite() || norm == 0.0 {
        return Err("modal residual smoothing requires a finite nonzero residual".into());
    }
    let size = vector.len();
    checkpoint(SolverStage::ModalVectorUpdate, 0)?;
    for (index, value) in direction.iter_mut().enumerate() {
        *value /= norm;
        checkpoint_chunk(SolverStage::ModalVectorUpdate, index + 1, size)?;
    }
    let mut projected = system.operator.apply_compensated(&direction)?;
    checkpoint(SolverStage::ModalVectorUpdate, 0)?;
    for (index, (a, v)) in projected.iter_mut().zip(&direction).enumerate() {
        *a = (-*eigenvalue).mul_add(*v, *a);
        checkpoint_chunk(SolverStage::ModalVectorUpdate, index + 1, size)?;
    }
    let mut scale = 0.0_f64;
    checkpoint(SolverStage::ModalVectorScan, 0)?;
    for (index, value) in projected.iter().enumerate() {
        if !value.is_finite() {
            return Err("modal residual smoothing produced a non-finite direction".into());
        }
        scale = scale.max(value.abs());
        checkpoint_chunk(SolverStage::ModalVectorScan, index + 1, size)?;
    }
    if scale == 0.0 {
        return Ok(CheckedResidual {
            applied,
            relative: initial,
        });
    }
    checkpoint(SolverStage::ModalVectorUpdate, 0)?;
    for (index, value) in projected.iter_mut().enumerate() {
        *value /= scale;
        checkpoint_chunk(SolverStage::ModalVectorUpdate, index + 1, size)?;
    }
    let step = (norm / scale) * (dot(&direction, &projected)? / dot(&projected, &projected)?);
    if !step.is_finite() || step == 0.0 {
        return Ok(CheckedResidual {
            applied,
            relative: initial,
        });
    }
    checkpoint(SolverStage::ModalVectorUpdate, 0)?;
    let mut candidate = Vec::with_capacity(size);
    for (index, (v, d)) in vector.iter().zip(direction).enumerate() {
        candidate.push((-step).mul_add(d, *v));
        checkpoint_chunk(SolverStage::ModalVectorUpdate, index + 1, size)?;
    }
    orthogonalize(&mut candidate, previous)?;
    let candidate_applied = system.operator.apply_compensated(&candidate)?;
    let value = rayleigh(&candidate, &candidate_applied)?;
    let relative = relative_residual(value, &candidate, &candidate_applied)?;
    if relative < initial {
        *vector = candidate;
        *eigenvalue = value;
        return Ok(CheckedResidual {
            applied: candidate_applied,
            relative,
        });
    }
    Ok(CheckedResidual {
        applied,
        relative: initial,
    })
}

fn relative_residual(value: f64, vector: &[f64], applied: &[f64]) -> Result<f64, String> {
    if vector.is_empty() || vector.len() != applied.len() || !value.is_finite() || value <= 0.0 {
        return Err(
            "modal dense refinement requires finite positive eigenpairs with matching dimensions"
                .into(),
        );
    }
    checkpoint(SolverStage::ModalVectorScan, 0)?;
    for (index, (mode, applied)) in vector.iter().zip(applied).enumerate() {
        if !mode.is_finite() || !applied.is_finite() {
            return Err("modal dense refinement requires finite positive eigenpairs with matching dimensions".into());
        }
        checkpoint_chunk(SolverStage::ModalVectorScan, index + 1, vector.len())?;
    }
    let norm = vector_norm(vector.iter().copied())?;
    let scale = vector_norm(applied.iter().copied())?.max(value.abs() * norm);
    let residual = vector_norm(applied.iter().zip(vector).map(|(a, v)| a - value * v))?;
    let relative = residual / scale;
    if !relative.is_finite() || !scale.is_finite() || !norm.is_finite() || norm <= 0.0 {
        return Err("modal dense refinement produced an invalid residual or vector".into());
    }
    Ok(relative)
}

fn orthogonalize(vector: &mut [f64], previous: &[(f64, Vec<f64>)]) -> Result<(), String> {
    let norm = vector_norm(vector.iter().copied())?;
    if !norm.is_finite() || norm <= 0.0 {
        return Err("modal dense refinement produced a zero or non-finite vector".into());
    }
    let initial = scale_binary(vector, norm)?;
    for _ in 0..2 {
        for (index, (_, basis)) in previous.iter().enumerate() {
            if basis.len() != vector.len() {
                return Err(
                    "modal refinement basis must have matching finite nonzero directions".into(),
                );
            }
            let denominator = dot(basis, basis)?;
            if denominator <= 0.0 {
                return Err(
                    "modal refinement basis must have matching finite nonzero directions".into(),
                );
            }
            let overlap = dot(vector, basis)? / denominator;
            checkpoint(SolverStage::ModalVectorUpdate, 0)?;
            let size = vector.len();
            for (component, (value, basis)) in vector.iter_mut().zip(basis).enumerate() {
                *value = (-overlap).mul_add(*basis, *value);
                checkpoint_chunk(SolverStage::ModalVectorUpdate, component + 1, size)?;
            }
            checkpoint_chunk(SolverStage::ModalValidation, index + 1, previous.len())?;
        }
    }
    let norm = vector_norm(vector.iter().copied())?;
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
    checkpoint(SolverStage::ModalVectorUpdate, 0)?;
    let size = vector.len();
    for (index, value) in vector.iter_mut().enumerate() {
        let scaled = (*value / scale) * adjustment;
        if !scaled.is_finite() || (*value != 0.0 && scaled == 0.0) {
            return Err("modal binary scaling cannot discard a nonzero component".into());
        }
        *value = scaled;
        checkpoint_chunk(SolverStage::ModalVectorUpdate, index + 1, size)?;
    }
    Ok(bounded * adjustment)
}

fn vector_norm(values: impl ExactSizeIterator<Item = f64>) -> Result<f64, String> {
    cancellable_l2_norm(values, SolverStage::ModalVectorScan)
}

fn residual_vector(value: f64, vector: &[f64], applied: &[f64]) -> Result<Vec<f64>, String> {
    checkpoint(SolverStage::ModalVectorUpdate, 0)?;
    let size = vector.len();
    let mut residual = Vec::with_capacity(size);
    for (index, (a, v)) in applied.iter().zip(vector).enumerate() {
        residual.push((-value).mul_add(*v, *a));
        checkpoint_chunk(SolverStage::ModalVectorUpdate, index + 1, size)?;
    }
    Ok(residual)
}

fn rayleigh(vector: &[f64], applied: &[f64]) -> Result<f64, String> {
    let numerator = dot(vector, applied)?;
    let denominator = dot(vector, vector)?;
    let value = numerator / denominator;
    if !value.is_finite() || value <= 0.0 {
        return Err("modal dense refinement Rayleigh value must remain finite and positive".into());
    }
    Ok(value)
}

fn dot(left: &[f64], right: &[f64]) -> Result<f64, String> {
    if left.len() != right.len() {
        return Err("modal refinement dot product requires matching dimensions".into());
    }
    checkpoint(SolverStage::ModalVectorDot, 0)?;
    let size = left.len();
    let mut sum = 0.0;
    let mut correction = 0.0;
    for (index, (left, right)) in left.iter().zip(right).enumerate() {
        let product = left * right;
        let next = sum + product;
        correction += if sum.abs() >= product.abs() {
            (sum - next) + product
        } else {
            (product - next) + sum
        };
        sum = next;
        checkpoint_chunk(SolverStage::ModalVectorDot, index + 1, size)?;
    }
    let value = sum + correction;
    if !value.is_finite() {
        return Err("modal refinement dot product must remain finite".into());
    }
    Ok(value)
}

#[cfg(test)]
#[path = "modal_frame_refinement_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "modal_frame_polish_tests.rs"]
mod polish_tests;

#[cfg(test)]
#[path = "modal_frame_resolution_tests.rs"]
mod resolution_tests;

#[cfg(test)]
#[path = "modal_frame_refinement_control_tests.rs"]
mod control_tests;
