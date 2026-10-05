use super::*;

#[path = "modal_roundoff_material_iterated_beam_control_tests.rs"]
mod controls;

fn attempt_extended(record: &mut Record, input: &SolveModalFrame2dRequest) -> Result<(), String> {
    let layout = Rebuilt::prepare_bounded(input)?;
    let (fixture, refined) = prepare(layout.canonical.clone())?;
    record.refined = refined;
    record.root = Some(fixture.value.to_bits());
    record.seed = bits(&fixture.seed);
    record.stage = Stage::Initializer;
    let direction = rounded(
        &Factor::prepare_shifted(&fixture.directions, fixture.value)?
            .final_direction(&fixture.seed, 4)?,
    );
    record.stage = Stage::Internal;
    let (selected, factors) = measured(|| {
        let mut eligible = false;
        let old = search_canonical_direction_ranked(&fixture.directions, &direction, 1e-8, |v| {
            if record.certificates[0] >= 79 {
                return Err("extended internal exhausted its aggregate receipt budget".into());
            }
            record.certificates[0] += 1;
            let receipt = fixture.checked(v)?;
            eligible |= receipt.0 <= 1e-8;
            Ok(receipt)
        });
        match old {
            Ok(accepted) => return Ok((accepted.shape, accepted.order)),
            Err(error)
                if error
                    .starts_with("triangular grid portfolio exhausted its bounded policies") => {}
            Err(error) => return Err(error),
        }
        if eligible {
            return Err("supplementary internal proposal lost a passing legacy receipt".into());
        }
        checkpoint(SolverStage::ModalRoundoffPrepare, 4)?;
        // Cold, preselected supplementary construction, never recipe-specific dispatch.
        let case = Case::new(&fixture, &direction);
        let fit = GridFit::prepare(
            &case.matrix,
            &case.seed,
            case.anchor,
            &case.order(Order::Reverse),
        )?;
        let actual = fit.attempt_iterated_beam(1 << 22, 64, 1, 1e-8, |v| {
            if record.certificates[0] >= 79 {
                return Err("extended internal exhausted its aggregate receipt budget".into());
            }
            record.certificates[0] += 1;
            let (relative, residual) = fixture.checked(&case.restore(v))?;
            Ok((
                relative,
                case.permutation.iter().map(|&i| residual[i]).collect(),
            ))
        })?;
        match actual {
            Attempt::Accepted(shape) => {
                assert_eq!(
                    shape[case.anchor].to_bits(),
                    case.seed[case.anchor].to_bits()
                );
                Ok((case.restore(&shape), Order::Reverse))
            }
            rejected => Err(format!(
                "supplementary internal proposal numerically rejected: {rejected:?}"
            )),
        }
    });
    record.factors[0] = factors;
    assert!(factors <= 3 && record.certificates[0] <= 79);
    assert_eq!(fixture.value.to_bits(), record.root.unwrap());
    assert_eq!(bits(&fixture.seed), record.seed);
    let (internal, order) = selected?;
    record.order = Some(order);
    finish_hybrid(record, fixture, layout, &internal, order)
}

fn run_extended(input: &SolveModalFrame2dRequest) -> Record {
    let frozen = serde_json::to_vec(input).unwrap();
    let mut record = Record::new();
    if let Err(error) = attempt_extended(&mut record, input) {
        assert!(record.output.is_none());
        record.error = Some(error);
        record.readback = None;
    }
    assert_eq!(serde_json::to_vec(input).unwrap(), frozen);
    record
}

fn same(old: &Record, new: &Record) {
    assert_eq!(old.root, new.root);
    assert_eq!(old.seed, new.seed);
    assert_eq!(old.order, new.order);
    assert_eq!(old.refined, new.refined);
    assert_eq!(old.factors, new.factors);
    assert_eq!(old.certificates, new.certificates);
    assert_eq!(
        old.readback.map(f64::to_bits),
        new.readback.map(f64::to_bits)
    );
    assert_eq!(
        serde_json::to_vec(&old.output).unwrap(),
        serde_json::to_vec(&new.output).unwrap()
    );
}

#[test]
fn modal_material_iterated_beam_full_legacy_retention_and_readback() {
    let mut stages = [[0; 7]; 2];
    let mut exact = 0;
    let mut gains = 0;
    let mut inputs = Vec::new();
    for recipe in Recipe::ALL {
        for members in [80, 100, 128] {
            for scale in [1.0, 1e14, 1e-10] {
                inputs.push(recipe.request(members, scale));
            }
        }
    }
    for profile in [Profile::Graded, Profile::Layered] {
        for scale in [1.0, 1e14, 1e-10] {
            inputs.push(Fixture::request(profile, 128, scale));
        }
    }
    for (index, input) in inputs.iter().enumerate() {
        let input = renumber(input, 7);
        let old = run_ablation(&input, false, Tail::Hybrid);
        let new = run_extended(&input);
        assert_eq!(new.root, old.root);
        assert_eq!(new.seed, old.seed);
        assert_eq!(new.output.is_some(), new.stage == Stage::Accepted);
        assert_eq!(new.readback.is_some(), new.output.is_some());
        stages[0][old.stage as usize] += 1;
        stages[1][new.stage as usize] += 1;
        if old.stage == Stage::Accepted {
            same(&old, &new);
            exact += 1;
        } else if new.stage == Stage::Accepted {
            assert_eq!(index, 23);
            assert_eq!(new.factors, [3, 1]);
            assert!(new.readback.unwrap() <= 1e-8);
            gains += 1;
        }
        assert_eq!(
            new.stage,
            if (24..=26).contains(&index) {
                Stage::Internal
            } else {
                Stage::Accepted
            },
            "input={index}: {new:?}"
        );
        println!(
            "material iterated extension input={index} stage={:?} order={:?} readback={:?} factors={:?} certificates={:?} error={:?}",
            new.stage, new.order, new.readback, new.factors, new.certificates, new.error
        );
    }
    assert_eq!(stages, [[0, 0, 4, 0, 0, 0, 38], [0, 0, 3, 0, 0, 0, 39]]);
    assert_eq!(exact, 38);
    assert_eq!(gains, 1);
    println!(
        "material iterated extension full_inputs=42 cold_routes=84 exact_legacy_outputs=38 new_physical_outputs=1 lost_outputs=0 stages={stages:?} production_admission=unchanged"
    );
}
