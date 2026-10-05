use super::*;

#[path = "modal_roundoff_grid_coupled_control_tests.rs"]
mod controls;

#[derive(Debug)]
pub(in crate::modal_frame_spectrum::refinement::roundoff) struct Receipt {
    pub(in crate::modal_frame_spectrum::refinement::roundoff) internal: f64,
    pub(in crate::modal_frame_spectrum::refinement::roundoff) mapped: f64,
    pub(in crate::modal_frame_spectrum::refinement::roundoff) residual: Vec<f64>,
}

fn measure(
    size: usize,
    shape: &[f64],
    checks: &mut Checks,
    checked: &mut impl FnMut(&[f64]) -> Result<Receipt, String>,
) -> Result<Receipt, String> {
    checks.next()?;
    let receipt = checked(shape)?;
    validate_receipt(size, receipt.internal, &receipt.residual)?;
    if !receipt.mapped.is_finite() || receipt.mapped < 0.0 {
        return Err("coupled grid requires a finite nonnegative mapped certificate".into());
    }
    Ok(receipt)
}

impl GridFit {
    // Mapped physical error ranks eligible internal proposals, not published results.
    pub(in crate::modal_frame_spectrum::refinement::roundoff) fn attempt_rounded_coupled(
        &self,
        radius: usize,
        width: usize,
        tolerance: f64,
        mut checked: impl FnMut(&[f64]) -> Result<Receipt, String>,
    ) -> Result<Attempt, String> {
        Plan::rounded_joint(self.seed.len(), width)?;
        if !(1..=MAX_GRID_RADIUS).contains(&radius)
            || !tolerance.is_finite()
            || tolerance <= 0.0
            || !matches!(self.factor, Factor::Rounded(_))
        {
            return Err("coupled grid requires its bounded preselected rounded factor".into());
        }
        if !(0.25..=2.0).contains(&super::super::vector_norm(self.seed.iter().copied())?) {
            return Err("coupled grid requires unchanged direction norm bounds".into());
        }
        let mut checks = Checks::new();
        let initial = measure(self.seed.len(), &self.seed, &mut checks, &mut checked)?;
        let mut best = initial.internal;
        let mut fallback = self.seed.clone();
        let mut retained = (initial.internal <= tolerance).then(|| self.seed.clone());
        let mut score = (initial.mapped, initial.internal);
        if retained.is_none() {
            self.visit_rounded_beam(&initial.residual, radius, width, |candidate| {
                let receipt = measure(self.seed.len(), &candidate, &mut checks, &mut checked)?;
                if receipt.internal < best {
                    best = receipt.internal;
                    fallback = candidate.clone();
                }
                let next = (receipt.mapped, receipt.internal);
                if receipt.internal <= tolerance && (retained.is_none() || next < score) {
                    retained = Some(candidate);
                    score = next;
                }
                Ok(())
            })?;
        }
        let shape = retained.as_deref().unwrap_or(&fallback);
        let final_receipt = measure(self.seed.len(), shape, &mut checks, &mut checked)?;
        if retained.is_none() && final_receipt.internal <= tolerance {
            return Err("coupled grid final receipt changed its earlier eligibility".into());
        }
        if final_receipt.internal > tolerance {
            return Ok(Attempt::Rejected(Rejection::Residual(
                final_receipt.internal,
            )));
        }
        if !(0.25..=2.0).contains(&super::super::vector_norm(shape.iter().copied())?) {
            return Err("coupled grid changed its unchanged direction norm bounds".into());
        }
        checkpoint(SolverStage::ModalRoundoffValidate, 1)?;
        Ok(Attempt::Accepted(retained.unwrap()))
    }
}
