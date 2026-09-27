use crate::linear_algebra::SparseMatrix;
use crate::solver_control::{SolverStage, checkpoint, checkpoint_chunk};

pub(crate) fn validate_stiffness(context: &str, id: &str, value: f64) -> Result<(), String> {
    if !value.is_finite() || value <= 0.0 {
        return Err(format!(
            "{context} element {id}: stiffness is not representable"
        ));
    }
    Ok(())
}

pub(crate) fn validate_fields(
    context: &str,
    id: &str,
    field: &str,
    values: &[f64],
) -> Result<(), String> {
    if values.iter().any(|value| !value.is_finite()) {
        return Err(format!("{context} {id}: {field} is not representable"));
    }
    Ok(())
}

pub(crate) fn validate_assembled_stiffness(
    context: &str,
    matrix: &SparseMatrix,
) -> Result<(), String> {
    checkpoint(SolverStage::SparseValidateMatrix, 0)?;
    for row in 0..matrix.size() {
        if matrix
            .row_entries(row)
            .iter()
            .any(|(_, value)| !value.is_finite())
        {
            return Err(format!(
                "{context} assembled stiffness at dof {row} is not representable"
            ));
        }
        checkpoint_chunk(SolverStage::SparseValidateMatrix, row + 1, matrix.size())?;
    }
    Ok(())
}

pub(crate) fn validate_small_displacement<const N: usize>(
    context: &str,
    points: impl Iterator<Item = [f64; N]>,
    max_displacement: f64,
) -> Result<(), String> {
    let mut minima = [f64::INFINITY; N];
    let mut maxima = [f64::NEG_INFINITY; N];
    for point in points {
        for axis in 0..N {
            minima[axis] = minima[axis].min(point[axis]);
            maxima[axis] = maxima[axis].max(point[axis]);
        }
    }
    let characteristic_length = (0..N)
        .map(|axis| maxima[axis] - minima[axis])
        .fold(0.0_f64, f64::max);
    if !max_displacement.is_finite()
        || !characteristic_length.is_finite()
        || characteristic_length <= 0.0
    {
        return Err(format!(
            "{context} displacement or model extent is not representable"
        ));
    }
    if max_displacement > characteristic_length * 0.25 {
        return Err(format!(
            "{context} response exceeds the small-deformation limit; check supports or connectivity"
        ));
    }
    Ok(())
}
