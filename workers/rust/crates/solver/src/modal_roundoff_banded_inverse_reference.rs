use crate::modal_test_wide::Wide;
use crate::solver_control::{SolverStage, checkpoint, checkpoint_chunk};

pub(super) const MAX_STEPS: usize = 4;

// Test-only SPD bandwidth-three factor: proposals, not spectral certificates.
pub(super) struct Factor {
    lower: Vec<[Wide; 3]>,
    diagonal: Vec<Wide>,
    scale: Wide,
}

impl Factor {
    pub(super) fn prepare(matrix: &[Vec<Wide>]) -> Result<Self, String> {
        Self::prepare_shifted(matrix, 0.0)
    }

    pub(super) fn prepare_shifted(matrix: &[Vec<Wide>], shift: f64) -> Result<Self, String> {
        let size = matrix.len();
        if !(2..=256).contains(&size)
            || matrix.iter().any(|row| row.len() != size)
            || matrix.iter().flatten().any(|&v| !bounded(v))
            || !shift.is_finite()
            || !(0.0..=1e50).contains(&shift)
        {
            return Err("banded inverse requires bounded square data of size 2..=256".into());
        }
        // Borrow the shifted matrix; reconstruct only accessed diagonal entries.
        let entry = |i: usize, j: usize| {
            if i == j && shift != 0.0 {
                matrix[i][j].add(Wide::from(shift))
            } else {
                matrix[i][j]
            }
        };
        if (0..size).any(|i| !bounded(entry(i, i))) {
            return Err("banded inverse shifted diagonal exceeds its range".into());
        }
        checkpoint(SolverStage::BandedFactor, 0)?;
        let mut maximum = 0.0_f64;
        for (i, row) in matrix.iter().enumerate() {
            for j in 0..row.len() {
                let value = entry(i, j);
                let other = entry(j, i);
                let magnitude = value.rounded().abs().max(other.rounded().abs());
                if (i.abs_diff(j) > 3 && (value.high != 0.0 || value.low != 0.0))
                    || value.sub(other).rounded().abs() > 1e-27 * magnitude
                {
                    return Err("banded inverse requires symmetric bandwidth-three data".into());
                }
                maximum = maximum.max(value.rounded().abs());
            }
            checkpoint_chunk(SolverStage::BandedFactor, i + 1, size)?;
        }
        let scale = f64::from_bits(maximum.to_bits() & (0x7ff_u64 << 52));
        if scale == 0.0 || !scale.is_finite() {
            return Err("banded inverse cannot scale zero or subnormal data".into());
        }
        let scale = Wide::from(scale);
        let mut lower = vec![[Wide::default(); 3]; size];
        let mut diagonal = vec![Wide::default(); size];
        for i in 0..size {
            for j in i.saturating_sub(3)..i {
                let mut value = scale_entry(entry(i, j), scale)?;
                for (k, &previous) in diagonal
                    .iter()
                    .enumerate()
                    .take(j)
                    .skip(i.saturating_sub(3))
                {
                    value = value.sub(lower[i][i - k - 1].mul(lower[j][j - k - 1]).mul(previous));
                }
                let value = value.div(diagonal[j]);
                if !bounded(value) || value.rounded().abs() > 1e14 {
                    return Err("banded inverse exceeds its bounded factor range".into());
                }
                lower[i][i - j - 1] = value;
            }
            let mut pivot = scale_entry(entry(i, i), scale)?;
            for (j, &previous) in diagonal
                .iter()
                .enumerate()
                .take(i)
                .skip(i.saturating_sub(3))
            {
                let value = lower[i][i - j - 1];
                pivot = pivot.sub(value.mul(value).mul(previous));
            }
            if !bounded(pivot) || !(1e-100..=1e50).contains(&pivot.rounded()) {
                return Err("banded inverse requires bounded positive definite pivots".into());
            }
            diagonal[i] = pivot;
            checkpoint(SolverStage::BandedFactor, size + i + 1)?;
        }
        Ok(Self {
            lower,
            diagonal,
            scale,
        })
    }

    pub(super) fn solve(&self, rhs: &[Wide]) -> Result<Vec<Wide>, String> {
        let size = self.diagonal.len();
        if rhs.len() != size || rhs.iter().any(|&v| !bounded(v)) {
            return Err("banded inverse requires matching bounded right-hand side".into());
        }
        checkpoint(SolverStage::BandedSubstitution, 0)?;
        let mut solution = Vec::with_capacity(size);
        for (i, &value) in rhs.iter().enumerate() {
            let scaled = value.high / self.scale.high;
            if !scaled.is_finite() || scaled.abs() > 1e50 || (value.high != 0.0 && scaled == 0.0) {
                return Err("banded inverse right-hand side scaling exceeds its range".into());
            }
            let mut value = value.div(self.scale);
            if !bounded(value) {
                return Err("banded inverse right-hand side scaling exceeds its range".into());
            }
            for (j, &previous) in solution
                .iter()
                .enumerate()
                .take(i)
                .skip(i.saturating_sub(3))
            {
                value = value.sub(self.lower[i][i - j - 1].mul(previous));
            }
            if !bounded(value) {
                return Err("banded inverse forward substitution exceeds its range".into());
            }
            solution.push(value);
            checkpoint_chunk(SolverStage::BandedSubstitution, i + 1, size)?;
        }
        for (i, (value, &pivot)) in solution.iter_mut().zip(&self.diagonal).enumerate() {
            *value = value.div(pivot);
            if !bounded(*value) {
                return Err("banded inverse diagonal substitution exceeds its range".into());
            }
            checkpoint_chunk(SolverStage::BandedSubstitution, size + i + 1, 2 * size)?;
        }
        for i in (0..size).rev() {
            for j in i + 1..(i + 4).min(size) {
                solution[i] = solution[i].sub(self.lower[j][j - i - 1].mul(solution[j]));
            }
            if !bounded(solution[i]) {
                return Err("banded inverse back substitution exceeds its range".into());
            }
            checkpoint_chunk(SolverStage::BandedSubstitution, 3 * size - i, 3 * size)?;
        }
        Ok(solution)
    }

    pub(super) fn owned_numeric_bytes(&self) -> usize {
        self.lower.capacity() * std::mem::size_of::<[Wide; 3]>()
            + self.diagonal.capacity() * std::mem::size_of::<Wide>()
            + std::mem::size_of::<Wide>()
    }

    pub(super) fn directions(&self, seed: &[f64], steps: usize) -> Result<Vec<Vec<Wide>>, String> {
        let mut candidates = Vec::with_capacity(steps.min(MAX_STEPS));
        self.iterate(seed, steps, |direction| candidates.push(direction.to_vec()))?;
        Ok(candidates)
    }

    pub(super) fn final_direction(&self, seed: &[f64], steps: usize) -> Result<Vec<Wide>, String> {
        self.iterate(seed, steps, |_| {})
    }

    fn iterate(
        &self,
        seed: &[f64],
        steps: usize,
        mut observe: impl FnMut(&[Wide]),
    ) -> Result<Vec<Wide>, String> {
        if seed.len() != self.diagonal.len()
            || seed.iter().any(|v| !v.is_finite() || v.abs() > 1e50)
            || !(1..=MAX_STEPS).contains(&steps)
        {
            return Err("banded inverse directions require matching data and 1..=4 steps".into());
        }
        let anchor = seed
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.abs().total_cmp(&b.abs()))
            .unwrap()
            .0;
        if seed[anchor] == 0.0 {
            return Err("banded inverse requires a nonzero retained direction".into());
        }
        let mut iterate: Vec<_> = seed.iter().copied().map(Wide::from).collect();
        for step in 0..steps {
            checkpoint(SolverStage::ModalIteration, step)?;
            iterate = self.solve(&iterate)?;
            if iterate[anchor].rounded() == 0.0 {
                return Err("banded inverse lost its retained anchor".into());
            }
            let quotient = seed[anchor] / iterate[anchor].high;
            if !quotient.is_finite() || quotient.abs() > 1e50 || quotient == 0.0 {
                return Err("banded inverse anchor scaling exceeds its range".into());
            }
            let scale = Wide::from(seed[anchor]).div(iterate[anchor]);
            for (i, value) in iterate.iter_mut().enumerate() {
                *value = value.mul(scale);
                if !bounded(*value) {
                    return Err("banded inverse direction normalization exceeds its range".into());
                }
                checkpoint_chunk(SolverStage::ModalVectorUpdate, i + 1, seed.len())?;
            }
            observe(&iterate);
        }
        checkpoint(SolverStage::ModalIteration, steps)?;
        Ok(iterate)
    }
}

fn bounded(value: Wide) -> bool {
    value.high.is_finite()
        && value.low.is_finite()
        && value.high.abs() <= 1e50
        && value.low.abs() <= value.high.abs()
}

fn scale_entry(value: Wide, scale: Wide) -> Result<Wide, String> {
    let quotient = value.high / scale.high;
    if !quotient.is_finite() || (value.high != 0.0 && quotient == 0.0) {
        return Err("banded inverse matrix scaling lost a nonzero entry".into());
    }
    Ok(value.div(scale))
}
