use crate::linear_solver_profile::SpdSolveProfile;
use crate::solver_control::{SolverStage, checkpoint};

use super::{SparseMatrix, product::visit_row_chunks};

const VECTOR_CHUNK: usize = 1024;

#[inline]
fn for_rows(
    matrix: &SparseMatrix,
    stage: SolverStage,
    mut visit: impl FnMut(usize, &[(usize, f64)]) -> Result<(), String>,
) -> Result<(), String> {
    checkpoint(stage, 0)?;
    for (block, rows) in matrix.rows.chunks(64).enumerate() {
        let start = block * 64;
        for (offset, row) in rows.iter().enumerate() {
            visit(start + offset, row)?;
        }
        checkpoint(stage, start + rows.len())?;
    }
    Ok(())
}

pub(super) fn diagonal_sparse_scaling(matrix: &SparseMatrix) -> Result<Vec<f64>, String> {
    let mut scaling = Vec::with_capacity(matrix.size());
    for_rows(matrix, SolverStage::SparseDiagonalScale, |index, _| {
        // SparseMatrix keeps sorted, unique columns; reuse its logarithmic lookup.
        let diagonal = matrix.diagonal_value(index).abs();
        scaling.push(if diagonal > 0.0 {
            diagonal.sqrt().recip()
        } else {
            1.0
        });
        Ok(())
    })?;
    Ok(scaling)
}

pub(super) fn scale_sparse_matrix(
    matrix: &SparseMatrix,
    scaling: &[f64],
) -> Result<SparseMatrix, String> {
    debug_assert_eq!(matrix.size(), scaling.len());
    let capacity = row_capacity_hint(matrix)?;
    let mut scaled = SparseMatrix {
        rows: Vec::with_capacity(matrix.size()),
    };
    for_rows(matrix, SolverStage::SparseMatrixScale, |row_index, row| {
        scaled.rows.push(Vec::with_capacity(capacity));
        let row_scale = scaling[row_index];
        visit_row_chunks(row, SolverStage::SparseMatrixScaleRow, |_, chunk| {
            for &(column, value) in chunk {
                scaled.push_sorted_entry(row_index, column, value * row_scale * scaling[column]);
            }
        })
    })?;
    Ok(scaled)
}

pub(super) fn scale_sparse_rhs(rhs: &[f64], scaling: &[f64]) -> Result<Vec<f64>, String> {
    scale_vector(rhs, scaling, SolverStage::SparseRhsScale)
}

pub(super) fn unscale_profile(
    profile: SpdSolveProfile,
    scaling: &[f64],
) -> Result<SpdSolveProfile, String> {
    Ok(SpdSolveProfile {
        solution: scale_vector(
            &profile.solution,
            scaling,
            SolverStage::SparseSolutionUnscale,
        )?,
        iterations: profile.iterations,
        matrix_non_zero_count: profile.matrix_non_zero_count,
        residual_norm: profile.residual_norm,
        stages: profile.stages,
    })
}

pub(super) fn average_scaled_diagonal_magnitude(
    matrix: &SparseMatrix,
    scaling: &[f64],
) -> Result<f64, String> {
    debug_assert_eq!(matrix.size(), scaling.len());
    let size = matrix.size().max(1);
    // Match Iterator::sum's identity and retain its sequential addition order.
    let mut diagonal_sum = -0.0;
    for_rows(matrix, SolverStage::SparseDiagonalMagnitude, |index, _| {
        diagonal_sum += matrix.diagonal_value(index).abs() * scaling[index] * scaling[index];
        Ok(())
    })?;
    Ok(diagonal_sum / size as f64)
}

pub(super) fn regularize_sparse_diagonal(
    matrix: &SparseMatrix,
    epsilon: f64,
) -> Result<SparseMatrix, String> {
    let capacity = row_capacity_hint(matrix)? + 1;
    let mut regularized = SparseMatrix {
        rows: Vec::with_capacity(matrix.size()),
    };
    for_rows(
        matrix,
        SolverStage::SparseRegularizeCopy,
        |row_index, row| {
            regularized.rows.push(Vec::with_capacity(capacity));
            visit_row_chunks(row, SolverStage::SparseRegularizeCopyRow, |_, chunk| {
                regularized.rows[row_index].extend_from_slice(chunk);
            })
        },
    )?;
    for_rows(matrix, SolverStage::SparseRegularizeDiagonal, |row, _| {
        regularized.add_at(row, row, epsilon);
        Ok(())
    })?;
    Ok(regularized)
}

pub(super) fn validate_sparse_system_finite(
    matrix: &SparseMatrix,
    rhs: &[f64],
) -> Result<(), String> {
    checkpoint(SolverStage::SparseValidateRhs, 0)?;
    for (block, values) in rhs.chunks(VECTOR_CHUNK).enumerate() {
        if !all_finite(values.iter().copied()) {
            return Err("linear system vector contains non-finite value".to_string());
        }
        checkpoint(
            SolverStage::SparseValidateRhs,
            block * VECTOR_CHUNK + values.len(),
        )?;
    }
    for_rows(matrix, SolverStage::SparseValidateMatrix, |_, row| {
        if row.len() <= VECTOR_CHUNK {
            return if row.iter().any(|(_, value)| !value.is_finite()) {
                Err("linear system matrix contains non-finite value".to_string())
            } else {
                Ok(())
            };
        }
        checkpoint(SolverStage::SparseValidateMatrixRow, 0)?;
        for (block, chunk) in row.chunks(VECTOR_CHUNK).enumerate() {
            if !all_finite(chunk.iter().map(|(_, value)| *value)) {
                return Err("linear system matrix contains non-finite value".to_string());
            }
            checkpoint(
                SolverStage::SparseValidateMatrixRow,
                block * VECTOR_CHUNK + chunk.len(),
            )?;
        }
        Ok(())
    })
}

fn scale_vector(values: &[f64], scaling: &[f64], stage: SolverStage) -> Result<Vec<f64>, String> {
    debug_assert_eq!(values.len(), scaling.len());
    checkpoint(stage, 0)?;
    let mut result = Vec::with_capacity(values.len());
    for (block, values) in values.chunks(VECTOR_CHUNK).enumerate() {
        let offset = block * VECTOR_CHUNK;
        result.extend(
            values
                .iter()
                .enumerate()
                .map(|(index, value)| value * scaling[offset + index]),
        );
        checkpoint(stage, offset + values.len())?;
    }
    Ok(result)
}

// A non-short-circuit boolean reduction vectorizes within one bounded input block.
// Validation reports no offending index; RHS/matrix error precedence is unchanged.
#[inline]
fn all_finite(values: impl Iterator<Item = f64>) -> bool {
    values.fold(true, |valid, value| valid & value.is_finite())
}

fn row_capacity_hint(matrix: &SparseMatrix) -> Result<usize, String> {
    checkpoint(SolverStage::SparseCapacityScan, 0)?;
    let mut count = 0;
    for (block, rows) in matrix.rows.chunks(VECTOR_CHUNK).enumerate() {
        count += rows.iter().map(Vec::len).sum::<usize>();
        checkpoint(
            SolverStage::SparseCapacityScan,
            block * VECTOR_CHUNK + rows.len(),
        )?;
    }
    Ok(count.div_ceil(matrix.size().max(1)).max(1))
}
