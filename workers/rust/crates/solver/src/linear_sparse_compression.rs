use super::{CompressedSparseMatrix, IncompleteCholesky, SparseMatrix, safe_diagonal};
use crate::linear_solver_profile::SpdPreconditioner;
use crate::solver_control::{SolverStage, checkpoint, checkpoint_chunk};

impl SparseMatrix {
    pub(crate) fn compress(
        &self,
        preconditioner: SpdPreconditioner,
    ) -> Result<CompressedSparseMatrix, String> {
        checkpoint(SolverStage::SparseCompress, 0)?;
        let size = self.size();
        let mut row_offsets = Vec::with_capacity(size + 1);
        let mut lower_end_offsets = Vec::with_capacity(size);
        let mut upper_start_offsets = Vec::with_capacity(size);
        let mut columns = Vec::new();
        let mut values = Vec::new();
        let mut diagonal = vec![0.0; size];

        row_offsets.push(0);
        for (row_index, row) in self.rows.iter().enumerate() {
            let row_start = columns.len();
            lower_end_offsets
                .push(row_start + row.partition_point(|(column, _)| *column < row_index));
            upper_start_offsets
                .push(row_start + row.partition_point(|(column, _)| *column <= row_index));
            for &(column, value) in row {
                if column == row_index {
                    diagonal[row_index] = value;
                }
                columns.push(column);
                values.push(value);
            }
            row_offsets.push(columns.len());
            checkpoint_chunk(SolverStage::SparseCompress, row_index + 1, size)?;
        }
        CompressedSparseMatrix {
            row_offsets,
            lower_end_offsets,
            upper_start_offsets,
            columns,
            values,
            diagonal,
            incomplete_cholesky: None,
            inverse_diagonal: Vec::new(),
        }
        .prepare_preconditioner(preconditioner)
    }

    pub(super) fn compress_scaled(
        &self,
        scaling: &[f64],
        preconditioner: SpdPreconditioner,
    ) -> Result<CompressedSparseMatrix, String> {
        checkpoint(SolverStage::SparseCompress, 0)?;
        let size = self.size();
        debug_assert_eq!(scaling.len(), size);
        let mut row_offsets = Vec::with_capacity(size + 1);
        let mut lower_end_offsets = Vec::with_capacity(size);
        let mut upper_start_offsets = Vec::with_capacity(size);
        let non_zero_hint = self.non_zero_count();
        let mut columns = Vec::with_capacity(non_zero_hint);
        let mut values = Vec::with_capacity(non_zero_hint);
        let mut diagonal = vec![0.0; size];

        row_offsets.push(0);
        for (row_index, row) in self.rows.iter().enumerate() {
            let row_start = columns.len();
            lower_end_offsets
                .push(row_start + row.partition_point(|(column, _)| *column < row_index));
            upper_start_offsets
                .push(row_start + row.partition_point(|(column, _)| *column <= row_index));
            let row_scale = scaling[row_index];
            for &(column, value) in row {
                let scaled_value = value * row_scale * scaling[column];
                if column == row_index {
                    diagonal[row_index] = scaled_value;
                }
                columns.push(column);
                values.push(scaled_value);
            }
            row_offsets.push(columns.len());
            checkpoint_chunk(SolverStage::SparseCompress, row_index + 1, size)?;
        }
        CompressedSparseMatrix {
            row_offsets,
            lower_end_offsets,
            upper_start_offsets,
            columns,
            values,
            diagonal,
            incomplete_cholesky: None,
            inverse_diagonal: Vec::new(),
        }
        .prepare_preconditioner(preconditioner)
    }
}

impl CompressedSparseMatrix {
    fn prepare_preconditioner(mut self, preconditioner: SpdPreconditioner) -> Result<Self, String> {
        checkpoint(SolverStage::PreconditionerSetup, 0)?;
        self.inverse_diagonal = Vec::with_capacity(self.size());
        for (index, value) in self.diagonal.iter().enumerate() {
            self.inverse_diagonal.push(safe_diagonal(*value).recip());
            checkpoint_chunk(SolverStage::PreconditionerSetup, index + 1, self.size())?;
        }
        if matches!(preconditioner, SpdPreconditioner::IncompleteCholesky) {
            self.incomplete_cholesky = Some(IncompleteCholesky::build(
                &self.row_offsets,
                &self.lower_end_offsets,
                &self.columns,
                &self.values,
                &self.diagonal,
            )?);
        }
        Ok(self)
    }
}
