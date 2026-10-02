use super::SparseMassNormalizedOperator;
use crate::linear_algebra::{SparseMatrix, add_at};
use crate::modal_frame_assembly::positive_product;
use crate::modal_math::ensure_dense_modal_size;
use crate::solver_control::{SolverStage, checkpoint, checkpoint_chunk};

impl SparseMassNormalizedOperator {
    /// Fill the bounded Jacobi matrix directly, without one sparse product per column.
    pub(crate) fn dense_fallback_matrix(&self) -> Result<Vec<Vec<f64>>, String> {
        let size = self.inverse_mass_sqrt.len();
        ensure_dense_modal_size(size, "modal frame")?;
        checkpoint(SolverStage::SparseMatrixScale, 0)?;
        let mut dense = vec![vec![0.0; size]; size];
        self.visit_normalized_entries(|row, column, value| dense[row][column] = value)?;
        Ok(dense)
    }

    pub(super) fn normalized_stiffness(&self) -> Result<SparseMatrix, String> {
        checkpoint(SolverStage::SparseMatrixScale, 0)?;
        let mut normalized = SparseMatrix::new(self.inverse_mass_sqrt.len());
        self.visit_normalized_entries(|row, column, value| {
            add_at(&mut normalized, row, column, value);
        })?;
        Ok(normalized)
    }

    fn visit_normalized_entries(
        &self,
        mut emit: impl FnMut(usize, usize, f64),
    ) -> Result<(), String> {
        let size = self.inverse_mass_sqrt.len();
        for row in 0..size {
            let start = self.stiffness.row_offsets[row];
            let end = self.stiffness.row_offsets[row + 1];
            for entry in start..end {
                let column = self.stiffness.columns[entry];
                let value = self.stiffness.values[entry];
                // Identical factors take an identical multiplication order in both triangles.
                // A lost coupling must be an error, never a new independent component.
                let normalized = positive_product([
                    value.abs(),
                    self.inverse_mass_sqrt[row],
                    self.inverse_mass_sqrt[column],
                ])
                .copysign(value);
                if !normalized.is_finite() || (value != 0.0 && normalized == 0.0) {
                    return Err("modal normalized stiffness loses a nonzero or finite entry".into());
                }
                emit(row, column, normalized);
                checkpoint_chunk(
                    SolverStage::SparseMatrixScaleRow,
                    entry - start + 1,
                    end - start,
                )?;
            }
            checkpoint_chunk(SolverStage::SparseMatrixScale, row + 1, size)?;
        }
        Ok(())
    }
}
