use super::{
    budget::Checks,
    qr::{MAX_GRID_RADIUS, QrFit},
    wide_factor::bounded,
};
use crate::modal_test_wide::Wide;
use crate::solver_control::{SolverStage, checkpoint, checkpoint_chunk};

pub(super) const PASSES: usize = 4;
pub(super) const MAX_CERTIFICATES: usize = PASSES + 2;
pub(super) const UNIT_SHAPE_TOLERANCE: f64 = 1e-10;

pub(super) struct Plan {
    pub(super) payload_bytes: usize,
    pub(super) component_visits: usize,
}

#[path = "modal_roundoff_triangular_grid_control_tests.rs"]
mod control_tests;

impl Plan {
    pub(super) fn new(size: usize) -> Result<Self, String> {
        super::budget::Plan::new(size)?;
        let payload_bytes = 48 * size * size + 2048 * size;
        let component_visits = 12 * size * size * size + 2048 * size * size;
        if payload_bytes > 8 * 1024 * 1024 || component_visits > 350_000_000 {
            return Err("triangular grid exceeds its separate proposal budget".into());
        }
        Ok(Self {
            payload_bytes,
            component_visits,
        })
    }
}

fn input(value: f64) -> bool {
    value.is_finite() && (value == 0.0 || (1e-50..=1e50).contains(&value.abs()))
}

#[derive(Clone, Copy, Debug)]
pub(super) enum Policy {
    Independent,
    InBacksolve,
}

// Test-only fixed-grid proposals; the physical operator alone certifies them.
pub(super) struct GridFit {
    seed: Vec<f64>,
    grids: Vec<f64>,
    order: Vec<usize>,
    factor: QrFit,
}

impl GridFit {
    pub(super) fn correct_unit_shape(
        &self,
        radius: usize,
        tolerance: f64,
        checked: impl FnMut(&[f64]) -> Result<(f64, Vec<f64>), String>,
    ) -> Result<Vec<f64>, String> {
        require_unit_shape(&self.seed)?;
        let candidate = self.correct(radius, Policy::InBacksolve, tolerance, checked)?;
        // Renormalizing here would invalidate the already checked physical shape.
        require_unit_shape(&candidate)?;
        checkpoint(SolverStage::ModalRoundoffValidate, 2)?;
        Ok(candidate)
    }

    pub(super) fn prepare(
        matrix: &[Vec<Wide>],
        seed: &[f64],
        anchor: usize,
        order: &[usize],
    ) -> Result<Self, String> {
        let size = matrix.len();
        Plan::new(size)?;
        if seed.len() != size
            || seed.iter().any(|&v| !input(v))
            || anchor >= size
            || order.len() != size - 1
            || matrix
                .iter()
                .any(|r| r.len() != size || r.iter().any(|&v| !bounded(v) || !input(v.rounded())))
        {
            return Err(
                "triangular grid requires bounded square data and a matching seed/anchor".into(),
            );
        }
        let mut seen = vec![false; size];
        seen[anchor] = true;
        for &i in order {
            if i >= size || seen[i] {
                return Err("triangular grid order must list every nonanchor exactly once".into());
            }
            seen[i] = true;
        }
        let grids: Vec<_> = seed.iter().map(|v| v.next_up() - v).collect();
        let mut columns = Vec::with_capacity(order.len());
        for &j in order {
            if !grids[j].is_finite() || grids[j] <= 0.0 || seed[j] == 0.0 {
                return Err("triangular grid has no resolved nonzero coordinate step".into());
            }
            let mut column = Vec::with_capacity(size);
            for (row, entries) in matrix.iter().enumerate() {
                let product = entries[j].mul(Wide::from(grids[j]));
                let rounded = product.rounded();
                if !rounded.is_finite() || (entries[j].high != 0.0 && rounded == 0.0) {
                    return Err("triangular grid column scaling lost a nonzero entry".into());
                }
                column.push(rounded);
                checkpoint_chunk(SolverStage::ModalVectorUpdate, row + 1, size)?;
            }
            columns.push(column);
        }
        let factor = QrFit::factor(&columns)?;
        Ok(Self {
            seed: seed.to_vec(),
            grids,
            order: order.to_vec(),
            factor,
        })
    }

    pub(super) fn correct(
        &self,
        radius: usize,
        policy: Policy,
        tolerance: f64,
        mut checked: impl FnMut(&[f64]) -> Result<(f64, Vec<f64>), String>,
    ) -> Result<Vec<f64>, String> {
        if !(1..=MAX_GRID_RADIUS).contains(&radius) || !tolerance.is_finite() || tolerance <= 0.0 {
            return Err("triangular grid requires a bounded positive radius and tolerance".into());
        }
        let mut checks = Checks::new();
        let mut retained = self.seed.clone();
        let (mut relative, mut residual) = self.measure(&retained, &mut checked, &mut checks)?;
        for pass in 0..PASSES {
            if relative <= tolerance {
                break;
            }
            checkpoint(SolverStage::ModalIteration, pass)?;
            let rhs: Vec<_> = residual.iter().map(|v| -v).collect();
            let decisions = match policy {
                Policy::InBacksolve => self.factor.quantized_solve(&rhs, radius)?,
                Policy::Independent => self
                    .factor
                    .solve(&rhs)?
                    .into_iter()
                    .map(|v| v.round().clamp(-(radius as f64), radius as f64))
                    .collect(),
            };
            let mut candidate = retained.clone();
            for (visit, (&i, &decision)) in self.order.iter().zip(&decisions).enumerate() {
                candidate[i] = Wide::from(retained[i])
                    .add(Wide::from(self.grids[i]).mul(Wide::from(decision)))
                    .rounded();
                if !input(candidate[i]) {
                    return Err("triangular grid candidate exceeds its range".into());
                }
                checkpoint_chunk(SolverStage::ModalVectorUpdate, visit + 1, self.order.len())?;
            }
            let (next, refreshed) = self.measure(&candidate, &mut checked, &mut checks)?;
            if next >= relative {
                break;
            }
            retained = candidate;
            relative = next;
            residual = refreshed;
        }
        let (relative, _) = self.measure(&retained, &mut checked, &mut checks)?;
        if relative > tolerance {
            return Err(format!(
                "triangular grid did not reach its unchanged residual gate (relative={relative:e})"
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
    ) -> Result<(f64, Vec<f64>), String> {
        checks.next()?;
        let result = checked(vector)?;
        if !result.0.is_finite()
            || result.0 < 0.0
            || result.1.len() != self.seed.len()
            || result.1.iter().any(|&v| !input(v))
        {
            return Err("triangular grid requires a finite matching certificate".into());
        }
        Ok(result)
    }
}

fn require_unit_shape(shape: &[f64]) -> Result<(), String> {
    let norm = crate::modal_math::checked_shape_norm(shape)?;
    if (norm - 1.0).abs() >= UNIT_SHAPE_TOLERANCE {
        return Err(format!(
            "triangular grid unit shape exceeds its unchanged norm gate (norm={norm:e})"
        ));
    }
    Ok(())
}
