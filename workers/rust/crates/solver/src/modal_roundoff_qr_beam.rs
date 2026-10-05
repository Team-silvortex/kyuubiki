use super::{MAX_BEAM_WIDTH, MAX_GRID_RADIUS, QrFit, apply, dot};
use crate::solver_control::{SolverStage, checkpoint};

#[derive(Clone)]
struct Partial {
    integers: Vec<f64>,
    scaled: Vec<f64>,
    score: f64,
}

fn tie(left: &[f64], right: &[f64], orientation: f64) -> std::cmp::Ordering {
    for (&a, &b) in left.iter().zip(right) {
        let order = a
            .abs()
            .total_cmp(&b.abs())
            .then((orientation * a).total_cmp(&(orientation * b)));
        if !order.is_eq() {
            return order;
        }
    }
    std::cmp::Ordering::Equal
}

impl QrFit {
    // Test-only bounded beam. Transformed scores rank; callbacks still certify.
    pub(crate) fn quantized_beam(
        &self,
        rhs: &[f64],
        radius: usize,
        width: usize,
    ) -> Result<Vec<Vec<f64>>, String> {
        if rhs.len() != self.columns[0].len()
            || rhs.iter().any(|v| !v.is_finite() || v.abs() > 1e50)
            || !(1..=MAX_GRID_RADIUS).contains(&radius)
            || !(1..=MAX_BEAM_WIDTH).contains(&width)
        {
            return Err("rounded beam QR requires bounded data, radius and width".into());
        }
        checkpoint(SolverStage::DenseSubstitution, 0)?;
        let orientation = rhs
            .iter()
            .find(|v| **v != 0.0)
            .map_or(1.0, |v| 1.0_f64.copysign(*v));
        let mut transformed = rhs.to_vec();
        for (index, (direction, tau)) in self.reflectors.iter().enumerate() {
            apply(direction, *tau, &mut transformed[index..])?;
            checkpoint(SolverStage::DenseSubstitution, index + 1)?;
        }
        let count = self.columns.len();
        let mut beam = vec![Partial {
            integers: vec![0.0; count],
            scaled: vec![0.0; count],
            score: 0.0,
        }];
        for row in (0..count).rev() {
            let coefficients: Vec<_> = self.columns[row + 1..].iter().map(|c| c[row]).collect();
            let mut next = Vec::with_capacity(3 * width);
            for partial in beam {
                let target = transformed[row] - dot(&coefficients, &partial.scaled[row + 1..])?;
                let (scale, adjustment) = self.scales[row];
                let coefficient = (target / self.columns[row][row]) * adjustment / scale;
                if !coefficient.is_finite() {
                    return Err("rounded beam QR coefficient exceeds its range".into());
                }
                let nearest = coefficient.round().clamp(-(radius as f64), radius as f64);
                for integer in [nearest, nearest - 1.0, nearest + 1.0] {
                    if integer.abs() > radius as f64 {
                        continue;
                    }
                    let mut trial = partial.clone();
                    trial.integers[row] = integer;
                    trial.scaled[row] = integer * scale / adjustment;
                    if !trial.scaled[row].is_finite() {
                        return Err("rounded beam QR decision exceeds its range".into());
                    }
                    let error = target - self.columns[row][row] * trial.scaled[row];
                    let square = error * error;
                    trial.score += square;
                    if !trial.score.is_finite() || (error != 0.0 && square == 0.0) {
                        return Err("rounded beam QR score exceeds its range".into());
                    }
                    next.push(trial);
                }
            }
            next.sort_by(|a, b| {
                a.score
                    .total_cmp(&b.score)
                    .then_with(|| tie(&a.integers, &b.integers, orientation))
            });
            next.truncate(width);
            beam = next;
            checkpoint(SolverStage::DenseSubstitution, count + count - row)?;
        }
        Ok(beam.into_iter().map(|p| p.integers).collect())
    }
}

#[path = "modal_roundoff_qr_beam_tests.rs"]
mod tests;
