use super::super::super::super::super::triangular_grid::{Attempt, Rejection};
use super::super::super::canonical_permutation;
use super::{GridFit, Prepared, Profile, vector_norm};
use crate::modal_test_wide::Wide;

#[test]
fn triangular_grid_normalized_order_isolated_descending_comparison() {
    for profile in [Profile::Graded, Profile::Layered] {
        for scale in [1.0, 1e14, 1e-10] {
            let fixture = Prepared::new(profile, scale);
            let size = fixture.seed.len();
            let permutation =
                canonical_permutation(&fixture.directions, &fixture.seed, &vec![1.0; size])
                    .unwrap();
            let seed: Vec<_> = permutation.iter().map(|&i| fixture.seed[i]).collect();
            let matrix: Vec<Vec<_>> = permutation
                .iter()
                .map(|&i| {
                    permutation
                        .iter()
                        .map(|&j| fixture.directions[i][j])
                        .collect()
                })
                .collect();
            let anchor = seed
                .iter()
                .enumerate()
                .max_by(|(_, a), (_, b)| a.abs().total_cmp(&b.abs()))
                .unwrap()
                .0;
            let natural: Vec<_> = (0..size).filter(|&i| i != anchor).collect();
            let mut ranked = Vec::new();
            for &j in &natural {
                let grid = seed[j].next_up() - seed[j];
                let norm = vector_norm(
                    matrix
                        .iter()
                        .map(|row| row[j].mul(Wide::from(grid)).rounded()),
                )
                .unwrap();
                ranked.push((j, norm));
            }
            ranked.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
            let orders = [
                ("reverse", natural.iter().copied().rev().collect::<Vec<_>>()),
                ("natural", natural),
                ("ascending", ranked.iter().map(|&(j, _)| j).collect()),
                ("descending", ranked.iter().rev().map(|&(j, _)| j).collect()),
            ];
            for (name, order) in orders {
                let mut calls = 0;
                let mut best = f64::INFINITY;
                let restore = |v: &[f64]| {
                    let mut original = vec![0.0; size];
                    for (&i, &value) in permutation.iter().zip(v) {
                        original[i] = value;
                    }
                    original
                };
                let result = GridFit::prepare(&matrix, &seed, anchor, &order)
                    .unwrap()
                    .attempt_direction(1 << 22, 1e-8, |v| {
                        calls += 1;
                        let (relative, residual) = fixture.checked(&restore(v))?;
                        best = best.min(relative);
                        Ok((relative, permutation.iter().map(|&i| residual[i]).collect()))
                    })
                    .unwrap();
                let accepted = matches!(&result, Attempt::Accepted(_));
                let expected = (name == "ascending"
                    && !(matches!(profile, Profile::Graded) && scale == 1e14)
                    && !(matches!(profile, Profile::Layered) && scale == 1e-10))
                    || (name == "reverse" && matches!(profile, Profile::Layered) && scale != 1.0);
                assert_eq!(accepted, expected, "{profile:?}/{scale:e}/{name}");
                let publication = match result {
                    Attempt::Accepted(v) => {
                        assert_eq!(v[anchor].to_bits(), seed[anchor].to_bits());
                        let original = restore(&v);
                        assert!(fixture.checked(&original).unwrap().0 <= 1e-8);
                        Some(
                            fixture
                                .publish_grid(&original)
                                .map(|(_, residual)| residual),
                        )
                    }
                    Attempt::Rejected(Rejection::Residual(relative)) => {
                        assert!(relative > 1e-8);
                        None
                    }
                    Attempt::Rejected(Rejection::UnitNorm(_)) => unreachable!(),
                };
                assert!(calls <= 6);
                assert!(publication.as_ref().is_none_or(|v| v.is_ok()));
                println!(
                    "normalized order profile={} scale={scale:e} order={name} best={best:e} calls={calls} physical={publication:?}",
                    profile.name()
                );
            }
        }
    }
}

#[test]
fn triangular_grid_normalized_order_isolated_radius_comparison() {
    let fixture = Prepared::new(Profile::Graded, 1e14);
    let size = fixture.seed.len();
    let permutation =
        canonical_permutation(&fixture.directions, &fixture.seed, &vec![1.0; size]).unwrap();
    let seed: Vec<_> = permutation.iter().map(|&i| fixture.seed[i]).collect();
    let matrix: Vec<Vec<_>> = permutation
        .iter()
        .map(|&i| {
            permutation
                .iter()
                .map(|&j| fixture.directions[i][j])
                .collect()
        })
        .collect();
    let anchor = seed
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.abs().total_cmp(&b.abs()))
        .unwrap()
        .0;
    let mut ranked = Vec::new();
    for j in 0..size {
        if j != anchor {
            let grid = seed[j].next_up() - seed[j];
            ranked.push((
                j,
                vector_norm(
                    matrix
                        .iter()
                        .map(|row| row[j].mul(Wide::from(grid)).rounded()),
                )
                .unwrap(),
            ));
        }
    }
    ranked.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
    for (name, order) in [
        (
            "reverse",
            (0..size).rev().filter(|&i| i != anchor).collect::<Vec<_>>(),
        ),
        ("ascending", ranked.iter().map(|&(j, _)| j).collect()),
    ] {
        let fit = GridFit::prepare(&matrix, &seed, anchor, &order).unwrap();
        for radius in [1, 16, 256, 4096, 65536, 1 << 22] {
            let mut calls = 0;
            let mut best = f64::INFINITY;
            let outcome = fit
                .attempt_direction(radius, 1e-8, |v| {
                    calls += 1;
                    let mut original = vec![0.0; size];
                    for (&i, &a) in permutation.iter().zip(v) {
                        original[i] = a;
                    }
                    let (relative, residual) = fixture.checked(&original)?;
                    best = best.min(relative);
                    Ok((relative, permutation.iter().map(|&i| residual[i]).collect()))
                })
                .unwrap();
            assert!(calls <= 6);
            assert!(matches!(outcome, Attempt::Rejected(Rejection::Residual(_))));
            assert_eq!(
                best.to_bits(),
                fixture.checked(&fixture.seed).unwrap().0.to_bits()
            );
            println!(
                "normalized radius order={name} radius={radius} best={best:e} calls={calls} accepted={}",
                matches!(outcome, Attempt::Accepted(_))
            );
        }
    }
}
