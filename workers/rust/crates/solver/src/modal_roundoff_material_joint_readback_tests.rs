use super::*;

#[path = "modal_roundoff_material_joint_control_tests.rs"]
mod controls;

fn attempt(
    record: &mut Record,
    input: &SolveModalFrame2dRequest,
    order: Order,
) -> Result<(), String> {
    let layout = Rebuilt::prepare_bounded(input)?;
    let (fixture, refined) = prepare(layout.canonical.clone())?;
    record.refined = refined;
    record.root = Some(fixture.value.to_bits());
    record.seed = bits(&fixture.seed);
    record.stage = Stage::Initializer;
    let wide = Factor::prepare_shifted(&fixture.directions, fixture.value)?
        .final_direction(&fixture.seed, 4)?;
    let case = PhysicalCase::new(&fixture, physical_seed(&fixture, &wide), &rounded(&wide));
    record.stage = Stage::Physical;
    record.order = Some(order);
    let (actual, factors) = measured(|| {
        let fit = GridFit::prepare(&case.matrix, &case.seed, case.anchor, &case.order(order))?;
        fit.attempt_rounded_joint(1 << 22, 16, 1e-8, |v| {
            record.certificates[0] += 1;
            record.certificates[1] += 1;
            let receipt = case.checked(&fixture, v)?;
            record.best = record.best.min(receipt.physical.max(receipt.internal));
            Ok(receipt)
        })
    });
    record.factors[1] = factors;
    assert!(factors <= 1 && record.certificates.iter().all(|&n| n <= 18));
    assert_eq!(fixture.value.to_bits(), record.root.unwrap());
    assert_eq!(bits(&fixture.seed), record.seed);
    let shape = match actual? {
        Outcome::Accepted(shape) => shape,
        Outcome::Rejected {
            physical,
            internal,
            norm,
        } => {
            return Err(format!(
                "joint physical-first rejection physical={physical:e} internal={internal:e} norm={norm:e}"
            ));
        }
    };
    assert_eq!(
        shape[case.anchor].to_bits(),
        case.seed[case.anchor].to_bits()
    );
    let shape = case.restore(&shape);
    if (crate::modal_math::checked_shape_norm(&shape)? - 1.0).abs() >= 1e-10 {
        return Err("joint restored unit shape exceeds its unchanged norm gate".into());
    }
    record.publish(fixture, layout, &shape)
}

fn run_joint(input: &SolveModalFrame2dRequest, order: Order) -> Record {
    let frozen = serde_json::to_vec(input).unwrap();
    let mut record = Record::new();
    if let Err(error) = attempt(&mut record, input, order) {
        assert!(record.output.is_none());
        record.readback = None;
        record.error = Some(error);
    }
    assert_eq!(serde_json::to_vec(input).unwrap(), frozen);
    record
}

fn compare(input: &SolveModalFrame2dRequest) -> [Stage; 3] {
    let old = run(input, Chart::Original);
    let mut boundary = [old.stage; 3];
    for (index, order) in [Order::GridNorm, Order::Reverse].into_iter().enumerate() {
        let record = run_joint(input, order);
        assert_eq!(record.root, old.root);
        assert_eq!(record.seed, old.seed);
        assert_eq!(record.output.is_some(), record.stage == Stage::Accepted);
        assert_eq!(record.readback.is_some(), record.output.is_some());
        boundary[index + 1] = record.stage;
        println!(
            "material joint readback order={order:?} stage={:?} best_joint={:e} readback={:?} factors={:?} certificates={:?} error={:?}",
            record.stage,
            record.best,
            record.readback,
            record.factors,
            record.certificates,
            record.error
        );
    }
    boundary
}

#[test]
fn modal_material_joint_full_readback_recipe_mesh_comparison() {
    let mut boundaries = Vec::new();
    for recipe in Recipe::ALL {
        for members in [80, 100, 128] {
            for scale in [1.0, 1e14, 1e-10] {
                let boundary = compare(&renumber(&recipe.request(members, scale), 7));
                let old = if matches!(recipe, Recipe::ThreeLayers)
                    && (members == 128 || (members == 100 && scale == 1e-10))
                {
                    Stage::Internal
                } else if matches!(recipe, Recipe::UnequalLengths)
                    && members == 128
                    && scale == 1e-10
                {
                    Stage::Physical
                } else {
                    Stage::Accepted
                };
                let accepted = matches!(recipe, Recipe::Jittered) && members == 80;
                assert_eq!(
                    boundary,
                    [
                        old,
                        if accepted && scale == 1e-10 {
                            Stage::Accepted
                        } else {
                            Stage::Physical
                        },
                        if accepted && scale != 1e14 {
                            Stage::Accepted
                        } else {
                            Stage::Physical
                        }
                    ],
                    "{recipe:?}/{members}/{scale:e}"
                );
                println!(
                    "material joint full boundary recipe={recipe:?} members={members} scale={scale:e} stages={boundary:?}"
                );
                boundaries.push(boundary);
            }
        }
    }
    for profile in [Profile::Graded, Profile::Layered] {
        for scale in [1.0, 1e14, 1e-10] {
            let boundary = compare(&renumber(&Fixture::request(profile, 128, scale), 7));
            assert_eq!(
                boundary,
                [Stage::Accepted, Stage::Physical, Stage::Physical]
            );
            println!(
                "material joint baseline profile={profile:?} scale={scale:e} stages={boundary:?}"
            );
            boundaries.push(boundary);
        }
    }
    let mut stages = [[0; 7]; 3];
    for b in &boundaries {
        for (policy, &stage) in b.iter().enumerate() {
            stages[policy][stage as usize] += 1;
        }
    }
    assert_eq!(boundaries.len(), 42);
    assert_eq!(
        stages,
        [
            [0, 0, 4, 1, 0, 0, 37],
            [0, 0, 0, 41, 0, 0, 1],
            [0, 0, 0, 40, 0, 0, 2]
        ]
    );
    println!(
        "material joint full_inputs=42 cold_routes=126 stages_old_gridnorm_reverse={stages:?} boundaries={boundaries:?} production_admission=unchanged"
    );
}
