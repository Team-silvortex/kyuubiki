use super::{Accepted, Contract, Plan, search_with_plan};
use crate::modal_test_wide::Wide;
use crate::solver_control::{SolverStage, checkpoint, checkpoint_chunk};

// A separate test-only entry, never an extra retry after the raw portfolio.
pub(super) fn search_canonical_unit_shape(
    matrix: &[Vec<Wide>],
    seed: &[f64],
    mass: &[f64],
    tolerance: f64,
    checked: impl FnMut(&[f64]) -> Result<(f64, Vec<f64>), String>,
) -> Result<Accepted, String> {
    search_canonical(matrix, seed, mass, tolerance, Contract::UnitShape, checked)
}

pub(super) fn search_canonical_direction(
    matrix: &[Vec<Wide>],
    seed: &[f64],
    tolerance: f64,
    checked: impl FnMut(&[f64]) -> Result<(f64, Vec<f64>), String>,
) -> Result<Accepted, String> {
    Plan::canonical(matrix.len())?;
    if seed.len() != matrix.len() {
        return Err("canonical internal direction requires matching dimensions".into());
    }
    // These coordinates are already mass-normalized; never weight them again.
    search_canonical(
        matrix,
        seed,
        &vec![1.0; seed.len()],
        tolerance,
        Contract::InternalDirection,
        checked,
    )
}

fn search_canonical(
    matrix: &[Vec<Wide>],
    seed: &[f64],
    mass: &[f64],
    tolerance: f64,
    contract: Contract,
    mut checked: impl FnMut(&[f64]) -> Result<(f64, Vec<f64>), String>,
) -> Result<Accepted, String> {
    let plan = Plan::canonical(matrix.len())?;
    // Reuse strict preflight, including range/cancellation, before canonical copies.
    super::validate_inputs(matrix, seed, mass, tolerance, contract)?;
    let permutation = canonical_permutation(matrix, seed, mass)?;
    let size = seed.len();
    let mut canonical = Vec::with_capacity(size);
    for &i in &permutation {
        let mut row = Vec::with_capacity(size);
        for (visit, &j) in permutation.iter().enumerate() {
            row.push(matrix[i][j]);
            checkpoint_chunk(SolverStage::ModalVectorUpdate, visit + 1, size)?;
        }
        canonical.push(row);
    }
    let mapped_seed: Vec<_> = permutation.iter().map(|&i| seed[i]).collect();
    let mapped_mass: Vec<_> = permutation.iter().map(|&i| mass[i]).collect();
    let mut result = search_with_plan(
        &canonical,
        &mapped_seed,
        &mapped_mass,
        tolerance,
        plan,
        contract,
        |v| {
            let original = restore(&permutation, v)?;
            let (relative, residual) = checked(&original)?;
            if residual.len() != size {
                return Err("canonical grid requires a matching original residual".into());
            }
            let mut mapped = Vec::with_capacity(size);
            for (visit, &i) in permutation.iter().enumerate() {
                mapped.push(residual[i]);
                checkpoint_chunk(SolverStage::ModalVectorUpdate, visit + 1, size)?;
            }
            Ok((relative, mapped))
        },
    )?;
    result.shape = restore(&permutation, &result.shape)?;
    result.anchor = permutation[result.anchor];
    // Norm summation order differs after restoration; do not assume invariance.
    contract.validate(&result.shape)?;
    if result.shape[result.anchor].to_bits() != seed[result.anchor].to_bits() {
        return Err("canonical grid changed its original frozen anchor".into());
    }
    checkpoint(SolverStage::ModalRoundoffValidate, 5)?;
    Ok(result)
}

fn canonical_permutation(
    matrix: &[Vec<Wide>],
    seed: &[f64],
    mass: &[f64],
) -> Result<Vec<usize>, String> {
    let mut keys = Vec::with_capacity(seed.len());
    for (i, &shape) in seed.iter().enumerate() {
        let normalized = matrix[i][i].rounded().abs() / mass[i].sqrt();
        if !normalized.is_finite() {
            return Err("canonical grid diagonal signature is not representable".into());
        }
        // Binary scale ignores insignificant diagonal mantissa differences.
        let exponent = (normalized.to_bits() >> 52) & 0x7ff;
        keys.push((exponent, shape.abs().to_bits(), i));
        checkpoint_chunk(SolverStage::ModalVectorUpdate, i + 1, seed.len())?;
    }
    keys.sort_unstable_by_key(|&(scale, amplitude, _)| (scale, amplitude));
    if keys
        .windows(2)
        .any(|pair| pair[0].0 == pair[1].0 && pair[0].1 == pair[1].1)
    {
        return Err("canonical grid has ambiguous coordinate signatures".into());
    }
    Ok(keys.into_iter().map(|(_, _, index)| index).collect())
}

fn restore(permutation: &[usize], vector: &[f64]) -> Result<Vec<f64>, String> {
    let mut original = vec![0.0; vector.len()];
    for (visit, (&i, &a)) in permutation.iter().zip(vector).enumerate() {
        original[i] = a;
        checkpoint_chunk(SolverStage::ModalVectorUpdate, visit + 1, vector.len())?;
    }
    Ok(original)
}

#[path = "modal_roundoff_grid_canonical_tests.rs"]
mod tests;

#[path = "modal_roundoff_grid_heterogeneous_tests.rs"]
mod heterogeneous_tests;
