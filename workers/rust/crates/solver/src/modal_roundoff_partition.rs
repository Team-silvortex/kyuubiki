use super::{budget::Plan, dot, vector_norm};
use crate::solver_control::{SolverStage, checkpoint, checkpoint_chunk};

// Grid size weights a proposal; only a fresh real-operator residual certifies it.
pub(crate) fn select_fine(matrix: &[Vec<f64>], vector: &[f64]) -> Result<Vec<usize>, String> {
    let size = matrix.len();
    Plan::new(size)?;
    if matrix
        .iter()
        .any(|r| r.len() != size || r.iter().any(|v| !v.is_finite()))
        || vector.len() != size
        || vector.iter().any(|v| !v.is_finite())
        || vector.iter().all(|v| *v == 0.0)
    {
        return Err("automatic partition requires bounded finite matching data".into());
    }
    let scale = matrix
        .iter()
        .flatten()
        .map(|v| v.abs())
        .fold(0.0_f64, f64::max);
    if scale == 0.0 {
        return Err("automatic partition has no independent columns".into());
    }
    let mut columns = Vec::with_capacity(size);
    for j in 0..size {
        let mut column = Vec::with_capacity(size);
        for (i, row) in matrix.iter().enumerate() {
            let scaled = row[j] / scale;
            if row[j] != 0.0 && scaled == 0.0 {
                return Err("partition scaling cannot discard a nonzero entry".into());
            }
            column.push(scaled);
            checkpoint_chunk(SolverStage::ModalVectorScan, i + 1, size)?;
        }
        columns.push(column);
        checkpoint_chunk(SolverStage::ModalVectorScan, j + 1, size)?;
    }
    let initial = columns
        .iter()
        .map(|c| vector_norm(c.iter().copied()))
        .collect::<Result<Vec<_>, _>>()?;
    let grids: Vec<_> = vector.iter().map(|v| grid_log(v.abs())).collect();
    let mut selected = vec![false; size];
    let mut fine = Vec::with_capacity(size / 2);
    for step in 0..size / 2 {
        checkpoint(SolverStage::ModalValidation, step)?;
        let mut best = None;
        for (j, column) in columns.iter().enumerate().filter(|(j, _)| !selected[*j]) {
            let norm = vector_norm(column.iter().copied())?;
            if norm <= 64.0 * f64::EPSILON * initial[j] || norm == 0.0 {
                continue;
            }
            let score = norm.log2() - 0.5 * grids[j];
            if best.is_none_or(|(_, current, _)| score > current) {
                best = Some((j, score, norm));
            }
        }
        let (pivot, _, norm) = best.ok_or("automatic partition lost independent columns")?;
        let basis: Vec<_> = columns[pivot].iter().map(|v| v / norm).collect();
        selected[pivot] = true;
        fine.push(pivot);
        for (j, column) in columns
            .iter_mut()
            .enumerate()
            .filter(|(j, _)| !selected[*j])
        {
            for _ in 0..2 {
                let overlap = dot(&basis, column)?;
                for (i, (v, &q)) in column.iter_mut().zip(&basis).enumerate() {
                    *v = (-overlap).mul_add(q, *v);
                    checkpoint_chunk(SolverStage::ModalVectorUpdate, i + 1, size)?;
                }
            }
            checkpoint_chunk(SolverStage::ModalVectorUpdate, j + 1, size)?;
        }
    }
    fine.sort_unstable();
    checkpoint(SolverStage::ModalValidation, size / 2)?;
    Ok(fine)
}

pub(crate) fn grid_log(value: f64) -> f64 {
    let upper = value.next_up() - value;
    let lower = value - value.next_down();
    (if upper.is_finite() {
        upper.max(lower)
    } else {
        lower
    })
    .log2()
}
