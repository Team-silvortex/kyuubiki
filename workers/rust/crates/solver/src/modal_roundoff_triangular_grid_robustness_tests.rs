use super::{
    hybrid_tests::{check_published, scaled_bending_parts},
    triangular_grid::{GridFit, MAX_CERTIFICATES, Policy},
    *,
};
use crate::modal_test_wide::Wide;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Outcome {
    Accepted,
    ResidualRejected,
    NormRejected,
}

pub(super) struct Fixture {
    pub(super) system: ReducedSparseModalSystem,
    pub(super) value: f64,
    pub(super) shape: Vec<f64>,
    pub(super) directions: Vec<Vec<Wide>>,
    anchors: [usize; 3],
    step: f64,
}

impl Fixture {
    pub(super) fn new(step: f64) -> Self {
        Self::with_segments(128, step)
    }

    pub(super) fn with_segments(segments: usize, step: f64) -> Self {
        let (system, physical) = scaled_bending_parts(segments, step);
        let spectrum = crate::modal_frame_spectrum::frame_eigenpairs(&system, Some(1)).unwrap();
        let (value, vector) = &spectrum.pairs[0];
        let size = system.mass.len();
        let (shape, _) = crate::modal_frame_spectrum::checked_mode_shape(
            vector,
            &system.mass,
            &(0..size).collect::<Vec<_>>(),
            size,
        )
        .unwrap();
        let shape = system
            .operator
            .roundoff_comparison_seed(*value, &shape, &system.mass)
            .unwrap();
        let directions =
            wide_projection::physical_directions(&physical, &system.mass, *value).unwrap();
        let mut ranked: Vec<_> = (0..size).collect();
        ranked.sort_by(|&a, &b| {
            let amplitude = |i: usize| shape[i].abs() * system.mass[i].sqrt();
            amplitude(b).total_cmp(&amplitude(a)).then(a.cmp(&b))
        });
        let other_family = *ranked.iter().find(|&&i| i % 2 != ranked[0] % 2).unwrap();
        let anchors = [ranked[0], ranked[1], other_family];
        assert_ne!(anchors[0], anchors[1]);
        assert_ne!(anchors[0], anchors[2]);
        Self {
            system,
            value: *value,
            shape,
            directions,
            anchors,
            step,
        }
    }

    pub(super) fn checked(&self, original: &[f64]) -> Result<(f64, Vec<f64>), String> {
        let applied = self.system.operator.apply_physical_compensated(original)?;
        self.system
            .operator
            .physical_residual(self.value, original, &self.system.mass, &applied)
    }

    fn probe(
        &self,
        permutation: &[usize],
        layout: &str,
        anchor: usize,
        sign: f64,
        strategy: &str,
    ) -> Outcome {
        let size = self.shape.len();
        let mut sorted = permutation.to_vec();
        sorted.sort_unstable();
        assert_eq!(sorted, (0..size).collect::<Vec<_>>());
        let seed: Vec<_> = permutation.iter().map(|&i| sign * self.shape[i]).collect();
        let matrix: Vec<Vec<_>> = permutation
            .iter()
            .map(|&i| permutation.iter().map(|&j| self.directions[i][j]).collect())
            .collect();
        let mapped_anchor = permutation.iter().position(|&i| i == anchor).unwrap();
        let mut order: Vec<_> = (0..size).filter(|&i| i != mapped_anchor).collect();
        match strategy {
            "natural" => {}
            "reverse" => order.reverse(),
            "grid-norm" => order.sort_by(|&a, &b| {
                let norm = |j: usize| {
                    matrix
                        .iter()
                        .map(|r| r[j].rounded().powi(2))
                        .sum::<f64>()
                        .sqrt()
                        * (seed[j].next_up() - seed[j])
                };
                norm(a).total_cmp(&norm(b)).then(a.cmp(&b))
            }),
            _ => panic!("unknown test strategy"),
        }
        let restore = |candidate: &[f64]| {
            let mut original = vec![0.0; size];
            for (&i, &v) in permutation.iter().zip(candidate) {
                original[i] = v;
            }
            original
        };
        let initial = self.checked(&restore(&seed)).unwrap().0;
        let fit = GridFit::prepare(&matrix, &seed, mapped_anchor, &order).unwrap();
        let (mut calls, mut best) = (0, initial);
        let result = fit.correct_unit_shape(qr::MAX_GRID_RADIUS, 1e-8, |v| {
            calls += 1;
            assert_eq!(v[mapped_anchor].to_bits(), seed[mapped_anchor].to_bits());
            let (relative, residual) = self.checked(&restore(v))?;
            best = best.min(relative);
            Ok((relative, permutation.iter().map(|&i| residual[i]).collect()))
        });
        println!(
            "robust grid step={:e} layout={layout} anchor={anchor} sign={sign} order={strategy} calls={calls} best={best:e} outcome={:?}",
            self.step,
            result.as_ref().map(|_| "pass")
        );
        assert!(calls <= MAX_CERTIFICATES);
        match result {
            Ok(candidate) => {
                assert_eq!(
                    candidate[mapped_anchor].to_bits(),
                    seed[mapped_anchor].to_bits()
                );
                let original = restore(&candidate);
                assert!(self.checked(&original).unwrap().0 <= 1e-8);
                check_published(128, self.step, self.value, &original);
                Outcome::Accepted
            }
            Err(error) => {
                if error.contains("unchanged norm gate") {
                    assert!(best <= 1e-8);
                    assert_eq!(calls, 3);
                    let norm: f64 = error
                        .split("norm=")
                        .nth(1)
                        .unwrap()
                        .trim_end_matches(')')
                        .parse()
                        .unwrap();
                    assert!((norm - 1.0).abs() >= triangular_grid::UNIT_SHAPE_TOLERANCE);
                    return Outcome::NormRejected;
                }
                assert!(error.contains("unchanged residual gate"), "{error}");
                let relative: f64 = error
                    .split("relative=")
                    .nth(1)
                    .unwrap()
                    .trim_end_matches(')')
                    .parse()
                    .unwrap();
                assert_eq!(relative, best);
                assert!(relative > 1e-8);
                assert_eq!(
                    calls,
                    if strategy == "grid-norm" && self.step == 1e-10 {
                        4
                    } else {
                        3
                    }
                );
                Outcome::ResidualRejected
            }
        }
    }
}

#[test]
fn triangular_grid_anchor_and_sign_comparison_keeps_the_original_physical_gate() {
    for step in [1.0, 1e14, 1e-10] {
        let fixture = Fixture::new(step);
        assert_eq!(fixture.anchors, [252, 250, 253]);
        let original_shape = fixture.shape.clone();
        let identity: Vec<_> = (0..fixture.shape.len()).collect();
        for anchor in fixture.anchors {
            for strategy in ["natural", "reverse", "grid-norm"] {
                let positive = fixture.probe(&identity, "identity", anchor, 1.0, strategy);
                let negative = fixture.probe(&identity, "identity", anchor, -1.0, strategy);
                let expected = if strategy == "natural"
                    || (step == 1e-10 && strategy == "grid-norm" && anchor != 253)
                {
                    Outcome::ResidualRejected
                } else if step == 1.0 && anchor == 253 {
                    Outcome::NormRejected
                } else {
                    Outcome::Accepted
                };
                assert_eq!(positive, expected, "{step:e}/{anchor}/{strategy}");
                assert_eq!(positive, negative, "{step:e}/{anchor}/{strategy}");
            }
        }
        assert_eq!(fixture.shape, original_shape);
    }
}

#[test]
fn triangular_grid_renumbering_comparison_restores_original_physical_coordinates() {
    for step in [1.0, 1e14, 1e-10] {
        let fixture = Fixture::new(step);
        let size = fixture.shape.len();
        let reverse: Vec<_> = (0..size).rev().collect();
        let family: Vec<_> = (0..size).step_by(2).chain((1..size).step_by(2)).collect();
        for (layout, permutation) in [("reversed", reverse), ("family-grouped", family)] {
            for strategy in ["natural", "reverse", "grid-norm"] {
                let outcome =
                    fixture.probe(&permutation, layout, fixture.anchors[0], 1.0, strategy);
                let expected = if (layout == "reversed" && strategy == "reverse")
                    || (layout == "family-grouped" && strategy == "natural")
                    || (step == 1e-10 && strategy == "grid-norm")
                {
                    Outcome::ResidualRejected
                } else {
                    Outcome::Accepted
                };
                assert_eq!(outcome, expected, "{step:e}/{layout}/{strategy}");
            }
        }
    }
}

#[test]
fn triangular_grid_closed_relabeling_recovers_exact_representable_targets() {
    let identity: Vec<Vec<_>> = (0..4)
        .map(|i| (0..4).map(|j| Wide::from(f64::from(i == j))).collect())
        .collect();
    for sign in [1.0, -1.0] {
        let seed = [sign * 1.5_f64; 4];
        let target = [
            seed[0].next_up(),
            seed[1].next_down(),
            seed[2].next_up(),
            seed[3],
        ];
        for permutation in [[0, 1, 2, 3], [3, 2, 1, 0], [0, 2, 1, 3]] {
            let mapped_seed = permutation.map(|i| seed[i]);
            let mapped_target = permutation.map(|i| target[i]);
            let matrix: Vec<Vec<_>> = permutation
                .iter()
                .map(|&i| permutation.iter().map(|&j| identity[i][j]).collect())
                .collect();
            let anchor = permutation.iter().position(|&i| i == 3).unwrap();
            for reverse in [false, true] {
                let mut order: Vec<_> = (0..4).filter(|&i| i != anchor).collect();
                if reverse {
                    order.reverse();
                }
                let fit = GridFit::prepare(&matrix, &mapped_seed, anchor, &order).unwrap();
                let candidate = fit
                    .correct(4, Policy::InBacksolve, 1e-8, |v| {
                        let residual: Vec<_> =
                            v.iter().zip(mapped_target).map(|(&a, b)| a - b).collect();
                        Ok((
                            vector_norm(residual.iter().copied())? / (seed[0].next_up() - seed[0]),
                            residual,
                        ))
                    })
                    .unwrap();
                assert_eq!(candidate, mapped_target);
                for (&i, &v) in permutation.iter().zip(&candidate) {
                    assert_eq!(v.to_bits(), target[i].to_bits());
                }
            }
        }
    }
}

#[test]
fn triangular_grid_unit_publication_rejects_residual_pass_with_norm_drift() {
    let seed = [3.0_f64.sqrt().recip(); 3];
    let matrix: Vec<Vec<_>> = (0..3)
        .map(|i| (0..3).map(|j| Wide::from(f64::from(i == j))).collect())
        .collect();
    let unit = seed[0].next_up() - seed[0];
    let fit = GridFit::prepare(&matrix, &seed, 2, &[0, 1]).unwrap();
    for steps in [1, 1 << 21] {
        let mut target = seed;
        target[0] += unit * steps as f64;
        let checked = |v: &[f64]| {
            let residual: Vec<_> = v.iter().zip(target).map(|(&a, b)| a - b).collect();
            Ok((vector_norm(residual.iter().copied())? / unit, residual))
        };
        let raw = fit
            .correct(qr::MAX_GRID_RADIUS, Policy::InBacksolve, 1e-8, checked)
            .unwrap();
        assert_eq!(raw, target);
        let result = fit.correct_unit_shape(qr::MAX_GRID_RADIUS, 1e-8, checked);
        if steps == 1 {
            assert_eq!(result.unwrap(), target);
        } else {
            assert!(crate::modal_math::checked_shape_norm(&target).unwrap() - 1.0 > 1e-10);
            assert!(result.unwrap_err().contains("unchanged norm gate"));
        }
    }
    let nonunit = GridFit::prepare(&matrix, &[1.5; 3], 2, &[0, 1]).unwrap();
    assert!(
        nonunit
            .correct_unit_shape(4, 1e-8, |_| unreachable!())
            .unwrap_err()
            .contains("unchanged norm gate")
    );
    assert_eq!(seed, [3.0_f64.sqrt().recip(); 3]);
}

#[test]
fn triangular_grid_cancelled_unit_validation_returns_no_shape_and_replays() {
    use crate::solver_control::{SolverControl, SolverStage, with_solver_observer};
    use std::{cell::Cell, rc::Rc};
    let seed = [std::f64::consts::FRAC_1_SQRT_2; 2];
    let matrix = vec![
        vec![Wide::from(1.0), Wide::from(0.0)],
        vec![Wide::from(0.0), Wide::from(1.0)],
    ];
    let fit = GridFit::prepare(&matrix, &seed, 1, &[0]).unwrap();
    for final_gate in [false, true] {
        let control = SolverControl::default();
        let cancel = control.clone();
        let norms = Rc::new(Cell::new(0));
        let seen = norms.clone();
        let error = with_solver_observer(
            &control,
            move |p| {
                if p.stage == SolverStage::ResultNodeSummary && p.completed_steps == 0 {
                    seen.set(seen.get() + 1);
                    if !final_gate && seen.get() == 2 {
                        cancel.request_cancel();
                    }
                }
                if final_gate
                    && p.stage == SolverStage::ModalRoundoffValidate
                    && p.completed_steps == 2
                {
                    cancel.request_cancel();
                }
            },
            || fit.correct_unit_shape(4, 1e-8, |_| Ok((0.0, vec![0.0; 2]))),
        )
        .unwrap_err();
        assert!(error.contains("cancel"), "{error}");
        assert_eq!(norms.get(), 2);
        assert_eq!(
            fit.correct_unit_shape(4, 1e-8, |_| Ok((0.0, vec![0.0; 2])))
                .unwrap(),
            seed
        );
    }
}
