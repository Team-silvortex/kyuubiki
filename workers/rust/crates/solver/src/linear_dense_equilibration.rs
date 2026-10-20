use crate::linear_binary_scale::{restore, split_finite};
use crate::solver_control::{SolverStage, checkpoint, checkpoint_chunk};

pub(super) struct RowEquilibration {
    // Original row order: pivoting only permutes the factors and RHS lookup.
    exponents: Vec<i32>,
}

impl RowEquilibration {
    pub(super) fn prepare(
        matrix: &mut [Vec<f64>],
        scales: &mut [f64],
    ) -> Result<Option<Self>, String> {
        if scales
            .iter()
            .all(|&scale| (-500..=500).contains(&split_finite(scale).1))
        {
            return Ok(None);
        }
        checkpoint(SolverStage::DenseRowScale, 0)?;
        let size = matrix.len();
        let mut exponents = Vec::with_capacity(size);
        for (row, (entries, scale)) in matrix.iter_mut().zip(scales).enumerate() {
            let exponent = -split_finite(*scale).1;
            for (column, value) in entries.iter_mut().enumerate() {
                *value = exact_scale(*value, exponent, row, "coefficient")?;
                checkpoint_chunk(
                    SolverStage::DenseRowScale,
                    row * size + column + 1,
                    size * size,
                )?;
            }
            *scale = restore(*scale, exponent);
            exponents.push(exponent);
        }
        Ok(Some(Self { exponents }))
    }

    pub(super) fn permute_rhs(
        &self,
        rhs: &[f64],
        permutation: &[usize],
    ) -> Result<Vec<f64>, String> {
        checkpoint(SolverStage::DenseRhsScale, 0)?;
        permutation
            .iter()
            .enumerate()
            .map(|(position, &row)| {
                let scaled = exact_scale(rhs[row], self.exponents[row], row, "right-hand side")?;
                checkpoint_chunk(SolverStage::DenseRhsScale, position + 1, rhs.len())?;
                Ok(scaled)
            })
            .collect()
    }
}

fn exact_scale(value: f64, exponent: i32, row: usize, kind: &str) -> Result<f64, String> {
    let scaled = restore(value, exponent);
    if !scaled.is_finite() || restore(scaled, -exponent) != value {
        return Err(format!(
            "dense row {row} {kind} cannot be scaled without range loss"
        ));
    }
    Ok(scaled)
}
