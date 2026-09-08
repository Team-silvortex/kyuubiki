use std::ops::Range;

use crate::solver_control::{SolverStage, checkpoint};

const VECTOR_CHUNK: usize = 1024;

// Keep one accumulator across blocks: cancellation must not regroup reductions.
#[inline]
fn visit_blocks(
    size: usize,
    stage: SolverStage,
    mut visit: impl FnMut(Range<usize>),
) -> Result<(), String> {
    checkpoint(stage, 0)?;
    for start in (0..size).step_by(VECTOR_CHUNK) {
        let end = start.saturating_add(VECTOR_CHUNK).min(size);
        visit(start..end);
        checkpoint(stage, end)?;
    }
    Ok(())
}

pub(super) fn rhs_scale(rhs: &[f64]) -> Result<f64, String> {
    let mut scale: f64 = 0.0;
    visit_blocks(rhs.len(), SolverStage::PcgRhsScale, |block| {
        scale = rhs[block]
            .iter()
            .map(|value| value.abs())
            .fold(scale, f64::max);
    })?;
    Ok(scale)
}

pub(super) fn normalize_rhs(rhs: &[f64], scale: f64) -> Result<Vec<f64>, String> {
    let mut normalized = Vec::with_capacity(rhs.len());
    visit_blocks(rhs.len(), SolverStage::PcgRhsNormalize, |block| {
        normalized.extend(rhs[block].iter().map(|value| value / scale));
    })?;
    Ok(normalized)
}

pub(super) fn copy_direction(source: &[f64], direction: &mut [f64]) -> Result<(), String> {
    debug_assert_eq!(source.len(), direction.len());
    visit_blocks(source.len(), SolverStage::PcgDirectionCopy, |block| {
        direction[block.clone()].copy_from_slice(&source[block]);
    })
}

pub(super) fn dot(lhs: &[f64], rhs: &[f64]) -> Result<f64, String> {
    debug_assert_eq!(lhs.len(), rhs.len());
    let mut sum = 0.0;
    visit_blocks(lhs.len(), SolverStage::PcgDot, |block| {
        for index in block {
            sum += lhs[index] * rhs[index];
        }
    })?;
    Ok(sum)
}

pub(super) fn l2_norm(values: &[f64]) -> Result<f64, String> {
    let mut scale = 0.0;
    let mut sum_squares = 1.0;
    visit_blocks(values.len(), SolverStage::PcgNorm, |block| {
        for value in values[block]
            .iter()
            .map(|value| value.abs())
            .filter(|value| *value > 0.0)
        {
            if scale < value {
                sum_squares = 1.0 + sum_squares * (scale / value).powi(2);
                scale = value;
            } else {
                sum_squares += (value / scale).powi(2);
            }
        }
    })?;
    Ok(if scale == 0.0 {
        0.0
    } else {
        scale * sum_squares.sqrt()
    })
}

pub(super) fn update_solution_residual(
    x: &mut [f64],
    residual: &mut [f64],
    direction: &[f64],
    product: &[f64],
    alpha: f64,
) -> Result<f64, String> {
    debug_assert_eq!(x.len(), residual.len());
    debug_assert_eq!(x.len(), direction.len());
    debug_assert_eq!(x.len(), product.len());
    let mut residual_squared = 0.0;
    visit_blocks(x.len(), SolverStage::PcgVectorUpdate, |block| {
        for index in block {
            x[index] += alpha * direction[index];
            residual[index] -= alpha * product[index];
            residual_squared += residual[index] * residual[index];
        }
    })?;
    Ok(residual_squared)
}

pub(super) fn update_residual(
    rhs: &[f64],
    scale: f64,
    product: &[f64],
    residual: &mut [f64],
) -> Result<f64, String> {
    debug_assert_eq!(rhs.len(), product.len());
    debug_assert_eq!(rhs.len(), residual.len());
    let mut residual_squared = 0.0;
    visit_blocks(rhs.len(), SolverStage::PcgResidualUpdate, |block| {
        for index in block {
            residual[index] = rhs[index] / scale - product[index];
            residual_squared += residual[index] * residual[index];
        }
    })?;
    Ok(residual_squared)
}

pub(super) fn update_direction(direction: &mut [f64], z: &[f64], beta: f64) -> Result<(), String> {
    debug_assert_eq!(direction.len(), z.len());
    visit_blocks(direction.len(), SolverStage::PcgDirectionUpdate, |block| {
        for index in block {
            direction[index] = z[index] + beta * direction[index];
        }
    })
}

pub(super) fn rescale_solution(mut solution: Vec<f64>, scale: f64) -> Result<Vec<f64>, String> {
    visit_blocks(solution.len(), SolverStage::PcgSolutionScale, |block| {
        for value in &mut solution[block] {
            *value *= scale;
        }
    })?;
    Ok(solution)
}

#[cfg(test)]
#[path = "linear_spd_vector_tests.rs"]
mod tests;
