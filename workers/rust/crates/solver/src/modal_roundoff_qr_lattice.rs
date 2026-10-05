use super::*;
use crate::modal_test_wide::Wide;

pub(crate) const MAX_STEPS: usize = 4096;
pub(crate) const MAX_COEFFICIENT: i64 = 1 << 20;

#[path = "modal_roundoff_qr_lattice_control_tests.rs"]
mod controls;

fn bounded(value: Wide) -> bool {
    value.high.is_finite()
        && value.low.is_finite()
        && value.low.abs() <= value.high.abs()
        && value.high.abs() <= 1e100
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Stop {
    Complete,
    StepBudget,
    IntegerBudget,
}

#[derive(Debug)]
pub(crate) struct Reduced {
    pub(crate) transform: Vec<Vec<i64>>,
    pub(crate) steps: usize,
    pub(crate) shears: usize,
    pub(crate) swaps: usize,
    pub(crate) stop: Stop,
}

impl QrFit {
    // Bounded triangular lattice reduction. QR scores propose; never certify.
    pub(crate) fn reduced_lattice(&self) -> Result<Reduced, String> {
        self.reduce_with_budget(MAX_STEPS, false)
    }

    pub(crate) fn bounded_lattice(&self) -> Result<Reduced, String> {
        self.reduce_with_budget(MAX_STEPS, true)
    }

    fn reduce_with_budget(&self, limit: usize, partial: bool) -> Result<Reduced, String> {
        if !(1..=MAX_STEPS).contains(&limit) {
            return Err("triangular lattice requires a bounded positive step budget".into());
        }
        let count = self.columns.len();
        let mut triangular = vec![vec![Wide::default(); count]; count];
        let mut result = Reduced {
            transform: (0..count)
                .map(|j| (0..count).map(|i| i64::from(i == j)).collect())
                .collect(),
            steps: 0,
            shears: 0,
            swaps: 0,
            stop: Stop::Complete,
        };
        for (j, column) in triangular.iter_mut().enumerate() {
            let (scale, adjustment) = self.scales[j];
            let restored_scale = scale / adjustment;
            if !restored_scale.is_finite() {
                return Err("triangular lattice column scaling exceeds its range".into());
            }
            for (i, entry) in column.iter_mut().enumerate().take(j + 1) {
                *entry = Wide::from(self.columns[j][i]).mul(Wide::from(restored_scale));
                if !bounded(*entry) {
                    return Err("triangular lattice restored entries exceed their range".into());
                }
            }
            if column[j].high.abs() < 1e-140 {
                return Err("triangular lattice diagonal is below its retained-entry range".into());
            }
        }
        let mut k = 1;
        while k < count {
            if result.steps >= limit {
                if partial {
                    result.stop = Stop::StepBudget;
                    return Ok(result);
                }
                return Err("triangular lattice exhausted its fixed reduction step budget".into());
            }
            checkpoint(SolverStage::ModalRoundoffPrepare, result.steps)?;
            result.steps += 1;
            for j in (0..k).rev() {
                if triangular[j][j].high.abs() < 1e-140 {
                    return Err(
                        "triangular lattice diagonal is below its retained-entry range".into(),
                    );
                }
                let q = triangular[k][j].div(triangular[j][j]).rounded().round();
                if !q.is_finite() {
                    return Err("triangular lattice shear is not finite".into());
                }
                if q.abs() > MAX_COEFFICIENT as f64 {
                    if partial {
                        result.stop = Stop::IntegerBudget;
                        return Ok(result);
                    }
                    return Err("triangular lattice shear exceeds its integer bound".into());
                }
                if q == 0.0 {
                    continue;
                }
                let next = result.transform[k]
                    .iter()
                    .zip(&result.transform[j])
                    .map(|(&a, &b)| {
                        a.checked_sub((q as i64).checked_mul(b)?)
                            .filter(|v| v.unsigned_abs() <= MAX_COEFFICIENT as u64)
                    })
                    .collect::<Option<Vec<_>>>();
                let Some(next) = next else {
                    if partial {
                        result.stop = Stop::IntegerBudget;
                        return Ok(result);
                    }
                    return Err("triangular lattice transform exceeds its integer bound".into());
                };
                for i in 0..=j {
                    triangular[k][i] = triangular[k][i].sub(triangular[j][i].mul(Wide::from(q)));
                    if !bounded(triangular[k][i]) {
                        return Err("triangular lattice shear exceeds its arithmetic range".into());
                    }
                }
                result.transform[k] = next;
                result.shears += 1;
            }
            let a = triangular[k - 1][k - 1];
            let b = triangular[k][k - 1];
            let c = triangular[k][k];
            let left = a.mul(a).mul(Wide::from(0.75));
            let right = b.mul(b).add(c.mul(c));
            if left.sub(right).high <= 0.0 {
                k += 1;
                continue;
            }
            triangular.swap(k - 1, k);
            result.transform.swap(k - 1, k);
            let upper = triangular[k - 1][k - 1].rounded();
            let lower = triangular[k - 1][k].rounded();
            let norm = upper.hypot(lower);
            if !norm.is_finite() || norm == 0.0 {
                return Err("triangular lattice rotation exceeds its range".into());
            }
            let cosine = Wide::from(upper / norm);
            let sine = Wide::from(lower / norm);
            for column in triangular.iter_mut().skip(k - 1) {
                let upper = column[k - 1];
                let lower = column[k];
                column[k - 1] = cosine.mul(upper).add(sine.mul(lower));
                column[k] = cosine.mul(lower).sub(sine.mul(upper));
                if !bounded(column[k - 1]) || !bounded(column[k]) {
                    return Err(
                        "triangular lattice rotation exceeds its retained-entry range".into(),
                    );
                }
            }
            triangular[k - 1][k] = Wide::default();
            result.swaps += 1;
            k = k.saturating_sub(1).max(1);
        }
        Ok(result)
    }
}
