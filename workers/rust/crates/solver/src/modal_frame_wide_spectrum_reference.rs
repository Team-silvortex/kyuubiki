use super::*;

// Separate bounded cyclic Jacobi arithmetic, never seeded by production pairs.
// The method shares the Jacobi family, so inertia brackets provide a second check.
pub(super) fn spectrum(
    matrix: &[Vec<Wide>],
    budget: usize,
) -> Result<Vec<(Wide, Vec<Wide>)>, String> {
    let size = matrix.len();
    if !bounded_symmetric(matrix) || budget > 40 {
        return Err("wide spectrum requires a bounded symmetric matrix and sweep budget".into());
    }
    let mut a = matrix.to_vec();
    let mut vectors = vec![vec![Wide::default(); size]; size];
    for (i, row) in vectors.iter_mut().enumerate() {
        row[i] = Wide::from(1.0);
    }
    for sweep in 0..=budget {
        let pending =
            (0..size).any(|p| (p + 1..size).any(|q| unresolved(a[p][q], a[p][p], a[q][q])));
        if !pending {
            let mut pairs: Vec<_> = (0..size)
                .map(|i| (a[i][i], vectors.iter().map(|row| row[i]).collect()))
                .collect();
            pairs.sort_by(|a, b| a.0.rounded().total_cmp(&b.0.rounded()));
            if pairs.iter().any(|p| p.0.rounded() <= 0.0) {
                return Err("wide spectrum requires positive roots".into());
            }
            return Ok(pairs);
        }
        if sweep == budget {
            break;
        }
        for p in 0..size {
            for q in p + 1..size {
                let off = a[p][q];
                if !unresolved(off, a[p][p], a[q][q]) {
                    continue;
                }
                let delta = a[q][q].sub(a[p][p]).mul(Wide::from(0.5));
                let sign = if delta.rounded() < 0.0 { -1.0 } else { 1.0 };
                let denominator = delta
                    .mul(Wide::from(sign))
                    .add(wide_sqrt(delta.mul(delta).add(off.mul(off))));
                let t = off.div(denominator).mul(Wide::from(sign));
                let c = Wide::from(1.0).div(wide_sqrt(Wide::from(1.0).add(t.mul(t))));
                let s = t.mul(c);
                a[p][p] = a[p][p].sub(t.mul(off));
                a[q][q] = a[q][q].add(t.mul(off));
                a[p][q] = Wide::default();
                a[q][p] = Wide::default();
                for k in 0..size {
                    if k != p && k != q {
                        let (left, right) = (a[k][p], a[k][q]);
                        a[k][p] = c.mul(left).sub(s.mul(right));
                        a[p][k] = a[k][p];
                        a[k][q] = s.mul(left).add(c.mul(right));
                        a[q][k] = a[k][q];
                    }
                    let (left, right) = (vectors[k][p], vectors[k][q]);
                    vectors[k][p] = c.mul(left).sub(s.mul(right));
                    vectors[k][q] = s.mul(left).add(c.mul(right));
                }
            }
        }
    }
    Err(format!("wide spectrum exhausted {budget} sweeps"))
}

fn unresolved(off: Wide, left: Wide, right: Wide) -> bool {
    let scale = left.rounded().abs().min(right.rounded().abs());
    off.rounded() != 0.0 && (scale == 0.0 || off.rounded().abs() / scale > 1e-26)
}

// Unpivoted LDLT inertia is valid here only with finite nonzero pivots. Reject
// singular brackets instead of inventing signs; this is not interval arithmetic.
pub(super) fn inertia(matrix: &[Vec<Wide>], shift: Wide) -> Result<usize, String> {
    let size = matrix.len();
    if !bounded_symmetric(matrix) {
        return Err("wide inertia requires a bounded symmetric matrix".into());
    }
    let mut lower = vec![vec![Wide::default(); size]; size];
    let mut pivots = vec![Wide::default(); size];
    let mut negative = 0;
    for i in 0..size {
        let mut pivot = matrix[i][i].sub(shift);
        for (j, &previous) in pivots.iter().enumerate().take(i) {
            pivot = pivot.sub(lower[i][j].mul(lower[i][j]).mul(previous));
        }
        if pivot.rounded() == 0.0 || !pivot.rounded().is_finite() {
            return Err("wide inertia cannot certify a zero or nonfinite pivot".into());
        }
        negative += usize::from(pivot.rounded() < 0.0);
        pivots[i] = pivot;
        for k in i + 1..size {
            let mut entry = matrix[k][i];
            for (j, &previous) in pivots.iter().enumerate().take(i) {
                entry = entry.sub(lower[k][j].mul(lower[i][j]).mul(previous));
            }
            lower[k][i] = entry.div(pivot);
        }
    }
    Ok(negative)
}

pub(super) fn apply_wide(matrix: &[Vec<Wide>], vector: &[Wide]) -> Vec<Wide> {
    assert!(matrix.iter().all(|row| row.len() == vector.len()));
    matrix
        .iter()
        .map(|row| {
            row.iter()
                .zip(vector)
                .fold(Wide::default(), |sum, (&a, &v)| sum.add(a.mul(v)))
        })
        .collect()
}

fn wide_dot(left: &[Wide], right: &[Wide]) -> Wide {
    assert_eq!(left.len(), right.len());
    left.iter()
        .zip(right)
        .fold(Wide::default(), |sum, (&a, &b)| sum.add(a.mul(b)))
}

fn wide_unit(vector: &[Wide]) -> Vec<Wide> {
    let norm = wide_sqrt(wide_dot(vector, vector));
    vector.iter().map(|&v| v.div(norm)).collect()
}

pub(super) fn projector_distance(actual: &[Vec<f64>], expected: &[Vec<Wide>]) -> f64 {
    assert_eq!(actual.len(), expected.len());
    assert!(!expected.is_empty());
    let size = expected[0].len();
    assert!(actual.iter().all(|v| v.len() == size));
    assert!(expected.iter().all(|v| v.len() == size));
    let actual: Vec<_> = actual.iter().map(|v| wide_unit(&wide_vector(v))).collect();
    let expected: Vec<_> = expected.iter().map(|v| wide_unit(v)).collect();
    let mut square = Wide::default();
    for i in 0..size {
        for j in 0..size {
            let entry = |vectors: &[Vec<Wide>]| {
                vectors
                    .iter()
                    .fold(Wide::default(), |sum, v| sum.add(v[i].mul(v[j])))
            };
            let delta = entry(&actual).sub(entry(&expected));
            square = square.add(delta.mul(delta));
        }
    }
    square.rounded().sqrt()
}

fn bounded_symmetric(matrix: &[Vec<Wide>]) -> bool {
    let size = matrix.len();
    (2..=96).contains(&size)
        && matrix.iter().all(|row| row.len() == size)
        && (0..size).all(|i| (0..size).all(|j| matrix[i][j].sub(matrix[j][i]).rounded() == 0.0))
}
