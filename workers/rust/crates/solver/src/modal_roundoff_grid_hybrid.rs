use super::*;

#[path = "modal_roundoff_grid_hybrid_control_tests.rs"]
mod controls;

fn measure(
    size: usize,
    limit: usize,
    calls: &mut usize,
    shape: &[f64],
    checked: &mut impl FnMut(&[f64]) -> Result<(f64, Vec<f64>), String>,
) -> Result<(f64, Vec<f64>), String> {
    if *calls >= limit {
        return Err("hybrid grid exhausted its aggregate receipt budget".into());
    }
    checkpoint(SolverStage::ModalRoundoffSearch, *calls)?;
    *calls += 1;
    let result = checked(shape)?;
    validate_certificate(size, &result)?;
    Ok(result)
}

impl Plan {
    pub(super) fn rounded_hybrid(size: usize, width: usize) -> Result<Self, String> {
        let mut plan = Self::rounded_beam(size, width)?;
        plan.payload_bytes += 16 * size;
        plan.component_visits += 64 * size * size + 128 * size * (PASSES + 3);
        if plan.payload_bytes > 8 * 1024 * 1024 || plan.component_visits > 650_000_000 {
            return Err("hybrid grid exceeds its separate proposal budget".into());
        }
        Ok(plan)
    }
}

impl GridFit {
    // Numerical rejection alone may use the beam, reusing the same frozen factor.
    pub(in crate::modal_frame_spectrum::refinement::roundoff) fn attempt_unit_hybrid(
        &self,
        radius: usize,
        width: usize,
        tolerance: f64,
        mut checked: impl FnMut(&[f64]) -> Result<(f64, Vec<f64>), String>,
    ) -> Result<Attempt, String> {
        Plan::rounded_hybrid(self.seed.len(), width)?;
        if !(1..=MAX_GRID_RADIUS).contains(&radius)
            || !tolerance.is_finite()
            || tolerance <= 0.0
            || !matches!(self.factor, Factor::Rounded(_))
        {
            return Err("hybrid grid requires its bounded preselected rounded factor".into());
        }
        require_unit_shape(&self.seed)?;
        let size = self.seed.len();
        let limit = MAX_CERTIFICATES + width + 3;
        let anchor = (0..size)
            .find(|i| !self.order.contains(i))
            .ok_or("hybrid grid requires its frozen anchor")?;
        let mut calls = 0;
        let mut eligible = false;
        let greedy = self.attempt_unit_shape(radius, tolerance, |shape| {
            let result = measure(size, limit, &mut calls, shape, &mut checked)?;
            eligible |= result.0 <= tolerance;
            Ok(result)
        })?;
        let chosen = match greedy {
            Attempt::Rejected(Rejection::Residual(_)) if eligible => {
                return Err("hybrid grid lost its greedy final physical certificate".into());
            }
            Attempt::Rejected(Rejection::Residual(_)) => {
                checkpoint(SolverStage::ModalIteration, PASSES)?;
                self.attempt_rounded_unit_beam(radius, width, tolerance, |shape| {
                    measure(size, limit, &mut calls, shape, &mut checked)
                })?
            }
            other => other,
        };
        let Attempt::Accepted(shape) = chosen else {
            return Ok(chosen);
        };
        let final_receipt = measure(size, limit, &mut calls, &shape, &mut checked)?;
        if final_receipt.0 > tolerance {
            return Err("hybrid grid lost its final physical certificate".into());
        }
        require_unit_shape(&shape)?;
        if shape[anchor].to_bits() != self.seed[anchor].to_bits() {
            return Err("hybrid grid changed its frozen physical anchor".into());
        }
        checkpoint(SolverStage::ModalRoundoffValidate, 7)?;
        Ok(Attempt::Accepted(shape))
    }
}
