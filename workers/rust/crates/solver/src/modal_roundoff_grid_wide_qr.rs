use super::super::{
    qr::MAX_GRID_RADIUS,
    wide_factor::{bounded, dot, less},
};
use crate::modal_test_wide::Wide;
use crate::solver_control::{SolverStage, checkpoint, checkpoint_chunk};

const BEAM_WIDTH: usize = super::MAX_CERTIFICATES - 2;

// Test-only double-double proposals, never an acceptance or interval oracle.
pub(super) struct QrFit {
    columns: Vec<Vec<Wide>>,
    reflectors: Vec<(Vec<Wide>, Wide)>,
    scales: Vec<Wide>,
}

impl QrFit {
    pub(super) fn factor(columns: &[Vec<Wide>]) -> Result<Self, String> {
        let rows = columns.first().map_or(0, Vec::len);
        super::Plan::wide(rows)?;
        let count = columns.len();
        if count == 0
            || count >= rows
            || columns
                .iter()
                .any(|c| c.len() != rows || c.iter().any(|&v| !bounded(v)))
        {
            return Err("wide QR requires bounded finite tall columns".into());
        }
        checkpoint(SolverStage::DenseFactor, 0)?;
        let mut work = Vec::with_capacity(count);
        let mut scales = Vec::with_capacity(count);
        let mut initial = Vec::with_capacity(count);
        for column in columns {
            let maximum = column
                .iter()
                .map(|v| v.rounded().abs())
                .fold(0.0_f64, f64::max);
            if maximum == 0.0 || !maximum.is_finite() {
                return Err("wide QR has a zero or invalid column".into());
            }
            let scale = Wide::from(f64::from_bits(maximum.to_bits() & (0x7ff_u64 << 52)));
            if scale.high == 0.0 {
                return Err("wide QR cannot scale a subnormal column".into());
            }
            let mut scaled = Vec::with_capacity(rows);
            for (i, &v) in column.iter().enumerate() {
                let next = v.div(scale);
                if !bounded(next) || (v.high != 0.0 && next.high == 0.0) {
                    return Err("wide QR column scaling lost a bounded nonzero entry".into());
                }
                scaled.push(next);
                checkpoint_chunk(SolverStage::ModalVectorUpdate, i + 1, rows)?;
            }
            initial.push(norm(&scaled)?.rounded());
            scales.push(scale);
            work.push(scaled);
        }
        let mut reflectors = Vec::with_capacity(count);
        for pivot in 0..count {
            let norm = norm(&work[pivot][pivot..])?;
            if norm.rounded() <= 64.0 * f64::EPSILON * initial[pivot] {
                return Err("wide QR lost independent columns".into());
            }
            let original = work[pivot][pivot];
            let diagonal = norm.mul(Wide::from(if original.rounded().is_sign_negative() {
                1.0
            } else {
                -1.0
            }));
            let head = original.sub(diagonal);
            let mut direction = vec![Wide::from(1.0)];
            for &v in &work[pivot][pivot + 1..] {
                let value = v.div(head);
                if !bounded(value) {
                    return Err("wide QR reflector exceeds its range".into());
                }
                direction.push(value);
            }
            let tau = Wide::from(1.0).sub(original.div(diagonal));
            for column in &mut work[pivot..] {
                apply(&direction, tau, &mut column[pivot..])?;
            }
            work[pivot][pivot] = diagonal;
            work[pivot][pivot + 1..].fill(Wide::default());
            reflectors.push((direction, tau));
            checkpoint(SolverStage::DenseFactor, pivot + 1)?;
        }
        Ok(Self {
            columns: work,
            reflectors,
            scales,
        })
    }

    pub(super) fn quantized_solve(&self, rhs: &[f64], radius: usize) -> Result<Vec<f64>, String> {
        let rows = self.columns[0].len();
        if rhs.len() != rows
            || rhs.iter().any(|&v| !v.is_finite() || v.abs() > 1e50)
            || !(1..=MAX_GRID_RADIUS).contains(&radius)
        {
            return Err("wide quantized QR requires bounded matching data and radius".into());
        }
        checkpoint(SolverStage::DenseSubstitution, 0)?;
        let mut transformed: Vec<_> = rhs.iter().copied().map(Wide::from).collect();
        for (index, (direction, tau)) in self.reflectors.iter().enumerate() {
            apply(direction, *tau, &mut transformed[index..])?;
            checkpoint(SolverStage::DenseSubstitution, index + 1)?;
        }
        let count = self.columns.len();
        let mut scaled = vec![Wide::default(); count];
        let mut integers = vec![0.0; count];
        for row in (0..count).rev() {
            let coefficients: Vec<_> = self.columns[row + 1..].iter().map(|c| c[row]).collect();
            let accumulated = dot(&coefficients, &scaled[row + 1..])?;
            let coefficient = transformed[row]
                .sub(accumulated)
                .div(self.columns[row][row])
                .div(self.scales[row]);
            if !bounded(coefficient) {
                return Err("wide quantized QR coefficient exceeds its range".into());
            }
            integers[row] = round_grid(coefficient, radius);
            scaled[row] = Wide::from(integers[row]).mul(self.scales[row]);
            if !bounded(scaled[row]) {
                return Err("wide quantized QR decision exceeds its range".into());
            }
            checkpoint(SolverStage::DenseSubstitution, count + count - row)?;
        }
        Ok(integers)
    }

    pub(super) fn beam_solve(&self, rhs: &[f64], radius: usize) -> Result<Vec<Vec<f64>>, String> {
        if rhs.len() != self.columns[0].len()
            || rhs.iter().any(|&v| !v.is_finite() || v.abs() > 1e50)
            || !(1..=MAX_GRID_RADIUS).contains(&radius)
        {
            return Err("wide beam QR requires bounded matching data and radius".into());
        }
        checkpoint(SolverStage::DenseSubstitution, 0)?;
        let orientation = rhs
            .iter()
            .find(|v| **v != 0.0)
            .map_or(1.0, |v| 1.0_f64.copysign(*v));
        let mut transformed: Vec<_> = rhs.iter().copied().map(Wide::from).collect();
        for (index, (direction, tau)) in self.reflectors.iter().enumerate() {
            apply(direction, *tau, &mut transformed[index..])?;
            checkpoint(SolverStage::DenseSubstitution, index + 1)?;
        }
        let count = self.columns.len();
        let mut beam = vec![Partial {
            integers: vec![0.0; count],
            scaled: vec![Wide::default(); count],
            square: Wide::default(),
        }];
        for row in (0..count).rev() {
            let coefficients: Vec<_> = self.columns[row + 1..].iter().map(|c| c[row]).collect();
            let mut next = Vec::with_capacity(3 * BEAM_WIDTH);
            for partial in beam {
                let accumulated = dot(&coefficients, &partial.scaled[row + 1..])?;
                let target = transformed[row].sub(accumulated);
                let coefficient = target.div(self.columns[row][row]).div(self.scales[row]);
                if !bounded(coefficient) {
                    return Err("wide beam QR coefficient exceeds its range".into());
                }
                let nearest = round_grid(coefficient, radius);
                for integer in [nearest, nearest - 1.0, nearest + 1.0] {
                    if integer.abs() > radius as f64 {
                        continue;
                    }
                    let mut trial = partial.clone();
                    trial.integers[row] = integer;
                    trial.scaled[row] = Wide::from(integer).mul(self.scales[row]);
                    if !bounded(trial.scaled[row]) {
                        return Err("wide beam QR decision exceeds its range".into());
                    }
                    let error = target.sub(self.columns[row][row].mul(trial.scaled[row]));
                    trial.square = trial.square.add(error.mul(error));
                    if !bounded(trial.square) {
                        return Err("wide beam QR score exceeds its range".into());
                    }
                    next.push(trial);
                }
            }
            next.sort_by(|a, b| {
                a.square
                    .high
                    .total_cmp(&b.square.high)
                    .then(a.square.low.total_cmp(&b.square.low))
                    .then_with(|| compare_integers(&a.integers, &b.integers, orientation))
            });
            next.truncate(BEAM_WIDTH);
            beam = next;
            checkpoint(SolverStage::DenseSubstitution, count + count - row)?;
        }
        Ok(beam.into_iter().map(|p| p.integers).collect())
    }
}

#[derive(Clone)]
struct Partial {
    integers: Vec<f64>,
    scaled: Vec<Wide>,
    square: Wide,
}

fn compare_integers(a: &[f64], b: &[f64], orientation: f64) -> std::cmp::Ordering {
    for (&left, &right) in a.iter().zip(b) {
        // Resolve exact score ties without preferring the sign of the mode.
        let order = left
            .abs()
            .total_cmp(&right.abs())
            .then((orientation * left).total_cmp(&(orientation * right)));
        if !order.is_eq() {
            return order;
        }
    }
    std::cmp::Ordering::Equal
}

fn norm(vector: &[Wide]) -> Result<Wide, String> {
    let square = dot(vector, vector)?;
    if !bounded(square) || square.rounded() <= 0.0 {
        return Err("wide QR requires a resolved positive norm".into());
    }
    let mut root = Wide::from(square.rounded().sqrt());
    for _ in 0..2 {
        root = root.add(square.div(root)).mul(Wide::from(0.5));
    }
    if !bounded(root) {
        return Err("wide QR norm exceeds its range".into());
    }
    Ok(root)
}

fn apply(direction: &[Wide], tau: Wide, vector: &mut [Wide]) -> Result<(), String> {
    let amount = tau.mul(dot(direction, vector)?);
    if !bounded(amount) {
        return Err("wide QR reflector amount exceeds its range".into());
    }
    let size = vector.len();
    for (i, (v, &a)) in vector.iter_mut().zip(direction).enumerate() {
        *v = v.sub(amount.mul(a));
        if !bounded(*v) {
            return Err("wide QR reflector result exceeds its range".into());
        }
        checkpoint_chunk(SolverStage::ModalVectorUpdate, i + 1, size)?;
    }
    Ok(())
}

fn magnitude(v: Wide) -> Wide {
    if v.high < 0.0 || (v.high == 0.0 && v.low < 0.0) {
        Wide {
            high: -v.high,
            low: -v.low,
        }
    } else {
        v
    }
}

fn round_grid(value: Wide, radius: usize) -> f64 {
    let limit = radius as f64;
    let rounded = value.rounded();
    if rounded.abs() > limit + 2.0 {
        return limit.copysign(rounded);
    }
    let mut best = rounded.round().clamp(-limit, limit);
    let mut error = magnitude(value.sub(Wide::from(best)));
    for trial in [best - 1.0, best + 1.0] {
        if trial.abs() <= limit {
            let next = magnitude(value.sub(Wide::from(trial)));
            let tied = next.high == error.high && next.low == error.low;
            if less(next, error) || (tied && trial.abs() > best.abs()) {
                best = trial;
                error = next;
            }
        }
    }
    best
}

#[path = "modal_roundoff_grid_wide_qr_tests.rs"]
mod tests;
