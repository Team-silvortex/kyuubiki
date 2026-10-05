use super::*;

const MAX_TRANSFORM: i64 = 1 << 20;
const PAIR_STEPS: usize = 8;

#[path = "modal_roundoff_grid_pair_lattice_control_tests.rs"]
mod controls;

pub(in super::super) struct PairFit {
    seed: Vec<f64>,
    grids: Vec<f64>,
    order: Vec<usize>,
    transform: Vec<Vec<i64>>,
    factor: QrFit,
    width: usize,
    pub(in super::super) pair_steps: usize,
    pub(in super::super) shears: usize,
    pub(in super::super) swaps: usize,
    pub(in super::super) stop: Option<super::super::qr::lattice::Stop>,
}

impl Plan {
    pub(in super::super) fn pair_lattice(
        size: usize,
        width: usize,
        sweeps: usize,
    ) -> Result<Self, String> {
        if !(1..=4).contains(&sweeps) {
            return Err("pair lattice requires one to four preselected sweeps".into());
        }
        let mut plan = Self::rounded_beam(size, width)?;
        plan.payload_bytes += 24 * size * size + 4096 * size;
        plan.component_visits += (128 * sweeps + 2 * width) * size * size;
        if plan.payload_bytes > 8 * 1024 * 1024 || plan.component_visits > 650_000_000 {
            return Err("pair lattice exceeds its separate construction budget".into());
        }
        Ok(plan)
    }
}

fn finite(value: Wide) -> bool {
    value.high.is_finite() && value.low.is_finite() && value.low.abs() <= value.high.abs()
}

fn inner(left: &[Wide], right: &[Wide]) -> Result<Wide, String> {
    let mut sum = Wide::default();
    for (&a, &b) in left.iter().zip(right) {
        let product = a.mul(b);
        if !finite(product) || (a.high != 0.0 && b.high != 0.0 && product.high == 0.0) {
            return Err("pair lattice inner product exceeds its retained-entry range".into());
        }
        sum = sum.add(product);
    }
    if !finite(sum) {
        return Err("pair lattice inner product is not finite".into());
    }
    Ok(sum)
}

impl PairFit {
    pub(in super::super) fn prepare_reduced(
        matrix: &[Vec<Wide>],
        seed: &[f64],
        anchor: usize,
        order: &[usize],
        width: usize,
    ) -> Result<Self, String> {
        let size = seed.len();
        Plan::triangular_lattice(size, width)?;
        let grids = validated_grids(matrix, seed, anchor, order)?;
        if !(0.25..=2.0).contains(&super::super::vector_norm(seed.iter().copied())?) {
            return Err("triangular lattice requires unchanged direction norm bounds".into());
        }
        let original: Vec<Vec<_>> = order
            .iter()
            .map(|&j| {
                matrix
                    .iter()
                    .map(|row| row[j].mul(Wide::from(grids[j])))
                    .collect()
            })
            .collect();
        let rounded: Vec<Vec<_>> = original
            .iter()
            .map(|column| column.iter().map(|v| v.rounded()).collect())
            .collect();
        let reduced = QrFit::factor(&rounded)?.bounded_lattice()?;
        let mut columns = Vec::with_capacity(order.len());
        for (j, coefficients) in reduced.transform.iter().enumerate() {
            let mut column = vec![0.0; size];
            for (i, value) in column.iter_mut().enumerate() {
                let mut sum = Wide::default();
                for (&coefficient, source) in coefficients.iter().zip(&original) {
                    if coefficient != 0 {
                        sum = sum.add(source[i].mul(Wide::from(coefficient as f64)));
                    }
                }
                *value = sum.rounded();
                if !finite(sum) || !value.is_finite() || (sum.high != 0.0 && *value == 0.0) {
                    return Err("triangular lattice reconstruction lost a retained entry".into());
                }
                checkpoint_chunk(SolverStage::ModalVectorUpdate, i + 1, size)?;
            }
            columns.push(column);
            checkpoint(SolverStage::ModalVectorUpdate, j + 1)?;
        }
        Ok(Self {
            seed: seed.to_vec(),
            grids,
            order: order.to_vec(),
            transform: reduced.transform,
            factor: QrFit::factor(&columns)?,
            width,
            pair_steps: reduced.steps,
            shears: reduced.shears,
            swaps: reduced.swaps,
            stop: Some(reduced.stop),
        })
    }

    pub(in super::super) fn prepare(
        matrix: &[Vec<Wide>],
        seed: &[f64],
        anchor: usize,
        order: &[usize],
        width: usize,
        sweeps: usize,
    ) -> Result<Self, String> {
        Plan::pair_lattice(seed.len(), width, sweeps)?;
        let grids = validated_grids(matrix, seed, anchor, order)?;
        if !(0.25..=2.0).contains(&super::super::vector_norm(seed.iter().copied())?) {
            return Err("pair lattice requires unchanged direction norm bounds".into());
        }
        let mut columns: Vec<Vec<Wide>> = order
            .iter()
            .map(|&j| {
                matrix
                    .iter()
                    .map(|row| row[j].mul(Wide::from(grids[j])))
                    .collect()
            })
            .collect();
        if columns
            .iter()
            .flatten()
            .any(|&v| !finite(v) || v.high.abs() > 1e90)
        {
            return Err("pair lattice scaled columns exceed their range".into());
        }
        let count = order.len();
        let mut transform: Vec<Vec<i64>> = (0..count)
            .map(|j| (0..count).map(|i| i64::from(i == j)).collect())
            .collect();
        let mut pair_steps = 0;
        let mut shears = 0;
        let mut swaps = 0;
        // Each swap/shear is unimodular. The anchor and original grid never move.
        for sweep in 0..sweeps {
            for pair in 0..count.saturating_sub(1) {
                let a = if sweep % 2 == 0 {
                    pair
                } else {
                    count - 2 - pair
                };
                let b = a + 1;
                for _ in 0..PAIR_STEPS {
                    checkpoint(SolverStage::ModalRoundoffPrepare, pair_steps)?;
                    pair_steps += 1;
                    let mut norm = inner(&columns[a], &columns[a])?;
                    let other = inner(&columns[b], &columns[b])?;
                    if norm.high <= 0.0 || other.high <= 0.0 {
                        return Err("pair lattice has a zero or invalid column".into());
                    }
                    if other.high < norm.high || (other.high == norm.high && other.low < norm.low) {
                        columns.swap(a, b);
                        transform.swap(a, b);
                        norm = other;
                        swaps += 1;
                    }
                    let quotient = inner(&columns[a], &columns[b])?.div(norm).rounded().round();
                    if !quotient.is_finite() || quotient.abs() > MAX_TRANSFORM as f64 {
                        return Err(
                            "pair lattice shear exceeds its integer coefficient bound".into()
                        );
                    }
                    if quotient == 0.0 {
                        break;
                    }
                    let q = quotient as i64;
                    let next: Vec<_> = transform[b]
                        .iter()
                        .zip(&transform[a])
                        .map(|(&v, &u)| {
                            v.checked_sub(q.checked_mul(u)?)
                                .filter(|x| x.abs() <= MAX_TRANSFORM)
                        })
                        .collect::<Option<Vec<_>>>()
                        .ok_or("pair lattice transform exceeds its integer coefficient bound")?;
                    for row in 0..seed.len() {
                        let value = columns[b][row].sub(columns[a][row].mul(Wide::from(quotient)));
                        if !finite(value) || value.high.abs() > 1e90 {
                            return Err("pair lattice shear exceeds its arithmetic range".into());
                        }
                        columns[b][row] = value;
                    }
                    transform[b] = next;
                    shears += 1;
                }
            }
            checkpoint(SolverStage::ModalIteration, sweep + 1)?;
        }
        let rounded: Vec<Vec<_>> = columns
            .iter()
            .map(|column| {
                column
                    .iter()
                    .map(|v| {
                        let rounded = v.rounded();
                        if !rounded.is_finite() || (v.high != 0.0 && rounded == 0.0) {
                            Err("pair lattice rounding cannot discard retained entries".into())
                        } else {
                            Ok(rounded)
                        }
                    })
                    .collect::<Result<_, String>>()
            })
            .collect::<Result<_, _>>()?;
        let factor = QrFit::factor(&rounded)?;
        Ok(Self {
            seed: seed.to_vec(),
            grids,
            order: order.to_vec(),
            transform,
            factor,
            width,
            pair_steps,
            shears,
            swaps,
            stop: None,
        })
    }

    fn original_offsets(
        &self,
        decisions: &[f64],
        radius: usize,
    ) -> Result<Option<Vec<i64>>, String> {
        if decisions.len() != self.order.len()
            || decisions
                .iter()
                .any(|v| !v.is_finite() || v.fract() != 0.0 || v.abs() > radius as f64)
        {
            return Err("pair lattice requires matching bounded integer decisions".into());
        }
        let mut result = vec![0_i64; self.order.len()];
        for (&decision, column) in decisions.iter().zip(&self.transform) {
            for (value, &coefficient) in result.iter_mut().zip(column) {
                *value = value
                    .checked_add(
                        coefficient
                            .checked_mul(decision as i64)
                            .ok_or("pair lattice decision integer overflow")?,
                    )
                    .ok_or("pair lattice decision integer overflow")?;
            }
        }
        if result.iter().any(|v| v.unsigned_abs() > radius as u64) {
            return Ok(None);
        }
        Ok(Some(result))
    }

    pub(in super::super) fn attempt(
        &self,
        radius: usize,
        tolerance: f64,
        mut checked: impl FnMut(&[f64]) -> Result<(f64, Vec<f64>), String>,
    ) -> Result<Attempt, String> {
        if !(1..=MAX_GRID_RADIUS).contains(&radius) || !tolerance.is_finite() || tolerance <= 0.0 {
            return Err("pair lattice requires bounded radius, width and tolerance".into());
        }
        let mut checks = Checks::new();
        let mut measure = |v: &[f64]| -> Result<(f64, Vec<f64>), String> {
            checks.next()?;
            let receipt = checked(v)?;
            validate_certificate(self.seed.len(), &receipt)?;
            Ok(receipt)
        };
        let mut retained = self.seed.clone();
        let (mut best, residual) = measure(&retained)?;
        if best > tolerance {
            let rhs: Vec<_> = residual.iter().map(|v| -v).collect();
            let proposals = self.factor.quantized_beam(&rhs, radius, self.width)?;
            if proposals.len() > self.width {
                return Err("pair lattice exceeded its fixed proposal budget".into());
            }
            for decisions in proposals {
                let Some(offsets) = self.original_offsets(&decisions, radius)? else {
                    continue;
                };
                let mut candidate = self.seed.clone();
                for (&i, &offset) in self.order.iter().zip(&offsets) {
                    candidate[i] = Wide::from(self.seed[i])
                        .add(Wide::from(self.grids[i]).mul(Wide::from(offset as f64)))
                        .rounded();
                    if !input(candidate[i]) {
                        return Err("pair lattice candidate exceeds its range".into());
                    }
                    let represented = Wide::from(candidate[i])
                        .sub(Wide::from(self.seed[i]))
                        .div(Wide::from(self.grids[i]))
                        .rounded();
                    if represented != offset as f64 {
                        return Err("pair lattice rounding left its original integer grid".into());
                    }
                }
                let (actual, _) = measure(&candidate)?;
                if actual < best {
                    best = actual;
                    retained = candidate;
                }
            }
        }
        let (actual, _) = measure(&retained)?;
        if actual > tolerance {
            return Ok(Attempt::Rejected(Rejection::Residual(actual)));
        }
        if !(0.25..=2.0).contains(&super::super::vector_norm(retained.iter().copied())?) {
            return Err("pair lattice changed its unchanged direction norm bounds".into());
        }
        checkpoint(SolverStage::ModalRoundoffValidate, 9)?;
        Ok(Attempt::Accepted(retained))
    }
}

impl Plan {
    pub(in super::super) fn triangular_lattice(size: usize, width: usize) -> Result<Self, String> {
        let mut plan = Self::pair_lattice(size, width, 4)?;
        plan.payload_bytes += 16 * size * size;
        plan.component_visits +=
            16 * size * size * size + 8 * super::super::qr::lattice::MAX_STEPS * size * size;
        if plan.payload_bytes > 10 * 1024 * 1024 || plan.component_visits > 3_100_000_000 {
            return Err("triangular lattice exceeds its separate construction budget".into());
        }
        Ok(plan)
    }
}
