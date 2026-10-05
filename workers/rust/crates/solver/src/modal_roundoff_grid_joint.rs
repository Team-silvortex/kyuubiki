use super::*;

#[path = "modal_roundoff_grid_joint_control_tests.rs"]
mod controls;

#[derive(Debug)]
pub(in crate::modal_frame_spectrum::refinement::roundoff) struct Receipt {
    pub(in crate::modal_frame_spectrum::refinement::roundoff) physical: f64,
    pub(in crate::modal_frame_spectrum::refinement::roundoff) internal: f64,
    pub(in crate::modal_frame_spectrum::refinement::roundoff) residual: Vec<f64>,
}

#[derive(Debug)]
pub(in crate::modal_frame_spectrum::refinement::roundoff) enum Outcome {
    Accepted(Vec<f64>),
    Rejected {
        physical: f64,
        internal: f64,
        norm: f64,
    },
}

fn measure(
    fit: &GridFit,
    shape: &[f64],
    checks: &mut Checks,
    checked: &mut impl FnMut(&[f64]) -> Result<Receipt, String>,
) -> Result<(Receipt, f64), String> {
    checks.next()?;
    let receipt = checked(shape)?;
    if !receipt.internal.is_finite() || receipt.internal < 0.0 {
        return Err("joint grid requires a finite matching internal certificate".into());
    }
    validate_receipt(fit.seed.len(), receipt.physical, &receipt.residual)?;
    let norm = crate::modal_math::checked_shape_norm(shape)?;
    Ok((receipt, norm))
}

impl GridFit {
    // Physical residual drives proposals; neither space alone can admit a result.
    pub(in crate::modal_frame_spectrum::refinement::roundoff) fn attempt_rounded_joint(
        &self,
        radius: usize,
        width: usize,
        tolerance: f64,
        mut checked: impl FnMut(&[f64]) -> Result<Receipt, String>,
    ) -> Result<Outcome, String> {
        Plan::rounded_joint(self.seed.len(), width)?;
        if !(1..=MAX_GRID_RADIUS).contains(&radius)
            || !tolerance.is_finite()
            || tolerance <= 0.0
            || !matches!(self.factor, Factor::Rounded(_))
        {
            return Err("joint grid requires its bounded preselected rounded factor".into());
        }
        require_unit_shape(&self.seed)?;
        let mut checks = Checks::new();
        let mut retained = self.seed.clone();
        let (initial, _) = measure(self, &retained, &mut checks, &mut checked)?;
        let mut score = initial.physical.max(initial.internal);
        if score > tolerance {
            self.visit_rounded_beam(&initial.residual, radius, width, |candidate| {
                let (receipt, norm) = measure(self, &candidate, &mut checks, &mut checked)?;
                let next = receipt.physical.max(receipt.internal);
                if (norm - 1.0).abs() < UNIT_SHAPE_TOLERANCE && next < score {
                    retained = candidate;
                    score = next;
                }
                Ok(())
            })?;
        }
        let (receipt, norm) = measure(self, &retained, &mut checks, &mut checked)?;
        if receipt.physical > tolerance
            || receipt.internal > tolerance
            || (norm - 1.0).abs() >= UNIT_SHAPE_TOLERANCE
        {
            return Ok(Outcome::Rejected {
                physical: receipt.physical,
                internal: receipt.internal,
                norm,
            });
        }
        checkpoint(SolverStage::ModalRoundoffValidate, 1)?;
        Ok(Outcome::Accepted(retained))
    }
}
