use super::{dot, relative_residual, residual_vector, vector_norm};
use crate::linear_dense::DenseLu;
use crate::modal_sparse::{ReducedSparseModalSystem, SparseMassNormalizedOperator};
use crate::solver_control::{SolverStage, checkpoint, checkpoint_chunk};

#[path = "modal_roundoff_budget.rs"]
mod budget;
#[path = "modal_roundoff_partition.rs"]
pub(crate) mod partition;
use budget::{Checks, FINE_STEPS, OUTER_STEPS, PARTNERS, Plan};
pub(crate) use budget::{MAX_DOFS, eligible};

#[cfg(test)]
#[path = "modal_roundoff_qr_reference.rs"]
mod qr;

#[cfg(test)]
#[path = "modal_roundoff_grid_reference.rs"]
mod grid;

enum FineFactor {
    Gram(DenseLu),
    #[cfg(test)]
    Qr(qr::QrFit),
}

impl FineFactor {
    fn gram(columns: &[Vec<f64>]) -> Result<Self, String> {
        let gram = columns
            .iter()
            .map(|a| columns.iter().map(|b| dot(a, b)).collect())
            .collect::<Result<Vec<Vec<_>>, _>>()?;
        Ok(Self::Gram(DenseLu::factor(gram)?))
    }

    fn solve(&self, columns: &[Vec<f64>], residual: &[f64]) -> Result<Vec<f64>, String> {
        match self {
            Self::Gram(factor) => {
                let rhs = columns
                    .iter()
                    .map(|a| dot(a, residual))
                    .collect::<Result<Vec<_>, _>>()?;
                factor.solve(&rhs)
            }
            #[cfg(test)]
            Self::Qr(factor) => factor.solve(residual),
        }
    }
}

pub(crate) fn normalized(
    system: &ReducedSparseModalSystem,
    value: f64,
    vector: &[f64],
    tolerance: f64,
) -> Result<Vec<f64>, String> {
    Plan::new(system.operator.dimension())?;
    if vector.len() != system.operator.dimension() || !value.is_finite() || value <= 0.0 {
        return Err("modal roundoff correction requires matching positive eigenpairs".into());
    }
    let mut matrix = system.operator.dense_fallback_matrix()?;
    for (i, row) in matrix.iter_mut().enumerate() {
        row[i] -= value;
        checkpoint_chunk(SolverStage::ModalVectorUpdate, i + 1, vector.len())?;
    }
    admit(matrix, vector, tolerance, |v| {
        let applied = system.operator.apply_compensated(v)?;
        Ok((
            relative_residual(value, v, &applied)?,
            residual_vector(value, v, &applied)?,
        ))
    })
    .map_err(|error| format!("normalized modal roundoff recovery failed: {error}"))
}

pub(crate) fn physical(
    operator: &SparseMassNormalizedOperator,
    value: f64,
    shape: &[f64],
    mass: &[f64],
    tolerance: f64,
) -> Result<Vec<f64>, String> {
    Plan::new(operator.dimension())?;
    if mass.len() != shape.len()
        || shape.len() != operator.dimension()
        || !value.is_finite()
        || value <= 0.0
        || mass.iter().any(|m| !m.is_finite() || *m <= 0.0)
    {
        return Err("modal roundoff shape requires matching positive mass".into());
    }
    let mut matrix = operator.dense_fallback_matrix()?;
    for (i, row) in matrix.iter_mut().enumerate() {
        for (j, (a, &m)) in row.iter_mut().zip(mass).enumerate() {
            *a /= m.sqrt().recip();
            checkpoint_chunk(SolverStage::ModalVectorUpdate, j + 1, shape.len())?;
        }
        row[i] -= value * mass[i] * mass[i].sqrt().recip();
    }
    admit(matrix, shape, tolerance, |v| {
        let applied = operator.apply_physical_compensated(v)?;
        operator.physical_residual(value, v, mass, &applied)
    })
    .map_err(|error| format!("physical modal roundoff recovery failed: {error}"))
}

fn admit(
    matrix: Vec<Vec<f64>>,
    vector: &[f64],
    tolerance: f64,
    mut checked: impl FnMut(&[f64]) -> Result<(f64, Vec<f64>), String>,
) -> Result<Vec<f64>, String> {
    if !tolerance.is_finite() || tolerance <= 0.0 || vector.iter().any(|v| !v.is_finite()) {
        return Err(
            "modal roundoff admission requires finite data and a positive tolerance".into(),
        );
    }
    let fit = BlockFit::prepare_automatic(matrix, vector)?;
    let candidate = fit.correct(vector, tolerance, &mut checked)?;
    let (relative, residual) = checked(&candidate)?;
    if !relative.is_finite()
        || relative < 0.0
        || relative > tolerance
        || residual.len() != vector.len()
        || residual.iter().any(|v| !v.is_finite())
    {
        return Err("modal roundoff correction lost its final residual certificate".into());
    }
    checkpoint(SolverStage::ModalRoundoffValidate, 1)?;
    Ok(candidate)
}

// Approximate dense columns propose candidates; they never certify acceptance.
pub(crate) struct BlockFit {
    fine: Vec<usize>,
    coarse: Vec<usize>,
    columns: Vec<Vec<f64>>,
    projected: Vec<Vec<f64>>,
    pairs: Vec<(usize, usize)>,
    factor: FineFactor,
}

impl BlockFit {
    pub(crate) fn prepare(matrix: Vec<Vec<f64>>, fine: Vec<usize>) -> Result<Self, String> {
        Self::prepare_with(matrix, fine, FineFactor::gram)
    }

    fn prepare_with(
        matrix: Vec<Vec<f64>>,
        fine: Vec<usize>,
        factorize: impl FnOnce(&[Vec<f64>]) -> Result<FineFactor, String>,
    ) -> Result<Self, String> {
        let size = matrix.len();
        Plan::new(size)?;
        if matrix
            .iter()
            .any(|row| row.len() != size || row.iter().any(|v| !v.is_finite()))
            || fine.is_empty()
            || fine.len() >= size
            || fine.iter().any(|&i| i >= size)
            || fine.windows(2).any(|pair| pair[0] >= pair[1])
        {
            return Err(
                "block fit requires a bounded square matrix and sorted proper subset".into(),
            );
        }
        let coarse: Vec<_> = (0..size)
            .filter(|i| fine.binary_search(i).is_err())
            .collect();
        let columns: Vec<Vec<_>> = fine
            .iter()
            .map(|&j| matrix.iter().map(|r| r[j]).collect())
            .collect();
        let factor = factorize(&columns)?;
        let mut projected = Vec::with_capacity(coarse.len());
        for &j in &coarse {
            let column: Vec<_> = matrix.iter().map(|r| r[j]).collect();
            let coefficients = factor.solve(&columns, &column)?;
            projected.push(project(&column, &columns, &coefficients)?);
        }
        Ok(Self {
            fine,
            pairs: (0..coarse.len().saturating_sub(1))
                .map(|i| (i, i + 1))
                .collect(),
            coarse,
            columns,
            projected,
            factor,
        })
    }

    pub(crate) fn prepare_coupled(matrix: Vec<Vec<f64>>, fine: Vec<usize>) -> Result<Self, String> {
        let mut fit = Self::prepare(matrix, fine)?;
        fit.pairs = coupled_pairs(&fit.projected)?;
        Ok(fit)
    }

    pub(crate) fn prepare_automatic(matrix: Vec<Vec<f64>>, vector: &[f64]) -> Result<Self, String> {
        let fine = partition::select_fine(&matrix, vector)?;
        Self::prepare_coupled(matrix, fine)
    }

    #[cfg(test)]
    fn prepare_qr_automatic(matrix: Vec<Vec<f64>>, vector: &[f64]) -> Result<Self, String> {
        let fine = partition::select_fine(&matrix, vector)?;
        Self::prepare_qr_partition(matrix, fine)
    }

    #[cfg(test)]
    fn prepare_qr_partition(matrix: Vec<Vec<f64>>, fine: Vec<usize>) -> Result<Self, String> {
        let mut fit = Self::prepare_with(matrix, fine, |columns| {
            Ok(FineFactor::Qr(qr::QrFit::factor(columns)?))
        })?;
        fit.pairs = coupled_pairs(&fit.projected)?;
        Ok(fit)
    }

    pub(crate) fn correct(
        &self,
        vector: &[f64],
        tolerance: f64,
        checked: impl FnMut(&[f64]) -> Result<(f64, Vec<f64>), String>,
    ) -> Result<Vec<f64>, String> {
        self.correct_internal(vector, tolerance, checked, None)
    }

    #[cfg(test)]
    fn correct_hybrid(
        &self,
        alternate: &Self,
        vector: &[f64],
        tolerance: f64,
        checked: impl FnMut(&[f64]) -> Result<(f64, Vec<f64>), String>,
    ) -> Result<Vec<f64>, String> {
        if self.fine.len() + self.coarse.len() != alternate.fine.len() + alternate.coarse.len() {
            return Err("hybrid proposal dimensions must match".into());
        }
        self.correct_internal(vector, tolerance, checked, Some(alternate))
    }

    fn correct_internal(
        &self,
        vector: &[f64],
        tolerance: f64,
        mut checked: impl FnMut(&[f64]) -> Result<(f64, Vec<f64>), String>,
        mut alternate: Option<&Self>,
    ) -> Result<Vec<f64>, String> {
        if vector.len() != self.fine.len() + self.coarse.len()
            || vector.iter().any(|v| !v.is_finite())
            || !tolerance.is_finite()
            || tolerance <= 0.0
        {
            return Err("block fit requires matching finite data and a positive tolerance".into());
        }
        let mut checks = Checks::new();
        let mut active = self;
        let mut retained = vector.to_vec();
        let (initial, _) = self.measure(&retained, &mut checked, &mut checks)?;
        if initial <= tolerance {
            checkpoint(SolverStage::ModalValidation, 0)?;
            return Ok(retained);
        }
        for step in 0..OUTER_STEPS {
            checkpoint(SolverStage::ModalIteration, step)?;
            active.fit_fine(&mut retained, &mut checked, &mut checks)?;
            let (before, residual) = active.measure(&retained, &mut checked, &mut checks)?;
            if before <= tolerance {
                checkpoint(SolverStage::ModalValidation, step)?;
                return Ok(retained);
            }
            let coefficients = active.project_coefficients(&residual)?;
            let mut residual = project(&residual, &active.columns, &coefficients)?;
            let mut candidate = retained.clone();
            active.single_sweep(&mut candidate, &mut residual)?;
            active.pair_sweep(&mut candidate, &mut residual)?;
            active.fit_fine(&mut candidate, &mut checked, &mut checks)?;
            let (after, _) = active.measure(&candidate, &mut checked, &mut checks)?;
            if after >= before {
                if let Some(next) = alternate.take() {
                    // Switching candidates does not reset the shared checks or outer-step budget.
                    active = next;
                    continue;
                }
                break;
            }
            retained = candidate;
            if after <= tolerance {
                checkpoint(SolverStage::ModalValidation, step)?;
                return Ok(retained);
            }
        }
        let (relative, _) = self.measure(&retained, &mut checked, &mut checks)?;
        Err(format!(
            "block fit did not reach its unchanged residual gate (relative={relative:e})"
        ))
    }

    fn measure(
        &self,
        vector: &[f64],
        checked: &mut impl FnMut(&[f64]) -> Result<(f64, Vec<f64>), String>,
        checks: &mut Checks,
    ) -> Result<(f64, Vec<f64>), String> {
        if vector.iter().any(|v| !v.is_finite()) {
            return Err("block fit cannot measure a non-finite candidate".into());
        }
        checks.next()?;
        let (relative, residual) = checked(vector)?;
        if !relative.is_finite()
            || relative < 0.0
            || residual.len() != vector.len()
            || residual.iter().any(|r| !r.is_finite())
        {
            return Err("block fit requires a finite matching residual certificate".into());
        }
        Ok((relative, residual))
    }

    fn project_coefficients(&self, residual: &[f64]) -> Result<Vec<f64>, String> {
        self.factor.solve(&self.columns, residual)
    }

    fn fit_fine(
        &self,
        vector: &mut Vec<f64>,
        checked: &mut impl FnMut(&[f64]) -> Result<(f64, Vec<f64>), String>,
        checks: &mut Checks,
    ) -> Result<(), String> {
        for _ in 0..FINE_STEPS {
            let (before, residual) = self.measure(vector, checked, checks)?;
            let correction = self.project_coefficients(&residual)?;
            let mut candidate = vector.clone();
            for (&i, &c) in self.fine.iter().zip(&correction) {
                candidate[i] -= c;
            }
            let (after, _) = self.measure(&candidate, checked, checks)?;
            if after >= before {
                break;
            }
            *vector = candidate;
        }
        Ok(())
    }

    fn single_sweep(&self, vector: &mut [f64], residual: &mut [f64]) -> Result<(), String> {
        for column in (0..self.coarse.len()).chain((0..self.coarse.len()).rev()) {
            let i = self.coarse[column];
            let direction = &self.projected[column];
            let square = dot(direction, direction)?;
            if square == 0.0 {
                continue;
            }
            let step = dot(direction, residual)? / square;
            let old = vector[i];
            let mut best = dot(residual, residual)?;
            let mut updated = old;
            for trial in [old - step, old.next_up(), old.next_down()] {
                if !trial.is_finite() {
                    continue;
                }
                let change = old - trial;
                let predicted = predict(residual, direction, change, None)?;
                let score = dot(&predicted, &predicted)?;
                if score < best {
                    best = score;
                    updated = trial;
                }
            }
            let change = old - updated;
            for (index, (r, &a)) in residual.iter_mut().zip(direction).enumerate() {
                *r = (-change).mul_add(a, *r);
                checkpoint_chunk(SolverStage::ModalVectorUpdate, index + 1, vector.len())?;
            }
            vector[i] = updated;
        }
        Ok(())
    }

    fn pair_sweep(&self, vector: &mut [f64], residual: &mut [f64]) -> Result<(), String> {
        for &(left, right) in self.pairs.iter().chain(self.pairs.iter().rev()) {
            let (a, b) = (&self.projected[left], &self.projected[right]);
            let (left, right) = (self.coarse[left], self.coarse[right]);
            let (old_left, old_right) = (vector[left], vector[right]);
            let mut best = dot(residual, residual)?;
            let mut updated = (old_left, old_right);
            for x in [old_left, old_left.next_up(), old_left.next_down()] {
                for y in [old_right, old_right.next_up(), old_right.next_down()] {
                    if !x.is_finite() || !y.is_finite() {
                        continue;
                    }
                    let predicted = predict(residual, a, old_left - x, Some((b, old_right - y)))?;
                    let score = dot(&predicted, &predicted)?;
                    if score < best {
                        best = score;
                        updated = (x, y);
                    }
                }
            }
            let (dx, dy) = (old_left - updated.0, old_right - updated.1);
            for (index, ((r, &a), &b)) in residual.iter_mut().zip(a).zip(b).enumerate() {
                *r = (-dy).mul_add(b, (-dx).mul_add(a, *r));
                checkpoint_chunk(SolverStage::ModalVectorUpdate, index + 1, vector.len())?;
            }
            vector[left] = updated.0;
            vector[right] = updated.1;
        }
        Ok(())
    }
}

pub(crate) fn coupled_pairs(columns: &[Vec<f64>]) -> Result<Vec<(usize, usize)>, String> {
    let rows = columns.first().map_or(0, Vec::len);
    if columns.len() > budget::MAX_DOFS
        || rows > budget::MAX_DOFS
        || columns
            .iter()
            .any(|c| c.len() != rows || c.iter().any(|v| !v.is_finite()))
    {
        return Err("coupling graph requires bounded finite matching columns".into());
    }
    let normalized = columns
        .iter()
        .map(|column| {
            let norm = vector_norm(column.iter().copied())?;
            if !norm.is_finite() {
                return Err("coupling graph has an invalid column norm".into());
            }
            Ok(column
                .iter()
                .map(|v| if norm > 0.0 { v / norm } else { 0.0 })
                .collect::<Vec<_>>())
        })
        .collect::<Result<Vec<_>, String>>()?;
    let mut pairs = Vec::new();
    for (left, a) in normalized.iter().enumerate() {
        checkpoint(SolverStage::ModalVectorScan, left)?;
        let mut neighbors = normalized
            .iter()
            .enumerate()
            .filter(|(right, _)| *right != left)
            .map(|(right, b)| Ok((dot(a, b)?.abs(), right)))
            .collect::<Result<Vec<_>, String>>()?;
        neighbors.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
        pairs.extend(
            neighbors
                .into_iter()
                .filter(|(strength, _)| *strength > 0.0)
                .take(PARTNERS)
                .map(|(_, right)| (left.min(right), left.max(right))),
        );
    }
    pairs.sort_unstable();
    pairs.dedup();
    if pairs.len() > PARTNERS * columns.len() {
        return Err("coupling graph exceeds its pair budget".into());
    }
    checkpoint(SolverStage::ModalVectorScan, normalized.len())?;
    Ok(pairs)
}

fn project(vector: &[f64], columns: &[Vec<f64>], coefficients: &[f64]) -> Result<Vec<f64>, String> {
    let mut projected = Vec::with_capacity(vector.len());
    for (i, &v) in vector.iter().enumerate() {
        projected.push(
            v - columns
                .iter()
                .zip(coefficients)
                .map(|(c, &x)| c[i] * x)
                .sum::<f64>(),
        );
        checkpoint_chunk(SolverStage::ModalVectorUpdate, i + 1, vector.len())?;
    }
    Ok(projected)
}

fn predict(
    residual: &[f64],
    a: &[f64],
    dx: f64,
    second: Option<(&[f64], f64)>,
) -> Result<Vec<f64>, String> {
    let mut predicted = Vec::with_capacity(residual.len());
    for (i, (&r, &a)) in residual.iter().zip(a).enumerate() {
        let first = (-dx).mul_add(a, r);
        predicted.push(second.map_or(first, |(b, dy)| (-dy).mul_add(b[i], first)));
        checkpoint_chunk(SolverStage::ModalVectorUpdate, i + 1, residual.len())?;
    }
    Ok(predicted)
}

#[cfg(test)]
#[path = "modal_roundoff_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "modal_roundoff_hybrid_tests.rs"]
mod hybrid_tests;

#[cfg(test)]
#[path = "modal_roundoff_wide_proposal.rs"]
mod wide;

#[cfg(test)]
#[path = "modal_roundoff_wide_tests.rs"]
mod wide_tests;

#[cfg(test)]
#[path = "modal_roundoff_joint_reference.rs"]
mod joint;

#[cfg(test)]
#[path = "modal_roundoff_joint_tests.rs"]
mod joint_tests;

#[cfg(test)]
#[path = "modal_roundoff_wide_factor.rs"]
mod wide_factor;

#[cfg(test)]
#[path = "modal_roundoff_wide_projection.rs"]
mod wide_projection;

#[cfg(test)]
#[path = "modal_roundoff_wide_projection_tests.rs"]
mod wide_projection_tests;

#[cfg(test)]
#[path = "modal_roundoff_physical_neighborhood.rs"]
mod physical_neighborhood;

#[cfg(test)]
#[path = "modal_roundoff_physical_neighborhood_tests.rs"]
mod physical_neighborhood_tests;

#[cfg(test)]
#[path = "modal_roundoff_coherent_phase.rs"]
mod coherent_phase;

#[cfg(test)]
#[path = "modal_roundoff_coherent_phase_tests.rs"]
mod coherent_phase_tests;

#[cfg(test)]
#[path = "modal_roundoff_triangular_grid.rs"]
mod triangular_grid;

#[cfg(test)]
#[path = "modal_roundoff_triangular_grid_tests.rs"]
mod triangular_grid_tests;

#[cfg(test)]
#[path = "modal_roundoff_triangular_grid_robustness_tests.rs"]
mod triangular_grid_robustness_tests;
