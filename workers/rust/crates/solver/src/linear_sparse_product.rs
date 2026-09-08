use super::CompressedSparseMatrix;
use crate::solver_control::{SolverStage, checkpoint};

const ROW_CHUNK: usize = 1024;

// Keep one accumulator across chunks: regrouping sums changes rounding and convergence.
#[inline]
pub(super) fn visit_row_chunks<T>(
    row: &[T],
    stage: SolverStage,
    mut visit: impl FnMut(usize, &[T]),
) -> Result<(), String> {
    let wide = row.len() > ROW_CHUNK;
    if wide {
        checkpoint(stage, 0)?;
    }
    for (index, chunk) in row.chunks(ROW_CHUNK).enumerate() {
        let offset = index * ROW_CHUNK;
        visit(offset, chunk);
        if wide {
            checkpoint(stage, offset + chunk.len())?;
        }
    }
    Ok(())
}

impl CompressedSparseMatrix {
    pub(crate) fn multiply_vector_into(
        &self,
        vector: &[f64],
        result: &mut [f64],
    ) -> Result<(), String> {
        debug_assert_eq!(result.len(), self.size());
        checkpoint(SolverStage::SparseMatvec, 0)?;
        if self.max_row_entries <= ROW_CHUNK {
            self.multiply_rows::<false>(vector, result)
        } else {
            self.multiply_rows::<true>(vector, result)
        }
    }

    // Compression records the immutable row bound once, not on every PCG product.
    fn multiply_rows<const WIDE: bool>(
        &self,
        vector: &[f64],
        result: &mut [f64],
    ) -> Result<(), String> {
        for (block, results) in result.chunks_mut(64).enumerate() {
            let first_row = block * 64;
            let completed = first_row + results.len();
            for (offset, result_value) in results.iter_mut().enumerate() {
                let row = first_row + offset;
                let start = self.row_offsets[row];
                let end = self.row_offsets[row + 1];
                let mut sum = 0.0;
                let columns = &self.columns[start..end];
                let values = &self.values[start..end];
                if !WIDE || columns.len() <= ROW_CHUNK {
                    for (&column, &value) in columns.iter().zip(values) {
                        sum += value * vector[column];
                    }
                } else {
                    visit_row_chunks(columns, SolverStage::SparseMatvecRow, |offset, columns| {
                        for (&column, &value) in columns.iter().zip(&values[offset..]) {
                            sum += value * vector[column];
                        }
                    })?;
                }
                *result_value = sum;
            }
            checkpoint(SolverStage::SparseMatvec, completed)?;
        }
        Ok(())
    }
}
