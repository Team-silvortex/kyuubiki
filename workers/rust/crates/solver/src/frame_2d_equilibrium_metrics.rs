use crate::linear_algebra::SparseMatrix;
use crate::solver_control::{SolverStage, checkpoint_chunk};

pub(crate) struct EquilibriumMetric {
    rows: Vec<RowScale>,
    reference_norm: f64,
    load_scale: f64,
}

struct RowScale {
    peak: f64,
    sum: f64,
}

impl RowScale {
    fn add(&mut self, value: f64) {
        if value > self.peak {
            self.sum = self.sum * (self.peak / value) + 1.0;
            self.peak = value;
        } else {
            self.sum += value / self.peak;
        }
    }
}

impl EquilibriumMetric {
    pub(crate) fn for_increment(
        tangent: &SparseMatrix,
        displacement: &[f64],
        reference: &[f64],
        load_factor: f64,
        free: &[usize],
        increment: &[f64],
    ) -> Result<Self, String> {
        if increment.len() != free.len() {
            return Err("frame 2d equilibrium increment dimensions differ".into());
        }
        let mut amplitudes = displacement
            .iter()
            .map(|value| value.abs())
            .collect::<Vec<_>>();
        for (&dof, &delta) in free.iter().zip(increment) {
            let Some(amplitude) = amplitudes.get_mut(dof) else {
                return Err("frame 2d equilibrium increment DOF is out of bounds".into());
            };
            *amplitude += delta.abs();
        }
        // The Newton predictor supplies a scale for initially unloaded rows.
        // Freeze it before backtracking, never recompute it from trial states.
        Self::new(tangent, &amplitudes, reference, load_factor, free)
    }

    pub(crate) fn new(
        tangent: &SparseMatrix,
        displacement: &[f64],
        reference: &[f64],
        load_factor: f64,
        free: &[usize],
    ) -> Result<Self, String> {
        if tangent.size() != displacement.len() || displacement.len() != reference.len() {
            return Err("frame 2d equilibrium metric dimensions differ".into());
        }
        if !load_factor.is_finite()
            || displacement
                .iter()
                .chain(reference)
                .any(|value| !value.is_finite())
        {
            return Err("frame 2d equilibrium metric input is non-finite".into());
        }
        let load_scale = load_factor.abs().max(1.0);
        let mut reference_norm = 1.0_f64;
        let mut rows = Vec::with_capacity(free.len());
        for (index, &dof) in free.iter().enumerate() {
            let Some(force) = reference.get(dof) else {
                return Err("frame 2d equilibrium metric free DOF is out of bounds".into());
            };
            reference_norm = reference_norm.max(force.abs());
            let mut row = RowScale {
                peak: force.abs().max(1.0),
                sum: 1.0,
            };
            // K_ij u_j has the units of this equation, including moment rows.
            // Keep its absolute terms: they retain a local cancellation scale
            // during unloading, without borrowing loads from unrelated rows.
            for &(column, coefficient) in tangent.row_entries(dof) {
                let term = coefficient * displacement[column];
                if !term.is_finite() {
                    return Err(format!(
                        "frame 2d equilibrium metric DOF {dof} tangent contribution is non-finite"
                    ));
                }
                row.add(term.abs() / load_scale);
            }
            rows.push(row);
            checkpoint_chunk(SolverStage::ResidualValidate, index + 1, free.len())?;
        }
        Ok(Self {
            rows,
            reference_norm,
            load_scale,
        })
    }

    pub(crate) fn norm(&self, residual: &[f64]) -> f64 {
        if residual.len() != self.rows.len() || residual.iter().any(|value| !value.is_finite()) {
            return f64::INFINITY;
        }
        // This is an additional guard, never a relaxation of the existing
        // free-load infinity norm. Reuse the same metric for every line-search
        // trial so growing the trial displacement cannot buy a smaller error.
        residual
            .iter()
            .zip(&self.rows)
            .fold(0.0_f64, |norm, (residual, row)| {
                let global = (residual.abs() / self.reference_norm) / self.load_scale;
                let local = ((residual.abs() / self.load_scale) / row.peak) / row.sum;
                norm.max(global).max(local)
            })
    }
}

#[cfg(test)]
#[path = "frame_2d_equilibrium_tests.rs"]
mod equilibrium_tests;

#[cfg(test)]
#[path = "frame_2d_equilibrium_row_tests.rs"]
mod row_tests;
