use super::*;

#[path = "modal_roundoff_grid_iterated_beam_control_tests.rs"]
mod controls;

impl Plan {
    pub(in super::super) fn iterated_beam(
        size: usize,
        width: usize,
        passes: usize,
    ) -> Result<Self, String> {
        if !(1..=4).contains(&passes)
            || !(1..=MAX_BEAM_WIDTH).contains(&width)
            || width * passes > MAX_BEAM_WIDTH
        {
            return Err("iterated beam requires 1..=4 passes and at most 64 total branches".into());
        }
        let mut plan = Self::rounded_beam(size, width * passes)?;
        plan.payload_bytes += 48 * size;
        plan.component_visits += 64 * size * size + 128 * size * (PASSES + 3);
        Ok(plan)
    }
}

impl GridFit {
    pub(in super::super) fn attempt_direction_iterated_hybrid(
        &self,
        radius: usize,
        width: usize,
        passes: usize,
        tolerance: f64,
        mut checked: impl FnMut(&[f64]) -> Result<(f64, Vec<f64>), String>,
    ) -> Result<Attempt, String> {
        Plan::iterated_beam(self.seed.len(), width, passes)?;
        if !matches!(self.factor, Factor::Rounded(_)) {
            return Err("iterated beam requires its preselected rounded factor".into());
        }
        if !(0.25..=2.0).contains(&super::super::vector_norm(self.seed.iter().copied())?) {
            return Err("iterated beam requires unchanged direction norm bounds".into());
        }
        let mut best = f64::INFINITY;
        let mut retained = self.seed.clone();
        let mut eligible = false;
        let mut calls = 0;
        let limit = MAX_CERTIFICATES + width * passes + 2;
        let mut measure = |v: &[f64]| {
            if calls >= limit {
                return Err("iterated hybrid exhausted its aggregate receipt budget".into());
            }
            checkpoint(SolverStage::ModalRoundoffSearch, calls)?;
            calls += 1;
            let receipt = checked(v)?;
            validate_certificate(self.seed.len(), &receipt)?;
            eligible |= receipt.0 <= tolerance;
            if receipt.0 < best {
                best = receipt.0;
                retained = v.to_vec();
            }
            Ok(receipt)
        };
        let old = self.attempt_direction(radius, tolerance, &mut measure)?;
        if matches!(
            old,
            Attempt::Accepted(_) | Attempt::Rejected(Rejection::UnitNorm(_))
        ) {
            return Ok(old);
        }
        if eligible {
            return Err("iterated hybrid lost a passing legacy final receipt".into());
        }
        let offsets: Vec<_> = self
            .order
            .iter()
            .map(|&i| {
                Wide::from(retained[i])
                    .sub(Wide::from(self.seed[i]))
                    .div(Wide::from(self.grids[i]))
                    .rounded()
            })
            .collect();
        for (&i, &offset) in self.order.iter().zip(&offsets) {
            if !offset.is_finite()
                || offset.fract() != 0.0
                || offset.abs() > radius as f64
                || Wide::from(self.seed[i])
                    .add(Wide::from(self.grids[i]).mul(Wide::from(offset)))
                    .rounded()
                    .to_bits()
                    != retained[i].to_bits()
            {
                return Err("iterated hybrid legacy center left its frozen grid radius".into());
            }
        }
        self.iterated_from(offsets, radius, width, passes, tolerance, |v| {
            if calls >= limit {
                return Err("iterated hybrid exhausted its aggregate receipt budget".into());
            }
            checkpoint(SolverStage::ModalRoundoffSearch, calls)?;
            calls += 1;
            checked(v)
        })
    }

    // Refresh the actual residual, but retain the original grid, factor and radius.
    pub(in super::super) fn attempt_iterated_beam(
        &self,
        radius: usize,
        width: usize,
        passes: usize,
        tolerance: f64,
        mut checked: impl FnMut(&[f64]) -> Result<(f64, Vec<f64>), String>,
    ) -> Result<Attempt, String> {
        self.iterated_from(
            vec![0.0; self.order.len()],
            radius,
            width,
            passes,
            tolerance,
            &mut checked,
        )
    }

    fn iterated_from(
        &self,
        mut offsets: Vec<f64>,
        radius: usize,
        width: usize,
        passes: usize,
        tolerance: f64,
        mut checked: impl FnMut(&[f64]) -> Result<(f64, Vec<f64>), String>,
    ) -> Result<Attempt, String> {
        Plan::iterated_beam(self.seed.len(), width, passes)?;
        let Factor::Rounded(factor) = &self.factor else {
            return Err("iterated beam requires its preselected rounded factor".into());
        };
        if !(1..=MAX_GRID_RADIUS).contains(&radius) || !tolerance.is_finite() || tolerance <= 0.0 {
            return Err("iterated beam requires a bounded radius and tolerance".into());
        }
        if !(0.25..=2.0).contains(&super::super::vector_norm(self.seed.iter().copied())?) {
            return Err("iterated beam requires unchanged direction norm bounds".into());
        }
        let mut checks = Checks::new();
        let mut retained = self.seed.clone();
        for (&i, &offset) in self.order.iter().zip(&offsets) {
            retained[i] = Wide::from(self.seed[i])
                .add(Wide::from(self.grids[i]).mul(Wide::from(offset)))
                .rounded();
        }
        let (mut relative, mut residual) = self.measure(&retained, &mut checked, &mut checks)?;
        for pass in 0..passes {
            if relative <= tolerance {
                break;
            }
            checkpoint(SolverStage::ModalIteration, pass)?;
            let rhs: Vec<_> = residual.iter().map(|v| -v).collect();
            let proposals = factor.quantized_beam(&rhs, radius, width)?;
            if proposals.len() > width || proposals.iter().any(|v| v.len() != self.order.len()) {
                return Err("iterated beam exceeded its fixed proposal budget".into());
            }
            let base = offsets.clone();
            let before = relative;
            for decisions in proposals {
                let next: Vec<_> = base.iter().zip(decisions).map(|(&a, b)| a + b).collect();
                if next.iter().any(|v| v.abs() > radius as f64) {
                    continue;
                }
                let mut candidate = self.seed.clone();
                for (position, (&i, &offset)) in self.order.iter().zip(&next).enumerate() {
                    candidate[i] = Wide::from(self.seed[i])
                        .add(Wide::from(self.grids[i]).mul(Wide::from(offset)))
                        .rounded();
                    if !input(candidate[i]) {
                        return Err("iterated beam candidate exceeds its range".into());
                    }
                    checkpoint_chunk(
                        SolverStage::ModalVectorUpdate,
                        position + 1,
                        self.order.len(),
                    )?;
                }
                let (actual, refreshed) = self.measure(&candidate, &mut checked, &mut checks)?;
                if actual < relative {
                    retained = candidate;
                    relative = actual;
                    residual = refreshed;
                    offsets = next;
                }
            }
            if relative >= before {
                break;
            }
        }
        let (actual, _) = self.measure(&retained, &mut checked, &mut checks)?;
        if actual > tolerance {
            return Ok(Attempt::Rejected(Rejection::Residual(actual)));
        }
        if !(0.25..=2.0).contains(&super::super::vector_norm(retained.iter().copied())?) {
            return Err("iterated beam changed its unchanged direction norm bounds".into());
        }
        checkpoint(SolverStage::ModalRoundoffValidate, 8)?;
        Ok(Attempt::Accepted(retained))
    }
}
