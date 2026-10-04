use super::{
    budget::{Checks, FINE_STEPS, OUTER_STEPS},
    wide_factor::{Factor, bounded, dot, less},
};
use crate::modal_test_wide::Wide;
use crate::solver_control::{SolverStage, checkpoint, checkpoint_chunk};

pub(super) struct Plan {
    pub(super) payload_bytes: usize,
    pub(super) component_visits: usize,
}

impl Plan {
    pub(super) fn new(size: usize) -> Result<Self, String> {
        let base = super::budget::Plan::new(size)?;
        let payload_bytes = 6 * size * size * std::mem::size_of::<Wide>() + size * 1024;
        let component_visits = 4 * base.component_visits + 2048 * size * size;
        if payload_bytes > 8 * 1024 * 1024 || component_visits > 1_500_000_000 {
            return Err("wide projection exceeds its separate proposal budget".into());
        }
        Ok(Self {
            payload_bytes,
            component_visits,
        })
    }
}

// Test-only wide candidate arithmetic. Certificates always come from the caller.
pub(super) struct Projection {
    fine: Vec<usize>,
    coarse: Vec<usize>,
    factor: Factor,
    projected: Vec<Vec<Wide>>,
    pairs: Vec<(usize, usize)>,
}

impl Projection {
    pub(super) fn physical(
        matrix: &[Vec<f64>],
        mass: &[f64],
        value: f64,
        vector: &[f64],
    ) -> Result<Self, String> {
        Plan::new(matrix.len())?;
        let rounded = super::joint::physical_columns(matrix, mass, value)?;
        let fine = super::partition::select_fine(&rounded, vector)?;
        Self::physical_partition(matrix, mass, value, fine)
    }

    pub(super) fn physical_partition(
        matrix: &[Vec<f64>],
        mass: &[f64],
        value: f64,
        fine: Vec<usize>,
    ) -> Result<Self, String> {
        Self::prepare(physical_directions(matrix, mass, value)?, fine)
    }

    pub(super) fn prepare(matrix: Vec<Vec<Wide>>, fine: Vec<usize>) -> Result<Self, String> {
        let size = matrix.len();
        Plan::new(size)?;
        if matrix
            .iter()
            .any(|r| r.len() != size || r.iter().any(|&v| !bounded(v)))
            || fine.is_empty()
            || fine.len() >= size
            || fine.iter().any(|&i| i >= size)
            || fine.windows(2).any(|w| w[0] >= w[1])
        {
            return Err(
                "wide projection requires bounded square data and a sorted proper subset".into(),
            );
        }
        let coarse: Vec<_> = (0..size)
            .filter(|i| fine.binary_search(i).is_err())
            .collect();
        let columns: Vec<Vec<_>> = fine
            .iter()
            .map(|&j| matrix.iter().map(|r| r[j]).collect())
            .collect();
        let factor = Factor::prepare(&columns)?;
        let mut projected = Vec::with_capacity(coarse.len());
        for &j in &coarse {
            let column: Vec<_> = matrix.iter().map(|r| r[j]).collect();
            projected.push(factor.project(&column)?);
        }
        let rounded: Vec<Vec<_>> = projected
            .iter()
            .map(|c| c.iter().map(|v| v.rounded()).collect())
            .collect();
        let pairs = super::coupled_pairs(&rounded)?;
        Ok(Self {
            fine,
            coarse,
            factor,
            projected,
            pairs,
        })
    }

    pub(super) fn correct(
        &self,
        vector: &[f64],
        tolerance: f64,
        checked: impl FnMut(&[f64]) -> Result<(f64, Vec<f64>), String>,
    ) -> Result<Vec<f64>, String> {
        self.correct_internal(vector, tolerance, checked, false)
    }

    pub(super) fn correct_rounding(
        &self,
        vector: &[f64],
        tolerance: f64,
        checked: impl FnMut(&[f64]) -> Result<(f64, Vec<f64>), String>,
    ) -> Result<Vec<f64>, String> {
        self.correct_internal(vector, tolerance, checked, true)
    }

    fn correct_internal(
        &self,
        vector: &[f64],
        tolerance: f64,
        mut checked: impl FnMut(&[f64]) -> Result<(f64, Vec<f64>), String>,
        round_neighbors: bool,
    ) -> Result<Vec<f64>, String> {
        if vector.len() != self.fine.len() + self.coarse.len()
            || vector.iter().any(|&v| !bounded(Wide { high: v, low: 0.0 }))
            || !tolerance.is_finite()
            || tolerance <= 0.0
        {
            return Err(
                "wide correction requires matching bounded data and a positive tolerance".into(),
            );
        }
        let mut checks = Checks::new();
        let mut retained = vector.to_vec();
        let (mut relative, _) = self.measure(&retained, &mut checked, &mut checks)?;
        for step in 0..OUTER_STEPS {
            if relative <= tolerance {
                break;
            }
            checkpoint(SolverStage::ModalIteration, step)?;
            self.fit_fine(&mut retained, &mut checked, &mut checks, round_neighbors)?;
            let (before, residual) = self.measure(&retained, &mut checked, &mut checks)?;
            if before <= tolerance {
                break;
            }
            let mut residual = self.factor.project(&residual)?;
            let mut candidate = retained.clone();
            self.sweep(&mut candidate, &mut residual)?;
            self.fit_fine(&mut candidate, &mut checked, &mut checks, round_neighbors)?;
            let (after, _) = self.measure(&candidate, &mut checked, &mut checks)?;
            if after >= before {
                break;
            }
            retained = candidate;
            relative = after;
        }
        let (relative, _) = self.measure(&retained, &mut checked, &mut checks)?;
        if relative > tolerance {
            return Err(format!(
                "wide projection did not reach its unchanged residual gate (relative={relative:e})"
            ));
        }
        checkpoint(SolverStage::ModalRoundoffValidate, 1)?;
        Ok(retained)
    }

    fn measure(
        &self,
        vector: &[f64],
        checked: &mut impl FnMut(&[f64]) -> Result<(f64, Vec<f64>), String>,
        checks: &mut Checks,
    ) -> Result<(f64, Vec<Wide>), String> {
        if vector.iter().any(|&v| !bounded(Wide { high: v, low: 0.0 })) {
            return Err("wide candidate exceeds its representation range".into());
        }
        checks.next()?;
        let (relative, residual) = checked(vector)?;
        if !relative.is_finite()
            || relative < 0.0
            || residual.len() != vector.len()
            || residual
                .iter()
                .any(|&v| !bounded(Wide { high: v, low: 0.0 }))
        {
            return Err("wide correction requires a finite matching residual certificate".into());
        }
        Ok((relative, residual.into_iter().map(Wide::from).collect()))
    }

    fn fit_fine(
        &self,
        vector: &mut Vec<f64>,
        checked: &mut impl FnMut(&[f64]) -> Result<(f64, Vec<f64>), String>,
        checks: &mut Checks,
        round_neighbors: bool,
    ) -> Result<(), String> {
        for _ in 0..FINE_STEPS {
            let (before, residual) = self.measure(vector, checked, checks)?;
            let correction = self.factor.coefficients(&residual)?;
            let mut candidate = vector.clone();
            for (&i, &c) in self.fine.iter().zip(&correction) {
                candidate[i] = Wide::from(vector[i]).sub(c).rounded();
            }
            if round_neighbors {
                self.factor
                    .choose_rounding(&self.fine, vector, &mut candidate, &residual)?;
            }
            let (after, _) = self.measure(&candidate, checked, checks)?;
            if after >= before {
                break;
            }
            *vector = candidate;
        }
        Ok(())
    }

    fn sweep(&self, vector: &mut [f64], residual: &mut [Wide]) -> Result<(), String> {
        let mut groups: Vec<_> = (0..self.coarse.len()).map(|i| vec![i]).collect();
        groups.extend(self.pairs.iter().map(|&(i, j)| vec![i, j]));
        // Preserve the existing single forward/reverse then pair forward/reverse policy.
        for family in [&groups[..self.coarse.len()], &groups[self.coarse.len()..]] {
            for group in family.iter().chain(family.iter().rev()) {
                let old: Vec<_> = group.iter().map(|&i| vector[self.coarse[i]]).collect();
                let mut options: Vec<Vec<_>> = old
                    .iter()
                    .map(|v| vec![*v, v.next_up(), v.next_down()])
                    .collect();
                if group.len() == 1 {
                    options[0].remove(0);
                    let direction = &self.projected[group[0]];
                    let square = dot(direction, direction)?;
                    if square.high > 0.0 {
                        let step = dot(direction, residual)?.div(square);
                        if !bounded(step) {
                            return Err("wide coarse correction exceeds its range".into());
                        }
                        options[0].insert(0, Wide::from(old[0]).sub(step).rounded());
                    }
                }
                let mut best = dot(residual, residual)?;
                let mut updated = old.clone();
                for &left in &options[0] {
                    let right = if group.len() == 2 {
                        options[1].clone()
                    } else {
                        vec![0.0]
                    };
                    for right in right {
                        let proposal = if group.len() == 2 {
                            vec![left, right]
                        } else {
                            vec![left]
                        };
                        if proposal
                            .iter()
                            .any(|&v| !bounded(Wide { high: v, low: 0.0 }))
                        {
                            continue;
                        }
                        let mut predicted = residual.to_vec();
                        self.update(group, &old, &proposal, &mut predicted)?;
                        let score = dot(&predicted, &predicted)?;
                        if less(score, best) {
                            best = score;
                            updated = proposal;
                        }
                    }
                }
                self.update(group, &old, &updated, residual)?;
                for (&i, &next) in group.iter().zip(&updated) {
                    vector[self.coarse[i]] = next;
                }
            }
        }
        Ok(())
    }

    fn update(
        &self,
        group: &[usize],
        old: &[f64],
        next: &[f64],
        residual: &mut [Wide],
    ) -> Result<(), String> {
        let size = residual.len();
        for ((&column, &old), &next) in group.iter().zip(old).zip(next) {
            let delta = Wide::from(next).sub(Wide::from(old));
            if !bounded(delta) {
                return Err("wide candidate delta exceeds its range".into());
            }
            for (i, (r, &direction)) in residual.iter_mut().zip(&self.projected[column]).enumerate()
            {
                *r = r.add(direction.mul(delta));
                if !bounded(*r) {
                    return Err("wide candidate prediction exceeds its range".into());
                }
                checkpoint_chunk(SolverStage::ModalVectorUpdate, i + 1, size)?;
            }
        }
        Ok(())
    }
}

pub(super) fn physical_directions(
    matrix: &[Vec<f64>],
    mass: &[f64],
    value: f64,
) -> Result<Vec<Vec<Wide>>, String> {
    Plan::new(matrix.len())?;
    super::joint::physical_columns(matrix, mass, value)?;
    let mut shifted = Vec::with_capacity(matrix.len());
    for (i, row) in matrix.iter().enumerate() {
        let inverse = Wide::from(mass[i].sqrt().recip());
        let mut wide = Vec::with_capacity(row.len());
        for (j, &v) in row.iter().enumerate() {
            let mut next = Wide::from(v);
            if i == j {
                next = next.sub(Wide::from(value).mul(Wide::from(mass[i])));
            }
            next = next.mul(inverse);
            if !bounded(next) {
                return Err("wide physical direction exceeds its representation range".into());
            }
            wide.push(next);
            checkpoint_chunk(SolverStage::ModalVectorUpdate, j + 1, row.len())?;
        }
        shifted.push(wide);
    }
    Ok(shifted)
}
