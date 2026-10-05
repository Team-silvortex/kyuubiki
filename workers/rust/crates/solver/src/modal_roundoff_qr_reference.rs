use super::{budget::Plan, dot, vector_norm};
use crate::solver_control::{SolverStage, checkpoint, checkpoint_chunk};

pub(super) const MAX_GRID_RADIUS: usize = 1 << 22;
pub(super) const MAX_BEAM_WIDTH: usize = 64;

// Test-only candidate factor: acceptance still uses the original sparse operator.
pub(super) struct QrFit {
    columns: Vec<Vec<f64>>,
    reflectors: Vec<(Vec<f64>, f64)>,
    scales: Vec<(f64, f64)>,
}

impl QrFit {
    pub(super) fn factor(columns: &[Vec<f64>]) -> Result<Self, String> {
        let rows = columns.first().map_or(0, Vec::len);
        Plan::new(rows)?;
        let count = columns.len();
        if count == 0
            || count >= rows
            || columns
                .iter()
                .any(|c| c.len() != rows || c.iter().any(|v| !v.is_finite()))
        {
            return Err("QR proposal requires bounded finite tall columns".into());
        }
        checkpoint(SolverStage::DenseFactor, 0)?;
        let mut work = Vec::with_capacity(count);
        let mut scales = Vec::with_capacity(count);
        let mut initial = Vec::with_capacity(count);
        for column in columns {
            let norm = vector_norm(column.iter().copied())?;
            if !norm.is_finite() || norm == 0.0 {
                return Err("QR proposal has a zero or invalid column".into());
            }
            let bits = norm.to_bits();
            let exponent = bits & (0x7ff_u64 << 52);
            let scale = f64::from_bits(if exponent == 0 {
                1_u64 << (63 - bits.leading_zeros())
            } else {
                exponent
            });
            let adjustment = if norm / scale > 1.0 { 0.5 } else { 1.0 };
            let mut scaled = Vec::with_capacity(rows);
            for (i, &v) in column.iter().enumerate() {
                let a = (v / scale) * adjustment;
                if !a.is_finite() || (v != 0.0 && a == 0.0) {
                    return Err("QR column scaling cannot discard a nonzero entry".into());
                }
                scaled.push(a);
                checkpoint_chunk(SolverStage::ModalVectorUpdate, i + 1, rows)?;
            }
            initial.push(vector_norm(scaled.iter().copied())?);
            scales.push((scale, adjustment));
            work.push(scaled);
        }
        let mut reflectors = Vec::with_capacity(count);
        for pivot in 0..count {
            let norm = vector_norm(work[pivot][pivot..].iter().copied())?;
            if !norm.is_finite() || norm <= 64.0 * f64::EPSILON * initial[pivot] {
                return Err("QR proposal lost independent columns".into());
            }
            let original = work[pivot][pivot];
            let diagonal = -norm.copysign(original);
            let head = original - diagonal;
            let mut direction = vec![1.0];
            for &v in &work[pivot][pivot + 1..] {
                direction.push(v / head);
            }
            let tau = 1.0 - original / diagonal;
            for column in &mut work[pivot..] {
                apply(&direction, tau, &mut column[pivot..])?;
            }
            work[pivot][pivot] = diagonal;
            work[pivot][pivot + 1..].fill(0.0);
            reflectors.push((direction, tau));
            checkpoint(SolverStage::DenseFactor, pivot + 1)?;
        }
        Ok(Self {
            columns: work,
            reflectors,
            scales,
        })
    }

    pub(super) fn solve(&self, residual: &[f64]) -> Result<Vec<f64>, String> {
        let rows = self.columns[0].len();
        if residual.len() != rows || residual.iter().any(|v| !v.is_finite()) {
            return Err("QR proposal requires a matching finite right-hand side".into());
        }
        checkpoint(SolverStage::DenseSubstitution, 0)?;
        let mut transformed = residual.to_vec();
        for (index, (direction, tau)) in self.reflectors.iter().enumerate() {
            apply(direction, *tau, &mut transformed[index..])?;
            checkpoint(SolverStage::DenseSubstitution, index + 1)?;
        }
        let count = self.columns.len();
        let mut solution = vec![0.0; count];
        for row in (0..count).rev() {
            let coefficients: Vec<_> = self.columns[row + 1..].iter().map(|c| c[row]).collect();
            let accumulated = dot(&coefficients, &solution[row + 1..])?;
            solution[row] = (transformed[row] - accumulated) / self.columns[row][row];
            if !solution[row].is_finite() {
                return Err("QR triangular proposal is not representable".into());
            }
            checkpoint(SolverStage::DenseSubstitution, count + (count - row))?;
        }
        for (i, (v, &(scale, adjustment))) in solution.iter_mut().zip(&self.scales).enumerate() {
            *v = (*v * adjustment) / scale;
            if !v.is_finite() {
                return Err("QR unscaled proposal is not representable".into());
            }
            checkpoint_chunk(SolverStage::ModalVectorUpdate, i + 1, count)?;
        }
        Ok(solution)
    }
}

fn apply(direction: &[f64], tau: f64, vector: &mut [f64]) -> Result<(), String> {
    let amount = tau * dot(direction, vector)?;
    if !amount.is_finite() {
        return Err("QR reflector proposal is not representable".into());
    }
    let size = vector.len();
    for (i, (v, &a)) in vector.iter_mut().zip(direction).enumerate() {
        *v = (-amount).mul_add(a, *v);
        if !v.is_finite() {
            return Err("QR reflector proposal is not representable".into());
        }
        checkpoint_chunk(SolverStage::ModalVectorUpdate, i + 1, size)?;
    }
    Ok(())
}

#[path = "modal_roundoff_qr_tests.rs"]
mod tests;

#[path = "modal_roundoff_qr_quantized.rs"]
mod quantized;

#[path = "modal_roundoff_qr_beam.rs"]
mod beam;

#[path = "modal_roundoff_qr_lattice.rs"]
pub(super) mod lattice;
