use crate::modal_test_wide::Wide;
use crate::solver_control::{SolverStage, checkpoint, checkpoint_chunk};

// Test-only scaled Gram/LDL proposal factor, not a rank or acceptance oracle.
pub(super) struct Factor {
    columns: Vec<Vec<Wide>>,
    scales: Vec<Wide>,
    lower: Vec<Vec<Wide>>,
    diagonal: Vec<Wide>,
}

impl Factor {
    pub(super) fn prepare(columns: &[Vec<Wide>]) -> Result<Self, String> {
        let rows = columns.first().map_or(0, Vec::len);
        super::budget::Plan::new(rows)?;
        let count = columns.len();
        if count == 0
            || count >= rows
            || columns
                .iter()
                .any(|c| c.len() != rows || c.iter().any(|&v| !bounded(v)))
        {
            return Err("wide fine factor requires bounded finite tall columns".into());
        }
        let mut normalized = Vec::with_capacity(count);
        let mut scales = Vec::with_capacity(count);
        for column in columns {
            let maximum = column
                .iter()
                .map(|v| v.rounded().abs())
                .fold(0.0_f64, f64::max);
            if maximum == 0.0 {
                return Err("wide fine factor has a zero column".into());
            }
            let scale = Wide::from(2.0_f64.powi(maximum.log2().floor() as i32));
            let mut values = Vec::with_capacity(rows);
            for (i, &v) in column.iter().enumerate() {
                let next = v.div(scale);
                if !bounded(next) || (v.high != 0.0 && next.high == 0.0) {
                    return Err("wide fine column scaling lost its bounded representation".into());
                }
                values.push(next);
                checkpoint_chunk(SolverStage::ModalVectorUpdate, i + 1, rows)?;
            }
            normalized.push(values);
            scales.push(scale);
        }
        let mut gram = vec![vec![Wide::default(); count]; count];
        for i in 0..count {
            for j in 0..=i {
                gram[i][j] = dot(&normalized[i], &normalized[j])?;
                gram[j][i] = gram[i][j];
            }
        }
        checkpoint(SolverStage::DenseFactor, 0)?;
        let mut lower = vec![vec![Wide::default(); count]; count];
        let mut diagonal = vec![Wide::default(); count];
        for i in 0..count {
            lower[i][i] = Wide::from(1.0);
            for j in 0..i {
                let mut value = gram[i][j];
                for (k, &pivot) in diagonal.iter().enumerate().take(j) {
                    value = value.sub(lower[i][k].mul(lower[j][k]).mul(pivot));
                    checkpoint_chunk(SolverStage::DenseFactor, k + 1, count)?;
                }
                lower[i][j] = value.div(diagonal[j]);
                if !bounded(lower[i][j]) || lower[i][j].rounded().abs() > 1e14 {
                    return Err("wide fine factor exceeds its coefficient range".into());
                }
            }
            let mut pivot = gram[i][i];
            for (k, &value) in diagonal.iter().enumerate().take(i) {
                pivot = pivot.sub(lower[i][k].mul(lower[i][k]).mul(value));
            }
            if !bounded(pivot) || pivot.rounded() <= 1e-28 * gram[i][i].rounded() {
                return Err("wide fine factor has no resolved positive pivot".into());
            }
            diagonal[i] = pivot;
            checkpoint(SolverStage::DenseFactor, i + 1)?;
        }
        Ok(Self {
            columns: normalized,
            scales,
            lower,
            diagonal,
        })
    }

    pub(super) fn coefficients(&self, residual: &[Wide]) -> Result<Vec<Wide>, String> {
        let mut result = self.scaled_coefficients(residual)?;
        for (i, (value, &scale)) in result.iter_mut().zip(&self.scales).enumerate() {
            *value = value.div(scale);
            if !bounded(*value) {
                return Err("wide unscaled correction exceeds its range".into());
            }
            checkpoint_chunk(SolverStage::ModalVectorUpdate, i + 1, self.scales.len())?;
        }
        Ok(result)
    }

    pub(super) fn project(&self, residual: &[Wide]) -> Result<Vec<Wide>, String> {
        let coefficients = self.scaled_coefficients(residual)?;
        let mut projected = residual.to_vec();
        for (i, value) in projected.iter_mut().enumerate() {
            for (column, &weight) in self.columns.iter().zip(&coefficients) {
                *value = value.sub(column[i].mul(weight));
            }
            if !bounded(*value) {
                return Err("wide projected residual exceeds its range".into());
            }
            checkpoint_chunk(SolverStage::ModalVectorUpdate, i + 1, residual.len())?;
        }
        Ok(projected)
    }

    pub(super) fn choose_rounding(
        &self,
        coordinates: &[usize],
        seed: &[f64],
        candidate: &mut [f64],
        residual: &[Wide],
    ) -> Result<(), String> {
        let size = self.columns[0].len();
        if coordinates.len() != self.columns.len()
            || seed.len() != size
            || candidate.len() != size
            || residual.len() != size
            || coordinates.iter().any(|&i| i >= size)
            || coordinates.windows(2).any(|w| w[0] >= w[1])
            || seed
                .iter()
                .chain(candidate.iter())
                .any(|&v| !bounded(Wide { high: v, low: 0.0 }))
            || residual.iter().any(|&v| !bounded(v))
        {
            return Err(
                "wide rounding requires matching bounded data and sorted coordinates".into(),
            );
        }
        let mut predicted = residual.to_vec();
        for (column, &coordinate) in coordinates.iter().enumerate() {
            self.update_column(
                column,
                seed[coordinate],
                candidate[coordinate],
                &mut predicted,
            )?;
        }
        for column in (0..coordinates.len()).chain((0..coordinates.len()).rev()) {
            let coordinate = coordinates[column];
            let old = candidate[coordinate];
            let mut best = dot(&predicted, &predicted)?;
            let mut retained = old;
            for trial in [old.next_up(), old.next_down()] {
                if !bounded(Wide {
                    high: trial,
                    low: 0.0,
                }) {
                    continue;
                }
                let mut next = predicted.clone();
                self.update_column(column, old, trial, &mut next)?;
                let score = dot(&next, &next)?;
                if less(score, best) {
                    best = score;
                    retained = trial;
                }
            }
            self.update_column(column, old, retained, &mut predicted)?;
            candidate[coordinate] = retained;
        }
        Ok(())
    }

    fn update_column(
        &self,
        column: usize,
        old: f64,
        next: f64,
        residual: &mut [Wide],
    ) -> Result<(), String> {
        let amount = Wide::from(next)
            .sub(Wide::from(old))
            .mul(self.scales[column]);
        if !bounded(amount) {
            return Err("wide rounding delta exceeds its range".into());
        }
        let size = residual.len();
        for (i, (value, &direction)) in residual.iter_mut().zip(&self.columns[column]).enumerate() {
            *value = value.add(direction.mul(amount));
            if !bounded(*value) {
                return Err("wide rounding prediction exceeds its range".into());
            }
            checkpoint_chunk(SolverStage::ModalVectorUpdate, i + 1, size)?;
        }
        Ok(())
    }

    fn scaled_coefficients(&self, residual: &[Wide]) -> Result<Vec<Wide>, String> {
        if residual.len() != self.columns[0].len() || residual.iter().any(|&v| !bounded(v)) {
            return Err("wide fine solve requires a matching bounded right-hand side".into());
        }
        checkpoint(SolverStage::DenseSubstitution, 0)?;
        let mut result = self
            .columns
            .iter()
            .map(|c| dot(c, residual))
            .collect::<Result<Vec<_>, _>>()?;
        let count = result.len();
        for i in 0..count {
            for j in 0..i {
                result[i] = result[i].sub(self.lower[i][j].mul(result[j]));
            }
            if !bounded(result[i]) {
                return Err("wide forward correction exceeds its range".into());
            }
            checkpoint(SolverStage::DenseSubstitution, i + 1)?;
        }
        for (value, &pivot) in result.iter_mut().zip(&self.diagonal) {
            *value = value.div(pivot);
            if !bounded(*value) {
                return Err("wide diagonal correction exceeds its range".into());
            }
        }
        for i in (0..count).rev() {
            for j in i + 1..count {
                result[i] = result[i].sub(self.lower[j][i].mul(result[j]));
            }
            if !bounded(result[i]) {
                return Err("wide backward correction exceeds its range".into());
            }
            checkpoint(SolverStage::DenseSubstitution, count + count - i)?;
        }
        Ok(result)
    }
}

pub(super) fn bounded(value: Wide) -> bool {
    value.high.is_finite()
        && value.low.is_finite()
        && value.low.abs() <= value.high.abs()
        && (value.high == 0.0 || (1e-100..=1e100).contains(&value.high.abs()))
}

pub(super) fn dot(left: &[Wide], right: &[Wide]) -> Result<Wide, String> {
    if left.len() != right.len() || left.iter().chain(right).any(|&v| !bounded(v)) {
        return Err("wide dot requires matching bounded components".into());
    }
    checkpoint(SolverStage::ModalVectorDot, 0)?;
    let mut sum = Wide::default();
    for (i, (&a, &b)) in left.iter().zip(right).enumerate() {
        sum = sum.add(a.mul(b));
        checkpoint_chunk(SolverStage::ModalVectorDot, i + 1, left.len())?;
    }
    Ok(sum)
}

pub(super) fn less(a: Wide, b: Wide) -> bool {
    a.high < b.high || (a.high == b.high && a.low < b.low)
}
