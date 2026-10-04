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

#[path = "modal_roundoff_grid_wide_qr.rs"]
mod wide_qr;

#[path = "modal_roundoff_grid_wide_beam_control_tests.rs"]
mod beam_control_tests;

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

    pub(super) fn wide(size: usize) -> Result<Self, String> {
        let mut plan = Self::new(size)?;
        plan.payload_bytes += 48 * size * size;
        if plan.payload_bytes > 8 * 1024 * 1024 {
            return Err("wide triangular grid exceeds its separate payload budget".into());
        }
        Ok(plan)
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

#[derive(Debug)]
pub(super) enum Rejection {
    Residual(f64),
    UnitNorm(f64),
}

#[derive(Debug)]
pub(super) enum Attempt {
    Accepted(Vec<f64>),
    Rejected(Rejection),
}

impl Attempt {
    fn into_result(self) -> Result<Vec<f64>, String> {
        match self {
            Self::Accepted(candidate) => Ok(candidate),
            Self::Rejected(Rejection::Residual(relative)) => Err(format!(
                "triangular grid did not reach its unchanged residual gate (relative={relative:e})"
            )),
            Self::Rejected(Rejection::UnitNorm(norm)) => Err(format!(
                "triangular grid unit shape exceeds its unchanged norm gate (norm={norm:e})"
            )),
        }
    }
}

// Test-only fixed-grid proposals; the physical operator alone certifies them.
pub(super) struct GridFit {
    seed: Vec<f64>,
    grids: Vec<f64>,
    order: Vec<usize>,
    factor: Factor,
}

enum Factor {
    Rounded(QrFit),
    Wide(wide_qr::QrFit),
}

impl GridFit {
    pub(super) fn correct_unit_shape(
        &self,
        radius: usize,
        tolerance: f64,
        checked: impl FnMut(&[f64]) -> Result<(f64, Vec<f64>), String>,
    ) -> Result<Vec<f64>, String> {
        self.attempt_unit_shape(radius, tolerance, checked)?
            .into_result()
    }

    pub(super) fn attempt_unit_shape(
        &self,
        radius: usize,
        tolerance: f64,
        checked: impl FnMut(&[f64]) -> Result<(f64, Vec<f64>), String>,
    ) -> Result<Attempt, String> {
        require_unit_shape(&self.seed)?;
        let candidate = match self.attempt(radius, Policy::InBacksolve, tolerance, checked)? {
            Attempt::Accepted(candidate) => candidate,
            rejected => return Ok(rejected),
        };
        // Renormalizing here would invalidate the already checked physical shape.
        let norm = crate::modal_math::checked_shape_norm(&candidate)?;
        if (norm - 1.0).abs() >= UNIT_SHAPE_TOLERANCE {
            return Ok(Attempt::Rejected(Rejection::UnitNorm(norm)));
        }
        checkpoint(SolverStage::ModalRoundoffValidate, 2)?;
        Ok(Attempt::Accepted(candidate))
    }

    pub(super) fn attempt_direction(
        &self,
        radius: usize,
        tolerance: f64,
        checked: impl FnMut(&[f64]) -> Result<(f64, Vec<f64>), String>,
    ) -> Result<Attempt, String> {
        self.attempt(radius, Policy::InBacksolve, tolerance, checked)
    }

    pub(super) fn attempt_wide_beam(
        &self,
        radius: usize,
        tolerance: f64,
        mut checked: impl FnMut(&[f64]) -> Result<(f64, Vec<f64>), String>,
    ) -> Result<Attempt, String> {
        let Factor::Wide(factor) = &self.factor else {
            return Err("triangular beam requires a wide factor".into());
        };
        if !(1..=MAX_GRID_RADIUS).contains(&radius) || !tolerance.is_finite() || tolerance <= 0.0 {
            return Err("triangular beam requires a bounded positive radius and tolerance".into());
        }
        let mut checks = Checks::new();
        let mut retained = self.seed.clone();
        let (mut relative, residual) = self.measure(&retained, &mut checked, &mut checks)?;
        if relative > tolerance {
            let rhs: Vec<_> = residual.iter().map(|v| -v).collect();
            let proposals = factor.beam_solve(&rhs, radius)?;
            if proposals.len() > MAX_CERTIFICATES - 2
                || proposals.iter().any(|p| p.len() != self.order.len())
            {
                return Err("triangular beam exceeds its fixed proposal budget".into());
            }
            for decisions in proposals {
                let mut candidate = self.seed.clone();
                for (visit, (&i, &decision)) in self.order.iter().zip(&decisions).enumerate() {
                    candidate[i] = Wide::from(self.seed[i])
                        .add(Wide::from(self.grids[i]).mul(Wide::from(decision)))
                        .rounded();
                    if !input(candidate[i]) {
                        return Err("triangular beam candidate exceeds its range".into());
                    }
                    checkpoint_chunk(SolverStage::ModalVectorUpdate, visit + 1, self.order.len())?;
                }
                let (next, _) = self.measure(&candidate, &mut checked, &mut checks)?;
                if next < relative {
                    retained = candidate;
                    relative = next;
                }
            }
        }
        let (relative, _) = self.measure(&retained, &mut checked, &mut checks)?;
        if relative > tolerance {
            return Ok(Attempt::Rejected(Rejection::Residual(relative)));
        }
        checkpoint(SolverStage::ModalRoundoffValidate, 1)?;
        Ok(Attempt::Accepted(retained))
    }

    pub(super) fn prepare(
        matrix: &[Vec<Wide>],
        seed: &[f64],
        anchor: usize,
        order: &[usize],
    ) -> Result<Self, String> {
        let grids = validated_grids(matrix, seed, anchor, order)?;
        let size = matrix.len();
        let mut columns = Vec::with_capacity(order.len());
        for &j in order {
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
        let factor = Factor::Rounded(QrFit::factor(&columns)?);
        Ok(Self {
            seed: seed.to_vec(),
            grids,
            order: order.to_vec(),
            factor,
        })
    }

    pub(super) fn prepare_wide(
        matrix: &[Vec<Wide>],
        seed: &[f64],
        anchor: usize,
        order: &[usize],
        rounded_columns: bool,
    ) -> Result<Self, String> {
        Plan::wide(matrix.len())?;
        let grids = validated_grids(matrix, seed, anchor, order)?;
        let size = matrix.len();
        let mut columns = Vec::with_capacity(order.len());
        for &j in order {
            let mut column = Vec::with_capacity(size);
            for (row, entries) in matrix.iter().enumerate() {
                let product = entries[j].mul(Wide::from(grids[j]));
                if !bounded(product) || (entries[j].high != 0.0 && product.high == 0.0) {
                    return Err("wide triangular grid lost a bounded nonzero column entry".into());
                }
                column.push(if rounded_columns {
                    Wide::from(product.rounded())
                } else {
                    product
                });
                checkpoint_chunk(SolverStage::ModalVectorUpdate, row + 1, size)?;
            }
            columns.push(column);
        }
        let factor = Factor::Wide(wide_qr::QrFit::factor(&columns)?);
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
        checked: impl FnMut(&[f64]) -> Result<(f64, Vec<f64>), String>,
    ) -> Result<Vec<f64>, String> {
        self.attempt(radius, policy, tolerance, checked)?
            .into_result()
    }

    fn attempt(
        &self,
        radius: usize,
        policy: Policy,
        tolerance: f64,
        mut checked: impl FnMut(&[f64]) -> Result<(f64, Vec<f64>), String>,
    ) -> Result<Attempt, String> {
        if !(1..=MAX_GRID_RADIUS).contains(&radius) || !tolerance.is_finite() || tolerance <= 0.0 {
            return Err("triangular grid requires a bounded positive radius and tolerance".into());
        }
        if matches!(
            (&self.factor, policy),
            (Factor::Wide(_), Policy::Independent)
        ) {
            return Err("wide triangular grid supports only in-backsolve quantization".into());
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
            let decisions = match (&self.factor, policy) {
                (Factor::Rounded(factor), Policy::InBacksolve) => {
                    factor.quantized_solve(&rhs, radius)?
                }
                (Factor::Rounded(factor), Policy::Independent) => factor
                    .solve(&rhs)?
                    .into_iter()
                    .map(|v| v.round().clamp(-(radius as f64), radius as f64))
                    .collect(),
                (Factor::Wide(factor), Policy::InBacksolve) => {
                    factor.quantized_solve(&rhs, radius)?
                }
                (Factor::Wide(_), Policy::Independent) => {
                    return Err(
                        "wide triangular grid supports only in-backsolve quantization".into(),
                    );
                }
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
            return Ok(Attempt::Rejected(Rejection::Residual(relative)));
        }
        checkpoint(SolverStage::ModalRoundoffValidate, 1)?;
        Ok(Attempt::Accepted(retained))
    }

    fn measure(
        &self,
        vector: &[f64],
        checked: &mut impl FnMut(&[f64]) -> Result<(f64, Vec<f64>), String>,
        checks: &mut Checks,
    ) -> Result<(f64, Vec<f64>), String> {
        checks.next()?;
        let result = checked(vector)?;
        validate_certificate(self.seed.len(), &result)?;
        Ok(result)
    }
}

fn validated_grids(
    matrix: &[Vec<Wide>],
    seed: &[f64],
    anchor: usize,
    order: &[usize],
) -> Result<Vec<f64>, String> {
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
    for &j in order {
        if !grids[j].is_finite() || grids[j] <= 0.0 || seed[j] == 0.0 {
            return Err("triangular grid has no resolved nonzero coordinate step".into());
        }
    }
    Ok(grids)
}

pub(super) fn validate_certificate(size: usize, result: &(f64, Vec<f64>)) -> Result<(), String> {
    if !result.0.is_finite()
        || result.0 < 0.0
        || result.1.len() != size
        || result.1.iter().any(|&v| !input(v))
    {
        return Err("triangular grid requires a finite matching certificate".into());
    }
    Ok(())
}

pub(super) fn require_unit_shape(shape: &[f64]) -> Result<(), String> {
    let norm = crate::modal_math::checked_shape_norm(shape)?;
    if (norm - 1.0).abs() >= UNIT_SHAPE_TOLERANCE {
        return Err(format!(
            "triangular grid unit shape exceeds its unchanged norm gate (norm={norm:e})"
        ));
    }
    Ok(())
}
