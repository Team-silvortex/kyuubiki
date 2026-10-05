use super::*;

#[path = "modal_roundoff_material_rounded_beam_readback_tests.rs"]
mod readback_tests;

#[path = "modal_roundoff_material_joint_tests.rs"]
mod joint_tests;

#[path = "modal_roundoff_material_coupled_tests.rs"]
mod coupled_tests;

fn observe(
    fixture: &Prepared,
    case: &Case,
    order: Order,
    beam: bool,
) -> (bool, f64, usize, Option<Vec<f64>>) {
    let mut calls = 0;
    let mut best = f64::INFINITY;
    let (result, factors) = measured(|| {
        let fit = GridFit::prepare(&case.matrix, &case.seed, case.anchor, &case.order(order))?;
        let checked = |v: &[f64]| {
            calls += 1;
            let (relative, residual) = fixture.checked(&case.restore(v))?;
            best = best.min(relative);
            Ok((
                relative,
                case.permutation.iter().map(|&i| residual[i]).collect(),
            ))
        };
        if beam {
            fit.attempt_rounded_beam(1 << 22, 16, 1e-8, checked)
        } else {
            fit.attempt_direction(1 << 22, 1e-8, checked)
        }
    });
    assert_eq!(factors, 1);
    assert!(calls <= if beam { 18 } else { 6 });
    let shape = match result.unwrap() {
        Attempt::Rejected(Rejection::Residual(relative)) => {
            assert!(relative > 1e-8);
            assert_eq!(relative.to_bits(), best.to_bits());
            None
        }
        Attempt::Accepted(shape) => {
            assert_eq!(
                shape[case.anchor].to_bits(),
                case.seed[case.anchor].to_bits()
            );
            let restored = case.restore(&shape);
            assert!(fixture.checked(&restored).unwrap().0 <= 1e-8);
            Some(restored)
        }
        Attempt::Rejected(Rejection::UnitNorm(_)) => unreachable!(),
    };
    (shape.is_some(), best, calls, shape)
}

fn compare(input: &SolveModalFrame2dRequest) -> [bool; 4] {
    let frozen = serde_json::to_vec(input).unwrap();
    let layout = Rebuilt::prepare_bounded(input).unwrap();
    let (fixture, _) = prepare(layout.canonical).unwrap();
    let root = fixture.value.to_bits();
    let seed = bits(&fixture.seed);
    let direction = rounded(
        &Factor::prepare_shifted(&fixture.directions, fixture.value)
            .unwrap()
            .final_direction(&fixture.seed, 4)
            .unwrap(),
    );
    let case = Case::new(&fixture, &direction);
    let mut boundary = [false; 4];
    for (index, order) in [Order::GridNorm, Order::Reverse].into_iter().enumerate() {
        for (policy, beam) in [false, true].into_iter().enumerate() {
            let (accepted, best, calls, _) = observe(&fixture, &case, order, beam);
            boundary[2 * index + policy] = accepted;
            println!(
                "material rounded beam comparison order={order:?} beam={beam} accepted={accepted} best={best:e} certificates={calls}"
            );
        }
    }
    assert_eq!(fixture.value.to_bits(), root);
    assert_eq!(bits(&fixture.seed), seed);
    assert_eq!(serde_json::to_vec(input).unwrap(), frozen);
    boundary
}

#[test]
fn modal_material_rounded_beam_recipe_mesh_isolated_comparison() {
    let mut boundaries = Vec::new();
    for recipe in Recipe::ALL {
        for members in [80, 100, 128] {
            for scale in [1.0, 1e14, 1e-10] {
                let boundary = compare(&renumber(&recipe.request(members, scale), 7));
                println!(
                    "material rounded beam boundary recipe={recipe:?} members={members} scale={scale:e} passed={boundary:?}"
                );
                boundaries.push(boundary);
            }
        }
    }
    for profile in [Profile::Graded, Profile::Layered] {
        for scale in [1.0, 1e14, 1e-10] {
            let boundary = compare(&renumber(&Fixture::request(profile, 128, scale), 7));
            println!(
                "material rounded beam baseline profile={profile:?} scale={scale:e} passed={boundary:?}"
            );
            boundaries.push(boundary);
        }
    }
    assert_eq!(boundaries.len(), 42);
    let mut pairs = [0; 4];
    for b in &boundaries {
        for offset in [0, 2] {
            pairs[match (b[offset], b[offset + 1]) {
                (true, true) => 0,
                (false, true) => 1,
                (true, false) => 2,
                (false, false) => 3,
            }] += 1;
        }
    }
    assert_eq!(pairs, [58, 10, 1, 15]);
    assert_eq!(
        boundaries,
        [
            [true, true, true, true],
            [true, true, true, true],
            [true, true, true, true],
            [true, true, false, false],
            [true, true, false, false],
            [true, true, false, false],
            [true, true, false, false],
            [true, true, false, false],
            [true, true, false, false],
            [true, true, true, true],
            [true, true, true, true],
            [true, true, true, true],
            [true, true, true, true],
            [true, true, true, true],
            [true, true, true, true],
            [true, true, false, true],
            [true, true, false, true],
            [true, true, true, true],
            [true, true, true, true],
            [true, true, true, true],
            [true, true, true, true],
            [true, true, false, true],
            [true, true, false, true],
            [false, false, false, true],
            [false, false, false, false],
            [false, false, false, false],
            [false, false, false, false],
            [true, true, true, true],
            [true, true, true, true],
            [true, true, true, true],
            [true, true, true, true],
            [true, true, true, true],
            [true, true, true, true],
            [true, true, false, true],
            [true, true, true, false],
            [true, true, false, true],
            [true, true, false, true],
            [true, true, false, false],
            [true, true, false, true],
            [true, true, false, true],
            [true, true, true, true],
            [false, false, true, true],
        ]
    );
    println!(
        "material rounded beam cold_inputs=42 cold_routes=168 same_accept_gain_loss_same_reject={pairs:?} boundaries={boundaries:?} physical_publications=0 production_admission=unchanged"
    );
}

#[test]
fn modal_material_rounded_beam_isolated_counterexamples() {
    let mut accepted = [[0; 4]; 2];
    for (members, scale) in [(100, 1e-10), (128, 1.0), (128, 1e14), (128, 1e-10)] {
        let input = renumber(&Recipe::ThreeLayers.request(members, scale), 7);
        let layout = Rebuilt::prepare_bounded(&input).unwrap();
        let (fixture, _) = prepare(layout.canonical).unwrap();
        let direction = rounded(
            &Factor::prepare_shifted(&fixture.directions, fixture.value)
                .unwrap()
                .final_direction(&fixture.seed, 4)
                .unwrap(),
        );
        let case = Case::new(&fixture, &direction);
        for order in [Order::GridNorm, Order::Reverse, Order::Natural] {
            for (index, width) in [1, 4, 16, 64].into_iter().enumerate() {
                for (radius_index, radius) in [4, 1 << 22].into_iter().enumerate() {
                    let mut calls = 0;
                    let mut best = f64::INFINITY;
                    let (actual, factors) = measured(|| {
                        let fit = GridFit::prepare(
                            &case.matrix,
                            &case.seed,
                            case.anchor,
                            &case.order(order),
                        )?;
                        fit.attempt_rounded_beam(radius, width, 1e-8, |v| {
                            calls += 1;
                            let restored = case.restore(v);
                            let (relative, residual) = fixture.checked(&restored)?;
                            best = best.min(relative);
                            Ok((
                                relative,
                                case.permutation.iter().map(|&i| residual[i]).collect(),
                            ))
                        })
                    });
                    assert_eq!(factors, 1);
                    assert!(calls <= width + 2);
                    assert_eq!(
                        matches!(actual, Ok(Attempt::Accepted(_))),
                        members == 100
                            && matches!(order, Order::Reverse)
                            && width >= 16
                            && radius_index == 1
                    );
                    if let Ok(Attempt::Accepted(shape)) = &actual {
                        assert_eq!(
                            shape[case.anchor].to_bits(),
                            case.seed[case.anchor].to_bits()
                        );
                        assert!(fixture.checked(&case.restore(shape)).unwrap().0 <= 1e-8);
                        accepted[radius_index][index] += 1;
                    }
                    assert!(actual.is_ok(), "{:?}", actual.as_ref().err());
                    println!(
                        "material rounded beam members={members} scale={scale:e} order={order:?} width={width} radius={radius} accepted={} best={best:e} factors={factors} certificates={calls}",
                        matches!(actual, Ok(Attempt::Accepted(_)))
                    );
                }
            }
        }
    }
    assert_eq!(accepted, [[0, 0, 0, 0], [0, 0, 1, 1]]);
    println!(
        "material rounded beam difficult_inputs=4 isolated_routes=96 accepted_by_radius_and_width={accepted:?} physical_publications=0 production_admission=unchanged"
    );
}
