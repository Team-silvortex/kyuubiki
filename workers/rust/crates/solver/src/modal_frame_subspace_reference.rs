use super::*;

#[path = "modal_frame_subspace_tests.rs"]
mod tests;

#[path = "modal_frame_connected_reference.rs"]
mod connected;

// Test-only normalized-coordinate admission, not a production deflated solver.
// A residual-qualified candidate must still satisfy its borrowed modal basis.
struct BasisGuard {
    size: usize,
    tolerance: f64,
    previous: Vec<(f64, Vec<f64>)>,
}

impl BasisGuard {
    fn prepare(size: usize, previous: &[Vec<f64>], tolerance: f64) -> Result<Self, String> {
        if !(2..=256).contains(&size)
            || previous.len() >= size
            || !tolerance.is_finite()
            || tolerance <= 0.0
            || tolerance > 1e-8
        {
            return Err(
                "test-only modal basis requires bounded dimensions and a strict gate".into(),
            );
        }
        let mut basis: Vec<(f64, Vec<f64>)> = Vec::with_capacity(previous.len());
        for (index, vector) in previous.iter().enumerate() {
            checkpoint(SolverStage::ModalValidation, index)?;
            let vector = bounded(vector, size)?;
            for (_, prior) in &basis {
                if cosine(&vector, prior)? > tolerance {
                    return Err("test-only modal basis is not independently orthogonal".into());
                }
            }
            basis.push((0.0, vector));
        }
        checkpoint(SolverStage::ModalValidation, previous.len())?;
        Ok(Self {
            size,
            tolerance,
            previous: basis,
        })
    }

    fn check(&self, vector: &[f64]) -> Result<f64, String> {
        let bounded = bounded(vector, self.size)?;
        let maximum = self.overlap(&bounded)?;
        if maximum > self.tolerance {
            return Err(format!(
                "test-only modal candidate failed its orthogonality gate (overlap={maximum:e})"
            ));
        }
        Ok(maximum)
    }

    fn overlap(&self, bounded: &[f64]) -> Result<f64, String> {
        let mut maximum = 0.0_f64;
        for (index, (_, prior)) in self.previous.iter().enumerate() {
            maximum = maximum.max(cosine(bounded, prior)?);
            checkpoint(SolverStage::ModalValidation, index)?;
        }
        Ok(maximum)
    }

    fn project_seed(&self, vector: &[f64]) -> Result<Vec<f64>, String> {
        let mut private = bounded(vector, self.size)?;
        if self.overlap(&private)? <= self.tolerance {
            return Ok(vector.to_vec());
        }
        // Reuse the production two-pass projection and independence-loss guard;
        // the post-projection residual is evaluated anew by BlockFit, never reused.
        orthogonalize(&mut private, &self.previous)?;
        self.check(&private)?;
        Ok(private)
    }

    fn correct(
        &self,
        fit: &BlockFit,
        vector: &[f64],
        tolerance: f64,
        mut checked: impl FnMut(&[f64]) -> Result<(f64, Vec<f64>), String>,
    ) -> Result<Vec<f64>, String> {
        if !tolerance.is_finite() || tolerance <= 0.0 {
            return Err("test-only joint admission requires a positive residual gate".into());
        }
        let private = self.project_seed(vector)?;
        let candidate = fit.correct(&private, tolerance, &mut checked)?;
        let (relative, residual) = checked(&candidate)?;
        if !relative.is_finite()
            || relative < 0.0
            || relative > tolerance
            || residual.len() != self.size
            || residual.iter().any(|v| !v.is_finite())
        {
            return Err("test-only joint admission lost its final residual certificate".into());
        }
        // Correction may leave the projected subspace. Reject rather than return
        // an individually accurate but duplicated direction, or retry indefinitely.
        self.check(&candidate)?;
        checkpoint(SolverStage::ModalValidation, self.previous.len() + 1)?;
        Ok(candidate)
    }
}

fn bounded(vector: &[f64], size: usize) -> Result<Vec<f64>, String> {
    if vector.len() != size || vector.iter().any(|v| !v.is_finite()) {
        return Err("test-only modal basis requires matching finite nonzero vectors".into());
    }
    let mut private = vector.to_vec();
    let norm = vector_norm(private.iter().copied())?;
    scale_binary(&mut private, norm)?;
    Ok(private)
}

fn cosine(left: &[f64], right: &[f64]) -> Result<f64, String> {
    Ok(dot(left, right)?.abs()
        / vector_norm(left.iter().copied())?
        / vector_norm(right.iter().copied())?)
}
