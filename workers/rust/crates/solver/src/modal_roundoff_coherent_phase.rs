use super::{
    budget::Checks,
    wide_factor::{bounded, dot, less},
};
use crate::modal_test_wide::Wide;
use crate::solver_control::{SolverStage, checkpoint, checkpoint_chunk};

pub(super) const SHORTLIST: usize = 64;
pub(super) const MAX_CERTIFICATES: usize = SHORTLIST + 2;

pub(super) struct Plan {
    pub(super) payload_bytes: usize,
    pub(super) component_visits: usize,
    pub(super) patterns: usize,
    pub(super) combinations: usize,
    pub(super) certificates: usize,
}

impl Plan {
    pub(super) fn new(size: usize) -> Result<Self, String> {
        super::budget::Plan::new(size)?;
        // Includes dense direction copies and retained patterns, not caller models/products.
        let payload_bytes = 64 * size * size + 2048 * size;
        let component_visits = 2 * size * size * size + 64 * size * size + 4096 * size;
        if payload_bytes > 8 * 1024 * 1024 || component_visits > 40_000_000 {
            return Err("coherent phase exceeds its separate proposal budget".into());
        }
        Ok(Self {
            payload_bytes,
            component_visits,
            patterns: size + 1,
            combinations: (size / 2 + 1) * (size - size / 2 + 1),
            certificates: ((size / 2 + 1) * (size - size / 2 + 1) + 2).min(MAX_CERTIFICATES),
        })
    }
}

fn input(value: Wide) -> bool {
    bounded(value) && (value.high == 0.0 || (1e-50..=1e50).contains(&value.high.abs()))
}

struct Brackets {
    nearest: Vec<f64>,
    lower: Vec<f64>,
    upper: Vec<f64>,
    events: Vec<(Wide, usize)>,
}

impl Brackets {
    fn new(center: &[Wide]) -> Result<Self, String> {
        Plan::new(center.len())?;
        checkpoint(SolverStage::ModalShapeScan, 0)?;
        if center.iter().any(|&v| !input(v)) {
            return Err("coherent phase requires bounded finite centers".into());
        }
        let nearest: Vec<_> = center.iter().map(|v| v.rounded()).collect();
        let mut lower = nearest.clone();
        let mut upper = nearest.clone();
        let mut events = Vec::new();
        for (i, (&value, &rounded)) in center.iter().zip(&nearest).enumerate() {
            let difference = value.sub(Wide::from(rounded));
            if less(Wide::default(), difference) {
                upper[i] = rounded.next_up();
            } else if less(difference, Wide::default()) {
                lower[i] = rounded.next_down();
            } else {
                continue;
            }
            if !input(Wide {
                high: lower[i],
                low: 0.0,
            }) || !input(Wide {
                high: upper[i],
                low: 0.0,
            }) {
                return Err("coherent phase adjacent coordinates exceed their range".into());
            }
            let width = Wide::from(upper[i]).sub(Wide::from(lower[i]));
            let fraction = value.sub(Wide::from(lower[i])).div(width);
            if !bounded(fraction)
                || !less(Wide::default(), fraction)
                || !less(fraction, Wide::from(1.0))
            {
                return Err("coherent phase fraction is not resolved inside its bracket".into());
            }
            events.push((fraction, i));
            checkpoint_chunk(SolverStage::ModalShapeScan, i + 1, center.len())?;
        }
        events.sort_by(|a, b| {
            b.0.high
                .total_cmp(&a.0.high)
                .then(b.0.low.total_cmp(&a.0.low))
                .then(a.1.cmp(&b.1))
        });
        checkpoint(SolverStage::ModalVectorScan, events.len())?;
        Ok(Self {
            nearest,
            lower,
            upper,
            events,
        })
    }
}

struct Ranked {
    score: Wide,
    vector: Vec<f64>,
    ordinal: usize,
}

struct AxisPhase {
    effect: Vec<Wide>,
    cutoff: usize,
}

// Test-only one/two-axis threshold rounding. Scores propose, callbacks certify.
pub(super) struct Search {
    columns: Vec<Vec<Wide>>,
}

impl Search {
    pub(super) fn prepare(matrix: Vec<Vec<Wide>>) -> Result<Self, String> {
        let size = matrix.len();
        Plan::new(size)?;
        if matrix
            .iter()
            .any(|r| r.len() != size || r.iter().any(|&v| !input(v)))
        {
            return Err("coherent phase requires bounded square directions".into());
        }
        let mut columns = Vec::with_capacity(size);
        for j in 0..size {
            columns.push(matrix.iter().map(|row| row[j]).collect());
            checkpoint_chunk(SolverStage::ModalVectorScan, j + 1, size)?;
        }
        Ok(Self { columns })
    }

    pub(super) fn correct(
        &self,
        center: &[Wide],
        tolerance: f64,
        checked: impl FnMut(&[f64]) -> Result<(f64, Vec<f64>), String>,
    ) -> Result<Vec<f64>, String> {
        self.correct_internal(center, None, tolerance, checked)
    }

    pub(super) fn correct_partitioned(
        &self,
        center: &[Wide],
        partition: &[usize],
        tolerance: f64,
        checked: impl FnMut(&[f64]) -> Result<(f64, Vec<f64>), String>,
    ) -> Result<Vec<f64>, String> {
        self.correct_internal(center, Some(partition), tolerance, checked)
    }

    fn correct_internal(
        &self,
        center: &[Wide],
        partition: Option<&[usize]>,
        tolerance: f64,
        mut checked: impl FnMut(&[f64]) -> Result<(f64, Vec<f64>), String>,
    ) -> Result<Vec<f64>, String> {
        if center.len() != self.columns.len() || !tolerance.is_finite() || tolerance <= 0.0 {
            return Err("coherent phase requires matching centers and positive tolerance".into());
        }
        if let Some(indices) = partition {
            if indices.is_empty()
                || indices.len() >= center.len()
                || indices.iter().any(|&i| i >= center.len())
                || indices.windows(2).any(|w| w[0] >= w[1])
            {
                return Err("coherent phase partition must be a sorted proper subset".into());
            }
        }
        let brackets = Brackets::new(center)?;
        let mut checks = Checks::new();
        let mut retained = brackets.nearest.clone();
        let (mut relative, residual) = self.measure(&retained, &mut checked, &mut checks)?;
        if relative > tolerance {
            let candidates = if let Some(indices) = partition {
                self.rank_partitioned(&brackets, &residual, indices)?
            } else {
                self.rank(&brackets, &residual)?
            };
            for (visit, candidate) in candidates.iter().take(SHORTLIST).enumerate() {
                checkpoint(SolverStage::ModalIteration, visit)?;
                let (next, _) = self.measure(&candidate.vector, &mut checked, &mut checks)?;
                if next < relative {
                    retained = candidate.vector.clone();
                    relative = next;
                }
                if relative <= tolerance {
                    break;
                }
            }
        }
        let (relative, _) = self.measure(&retained, &mut checked, &mut checks)?;
        if relative > tolerance {
            return Err(format!(
                "coherent phase did not reach its unchanged residual gate (relative={relative:e})"
            ));
        }
        checkpoint(SolverStage::ModalRoundoffValidate, 1)?;
        Ok(retained)
    }

    fn rank(&self, brackets: &Brackets, residual: &[Wide]) -> Result<Vec<Ranked>, String> {
        let mut vector = brackets.lower.clone();
        let mut predicted = residual.to_vec();
        for (i, &next) in vector.iter().enumerate() {
            self.update(i, brackets.nearest[i], next, &mut predicted)?;
        }
        let mut patterns = Vec::new();
        self.record(&mut patterns, &vector, &predicted, &brackets.nearest, 0)?;
        let mut start = 0;
        while start < brackets.events.len() {
            let mut end = start + 1;
            let fraction = brackets.events[start].0;
            while end < brackets.events.len()
                && brackets.events[end].0.high == fraction.high
                && brackets.events[end].0.low == fraction.low
            {
                end += 1;
            }
            // Equal fractions flip together, creating genuinely nonlocal proposals.
            for &(_, i) in &brackets.events[start..end] {
                self.update(i, vector[i], brackets.upper[i], &mut predicted)?;
                vector[i] = brackets.upper[i];
            }
            self.record(&mut patterns, &vector, &predicted, &brackets.nearest, end)?;
            checkpoint(SolverStage::ModalVectorScan, end)?;
            start = end;
        }
        if patterns.len() > Plan::new(vector.len())?.patterns {
            return Err("coherent phase exceeded its pattern cap".into());
        }
        patterns.sort_by(|a, b| {
            a.score
                .high
                .total_cmp(&b.score.high)
                .then(a.score.low.total_cmp(&b.score.low))
                .then(a.ordinal.cmp(&b.ordinal))
        });
        Ok(patterns)
    }

    fn record(
        &self,
        patterns: &mut Vec<Ranked>,
        vector: &[f64],
        residual: &[Wide],
        nearest: &[f64],
        ordinal: usize,
    ) -> Result<(), String> {
        if vector
            .iter()
            .zip(nearest)
            .all(|(a, b)| a.to_bits() == b.to_bits())
        {
            return Ok(());
        }
        patterns.push(Ranked {
            score: dot(residual, residual)?,
            vector: vector.to_vec(),
            ordinal,
        });
        Ok(())
    }

    fn rank_partitioned(
        &self,
        brackets: &Brackets,
        residual: &[Wide],
        partition: &[usize],
    ) -> Result<Vec<Ranked>, String> {
        let (left_events, right_events): (Vec<_>, Vec<_>) = brackets
            .events
            .iter()
            .copied()
            .partition(|(_, i)| partition.binary_search(i).is_ok());
        let left = self.axis_phases(brackets, &left_events)?;
        let right = self.axis_phases(brackets, &right_events)?;
        if left.len() * right.len() > Plan::new(residual.len())?.combinations {
            return Err("coherent phase exceeded its combination cap".into());
        }
        let mut base = residual.to_vec();
        for i in 0..base.len() {
            self.update(i, brackets.nearest[i], brackets.lower[i], &mut base)?;
        }
        let mut best: Vec<Ranked> = Vec::with_capacity(SHORTLIST + 1);
        for (i, a) in left.iter().enumerate() {
            for (j, b) in right.iter().enumerate() {
                let ordinal = i * right.len() + j;
                let mut predicted = Vec::with_capacity(base.len());
                for (row, ((&r, &a), &b)) in base.iter().zip(&a.effect).zip(&b.effect).enumerate() {
                    let next = r.add(a).add(b);
                    if !bounded(next) {
                        return Err("coherent phase combined prediction exceeds its range".into());
                    }
                    predicted.push(next);
                    checkpoint_chunk(SolverStage::ModalVectorUpdate, row + 1, base.len())?;
                }
                let score = dot(&predicted, &predicted)?;
                if best.len() < SHORTLIST || less(score, best.last().unwrap().score) {
                    let mut vector = brackets.lower.clone();
                    for &(_, index) in left_events[..a.cutoff]
                        .iter()
                        .chain(&right_events[..b.cutoff])
                    {
                        vector[index] = brackets.upper[index];
                    }
                    if vector
                        .iter()
                        .zip(&brackets.nearest)
                        .any(|(a, b)| a.to_bits() != b.to_bits())
                    {
                        let position = best.partition_point(|old| {
                            less(old.score, score)
                                || (old.score.high == score.high
                                    && old.score.low == score.low
                                    && old.ordinal < ordinal)
                        });
                        best.insert(
                            position,
                            Ranked {
                                score,
                                vector,
                                ordinal,
                            },
                        );
                        if best.len() > SHORTLIST {
                            best.pop();
                        }
                    }
                }
                checkpoint(SolverStage::ModalVectorScan, ordinal)?;
            }
        }
        Ok(best)
    }

    fn axis_phases(
        &self,
        brackets: &Brackets,
        events: &[(Wide, usize)],
    ) -> Result<Vec<AxisPhase>, String> {
        let mut effect = vec![Wide::default(); self.columns.len()];
        let mut phases = vec![AxisPhase {
            effect: effect.clone(),
            cutoff: 0,
        }];
        let mut start = 0;
        while start < events.len() {
            let fraction = events[start].0;
            let mut end = start + 1;
            while end < events.len()
                && events[end].0.high == fraction.high
                && events[end].0.low == fraction.low
            {
                end += 1;
            }
            for &(_, i) in &events[start..end] {
                self.update(i, brackets.lower[i], brackets.upper[i], &mut effect)?;
            }
            phases.push(AxisPhase {
                effect: effect.clone(),
                cutoff: end,
            });
            start = end;
        }
        Ok(phases)
    }

    fn update(
        &self,
        column: usize,
        old: f64,
        next: f64,
        residual: &mut [Wide],
    ) -> Result<(), String> {
        let delta = Wide::from(next).sub(Wide::from(old));
        for (i, (r, &a)) in residual.iter_mut().zip(&self.columns[column]).enumerate() {
            *r = r.add(a.mul(delta));
            if !bounded(*r) {
                return Err("coherent phase prediction exceeds its range".into());
            }
            checkpoint_chunk(SolverStage::ModalVectorUpdate, i + 1, self.columns.len())?;
        }
        Ok(())
    }

    fn measure(
        &self,
        vector: &[f64],
        checked: &mut impl FnMut(&[f64]) -> Result<(f64, Vec<f64>), String>,
        checks: &mut Checks,
    ) -> Result<(f64, Vec<Wide>), String> {
        if vector.iter().any(|&v| !input(Wide { high: v, low: 0.0 })) {
            return Err("coherent phase candidate exceeds its range".into());
        }
        checks.next()?;
        let (relative, residual) = checked(vector)?;
        if !relative.is_finite()
            || relative < 0.0
            || residual.len() != vector.len()
            || residual.iter().any(|&v| !input(Wide { high: v, low: 0.0 }))
        {
            return Err("coherent phase requires a finite matching certificate".into());
        }
        Ok((relative, residual.into_iter().map(Wide::from).collect()))
    }
}

#[path = "modal_roundoff_coherent_phase_control_tests.rs"]
mod control_tests;
