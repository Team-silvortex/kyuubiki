use crate::modal_test_wide::Wide;

// Separate banded LDL inertia, not Jacobi, a production seed, or interval arithmetic.
pub(super) fn bracket(shifted: &[Vec<Wide>], seed_value: f64) -> Result<(f64, f64), String> {
    let size = shifted.len();
    if !(2..=256).contains(&size)
        || !seed_value.is_finite()
        || !(1e-50..=1e50).contains(&seed_value)
        || shifted.iter().any(|row| row.len() != size)
    {
        return Err("normalized inertia requires bounded square data and a positive root".into());
    }
    if shifted.iter().flatten().any(|entry| {
        !entry.high.is_finite()
            || !entry.low.is_finite()
            || entry.high.abs() > 1e50
            || entry.low.abs() > entry.high.abs()
    }) {
        return Err("normalized inertia requires bounded finite wide parts".into());
    }
    for i in 0..size {
        for j in 0..size {
            let entry = shifted[i][j];
            let other = shifted[j][i];
            let scale = entry.rounded().abs().max(other.rounded().abs());
            if (i.abs_diff(j) > 3 && (entry.high != 0.0 || entry.low != 0.0))
                || entry.sub(other).rounded().abs() > 1e-27 * scale
            {
                return Err(
                    "normalized inertia requires finite symmetric bandwidth-three data".into(),
                );
            }
        }
    }
    let mut matrix = shifted.to_vec();
    for (i, row) in matrix.iter_mut().enumerate() {
        row[i] = row[i].add(Wide::from(seed_value));
    }
    let mut lower = seed_value * (1.0 - 1e-6);
    let mut upper = seed_value * (1.0 + 1e-6);
    if inertia(&matrix, lower)? != 0 || inertia(&matrix, upper)? != 1 {
        return Err("normalized inertia did not isolate the first root".into());
    }
    for _ in 0..32 {
        let middle = lower + (upper - lower) * 0.5;
        match inertia(&matrix, middle)? {
            0 => lower = middle,
            1 => upper = middle,
            _ => return Err("normalized inertia found an unexpected root count".into()),
        }
    }
    Ok((lower, upper))
}

fn inertia(matrix: &[Vec<Wide>], value: f64) -> Result<usize, String> {
    let size = matrix.len();
    let mut lower = vec![[Wide::default(); 3]; size];
    let mut diagonal = vec![Wide::default(); size];
    let mut negative = 0;
    for i in 0..size {
        let mut pivot = matrix[i][i].sub(Wide::from(value));
        for (j, &previous) in diagonal
            .iter()
            .enumerate()
            .take(i)
            .skip(i.saturating_sub(3))
        {
            let entry = lower[i][i - j - 1];
            pivot = pivot.sub(entry.mul(entry).mul(previous));
        }
        if !pivot.rounded().is_finite() || !(1e-100..=1e100).contains(&pivot.rounded().abs()) {
            return Err("normalized inertia cannot certify a zero or nonfinite pivot".into());
        }
        negative += usize::from(pivot.rounded() < 0.0);
        diagonal[i] = pivot;
        for k in i + 1..(i + 4).min(size) {
            let mut entry = matrix[k][i];
            for (j, &previous) in diagonal
                .iter()
                .enumerate()
                .take(i)
                .skip(k.saturating_sub(3))
            {
                entry = entry.sub(lower[k][k - j - 1].mul(lower[i][i - j - 1]).mul(previous));
            }
            lower[k][k - i - 1] = entry.div(pivot);
            if !lower[k][k - i - 1].rounded().is_finite()
                || lower[k][k - i - 1].rounded().abs() > 1e14
            {
                return Err("normalized inertia exceeds its bounded factor range".into());
            }
        }
    }
    Ok(negative)
}

#[test]
fn triangular_grid_normalized_inertia_analytic_roots_and_invalid_data() {
    let root = (5.0 - 5.0_f64.sqrt()) * 0.5;
    let matrix = vec![
        vec![Wide::from(2.0 - root), Wide::from(1.0)],
        vec![Wide::from(1.0), Wide::from(3.0 - root)],
    ];
    let (lower, upper) = bracket(&matrix, root).unwrap();
    assert!((root / ((lower + upper) * 0.5) - 1.0).abs() < 1e-14);
    for bad in [
        vec![],
        vec![vec![Wide::from(1.0)]],
        vec![vec![Wide::from(1.0); 2]],
        vec![
            vec![
                Wide {
                    high: f64::NAN,
                    low: 0.0
                };
                2
            ];
            2
        ],
    ] {
        assert!(bracket(&bad, root).is_err());
    }
    for value in [0.0, -1.0, f64::NAN, f64::INFINITY, 1e-60, 1e60] {
        assert!(bracket(&matrix, value).is_err());
    }
    let excessive = vec![vec![Wide::from(1e51); 2]; 2];
    assert!(bracket(&excessive, root).is_err());
    let mut nonsymmetric = matrix.clone();
    nonsymmetric[0][1] = Wide::from(0.5);
    assert!(bracket(&nonsymmetric, root).is_err());
    for invalid in [
        Wide {
            high: f64::NAN,
            low: 0.0,
        },
        Wide {
            high: 1.0,
            low: f64::INFINITY,
        },
        Wide {
            high: 1e300,
            low: -1e300,
        },
        Wide {
            high: 0.0,
            low: 1.0,
        },
    ] {
        let mut malformed = matrix.clone();
        malformed[1][0] = invalid;
        assert!(bracket(&malformed, root).is_err());
    }
    let mut out_of_band = vec![vec![Wide::default(); 5]; 5];
    out_of_band[0][4] = Wide::from(1.0);
    out_of_band[4][0] = Wide::from(1.0);
    assert!(bracket(&out_of_band, root).is_err());
    let singular = vec![
        vec![Wide::from(1.0), Wide::default()],
        vec![Wide::default(), Wide::from(2.0)],
    ];
    assert!(inertia(&singular, 1.0).is_err());
    assert!(bracket(&singular, 4.0).is_err());
}
