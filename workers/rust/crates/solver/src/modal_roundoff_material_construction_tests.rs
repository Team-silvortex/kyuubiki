use super::super::super::super::super::{search_canonical_direction_single, vector_norm};
use super::*;

#[path = "modal_roundoff_material_precision_tests.rs"]
mod precision;

#[path = "modal_roundoff_material_banded_grid_tests.rs"]
mod banded_grid;

#[path = "modal_roundoff_material_amplitude_tests.rs"]
mod amplitude;

#[test]
fn modal_material_construction_cold_internal_inward_readback() {
    let mut stages = [[0; 7]; 3];
    let mut improved = [0; 2];
    let mut regressed = [0; 2];
    for recipe in Recipe::ALL {
        for members in [80, 100, 128] {
            for scale in [1.0, 1e14, 1e-10] {
                let input = renumber(&recipe.request(members, scale), 7);
                let original = run(&input, Chart::Original);
                stages[0][original.stage as usize] += 1;
                for (index, physical) in [Chart::Original, Chart::Inward].into_iter().enumerate() {
                    let actual = run_charts(&input, Chart::Inward, physical);
                    assert_eq!(actual.root, original.root);
                    assert_eq!(actual.seed, original.seed);
                    assert_eq!(actual.refined, original.refined);
                    assert_eq!(actual.stage == Stage::Accepted, actual.output.is_some());
                    let expected = if matches!(recipe, Recipe::ThreeLayers) && members == 128 {
                        Stage::Internal
                    } else if matches!(recipe, Recipe::UnequalLengths)
                        && members == 128
                        && (scale == 1e14 || scale == 1e-10)
                    {
                        Stage::Physical
                    } else {
                        Stage::Accepted
                    };
                    assert_eq!(
                        actual.stage, expected,
                        "{recipe:?} members={members} scale={scale:e} physical={physical:?}"
                    );
                    stages[index + 1][actual.stage as usize] += 1;
                    improved[index] += usize::from(
                        original.stage != Stage::Accepted && actual.stage == Stage::Accepted,
                    );
                    regressed[index] += usize::from(
                        original.stage == Stage::Accepted && actual.stage != Stage::Accepted,
                    );
                    println!(
                        "material construction cold recipe={recipe:?} members={members} scale={scale:e} internal=inward physical={physical:?} stage={:?} best={:e} readback={:?} factors={:?} certificates={:?} error={:?}",
                        actual.stage,
                        actual.best,
                        actual.readback,
                        actual.factors,
                        actual.certificates,
                        actual.error
                    );
                }
            }
        }
    }
    assert!(stages.iter().all(|s| s.iter().sum::<usize>() == 36));
    assert_eq!(stages[0], [0, 0, 4, 1, 0, 0, 31]);
    assert_eq!(stages[1..], [[0, 0, 3, 2, 0, 0, 31]; 2]);
    assert_eq!(improved, [1; 2]);
    assert_eq!(regressed, [1; 2]);
    println!(
        "material construction cold_inputs=36 cold_routes=108 stages_original_internal_inward_original_physical_internal_inward_inward_physical={stages:?} improved={improved:?} regressed={regressed:?}"
    );
}

fn continuous(fixture: &Prepared, direction: &[Wide]) -> f64 {
    let residual = fixture.directions.iter().map(|row| {
        row.iter()
            .zip(direction)
            .fold(Wide::default(), |sum, (&a, &v)| sum.add(a.mul(v)))
            .rounded()
    });
    vector_norm(residual).unwrap()
        / (fixture.value * vector_norm(direction.iter().map(|v| v.rounded())).unwrap())
}

#[test]
fn modal_material_construction_isolated_initializers_and_orders() {
    let mut routes = 0;
    let mut successes = [[0; 3]; 4];
    for (members, scale) in [(100, 1e-10), (128, 1.0), (128, 1e14), (128, 1e-10)] {
        let input = renumber(&Recipe::ThreeLayers.request(members, scale), 7);
        let layout = Rebuilt::prepare_bounded(&input).unwrap();
        let (fixture, refined) = prepare(layout.canonical).unwrap();
        assert!(!refined);
        let root = fixture.value.to_bits();
        let original = bits(&fixture.seed);
        let factor = Factor::prepare_shifted(&fixture.directions, fixture.value).unwrap();
        let directions = factor.directions(&fixture.seed, 4).unwrap();
        for (step, wide) in directions.iter().enumerate() {
            let direction = rounded(wide);
            let frozen = bits(&direction);
            let before = fixture.checked(&direction).unwrap().0;
            assert!(before > 1e-8);
            assert!(continuous(&fixture, wide) < 1e-8);
            if step == 3 {
                assert!(continuous(&fixture, wide) < 1e-13);
            }
            println!(
                "material construction members={members} scale={scale:e} step={} continuous={:e} rounded={before:e}",
                step + 1,
                continuous(&fixture, wide)
            );
            for (index, order) in [Order::GridNorm, Order::Reverse, Order::Natural]
                .into_iter()
                .enumerate()
            {
                let mut calls = 0;
                let mut best = f64::INFINITY;
                let (result, factors) = measured(|| {
                    search_canonical_direction_single(
                        &fixture.directions,
                        &direction,
                        1e-8,
                        order,
                        |v| {
                            calls += 1;
                            let certificate = fixture.checked(v)?;
                            best = best.min(certificate.0);
                            Ok(certificate)
                        },
                    )
                });
                assert_eq!(factors, 1);
                assert!(calls <= 7);
                match &result {
                    Ok(accepted) => {
                        assert_eq!(accepted.order, order);
                        assert_eq!(accepted.usage.attempts, 1);
                        assert_eq!(accepted.usage.certificates, calls);
                        assert_eq!(
                            accepted.shape[accepted.anchor].to_bits(),
                            direction[accepted.anchor].to_bits()
                        );
                        assert!(fixture.checked(&accepted.shape).unwrap().0 <= 1e-8);
                        successes[step][index] += 1;
                    }
                    Err(error) => {
                        assert!(error.contains("exhausted its bounded policies"), "{error}")
                    }
                }
                println!(
                    "material construction members={members} scale={scale:e} step={} order={order:?} accepted={} best={best:e} factors={factors} certificates={calls}",
                    step + 1,
                    result.is_ok()
                );
                assert_eq!(bits(&direction), frozen);
                routes += 1;
            }
        }
        assert_eq!(fixture.value.to_bits(), root);
        assert_eq!(bits(&fixture.seed), original);
    }
    assert_eq!(routes, 48);
    assert_eq!(successes, [[0; 3]; 4]);
    println!("material construction isolated_routes=48 accepted_by_step_and_order={successes:?}");
}

#[test]
fn modal_material_construction_isolated_internal_anchor_charts() {
    let offsets = [-8_i32, -4, -2, -1, 0, 1, 2, 4, 8];
    let mut successes = [[0; 3]; 9];
    let mut routes = 0;
    for (members, scale) in [(100, 1e-10), (128, 1.0), (128, 1e14), (128, 1e-10)] {
        let input = renumber(&Recipe::ThreeLayers.request(members, scale), 7);
        let layout = Rebuilt::prepare_bounded(&input).unwrap();
        let (fixture, refined) = prepare(layout.canonical).unwrap();
        assert!(!refined);
        let root = fixture.value.to_bits();
        let original = bits(&fixture.seed);
        let direction = rounded(
            &Factor::prepare_shifted(&fixture.directions, fixture.value)
                .unwrap()
                .final_direction(&fixture.seed, 4)
                .unwrap(),
        );
        let anchor = (0..direction.len())
            .max_by(|&a, &b| direction[a].abs().total_cmp(&direction[b].abs()))
            .unwrap();
        for (index, offset) in offsets.into_iter().enumerate() {
            // Each is an isolated cold chart; never unfreeze an active fit's anchor.
            let mut chart = direction.clone();
            let mut amplitude = chart[anchor].abs();
            for _ in 0..offset.unsigned_abs() {
                amplitude = if offset < 0 {
                    amplitude.next_down()
                } else {
                    amplitude.next_up()
                };
            }
            chart[anchor] = amplitude.copysign(chart[anchor]);
            assert!(
                chart
                    .iter()
                    .enumerate()
                    .all(|(i, v)| i == anchor || v.abs() < amplitude)
            );
            let frozen = bits(&chart);
            for (order_index, order) in [Order::GridNorm, Order::Reverse, Order::Natural]
                .into_iter()
                .enumerate()
            {
                let mut calls = 0;
                let mut best = f64::INFINITY;
                let (result, factors) = measured(|| {
                    search_canonical_direction_single(
                        &fixture.directions,
                        &chart,
                        1e-8,
                        order,
                        |v| {
                            calls += 1;
                            let certificate = fixture.checked(v)?;
                            best = best.min(certificate.0);
                            Ok(certificate)
                        },
                    )
                });
                assert_eq!(factors, 1);
                assert!(calls <= 7);
                if let Ok(accepted) = &result {
                    assert_eq!(accepted.anchor, anchor);
                    assert_eq!(accepted.shape[anchor].to_bits(), chart[anchor].to_bits());
                    assert_eq!(accepted.usage.attempts, 1);
                    assert_eq!(accepted.usage.certificates, calls);
                    assert!(fixture.checked(&accepted.shape).unwrap().0 <= 1e-8);
                    successes[index][order_index] += 1;
                } else {
                    assert!(
                        result
                            .as_ref()
                            .unwrap_err()
                            .contains("exhausted its bounded policies")
                    );
                }
                println!(
                    "material internal chart members={members} scale={scale:e} offset={offset} order={order:?} accepted={} best={best:e} factors={factors} certificates={calls}",
                    result.is_ok()
                );
                assert_eq!(bits(&chart), frozen);
                routes += 1;
            }
        }
        assert_eq!(fixture.value.to_bits(), root);
        assert_eq!(bits(&fixture.seed), original);
    }
    assert_eq!(routes, 108);
    assert_eq!(
        successes,
        [
            [0, 1, 0],
            [0, 1, 0],
            [0, 1, 0],
            [0, 1, 0],
            [0, 0, 0],
            [0, 1, 0],
            [0, 1, 0],
            [0, 0, 0],
            [0, 0, 0]
        ]
    );
    println!(
        "material internal chart isolated_routes=108 accepted_by_offset_and_order={successes:?}"
    );
}

#[test]
fn modal_material_construction_real_internal_chart_cancel_rejection_and_replay() {
    let mut cancellations = 0;
    let mut rejections = 0;
    for physical in [Chart::Original, Chart::Inward] {
        let input = renumber(&Recipe::ThreeLayers.request(100, 1e-10), 7);
        assert_eq!(run(&input, physical).stage, Stage::Internal);
        let baseline = run_charts(&input, Chart::Inward, physical);
        assert_eq!(baseline.stage, Stage::Accepted);
        assert_eq!(baseline.factors, [2, 1]);
        assert_eq!(baseline.certificates, [8, 4]);
        assert_eq!(baseline.order, Some(Order::Reverse));
        same_case(
            &baseline,
            &run_charts(&renumber(&input, 113), Chart::Inward, physical),
        );
        for (stage, step, expected) in [
            (SolverStage::ModalRoundoffSearch, 1, Stage::Internal),
            (SolverStage::ModalRoundoffValidate, 6, Stage::Internal),
            (
                SolverStage::ResultNodes,
                input.nodes.len() as u64,
                Stage::Readback,
            ),
            (SolverStage::ResultTotals, 2, Stage::Readback),
        ] {
            let control = SolverControl::default();
            let cancel = control.clone();
            let saw = Rc::new(Cell::new(false));
            let observed = saw.clone();
            let mut stopped = Record::new();
            let error = with_solver_observer(
                &control,
                move |p| {
                    if p.stage == stage && p.completed_steps == step {
                        observed.set(true);
                        cancel.request_cancel();
                    }
                },
                || stopped.attempt_charts(&input, Chart::Inward, physical),
            )
            .unwrap_err();
            assert!(saw.get() && error.contains("cancel"), "{error}");
            assert_eq!(stopped.stage, expected);
            assert!(stopped.output.is_none());
            if expected == Stage::Internal {
                assert_eq!(stopped.factors[1], 0);
                assert_eq!(stopped.certificates[1], 0);
                assert!(stopped.readback.is_none());
            }
            same_case(&baseline, &run_charts(&input, Chart::Inward, physical));
            cancellations += 1;
        }
        for (recipe, scale, expected) in [
            (Recipe::ThreeLayers, 1e-10, Stage::Internal),
            (Recipe::UnequalLengths, 1e14, Stage::Physical),
        ] {
            let failed = renumber(&recipe.request(128, scale), 7);
            let rejected = run_charts(&failed, Chart::Inward, physical);
            assert_eq!(rejected.stage, expected);
            assert!(rejected.output.is_none() && rejected.readback.is_none());
            same_case(
                &rejected,
                &run_charts(&renumber(&failed, 113), Chart::Inward, physical),
            );
            same_case(&baseline, &run_charts(&input, Chart::Inward, physical));
            rejections += 1;
        }
    }
    assert_eq!(cancellations, 8);
    assert_eq!(rejections, 4);
    println!(
        "material internal chart real_restored_numbering_replays=2 cancellations=8 exact_healthy_replays=8 stage_rejections=4 failed_numbering_replays=4 healthy_after_failure_replays=4 no_partial_output=1"
    );
}
