use super::*;
use crate::modal_frame_spectrum::refinement::roundoff::triangular_grid::joint::{Outcome, Receipt};

#[path = "modal_roundoff_material_joint_readback_tests.rs"]
mod readback;

fn physical_seed(fixture: &Prepared, direction: &[Wide]) -> Vec<f64> {
    let physical: Vec<_> = direction
        .iter()
        .zip(&fixture.system.mass)
        .map(|(&v, m)| v.mul(Wide::from(m.sqrt().recip())))
        .collect();
    let square = physical
        .iter()
        .fold(Wide::default(), |sum, &v| sum.add(v.mul(v)));
    let mut norm = Wide::from(square.rounded().sqrt());
    for _ in 0..2 {
        norm = norm.add(square.div(norm)).mul(Wide::from(0.5));
    }
    let shape: Vec<_> = physical.iter().map(|v| v.div(norm).rounded()).collect();
    assert!((crate::modal_math::checked_shape_norm(&shape).unwrap() - 1.0).abs() < 1e-10);
    shape
}

struct PhysicalCase {
    seed: Vec<f64>,
    matrix: Vec<Vec<Wide>>,
    permutation: Vec<usize>,
    anchor: usize,
    internal_anchor: usize,
    internal_value: f64,
}

impl PhysicalCase {
    fn new(fixture: &Prepared, seed: Vec<f64>, reference: &[f64]) -> Self {
        let matrix =
            physical_directions(&fixture.physical, &fixture.system.mass, fixture.value).unwrap();
        let permutation = canonical_permutation(&matrix, &seed, &fixture.system.mass).unwrap();
        let mapped: Vec<_> = permutation.iter().map(|&i| seed[i]).collect();
        let anchor = (0..seed.len())
            .max_by(|&a, &b| {
                (mapped[a].abs() * fixture.system.mass[permutation[a]].sqrt())
                    .total_cmp(&(mapped[b].abs() * fixture.system.mass[permutation[b]].sqrt()))
            })
            .unwrap();
        let internal_anchor = (0..reference.len())
            .max_by(|&a, &b| reference[a].abs().total_cmp(&reference[b].abs()))
            .unwrap();
        Self {
            seed: mapped,
            matrix: permutation
                .iter()
                .map(|&i| permutation.iter().map(|&j| matrix[i][j]).collect())
                .collect(),
            permutation,
            anchor,
            internal_anchor,
            internal_value: reference[internal_anchor],
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

    fn restore(&self, v: &[f64]) -> Vec<f64> {
        let mut original = vec![0.0; v.len()];
        for (&i, &v) in self.permutation.iter().zip(v) {
            original[i] = v;
        }
        original
    }

    fn checked(&self, fixture: &Prepared, v: &[f64]) -> Result<Receipt, String> {
        let shape = self.restore(v);
        let applied = fixture.system.operator.apply_physical_compensated(&shape)?;
        let (physical, residual) = fixture.system.operator.physical_residual(
            fixture.value,
            &shape,
            &fixture.system.mass,
            &applied,
        )?;
        let internal: Vec<_> = shape
            .iter()
            .zip(&fixture.system.mass)
            .map(|(&v, m)| Wide::from(v).mul(Wide::from(m.sqrt())))
            .collect();
        let gauge = Wide::from(self.internal_value).div(internal[self.internal_anchor]);
        let direction: Vec<_> = internal.iter().map(|v| v.mul(gauge).rounded()).collect();
        assert_eq!(
            direction[self.internal_anchor].to_bits(),
            self.internal_value.to_bits()
        );
        assert!((0.25..=2.0).contains(&vector_norm(direction.iter().copied())?));
        let internal = fixture.checked(&direction)?.0;
        Ok(Receipt {
            physical,
            internal,
            residual: self.permutation.iter().map(|&i| residual[i]).collect(),
        })
    }
}

#[test]
fn modal_material_joint_physical_first_counterexamples() {
    for (members, scale) in [(100, 1e-10), (128, 1.0), (128, 1e14), (128, 1e-10)] {
        let layout =
            Rebuilt::prepare_bounded(&renumber(&Recipe::ThreeLayers.request(members, scale), 7))
                .unwrap();
        let (fixture, _) = prepare(layout.canonical).unwrap();
        let root = fixture.value.to_bits();
        let frozen = bits(&fixture.seed);
        let wide = Factor::prepare_shifted(&fixture.directions, fixture.value)
            .unwrap()
            .final_direction(&fixture.seed, 4)
            .unwrap();
        for policy in ["rounded-map", "wide-map"] {
            let shape = if policy == "wide-map" {
                physical_seed(&fixture, &wide)
            } else {
                checked_mode_shape(
                    &rounded(&wide),
                    &fixture.system.mass,
                    &(0..wide.len()).collect::<Vec<_>>(),
                    wide.len(),
                )
                .unwrap()
                .0
            };
            let case = PhysicalCase::new(&fixture, shape, &rounded(&wide));
            for order in [Order::GridNorm, Order::Reverse] {
                for width in [16, 64] {
                    let mut calls = 0;
                    let mut best = f64::INFINITY;
                    let (outcome, factors) = measured(|| {
                        let fit = GridFit::prepare(
                            &case.matrix,
                            &case.seed,
                            case.anchor,
                            &case.order(order),
                        )?;
                        fit.attempt_rounded_joint(1 << 22, width, 1e-8, |v| {
                            calls += 1;
                            let receipt = case.checked(&fixture, v)?;
                            best = best.min(receipt.physical.max(receipt.internal));
                            Ok(receipt)
                        })
                    });
                    assert_eq!(factors, 1);
                    assert_eq!(calls, width + 2);
                    let outcome = outcome.unwrap();
                    let Outcome::Rejected {
                        physical,
                        internal,
                        norm,
                    } = &outcome
                    else {
                        panic!("hard joint boundary unexpectedly accepted: {outcome:?}");
                    };
                    assert!(*physical > 1e-8 || *internal > 1e-8 || (norm - 1.0).abs() >= 1e-10);
                    assert_eq!(fixture.value.to_bits(), root);
                    assert_eq!(bits(&fixture.seed), frozen);
                    println!(
                        "material joint physical-first members={members} scale={scale:e} map={policy} order={order:?} width={width} accepted={} best_joint={best:e} factors={factors} paired_certificates={calls} outcome={outcome:?}",
                        matches!(outcome, Outcome::Accepted(_))
                    );
                }
            }
        }
    }
}

#[test]
fn modal_material_joint_isolated_candidate_physical_diagnosis() {
    for (members, scale) in [(100, 1e-10), (128, 1.0), (128, 1e14), (128, 1e-10)] {
        let layout =
            Rebuilt::prepare_bounded(&renumber(&Recipe::ThreeLayers.request(members, scale), 7))
                .unwrap();
        let (fixture, _) = prepare(layout.canonical).unwrap();
        let direction = rounded(
            &Factor::prepare_shifted(&fixture.directions, fixture.value)
                .unwrap()
                .final_direction(&fixture.seed, 4)
                .unwrap(),
        );
        let case = Case::new(&fixture, &direction);
        for order in [Order::GridNorm, Order::Reverse] {
            for width in [16, 64] {
                let mut calls = 0;
                let mut internal_passes = 0;
                let mut joint_passes = 0;
                let mut best_physical = f64::INFINITY;
                let mut best_joint = f64::INFINITY;
                let (outcome, factors) = measured(|| {
                    let fit = GridFit::prepare(
                        &case.matrix,
                        &case.seed,
                        case.anchor,
                        &case.order(order),
                    )?;
                    fit.attempt_rounded_beam(1 << 22, width, 1e-8, |v| {
                        calls += 1;
                        let restored = case.restore(v);
                        let (relative, residual) = fixture.checked(&restored)?;
                        let (shape, norm) = checked_mode_shape(&restored, &fixture.system.mass,
                            &(0..v.len()).collect::<Vec<_>>(), v.len())?;
                        assert!((norm - 1.0).abs() < 1e-10);
                        let applied = fixture.system.operator.apply_physical_compensated(&shape)?;
                        let physical = fixture.system.operator.physical_residual(fixture.value,
                            &shape, &fixture.system.mass, &applied)?.0;
                        best_physical = best_physical.min(physical);
                        best_joint = best_joint.min(relative.max(physical));
                        if relative <= 1e-8 {
                            internal_passes += 1;
                            joint_passes += usize::from(physical <= 1e-8);
                            println!("material joint eligible members={members} scale={scale:e} order={order:?} width={width} callback={calls} internal={relative:e} physical={physical:e}");
                        }
                        Ok((relative, case.permutation.iter().map(|&i| residual[i]).collect()))
                    })
                });
                assert_eq!(factors, 1);
                assert_eq!(calls, width + 2);
                assert_eq!(joint_passes, 0);
                assert_eq!(
                    internal_passes,
                    if members == 100 && matches!(order, Order::Reverse) {
                        width + 1
                    } else {
                        0
                    }
                );
                assert!(outcome.is_ok(), "{:?}", outcome.as_ref().err());
                println!(
                    "material joint diagnostic members={members} scale={scale:e} order={order:?} width={width} internal_passes={internal_passes} joint_passes={joint_passes} best_physical={best_physical:e} best_joint={best_joint:e} factors={factors} certificates={calls} physical_publications=0"
                );
            }
        }
    }
}
