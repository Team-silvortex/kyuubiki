use crate::solver_control::{SolverStage, checkpoint, checkpoint_chunk};

pub(super) use crate::modal_test_wide::Wide;

const BAND: usize = 3;
const ITERATIONS: usize = 32;

// Test-only first-mode proposals of supplied rounded K/M, not acceptance or a
// general eigensolver. Supplied coordinate scales must balance this banded case.
pub(super) struct WideProposal {
    matrix: Vec<Vec<Wide>>,
    mass: Vec<Wide>,
    original_mass: Vec<f64>,
    scales: Vec<Wide>,
    lower: Vec<Vec<Wide>>,
    diagonal: Vec<Wide>,
}

impl WideProposal {
    pub(super) fn prepare(
        matrix: &[Vec<f64>],
        mass: &[f64],
        scales: &[f64],
    ) -> Result<Self, String> {
        checkpoint(SolverStage::ModalRoundoffPrepare, 0)?;
        let size = matrix.len();
        if !(2..=256).contains(&size)
            || mass.len() != size
            || scales.len() != size
            || matrix.iter().any(|r| r.len() != size)
            || mass.iter().any(|&v| !raw_positive(v))
            || scales
                .iter()
                .any(|&v| !v.is_finite() || !(1e-20..=1e20).contains(&v))
        {
            return Err("wide proposal requires bounded matching finite data".into());
        }
        for (i, row) in matrix.iter().enumerate() {
            for (j, &v) in row.iter().enumerate() {
                if !v.is_finite()
                    || (v != 0.0 && !raw_positive(v.abs()))
                    || v != matrix[j][i]
                    || (i.abs_diff(j) > BAND && v != 0.0)
                {
                    return Err("wide proposal requires finite symmetric bandwidth-three K".into());
                }
            }
            checkpoint_chunk(SolverStage::ModalRoundoffPrepare, i + 1, size)?;
        }
        let scales: Vec<_> = scales.iter().map(|&v| Wide::from(v)).collect();
        let mut balanced = vec![vec![Wide::default(); size]; size];
        let mut weights = Vec::with_capacity(size);
        for i in 0..size {
            let weight = Wide::from(mass[i]).mul(scales[i]).mul(scales[i]);
            if !balanced_positive(weight.rounded()) {
                return Err("wide proposal mass is outside its balanced range".into());
            }
            weights.push(weight);
            for j in i.saturating_sub(BAND)..(i + BAND + 1).min(size) {
                let value = Wide::from(matrix[i][j]).mul(scales[i]).mul(scales[j]);
                if value.rounded() != 0.0 && !(1e-100..=1e12).contains(&value.rounded().abs()) {
                    return Err("wide proposal K is outside its balanced range".into());
                }
                balanced[i][j] = value;
            }
            checkpoint_chunk(SolverStage::ModalRoundoffPrepare, i + 1, size)?;
        }
        checkpoint(SolverStage::DenseFactor, 0)?;
        let mut lower = vec![vec![Wide::default(); size]; size];
        let mut diagonal = vec![Wide::default(); size];
        for row in 0..size {
            lower[row][row] = Wide::from(1.0);
            for column in row.saturating_sub(BAND)..row {
                let mut entry = balanced[row][column];
                for (k, &pivot) in diagonal
                    .iter()
                    .enumerate()
                    .take(column)
                    .skip(row.saturating_sub(BAND))
                {
                    entry = entry.sub(lower[row][k].mul(lower[column][k]).mul(pivot));
                }
                lower[row][column] = entry.div(diagonal[column]);
                if lower[row][column].rounded().abs() > 1e12 {
                    return Err("wide proposal factor exceeds its coefficient range".into());
                }
            }
            let mut pivot = balanced[row][row];
            for (k, &value) in diagonal
                .iter()
                .enumerate()
                .take(row)
                .skip(row.saturating_sub(BAND))
            {
                pivot = pivot.sub(lower[row][k].mul(lower[row][k]).mul(value));
            }
            if !pivot.rounded().is_finite() || pivot.rounded() <= 1e-12 {
                return Err("wide proposal has no resolved positive pivot".into());
            }
            diagonal[row] = pivot;
            checkpoint(SolverStage::DenseFactor, row + 1)?;
        }
        Ok(Self {
            matrix: balanced,
            mass: weights,
            original_mass: mass.to_vec(),
            scales,
            lower,
            diagonal,
        })
    }

    pub(super) fn first_mode(&self) -> Result<(f64, Vec<Wide>), String> {
        let size = self.mass.len();
        let mut vector = vec![Wide::from(1.0); size];
        for step in 0..ITERATIONS {
            checkpoint(SolverStage::ModalIteration, step)?;
            let rhs: Vec<_> = vector
                .iter()
                .zip(&self.mass)
                .map(|(&v, &m)| v.mul(m))
                .collect();
            vector = self.solve(&rhs)?;
            let maximum = vector
                .iter()
                .map(|v| v.rounded().abs())
                .fold(0.0_f64, f64::max);
            if !maximum.is_finite() || !(1e-100..=1e100).contains(&maximum) {
                return Err("wide proposal iterate cannot be binary scaled".into());
            }
            let power = Wide::from(2.0_f64.powi(-maximum.log2().floor() as i32));
            for (i, value) in vector.iter_mut().enumerate() {
                *value = value.mul(power);
                checkpoint_chunk(SolverStage::ModalVectorUpdate, i + 1, size)?;
            }
        }
        let applied = self.apply(&vector)?;
        let numerator = dot(&vector, &applied);
        let weighted: Vec<_> = vector
            .iter()
            .zip(&self.mass)
            .map(|(&v, &m)| v.mul(m))
            .collect();
        let value = numerator.div(dot(&vector, &weighted)).rounded();
        if !value.is_finite() || value <= 0.0 {
            return Err("wide proposal root is not positive and finite".into());
        }
        let mut physical = Vec::with_capacity(size);
        for (i, (&v, &s)) in vector.iter().zip(&self.scales).enumerate() {
            physical.push(v.mul(s));
            checkpoint_chunk(SolverStage::ModalVectorUpdate, i + 1, size)?;
        }
        checkpoint(SolverStage::ModalRoundoffValidate, 0)?;
        Ok((value, physical))
    }

    // Diagnostic only, sharing candidate arithmetic; not an independent oracle.
    pub(super) fn physical_residual(&self, value: f64, physical: &[Wide]) -> Result<f64, String> {
        if physical.len() != self.mass.len()
            || !raw_positive(value)
            || physical.iter().any(|&v| !bounded_iterate(v))
        {
            return Err("wide residual requires a matching positive pair".into());
        }
        let vector: Vec<_> = physical
            .iter()
            .zip(&self.scales)
            .map(|(&v, &s)| v.div(s))
            .collect();
        let applied = self.apply(&vector)?;
        let (mut residual, mut left, mut right) = (0.0_f64, 0.0_f64, 0.0_f64);
        for (i, (&v, &a)) in vector.iter().zip(&applied).enumerate() {
            let factor = Wide::from(self.original_mass[i].sqrt().recip()).div(self.scales[i]);
            let a = a.mul(factor);
            let b = Wide::from(value).mul(self.mass[i]).mul(v).mul(factor);
            residual = residual.hypot(a.sub(b).rounded());
            left = left.hypot(a.rounded());
            right = right.hypot(b.rounded());
            checkpoint_chunk(SolverStage::ResidualValidate, i + 1, vector.len())?;
        }
        let relative = residual / left.max(right);
        if !relative.is_finite() {
            return Err("wide residual is not finite".into());
        }
        Ok(relative)
    }

    fn solve(&self, rhs: &[Wide]) -> Result<Vec<Wide>, String> {
        let size = rhs.len();
        let mut result = rhs.to_vec();
        checkpoint(SolverStage::DenseSubstitution, 0)?;
        for row in 0..size {
            for column in row.saturating_sub(BAND)..row {
                result[row] = result[row].sub(self.lower[row][column].mul(result[column]));
            }
            if !bounded_iterate(result[row]) {
                return Err("wide forward substitution exceeds its range".into());
            }
            checkpoint_chunk(SolverStage::DenseSubstitution, row + 1, size)?;
        }
        for (i, (value, &pivot)) in result.iter_mut().zip(&self.diagonal).enumerate() {
            *value = value.div(pivot);
            if !bounded_iterate(*value) {
                return Err("wide diagonal substitution exceeds its range".into());
            }
            checkpoint_chunk(SolverStage::DenseSubstitution, i + 1, size)?;
        }
        for row in (0..size).rev() {
            for column in row + 1..(row + BAND + 1).min(size) {
                result[row] = result[row].sub(self.lower[column][row].mul(result[column]));
            }
            if !bounded_iterate(result[row]) {
                return Err("wide backward substitution exceeds its range".into());
            }
            checkpoint_chunk(SolverStage::DenseSubstitution, size - row, size)?;
        }
        Ok(result)
    }

    fn apply(&self, vector: &[Wide]) -> Result<Vec<Wide>, String> {
        let size = vector.len();
        let mut applied = Vec::with_capacity(size);
        for (i, row) in self.matrix.iter().enumerate() {
            let range = i.saturating_sub(BAND)..(i + BAND + 1).min(size);
            applied.push(dot(&row[range.clone()], &vector[range]));
            checkpoint_chunk(SolverStage::SparseMatvec, i + 1, size)?;
        }
        Ok(applied)
    }
}

fn raw_positive(value: f64) -> bool {
    value.is_finite() && (1e-100..=1e100).contains(&value)
}

fn balanced_positive(value: f64) -> bool {
    value.is_finite() && (1e-12..=1e12).contains(&value)
}

fn bounded_iterate(value: Wide) -> bool {
    value.high.is_finite()
        && value.low.is_finite()
        && value.high.abs() <= 1e100
        && value.low.abs() <= 1e100
}

fn dot(left: &[Wide], right: &[Wide]) -> Wide {
    left.iter()
        .zip(right)
        .fold(Wide::default(), |sum, (&a, &b)| sum.add(a.mul(b)))
}

pub(super) fn unit_shape(physical: &[Wide]) -> Result<Vec<f64>, String> {
    Ok(unit_shape_parts(physical)?
        .into_iter()
        .map(Wide::rounded)
        .collect())
}

pub(super) fn unit_shape_parts(physical: &[Wide]) -> Result<Vec<Wide>, String> {
    checkpoint(SolverStage::ModalShapeScan, 0)?;
    if !(2..=256).contains(&physical.len())
        || physical
            .iter()
            .any(|v| !v.high.is_finite() || !v.low.is_finite())
    {
        return Err("wide unit shape requires bounded finite data".into());
    }
    let maximum = physical
        .iter()
        .map(|v| v.rounded().abs())
        .fold(0.0_f64, f64::max);
    if !(1e-100..=1e100).contains(&maximum) {
        return Err("wide unit shape has no resolved bounded direction".into());
    }
    let scale = Wide::from(2.0_f64.powi(-maximum.log2().floor() as i32));
    let mut scaled = Vec::with_capacity(physical.len());
    let mut squared = Wide::default();
    for (i, &v) in physical.iter().enumerate() {
        let v = v.mul(scale);
        if physical[i].rounded() != 0.0 && v.rounded() == 0.0 {
            return Err("wide unit shape scaling lost a nonzero component".into());
        }
        scaled.push(v);
        squared = squared.add(v.mul(v));
        checkpoint_chunk(SolverStage::ModalShapeScan, i + 1, physical.len())?;
    }
    let mut norm = Wide::from(squared.rounded().sqrt());
    for _ in 0..2 {
        norm = norm.add(squared.div(norm)).mul(Wide::from(0.5));
    }
    let mut result = Vec::with_capacity(physical.len());
    for (i, v) in scaled.into_iter().enumerate() {
        let value = v.div(norm);
        if v.rounded() != 0.0 && value.rounded() == 0.0 {
            return Err("wide unit shape rounding lost a nonzero component".into());
        }
        result.push(value);
        checkpoint_chunk(SolverStage::ModalVectorUpdate, i + 1, physical.len())?;
    }
    checkpoint(SolverStage::ModalRoundoffValidate, 1)?;
    Ok(result)
}
