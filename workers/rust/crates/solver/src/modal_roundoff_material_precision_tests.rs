use super::super::super::super::super::super::super::super::canonical_permutation;
use super::super::super::super::super::super::{GridFit, reference};
use super::*;
use crate::modal_frame_spectrum::refinement::roundoff::triangular_grid::{Attempt, Rejection};

#[path = "modal_roundoff_material_givens_tests.rs"]
mod givens_tests;

#[path = "modal_roundoff_material_rounded_beam_tests.rs"]
mod rounded_beam;

struct Case {
    seed: Vec<f64>,
    matrix: Vec<Vec<Wide>>,
    permutation: Vec<usize>,
    anchor: usize,
}

impl Case {
    fn new(fixture: &Prepared, seed: &[f64]) -> Self {
        let permutation =
            canonical_permutation(&fixture.directions, seed, &vec![1.0; seed.len()]).unwrap();
        let seed: Vec<_> = permutation.iter().map(|&i| seed[i]).collect();
        let matrix = permutation
            .iter()
            .map(|&i| {
                permutation
                    .iter()
                    .map(|&j| fixture.directions[i][j])
                    .collect()
            })
            .collect();
        let anchor = (0..seed.len())
            .max_by(|&a, &b| seed[a].abs().total_cmp(&seed[b].abs()))
            .unwrap();
        Self {
            seed,
            matrix,
            permutation,
            anchor,
        }
    }

    fn order(&self, order: Order) -> Vec<usize> {
        let mut indices: Vec<_> = (0..self.seed.len()).filter(|&i| i != self.anchor).collect();
        match order {
            Order::Natural => (),
            Order::Reverse => indices.reverse(),
            Order::GridNorm => {
                let norms: Vec<_> = (0..self.seed.len())
                    .map(|j| {
                        vector_norm(self.matrix.iter().map(|row| {
                            row[j]
                                .mul(Wide::from(self.seed[j].next_up() - self.seed[j]))
                                .rounded()
                        }))
                        .unwrap()
                    })
                    .collect();
                indices.sort_by(|&a, &b| norms[a].total_cmp(&norms[b]).then(a.cmp(&b)));
            }
        }
        indices
    }

    fn restore(&self, vector: &[f64]) -> Vec<f64> {
        assert_eq!(vector.len(), self.permutation.len());
        let mut original = vec![0.0; vector.len()];
        for (&i, &v) in self.permutation.iter().zip(vector) {
            original[i] = v;
        }
        original
    }
}

#[test]
fn modal_material_construction_isolated_qr_precision_and_quantization() {
    let mut accepted = [0; 7];
    let mut faults = [0; 7];
    let mut routes = 0;
    for (members, scale) in [(100, 1e-10), (128, 1.0), (128, 1e14), (128, 1e-10)] {
        let input = renumber(&Recipe::ThreeLayers.request(members, scale), 7);
        let layout = Rebuilt::prepare_bounded(&input).unwrap();
        let (fixture, refined) = prepare(layout.canonical).unwrap();
        assert!(!refined);
        let root = fixture.value.to_bits();
        let original = bits(&fixture.seed);
        let (lower, upper) = reference::bracket(&fixture.directions, fixture.value).unwrap();
        assert!((upper - lower) / fixture.value < 1e-14);
        assert!((fixture.value / (lower + (upper - lower) * 0.5) - 1.0).abs() < 1e-12);
        let direction = rounded(
            &Factor::prepare_shifted(&fixture.directions, fixture.value)
                .unwrap()
                .final_direction(&fixture.seed, 4)
                .unwrap(),
        );
        let case = Case::new(&fixture, &direction);
        for order in [Order::GridNorm, Order::Reverse, Order::Natural] {
            let indices = case.order(order);
            for (index, precision) in [
                "rounded",
                "wide-rounded-columns",
                "wide",
                "wide-beam",
                "givens-rounded-columns",
                "givens",
                "givens-beam",
            ]
            .into_iter()
            .enumerate()
            {
                let mut calls = 0;
                let mut best = f64::INFINITY;
                let (result, factors) = measured(|| {
                    let fit = match index {
                        0 => GridFit::prepare(&case.matrix, &case.seed, case.anchor, &indices),
                        1..=3 => GridFit::prepare_wide(
                            &case.matrix,
                            &case.seed,
                            case.anchor,
                            &indices,
                            index == 1,
                        ),
                        _ => GridFit::prepare_givens(
                            &case.matrix,
                            &case.seed,
                            case.anchor,
                            &indices,
                            index == 4,
                        ),
                    }?;
                    let checked = |v: &[f64]| {
                        calls += 1;
                        let (relative, residual) = fixture.checked(&case.restore(v))?;
                        best = best.min(relative);
                        Ok((
                            relative,
                            case.permutation.iter().map(|&i| residual[i]).collect(),
                        ))
                    };
                    if index == 3 || index == 6 {
                        fit.attempt_wide_beam(1 << 22, 1e-8, checked)
                    } else {
                        fit.attempt_direction(1 << 22, 1e-8, checked)
                    }
                });
                assert_eq!(factors, 1);
                assert!(calls <= 6);
                let result = match result {
                    Ok(result) => result,
                    Err(error) => {
                        if index <= 3 {
                            assert!(
                                error.starts_with("wide QR reflector result exceeds its range;"),
                                "{error}"
                            );
                            assert!(
                                error.contains(if members == 100 {
                                    if index == 1 {
                                        "pivot=24 column=25"
                                    } else {
                                        "pivot=24 column=24"
                                    }
                                } else {
                                    "pivot=93 column=96"
                                }),
                                "{error}"
                            );
                        } else {
                            assert!(
                                members == 128
                                    && scale != 1e-10
                                    && matches!(order, Order::GridNorm)
                            );
                            assert!(error.starts_with("wide Givens result exceeds its unchanged range; pivot=108 row=232"), "{error}");
                        }
                        assert_eq!(calls, 0);
                        faults[index] += 1;
                        println!(
                            "material precision members={members} scale={scale:e} order={order:?} precision={precision} accepted=false preparation_fault={error:?} factors={factors} certificates={calls}"
                        );
                        routes += 1;
                        continue;
                    }
                };
                let passed = match result {
                    Attempt::Accepted(v) => {
                        assert_eq!(v[case.anchor].to_bits(), case.seed[case.anchor].to_bits());
                        assert!(fixture.checked(&case.restore(&v)).unwrap().0 <= 1e-8);
                        accepted[index] += 1;
                        true
                    }
                    Attempt::Rejected(Rejection::Residual(relative)) => {
                        assert!(relative > 1e-8);
                        assert_eq!(relative.to_bits(), best.to_bits());
                        false
                    }
                    Attempt::Rejected(Rejection::UnitNorm(_)) => unreachable!(),
                };
                println!(
                    "material precision members={members} scale={scale:e} order={order:?} precision={precision} accepted={passed} best={best:e} factors={factors} certificates={calls}"
                );
                routes += 1;
            }
        }
        assert_eq!(fixture.value.to_bits(), root);
        assert_eq!(bits(&fixture.seed), original);
    }
    assert_eq!(routes, 84);
    assert_eq!(accepted, [0; 7]);
    assert_eq!(faults, [0, 2, 2, 2, 2, 2, 2]);
    println!(
        "material precision isolated_routes=48 accepted_by_precision={:?} preparation_faults_by_precision={:?}",
        &accepted[..4],
        &faults[..4]
    );
    println!(
        "material precision isolated_routes=84 accepted_by_precision={accepted:?} preparation_faults_by_precision={faults:?}"
    );
}
