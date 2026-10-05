use super::*;
use crate::modal_frame_spectrum::refinement::roundoff::banded_grid::{
    Plan as GridPlan, checked_passes, propose_passes,
};

#[test]
fn modal_material_banded_grid_three_layer_counterexamples() {
    let mut accepted = 0;
    for (members, scale) in [(100, 1e-10), (128, 1.0), (128, 1e14), (128, 1e-10)] {
        let input = renumber(&Recipe::ThreeLayers.request(members, scale), 7);
        let layout = Rebuilt::prepare_bounded(&input).unwrap();
        let (fixture, refined) = prepare(layout.canonical).unwrap();
        assert!(!refined);
        let root = fixture.value.to_bits();
        let seed = bits(&fixture.seed);
        let wide = Factor::prepare_shifted(&fixture.directions, fixture.value)
            .unwrap()
            .final_direction(&fixture.seed, 4)
            .unwrap();
        let anchor = (0..wide.len())
            .max_by(|&a, &b| wide[a].rounded().abs().total_cmp(&wide[b].rounded().abs()))
            .unwrap();
        let initial = fixture.checked(&rounded(&wide)).unwrap().0;
        for passes in [1, 4] {
            let mut calls = 0;
            let mut best = f64::INFINITY;
            let mut prediction = f64::INFINITY;
            let (outcome, factors) = measured(|| {
                checked_passes(&fixture.directions, &wide, anchor, passes, |v| {
                    calls += 1;
                    prediction = continuous(
                        &fixture,
                        &v.iter().copied().map(Wide::from).collect::<Vec<_>>(),
                    );
                    let receipt = fixture.checked(v)?;
                    best = best.min(receipt.0);
                    Ok(receipt)
                })
            });
            assert_eq!(factors, 0);
            assert!((1..=2).contains(&calls));
            if let Ok(proposal) = &outcome {
                assert_eq!(
                    proposal.shape[anchor].to_bits(),
                    wide[anchor].rounded().to_bits()
                );
                assert!(fixture.checked(&proposal.shape).unwrap().0 <= 1e-8);
                assert_eq!(calls, 2);
                accepted += 1;
            }
            assert_eq!(fixture.value.to_bits(), root);
            assert_eq!(bits(&fixture.seed), seed);
            println!(
                "material banded grid members={members} scale={scale:e} passes={passes} initial={initial:e} prediction={prediction:e} actual={best:e} accepted={} factors={factors} certificates={calls} payload_bound={} error={:?}",
                outcome.is_ok(),
                GridPlan::for_passes(wide.len(), passes)
                    .unwrap()
                    .payload_bytes,
                outcome.as_ref().err()
            );
        }
    }
    assert_eq!(accepted, 0);
    println!(
        "material banded grid counterexamples=4 cold_routes=8 accepted={accepted} independent_physical_readbacks=0 production_admission=unchanged"
    );
}

fn anchor(direction: &[Wide]) -> usize {
    (0..direction.len())
        .max_by(|&a, &b| {
            direction[a]
                .rounded()
                .abs()
                .total_cmp(&direction[b].rounded().abs())
        })
        .unwrap()
}

fn compare(input: &SolveModalFrame2dRequest, label: &str) -> [bool; 2] {
    let before = serde_json::to_value(input).unwrap();
    let layout = Rebuilt::prepare_bounded(input).unwrap();
    let (fixture, _) = prepare(layout.canonical).unwrap();
    let wide = Factor::prepare_shifted(&fixture.directions, fixture.value)
        .unwrap()
        .final_direction(&fixture.seed, 4)
        .unwrap();
    let root = fixture.value.to_bits();
    let seed = bits(&fixture.seed);
    let mut stages = [false; 2];
    for (i, passes) in [1, 4].into_iter().enumerate() {
        let mut calls = 0;
        let mut best = f64::INFINITY;
        let (outcome, factors) = measured(|| {
            checked_passes(&fixture.directions, &wide, anchor(&wide), passes, |v| {
                calls += 1;
                let receipt = fixture.checked(v)?;
                best = best.min(receipt.0);
                Ok(receipt)
            })
        });
        assert_eq!(factors, 0);
        assert!((1..=2).contains(&calls));
        stages[i] = outcome.is_ok();
        if let Ok(proposal) = &outcome {
            let plan = GridPlan::for_passes(wide.len(), passes).unwrap();
            assert!(proposal.usage.nodes <= plan.nodes && proposal.usage.terms <= plan.terms);
            assert!(fixture.checked(&proposal.shape).unwrap().0 <= 1e-8);
            assert_eq!(calls, 2);
        } else {
            assert!(outcome.unwrap_err().contains("unchanged residual gate"));
            assert_eq!(calls, 1);
        }
        println!(
            "material banded grid broader case={label} passes={passes} accepted={} actual={best:e} certificates={calls} factors={factors}",
            stages[i]
        );
    }
    assert_eq!(fixture.value.to_bits(), root);
    assert_eq!(bits(&fixture.seed), seed);
    assert_eq!(serde_json::to_value(input).unwrap(), before);
    stages
}

#[test]
fn modal_material_banded_grid_recipe_mesh_and_old_baseline_comparison() {
    let mut accepted = [0; 2];
    let mut pairs = [0; 4];
    for recipe in Recipe::ALL {
        for members in [80, 100, 128] {
            for scale in [1.0, 1e14, 1e-10] {
                let stages = compare(
                    &renumber(&recipe.request(members, scale), 7),
                    &format!("{recipe:?}/{members}/{scale:e}"),
                );
                let expected = members == 80 && !matches!(recipe, Recipe::ThreeLayers);
                assert_eq!(stages, [expected; 2], "{recipe:?}/{members}/{scale:e}");
                pairs[usize::from(stages[0]) * 2 + usize::from(stages[1])] += 1;
                for i in 0..2 {
                    accepted[i] += usize::from(stages[i]);
                }
            }
        }
    }
    let mut old = [0; 2];
    for profile in [Profile::Graded, Profile::Layered] {
        for scale in [1.0, 1e14, 1e-10] {
            let stages = compare(
                &renumber(&Fixture::request(profile, 128, scale), 7),
                &format!("{profile:?}/128/{scale:e}"),
            );
            assert_eq!(stages, [false; 2], "{profile:?}/{scale:e}");
            for i in 0..2 {
                old[i] += usize::from(stages[i]);
            }
        }
    }
    assert_eq!(accepted, [9; 2]);
    assert_eq!(pairs, [27, 0, 0, 9]);
    assert_eq!(old, [0; 2]);
    println!(
        "material banded grid broader inputs=42 cold_routes=84 holdout_acceptances={accepted:?} one_four_pass_pairs={pairs:?} old_baseline_acceptances={old:?} independent_physical_readbacks=0 production_admission=unchanged"
    );
}

#[test]
fn modal_material_banded_grid_real_final_fault_cancel_and_fresh_replay() {
    let input = renumber(&Recipe::UnequalLengths.request(80, 1.0), 7);
    let layout = Rebuilt::prepare_bounded(&input).unwrap();
    let (fixture, _) = prepare(layout.canonical).unwrap();
    let wide = Factor::prepare_shifted(&fixture.directions, fixture.value)
        .unwrap()
        .final_direction(&fixture.seed, 4)
        .unwrap();
    let anchor = anchor(&wide);
    let mut calls = 0;
    let baseline = checked_passes(&fixture.directions, &wide, anchor, 4, |v| {
        calls += 1;
        fixture.checked(v)
    })
    .unwrap();
    assert_eq!(calls, 2);
    for failure in 0..3 {
        let mut calls = 0;
        let (outcome, factors) = measured(|| {
            checked_passes(&fixture.directions, &wide, anchor, 4, |v| {
                calls += 1;
                if calls == 2 {
                    match failure {
                        0 => return Err("banded real final operator fault".into()),
                        1 => return Ok((0.0, vec![0.0; v.len() - 1])),
                        _ => return Ok((1.0, vec![0.0; v.len()])),
                    }
                }
                fixture.checked(v)
            })
        });
        let error = outcome.unwrap_err();
        assert!(
            error.contains(match failure {
                0 => "banded real final operator fault",
                1 => "finite matching certificate",
                _ => "unchanged residual gate",
            }),
            "{error}"
        );
        assert_eq!(calls, 2);
        assert_eq!(factors, 0);
    }
    for step in [1, 2] {
        let control = SolverControl::default();
        let cancel = control.clone();
        let mut calls = 0;
        let outcome = with_solver_observer(
            &control,
            move |p| {
                if p.stage == SolverStage::ModalRoundoffValidate && p.completed_steps == step {
                    cancel.request_cancel();
                }
            },
            || {
                checked_passes(&fixture.directions, &wide, anchor, 4, |v| {
                    calls += 1;
                    fixture.checked(v)
                })
            },
        );
        assert!(outcome.unwrap_err().contains("cancel"));
        assert_eq!(calls, step as usize);
        let layout = Rebuilt::prepare_bounded(&renumber(&input, 113)).unwrap();
        let (fresh, _) = prepare(layout.canonical).unwrap();
        assert_eq!(fixture.value.to_bits(), fresh.value.to_bits());
        assert_eq!(bits(&fixture.seed), bits(&fresh.seed));
        let direction = Factor::prepare_shifted(&fresh.directions, fresh.value)
            .unwrap()
            .final_direction(&fresh.seed, 4)
            .unwrap();
        let mut calls = 0;
        let replay = checked_passes(&fresh.directions, &direction, anchor, 4, |v| {
            calls += 1;
            fresh.checked(v)
        })
        .unwrap();
        assert_eq!(calls, 2);
        assert_eq!(bits(&baseline.shape), bits(&replay.shape));
    }
    let plan = GridPlan::for_passes(wide.len(), 4).unwrap();
    println!(
        "material banded grid real_final_controls faults=3 cancellations=2 fresh_numbering_replays=2 partial_candidates=0 physical_publications=0 payload_bound={} trial_bound={} term_bound={} actual_trials={} actual_terms={} actual_nodes={}",
        plan.payload_bytes,
        plan.trials,
        plan.terms,
        baseline.usage.trials,
        baseline.usage.terms,
        baseline.usage.nodes
    );
}

#[test]
fn modal_material_banded_grid_real_sign_numbering_cancel_and_replay() {
    let input = renumber(&Recipe::ThreeLayers.request(128, 1.0), 7);
    let layout = Rebuilt::prepare_bounded(&input).unwrap();
    let (fixture, _) = prepare(layout.canonical).unwrap();
    let wide = Factor::prepare_shifted(&fixture.directions, fixture.value)
        .unwrap()
        .final_direction(&fixture.seed, 4)
        .unwrap();
    let anchor = anchor(&wide);
    let baseline = propose_passes(&fixture.directions, &wide, anchor, 4).unwrap();
    assert!(fixture.checked(&baseline.shape).unwrap().0 > 1e-8);
    for scale in [-1.0, 0.5, 2.0] {
        let changed = wide
            .iter()
            .map(|v| v.mul(Wide::from(scale)))
            .collect::<Vec<_>>();
        let actual = propose_passes(&fixture.directions, &changed, anchor, 4).unwrap();
        assert_eq!(
            bits(&actual.shape),
            bits(&baseline.shape.iter().map(|v| v * scale).collect::<Vec<_>>())
        );
    }
    for (stage, steps) in [
        (SolverStage::ModalRoundoffPrepare, 512),
        (SolverStage::ModalVectorScan, 128),
        (SolverStage::ModalRoundoffSearch, 128),
        (SolverStage::ModalVectorDot, 1),
        (SolverStage::ModalVectorUpdate, 256),
        (SolverStage::ModalRoundoffValidate, 0),
    ] {
        let control = SolverControl::default();
        let cancel = control.clone();
        let mut calls = 0;
        let error = with_solver_observer(
            &control,
            move |p| {
                if p.stage == stage && p.completed_steps == steps {
                    cancel.request_cancel();
                }
            },
            || {
                checked_passes(&fixture.directions, &wide, anchor, 4, |v| {
                    calls += 1;
                    fixture.checked(v)
                })
            },
        )
        .unwrap_err();
        assert!(error.contains("cancel"), "{error}");
        assert_eq!(calls, 0);
        let fresh = renumber(&input, 113);
        let layout = Rebuilt::prepare_bounded(&fresh).unwrap();
        let (fresh, _) = prepare(layout.canonical).unwrap();
        assert_eq!(fixture.value.to_bits(), fresh.value.to_bits());
        assert_eq!(bits(&fixture.seed), bits(&fresh.seed));
        let direction = Factor::prepare_shifted(&fresh.directions, fresh.value)
            .unwrap()
            .final_direction(&fresh.seed, 4)
            .unwrap();
        let replay = propose_passes(&fresh.directions, &direction, anchor, 4).unwrap();
        assert_eq!(bits(&baseline.shape), bits(&replay.shape));
    }
    println!(
        "material banded grid real_controls sign_binary=3 cancellations=6 fresh_numbering_replays=6 partial_candidates=0 physical_publications=0"
    );
}
