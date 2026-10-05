use super::{QrFit, Transform, Wide, bounded, norm, scaled_columns};
use crate::solver_control::{SolverStage, checkpoint, checkpoint_chunk};

pub(super) struct Rotation {
    pub(super) pivot: usize,
    pub(super) row: usize,
    cosine: Wide,
    sine: Wide,
}

impl Rotation {
    fn new(pivot: usize, row: usize, a: Wide, b: Wide) -> Result<(Self, Wide), String> {
        if !bounded(a) || !bounded(b) || b.high == 0.0 {
            return Err("wide Givens requires bounded entries and a nonzero target".into());
        }
        let maximum = a.rounded().abs().max(b.rounded().abs());
        if maximum == 0.0 || !maximum.is_finite() {
            return Err("wide Givens requires a resolved finite rotation scale".into());
        }
        let scale = Wide::from(f64::from_bits(maximum.to_bits() & (0x7ff_u64 << 52)));
        let left = a.div(scale);
        let right = b.div(scale);
        if !bounded(left)
            || !bounded(right)
            || (a.high != 0.0 && left.high == 0.0)
            || (b.high != 0.0 && right.high == 0.0)
        {
            return Err("wide Givens scaling exceeds its range".into());
        }
        let hypotenuse = norm(&[left, right])?;
        let cosine = left.div(hypotenuse);
        let sine = right.div(hypotenuse);
        let diagonal = hypotenuse.mul(scale);
        if !bounded(cosine)
            || !bounded(sine)
            || !bounded(diagonal)
            || cosine.rounded().abs() > 1.0
            || sine.rounded().abs() > 1.0
        {
            return Err("wide Givens rotation exceeds its unchanged range".into());
        }
        Ok((
            Self {
                pivot,
                row,
                cosine,
                sine,
            },
            diagonal,
        ))
    }

    pub(super) fn apply(&self, a: Wide, b: Wide) -> Result<(Wide, Wide), String> {
        if !bounded(a) || !bounded(b) {
            return Err("wide Givens requires bounded rotation inputs".into());
        }
        let left = self.cosine.mul(a).add(self.sine.mul(b));
        let right = self.cosine.mul(b).sub(self.sine.mul(a));
        if !bounded(left) || !bounded(right) {
            return Err(format!(
                "wide Givens result exceeds its unchanged range; pivot={} row={} left={:e} right={:e}",
                self.pivot,
                self.row,
                left.rounded(),
                right.rounded()
            ));
        }
        Ok((left, right))
    }
}

impl QrFit {
    // Separate cold factorization, never a retry after a Householder fault.
    pub(in super::super) fn factor_givens(columns: &[Vec<Wide>]) -> Result<Self, String> {
        let rows = columns.first().map_or(0, Vec::len);
        super::super::Plan::givens(rows)?;
        let (mut work, scales, initial) = scaled_columns(columns)?;
        let count = work.len();
        let mut rotations = Vec::with_capacity(rows * (rows - 1) / 2);
        for pivot in 0..count {
            for row in (pivot + 1..rows).rev() {
                if work[pivot][row].high != 0.0 {
                    let (rotation, diagonal) =
                        Rotation::new(pivot, row, work[pivot][pivot], work[pivot][row])?;
                    // Only the target column has an analytically eliminated entry.
                    // Other columns and right-hand sides never drop tiny fill-in.
                    for (index, column) in work[pivot + 1..].iter_mut().enumerate() {
                        let (a, b) = rotation.apply(column[pivot], column[row])?;
                        column[pivot] = a;
                        column[row] = b;
                        checkpoint_chunk(
                            SolverStage::ModalVectorUpdate,
                            index + 1,
                            count - pivot - 1,
                        )?;
                    }
                    work[pivot][pivot] = diagonal;
                    work[pivot][row] = Wide::default();
                    rotations.push(rotation);
                }
                checkpoint(SolverStage::DenseFactor, 1 + pivot * rows + rows - row)?;
            }
            if work[pivot][pivot].rounded().abs() <= 64.0 * f64::EPSILON * initial[pivot] {
                return Err("wide Givens lost independent columns".into());
            }
        }
        Ok(Self {
            columns: work,
            transform: Transform::Givens(rotations),
            scales,
        })
    }
}

#[path = "modal_roundoff_grid_wide_givens_tests.rs"]
mod tests;
