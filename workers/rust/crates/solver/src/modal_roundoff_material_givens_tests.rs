use super::*;

#[derive(Debug)]
struct Observation {
    stage: u8, // 0: guarded factor fault, 1: numerical rejection, 2: normalized acceptance.
    best: f64,
    certificates: usize,
    shape: Option<Vec<f64>>,
    error: Option<String>,
}

fn observe(fixture: &Prepared, case: &Case, order: Order, givens: bool) -> Observation {
    let mut observation = Observation {
        stage: 0,
        best: f64::INFINITY,
        certificates: 0,
        shape: None,
        error: None,
    };
    let (result, factors) = measured(|| {
        let indices = case.order(order);
        let fit = if givens {
            GridFit::prepare_givens(&case.matrix, &case.seed, case.anchor, &indices, false)
        } else {
            GridFit::prepare(&case.matrix, &case.seed, case.anchor, &indices)
        }?;
        let checked = |v: &[f64]| {
            observation.certificates += 1;
            let (relative, residual) = fixture.checked(&case.restore(v))?;
            observation.best = observation.best.min(relative);
            Ok((
                relative,
                case.permutation.iter().map(|&i| residual[i]).collect(),
            ))
        };
        if givens {
            fit.attempt_wide_beam(1 << 22, 1e-8, checked)
        } else {
            fit.attempt_direction(1 << 22, 1e-8, checked)
        }
    });
    assert_eq!(factors, 1);
    assert!(observation.certificates <= 6);
    match result {
        Err(error) => {
            assert!(
                givens && error.starts_with("wide Givens result exceeds its unchanged range;"),
                "{error}"
            );
            assert_eq!(observation.certificates, 0);
            observation.error = Some(error);
        }
        Ok(Attempt::Rejected(Rejection::Residual(relative))) => {
            assert!(relative > 1e-8);
            assert_eq!(relative.to_bits(), observation.best.to_bits());
            observation.stage = 1;
        }
        Ok(Attempt::Accepted(shape)) => {
            assert_eq!(
                shape[case.anchor].to_bits(),
                case.seed[case.anchor].to_bits()
            );
            let restored = case.restore(&shape);
            assert!(fixture.checked(&restored).unwrap().0 <= 1e-8);
            assert!((0.25..=2.0).contains(&vector_norm(restored.iter().copied()).unwrap()));
            observation.stage = 2;
            observation.shape = Some(restored);
        }
        Ok(Attempt::Rejected(Rejection::UnitNorm(_))) => unreachable!(),
    }
    observation
}

fn prepared(recipe: Recipe, members: usize, scale: f64, key: u64) -> (Prepared, Case) {
    let layout = Rebuilt::prepare_bounded(&renumber(&recipe.request(members, scale), key)).unwrap();
    let (fixture, _) = prepare(layout.canonical).unwrap();
    let direction = rounded(
        &Factor::prepare_shifted(&fixture.directions, fixture.value)
            .unwrap()
            .final_direction(&fixture.seed, 4)
            .unwrap(),
    );
    let case = Case::new(&fixture, &direction);
    (fixture, case)
}

#[test]
fn modal_material_construction_givens_recipe_mesh_isolated_comparison() {
    let mut stages = [[0; 3]; 2];
    let mut pairs = [0; 4];
    let mut boundaries = Vec::new();
    for recipe in Recipe::ALL {
        for members in [80, 100, 128] {
            for scale in [1.0, 1e14, 1e-10] {
                let (fixture, case) = prepared(recipe, members, scale, 7);
                let frozen_seed = bits(&fixture.seed);
                let frozen_root = fixture.value.to_bits();
                let mut boundary = [0; 4];
                for (index, order) in [Order::GridNorm, Order::Reverse].into_iter().enumerate() {
                    let original = observe(&fixture, &case, order, false);
                    let givens = observe(&fixture, &case, order, true);
                    boundary[2 * index] = original.stage;
                    boundary[2 * index + 1] = givens.stage;
                    for (policy, observation) in [&original, &givens].into_iter().enumerate() {
                        stages[policy][observation.stage as usize] += 1;
                        println!(
                            "material givens recipe={recipe:?} members={members} scale={scale:e} order={order:?} policy={policy} stage={} best={:e} certificates={} error={:?}",
                            observation.stage,
                            observation.best,
                            observation.certificates,
                            observation.error
                        );
                    }
                    pairs[match (original.stage == 2, givens.stage == 2) {
                        (true, true) => 0,
                        (false, true) => 1,
                        (true, false) => 2,
                        (false, false) => 3,
                    }] += 1;
                }
                assert_eq!(bits(&fixture.seed), frozen_seed);
                assert_eq!(fixture.value.to_bits(), frozen_root);
                boundaries.push(boundary);
            }
        }
    }
    assert_eq!(boundaries.len(), 36);
    assert!(stages.iter().all(|s| s.iter().sum::<usize>() == 72));
    assert_eq!(stages, [[0, 20, 52], [5, 17, 50]]);
    assert_eq!(pairs, [47, 3, 5, 17]);
    assert_eq!(
        boundaries,
        [
            [2, 2, 2, 2],
            [2, 2, 2, 2],
            [2, 2, 2, 2],
            [2, 2, 1, 1],
            [2, 2, 1, 1],
            [2, 2, 1, 1],
            [2, 2, 1, 1],
            [2, 0, 1, 1],
            [2, 1, 1, 1],
            [2, 2, 2, 2],
            [2, 2, 2, 2],
            [2, 2, 2, 2],
            [2, 2, 2, 2],
            [2, 2, 2, 2],
            [2, 2, 2, 2],
            [2, 2, 1, 2],
            [2, 0, 1, 1],
            [2, 2, 2, 1],
            [2, 2, 2, 2],
            [2, 2, 2, 2],
            [2, 2, 2, 2],
            [2, 2, 1, 2],
            [2, 2, 1, 2],
            [1, 1, 1, 1],
            [1, 0, 1, 1],
            [1, 0, 1, 1],
            [1, 1, 1, 1],
            [2, 2, 2, 2],
            [2, 2, 2, 0],
            [2, 2, 2, 2],
            [2, 2, 2, 2],
            [2, 2, 2, 2],
            [2, 2, 2, 2],
            [2, 2, 1, 1],
            [2, 2, 2, 2],
            [2, 2, 1, 1],
        ]
    );
    println!(
        "material givens isolated_inputs=36 isolated_routes=144 stages={stages:?} pairs={pairs:?} boundaries={boundaries:?} physical_publications=0"
    );
}

fn same_observation(actual: &Observation, reference: &Observation) {
    assert_eq!(actual.stage, reference.stage);
    assert_eq!(actual.best.to_bits(), reference.best.to_bits());
    assert_eq!(actual.certificates, reference.certificates);
    assert_eq!(actual.error, reference.error);
    assert_eq!(
        actual.shape.as_ref().map(|v| bits(v)),
        reference.shape.as_ref().map(|v| bits(v))
    );
}

#[test]
fn modal_material_construction_givens_real_fault_cancel_sign_numbering_replay() {
    let (fixture, case) = prepared(Recipe::Jittered, 128, 1.0, 7);
    let frozen_seed = bits(&fixture.seed);
    let frozen_root = fixture.value.to_bits();
    let baseline = observe(&fixture, &case, Order::GridNorm, true);
    assert_eq!(baseline.stage, 2);
    for amplitude in [-1.0, 0.5] {
        let mut mapped = Case::new(&fixture, &case.restore(&case.seed));
        for value in &mut mapped.seed {
            *value *= amplitude;
        }
        let actual = observe(&fixture, &mapped, Order::GridNorm, true);
        assert_eq!(actual.stage, 2);
        assert_eq!(actual.certificates, baseline.certificates);
        assert_eq!(actual.best.to_bits(), baseline.best.to_bits());
        assert_eq!(
            bits(
                &actual
                    .shape
                    .unwrap()
                    .iter()
                    .map(|v| v / amplitude)
                    .collect::<Vec<_>>()
            ),
            bits(baseline.shape.as_ref().unwrap())
        );
    }
    let (mapped_fixture, mapped_case) = prepared(Recipe::Jittered, 128, 1.0, 113);
    same_observation(
        &observe(&mapped_fixture, &mapped_case, Order::GridNorm, true),
        &baseline,
    );
    for (stage, steps, expected_calls) in [
        (SolverStage::DenseFactor, 2, 0),
        (SolverStage::ModalVectorUpdate, 256, 0),
        (SolverStage::DenseSubstitution, 0, 1),
        (SolverStage::ModalRoundoffSearch, 5, 5),
        (SolverStage::ModalRoundoffValidate, 1, 6),
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
                let fit = GridFit::prepare_givens(
                    &case.matrix,
                    &case.seed,
                    case.anchor,
                    &case.order(Order::GridNorm),
                    false,
                )?;
                fit.attempt_wide_beam(1 << 22, 1e-8, |v| {
                    calls += 1;
                    let (relative, residual) = fixture.checked(&case.restore(v))?;
                    Ok((
                        relative,
                        case.permutation.iter().map(|&i| residual[i]).collect(),
                    ))
                })
            },
        )
        .unwrap_err();
        assert!(error.contains("cancel"), "{stage:?}: {error}");
        assert_eq!(calls, expected_calls);
        let (fresh_fixture, fresh_case) = prepared(Recipe::Jittered, 128, 1.0, 7);
        same_observation(
            &observe(&fresh_fixture, &fresh_case, Order::GridNorm, true),
            &baseline,
        );
    }
    for (recipe, members, scale, order) in [
        (Recipe::UnequalLengths, 128, 1e14, Order::GridNorm),
        (Recipe::NonbinaryGradient, 128, 1e14, Order::GridNorm),
        (Recipe::ThreeLayers, 128, 1.0, Order::GridNorm),
        (Recipe::ThreeLayers, 128, 1e14, Order::GridNorm),
        (Recipe::Jittered, 80, 1e14, Order::Reverse),
    ] {
        let (failed_fixture, failed_case) = prepared(recipe, members, scale, 7);
        let failed = observe(&failed_fixture, &failed_case, order, true);
        assert_eq!(failed.stage, 0);
        assert!(failed.shape.is_none() && failed.certificates == 0);
        let (fresh_fixture, fresh_case) = prepared(recipe, members, scale, 113);
        same_observation(&observe(&fresh_fixture, &fresh_case, order, true), &failed);
        let (healthy_fixture, healthy_case) = prepared(Recipe::Jittered, 128, 1.0, 7);
        same_observation(
            &observe(&healthy_fixture, &healthy_case, Order::GridNorm, true),
            &baseline,
        );
    }
    assert_eq!(bits(&fixture.seed), frozen_seed);
    assert_eq!(fixture.value.to_bits(), frozen_root);
    println!(
        "material givens real_controls sign_binary_checks=2 numbering_replay=1 cancellations=5 healthy_cancel_replays=5 failed_numbering_replays=5 healthy_after_fault=5 no_partial_candidate=1 physical_publications=0"
    );
}
