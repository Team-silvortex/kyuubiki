use super::{
    budget::{Checks, OUTER_STEPS, Plan},
    coupled_pairs, dot, predict,
};
use crate::solver_control::{SolverStage, checkpoint};

const SHORTLIST: usize = 16;

struct Proposal {
    score: f64,
    left: usize,
    value: f64,
    right: Option<(usize, f64)>,
}

// Test-only true-residual microsearch; approximate columns only rank its proposals.
pub(super) fn correct(
    matrix: &[Vec<f64>],
    vector: &[f64],
    tolerance: f64,
    mut checked: impl FnMut(&[f64]) -> Result<(f64, Vec<f64>), String>,
) -> Result<Vec<f64>, String> {
    Plan::new(matrix.len())?;
    if matrix.len() != vector.len()
        || matrix
            .iter()
            .any(|r| r.len() != vector.len() || r.iter().any(|v| !v.is_finite()))
        || vector.iter().any(|v| !v.is_finite())
        || !tolerance.is_finite()
        || tolerance <= 0.0
    {
        return Err("grid proposals require bounded finite matching data".into());
    }
    let columns: Vec<Vec<_>> = (0..vector.len())
        .map(|j| matrix.iter().map(|r| r[j]).collect())
        .collect();
    let pairs = coupled_pairs(&columns)?;
    let mut checks = Checks::new();
    let mut retained = vector.to_vec();
    let (mut relative, mut residual) = measure(&retained, &mut checked, &mut checks)?;
    for pass in 0..OUTER_STEPS {
        if relative <= tolerance {
            break;
        }
        checkpoint(SolverStage::ModalIteration, pass)?;
        let mut proposals = Vec::with_capacity(SHORTLIST + 1);
        for i in 0..vector.len() {
            for value in [retained[i].next_up(), retained[i].next_down()] {
                rank(
                    &mut proposals,
                    &residual,
                    &columns,
                    &retained,
                    i,
                    value,
                    None,
                )?;
            }
        }
        for &(left, right) in &pairs {
            for x in [retained[left].next_up(), retained[left].next_down()] {
                for y in [retained[right].next_up(), retained[right].next_down()] {
                    rank(
                        &mut proposals,
                        &residual,
                        &columns,
                        &retained,
                        left,
                        x,
                        Some((right, y)),
                    )?;
                }
            }
        }
        let mut improved = None;
        for proposal in proposals {
            let mut candidate = retained.clone();
            candidate[proposal.left] = proposal.value;
            if let Some((i, value)) = proposal.right {
                candidate[i] = value;
            }
            let certificate = measure(&candidate, &mut checked, &mut checks)?;
            if certificate.0 < relative {
                relative = certificate.0;
                improved = Some((candidate, certificate.1));
            }
        }
        if let Some((candidate, next)) = improved {
            retained = candidate;
            residual = next;
        } else {
            break;
        }
    }
    let (relative, _) = measure(&retained, &mut checked, &mut checks)?;
    if relative > tolerance {
        return Err(format!(
            "grid proposal did not reach its unchanged residual gate (relative={relative:e})"
        ));
    }
    checkpoint(SolverStage::ModalRoundoffValidate, 1)?;
    Ok(retained)
}

fn measure(
    vector: &[f64],
    checked: &mut impl FnMut(&[f64]) -> Result<(f64, Vec<f64>), String>,
    checks: &mut Checks,
) -> Result<(f64, Vec<f64>), String> {
    checks.next()?;
    let result = checked(vector)?;
    if !result.0.is_finite()
        || result.0 < 0.0
        || result.1.len() != vector.len()
        || result.1.iter().any(|v| !v.is_finite())
    {
        return Err("grid proposal requires a finite matching residual certificate".into());
    }
    Ok(result)
}

fn rank(
    proposals: &mut Vec<Proposal>,
    residual: &[f64],
    columns: &[Vec<f64>],
    vector: &[f64],
    left: usize,
    value: f64,
    right: Option<(usize, f64)>,
) -> Result<(), String> {
    if !value.is_finite() || right.is_some_and(|(_, v)| !v.is_finite()) {
        return Ok(());
    }
    let second = right.map(|(i, v)| (columns[i].as_slice(), vector[i] - v));
    let predicted = predict(residual, &columns[left], vector[left] - value, second)?;
    let score = dot(&predicted, &predicted)?;
    if !score.is_finite() {
        return Ok(());
    }
    proposals.push(Proposal {
        score,
        left,
        value,
        right,
    });
    proposals.sort_by(|a, b| {
        a.score
            .total_cmp(&b.score)
            .then(a.left.cmp(&b.left))
            .then(a.value.total_cmp(&b.value))
            .then(a.right.map(|p| p.0).cmp(&b.right.map(|p| p.0)))
            .then_with(|| {
                a.right
                    .map_or(0.0, |p| p.1)
                    .total_cmp(&b.right.map_or(0.0, |p| p.1))
            })
    });
    proposals.truncate(SHORTLIST);
    Ok(())
}
