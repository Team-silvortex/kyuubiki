use super::{
    BlockFit,
    budget::{Checks, OUTER_STEPS, PARTNERS},
    coupled_pairs, dot, vector_norm,
};
use crate::modal_test_wide::Wide;
use crate::solver_control::{SolverStage, checkpoint, checkpoint_chunk};

pub(super) struct Plan {
    pub(super) groups: usize,
    pub(super) payload_bytes: usize,
    pub(super) component_visits: usize,
}

impl Plan {
    pub(super) fn new(size: usize) -> Result<Self, String> {
        let base = super::budget::Plan::new(size)?;
        let square = size * size;
        // Additional normalized graph storage, nominations, triples and scratch.
        let payload_bytes = base.payload_bytes + square * 8 + size * 512;
        // Four forward/reverse sweeps, <=4*n groups, <=26 nontrivial trials,
        // residual copies, three updates and scoring, plus graph bookkeeping.
        let component_visits = base.component_visits + 5000 * square;
        if payload_bytes > 8 * 1024 * 1024 || component_visits > 700_000_000 {
            return Err("joint correction exceeds its separate proposal-work budget".into());
        }
        Ok(Self {
            groups: PARTNERS * size,
            payload_bytes,
            component_visits,
        })
    }
}

// Test-only direction construction. It is not a numerical acceptance oracle.
pub(super) fn physical_columns(
    matrix: &[Vec<f64>],
    mass: &[f64],
    value: f64,
) -> Result<Vec<Vec<f64>>, String> {
    super::budget::Plan::new(matrix.len())?;
    let size = matrix.len();
    if mass.len() != size
        || !value.is_finite()
        || !(1e-100..=1e100).contains(&value)
        || mass
            .iter()
            .any(|v| !v.is_finite() || !(1e-100..=1e100).contains(v))
        || matrix
            .iter()
            .any(|r| r.len() != size || r.iter().any(|v| !v.is_finite() || v.abs() > 1e100))
    {
        return Err("joint physical directions require bounded finite positive data".into());
    }
    for i in 0..size {
        for j in 0..i {
            if matrix[i][j] != matrix[j][i] {
                return Err(
                    "joint physical directions require symmetric physical stiffness".into(),
                );
            }
        }
    }
    let mut shifted = vec![vec![0.0; size]; size];
    for (i, row) in matrix.iter().enumerate() {
        let inverse = Wide::from(mass[i].sqrt().recip());
        for (j, &a) in row.iter().enumerate() {
            let mut coefficient = Wide::from(a);
            if i == j {
                coefficient = coefficient.sub(Wide::from(value).mul(Wide::from(mass[i])));
            }
            shifted[i][j] = coefficient.mul(inverse).rounded();
            if a != 0.0 && i != j && shifted[i][j] == 0.0 {
                return Err("joint physical direction lost a nonzero coupling".into());
            }
            checkpoint_chunk(SolverStage::ModalVectorUpdate, j + 1, size)?;
        }
    }
    Ok(shifted)
}

pub(super) fn triples(columns: &[Vec<f64>]) -> Result<Vec<[usize; 3]>, String> {
    let pairs = coupled_pairs(columns)?;
    let normalized: Vec<Vec<_>> = columns
        .iter()
        .map(|c| {
            let norm = vector_norm(c.iter().copied())?;
            Ok(c.iter()
                .map(|v| if norm > 0.0 { v / norm } else { 0.0 })
                .collect())
        })
        .collect::<Result<_, String>>()?;
    let mut nominations = Vec::with_capacity(6 * columns.len());
    for (root, column) in normalized.iter().enumerate() {
        let mut neighbors = pairs
            .iter()
            .filter_map(|&(a, b)| {
                if a == root {
                    Some(b)
                } else if b == root {
                    Some(a)
                } else {
                    None
                }
            })
            .map(|i| Ok((dot(column, &normalized[i])?.abs(), i)))
            .collect::<Result<Vec<_>, String>>()?;
        neighbors.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
        neighbors.truncate(PARTNERS);
        for a in 0..neighbors.len() {
            for b in a + 1..neighbors.len() {
                let mut indices = [root, neighbors[a].1, neighbors[b].1];
                indices.sort_unstable();
                nominations.push((neighbors[a].0 + neighbors[b].0, indices));
            }
        }
        checkpoint(SolverStage::ModalVectorScan, root)?;
    }
    nominations.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    let mut retained = Vec::with_capacity(PARTNERS * columns.len());
    for (visit, (_, indices)) in nominations.into_iter().enumerate() {
        if !retained.contains(&indices) {
            retained.push(indices);
        }
        checkpoint_chunk(SolverStage::ModalVectorScan, visit + 1, 6 * columns.len())?;
        if retained.len() == PARTNERS * columns.len() {
            break;
        }
    }
    retained.sort_unstable();
    checkpoint(SolverStage::ModalVectorScan, columns.len())?;
    Ok(retained)
}

pub(super) fn correct(
    fit: &BlockFit,
    vector: &[f64],
    tolerance: f64,
    mut checked: impl FnMut(&[f64]) -> Result<(f64, Vec<f64>), String>,
) -> Result<Vec<f64>, String> {
    let plan = Plan::new(fit.fine.len() + fit.coarse.len())?;
    if vector.len() != fit.fine.len() + fit.coarse.len()
        || vector.iter().any(|v| !v.is_finite())
        || !tolerance.is_finite()
        || tolerance <= 0.0
    {
        return Err("joint correction requires matching finite data and positive tolerance".into());
    }
    let groups = triples(&fit.projected)?;
    if groups.len() > plan.groups {
        return Err("joint correction exceeds its triple budget".into());
    }
    let mut checks = Checks::new();
    let mut retained = vector.to_vec();
    let (mut relative, _) = fit.measure(&retained, &mut checked, &mut checks)?;
    for pass in 0..OUTER_STEPS {
        if relative <= tolerance {
            break;
        }
        checkpoint(SolverStage::ModalIteration, pass)?;
        fit.fit_fine(&mut retained, &mut checked, &mut checks)?;
        let (before, residual) = fit.measure(&retained, &mut checked, &mut checks)?;
        if before <= tolerance {
            break;
        }
        let coefficients = fit.project_coefficients(&residual)?;
        let mut residual = super::project(&residual, &fit.columns, &coefficients)?;
        let mut candidate = retained.clone();
        fit.single_sweep(&mut candidate, &mut residual)?;
        fit.pair_sweep(&mut candidate, &mut residual)?;
        sweep(fit, &groups, &mut candidate, &mut residual)?;
        fit.fit_fine(&mut candidate, &mut checked, &mut checks)?;
        let (after, _) = fit.measure(&candidate, &mut checked, &mut checks)?;
        if after >= before {
            break;
        }
        retained = candidate;
        relative = after;
    }
    let (relative, _) = fit.measure(&retained, &mut checked, &mut checks)?;
    if relative > tolerance {
        return Err(format!(
            "joint correction did not reach its unchanged residual gate (relative={relative:e})"
        ));
    }
    checkpoint(SolverStage::ModalRoundoffValidate, 1)?;
    Ok(retained)
}

pub(super) fn sweep(
    fit: &BlockFit,
    groups: &[[usize; 3]],
    vector: &mut [f64],
    residual: &mut [f64],
) -> Result<(), String> {
    let size = residual.len();
    for group in groups.iter().chain(groups.iter().rev()) {
        let coordinates = group.map(|i| fit.coarse[i]);
        let old = coordinates.map(|i| vector[i]);
        let options = old.map(|v| [v, v.next_up(), v.next_down()]);
        let mut best = dot(residual, residual)?;
        let mut update = old;
        for &a in &options[0] {
            for &b in &options[1] {
                for &c in &options[2] {
                    let candidate = [a, b, c];
                    if candidate == old || candidate.iter().any(|v| !v.is_finite()) {
                        continue;
                    }
                    let mut predicted = residual.to_vec();
                    for (i, (&previous, &next)) in old.iter().zip(&candidate).enumerate() {
                        let change = previous - next;
                        for (j, (r, &direction)) in predicted
                            .iter_mut()
                            .zip(&fit.projected[group[i]])
                            .enumerate()
                        {
                            *r = (-change).mul_add(direction, *r);
                            checkpoint_chunk(SolverStage::ModalVectorUpdate, j + 1, size)?;
                        }
                    }
                    let score = dot(&predicted, &predicted)?;
                    if score < best {
                        best = score;
                        update = candidate;
                    }
                }
            }
        }
        for (i, (&previous, &next)) in old.iter().zip(&update).enumerate() {
            let change = previous - next;
            for (j, (r, &direction)) in residual
                .iter_mut()
                .zip(&fit.projected[group[i]])
                .enumerate()
            {
                *r = (-change).mul_add(direction, *r);
                checkpoint_chunk(SolverStage::ModalVectorUpdate, j + 1, size)?;
            }
            vector[coordinates[i]] = next;
        }
    }
    Ok(())
}
