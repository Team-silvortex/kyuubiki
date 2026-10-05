use super::*;

fn attempt(
    record: &mut Record,
    input: &SolveModalFrame2dRequest,
    chart: Chart,
) -> Result<(), String> {
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
    let case = Case::new(&fixture, &direction);
    record.stage = Stage::Internal;
    let (result, factors) = measured(|| {
        let fit = GridFit::prepare(
            &case.matrix,
            &case.seed,
            case.anchor,
            &case.order(Order::Reverse),
        )?;
        fit.attempt_rounded_beam(1 << 22, 16, 1e-8, |v| {
            record.certificates[0] += 1;
            let (relative, residual) = fixture.checked(&case.restore(v))?;
            Ok((
                relative,
                case.permutation.iter().map(|&i| residual[i]).collect(),
            ))
        })
    });
    record.factors[0] = factors;
    assert!(factors <= 1 && record.certificates[0] <= 18);
    assert_eq!(fixture.value.to_bits(), record.root.unwrap());
    assert_eq!(bits(&fixture.seed), record.seed);
    let internal = match result? {
        Attempt::Accepted(shape) => shape,
        rejected => {
            return Err(format!(
                "rounded beam rejected internal direction: {rejected:?}"
            ));
        }
    };
    assert_eq!(
        internal[case.anchor].to_bits(),
        case.seed[case.anchor].to_bits()
    );
    record.order = Some(Order::Reverse);
    record.finish(
        fixture,
        layout,
        &case.restore(&internal),
        Order::Reverse,
        chart,
    )
}

fn run_beam(input: &SolveModalFrame2dRequest, chart: Chart) -> Record {
    let frozen = serde_json::to_vec(input).unwrap();
    let mut record = Record::new();
    if let Err(error) = attempt(&mut record, input, chart) {
        assert!(record.output.is_none());
        record.readback = None;
        record.error = Some(error);
    }
    assert_eq!(serde_json::to_vec(input).unwrap(), frozen);
    record
}

#[test]
fn modal_material_rounded_beam_physical_numbering_json_readback() {
    let mut stages = Vec::new();
    for key in [7, 113] {
        for (members, scale) in [(100, 1e-10), (128, 1.0), (128, 1e14), (128, 1e-10)] {
            let input = renumber(&Recipe::ThreeLayers.request(members, scale), key);
            for chart in [Chart::Original, Chart::Inward] {
                let record = run_beam(&input, chart);
                println!(
                    "material rounded beam readback key={key} members={members} scale={scale:e} physical={chart:?} stage={:?} best={:e} readback={:?} factors={:?} certificates={:?} error={:?}",
                    record.stage,
                    record.best,
                    record.readback,
                    record.factors,
                    record.certificates,
                    record.error
                );
                assert_eq!(record.output.is_some(), record.stage == Stage::Accepted);
                assert_eq!(record.readback.is_some(), record.output.is_some());
                assert_eq!(
                    record.stage,
                    if members == 100 {
                        Stage::Physical
                    } else {
                        Stage::Internal
                    }
                );
                assert_eq!(record.factors, if members == 100 { [1, 1] } else { [1, 0] });
                assert_eq!(
                    record.certificates,
                    if members == 100 { [18, 4] } else { [18, 0] }
                );
                stages.push(record.stage);
            }
        }
    }
    println!(
        "material rounded beam physical_readback_routes=16 stages={stages:?} production_admission=unchanged"
    );
}

#[test]
fn modal_material_rounded_beam_real_final_fault_cancel_sign_numbering_replay() {
    let layout =
        Rebuilt::prepare_bounded(&renumber(&Recipe::ThreeLayers.request(100, 1e-10), 7)).unwrap();
    let (fixture, _) = prepare(layout.canonical).unwrap();
    let root = fixture.value.to_bits();
    let frozen = bits(&fixture.seed);
    let direction = rounded(
        &Factor::prepare_shifted(&fixture.directions, fixture.value)
            .unwrap()
            .final_direction(&fixture.seed, 4)
            .unwrap(),
    );
    let case = Case::new(&fixture, &direction);
    let baseline = observe(&fixture, &case, Order::Reverse, true);
    assert!(baseline.0);
    assert_eq!(baseline.2, 18);
    for scale in [-1.0, 0.5] {
        let signed = Case::new(
            &fixture,
            &direction.iter().map(|v| v * scale).collect::<Vec<_>>(),
        );
        let actual = observe(&fixture, &signed, Order::Reverse, true);
        assert_eq!(actual.0, baseline.0);
        assert_eq!(actual.2, baseline.2);
        assert_eq!(
            bits(
                &actual
                    .3
                    .unwrap()
                    .iter()
                    .map(|v| v / scale)
                    .collect::<Vec<_>>()
            ),
            bits(baseline.3.as_ref().unwrap())
        );
    }
    for last in 0..3 {
        let mut calls = 0;
        let (outcome, factors) = measured(|| {
            let fit = GridFit::prepare(
                &case.matrix,
                &case.seed,
                case.anchor,
                &case.order(Order::Reverse),
            )?;
            fit.attempt_rounded_beam(1 << 22, 16, 1e-8, |v| {
                calls += 1;
                let (relative, residual) = fixture.checked(&case.restore(v))?;
                if calls == 18 {
                    if last == 0 {
                        return Err("real rounded beam final operator fault".into());
                    }
                    if last == 1 {
                        return Ok((f64::NAN, residual));
                    }
                }
                Ok((
                    if last == 2 && calls == 18 {
                        1.0
                    } else {
                        relative
                    },
                    case.permutation.iter().map(|&i| residual[i]).collect(),
                ))
            })
        });
        assert_eq!(factors, 1);
        assert_eq!(calls, 18);
        match last {
            0 => assert_eq!(
                outcome.unwrap_err(),
                "real rounded beam final operator fault"
            ),
            1 => assert!(outcome.unwrap_err().contains("finite matching certificate")),
            _ => assert!(matches!(
                outcome.unwrap(),
                Attempt::Rejected(Rejection::Residual(1.0))
            )),
        }
        let replay = observe(&fixture, &case, Order::Reverse, true);
        assert_eq!(replay, baseline);
    }
    for (stage, steps) in [
        (SolverStage::DenseFactor, 2),
        (SolverStage::DenseSubstitution, 0),
        (SolverStage::DenseSubstitution, 298),
        (SolverStage::ModalRoundoffSearch, 17),
        (SolverStage::ModalRoundoffValidate, 1),
    ] {
        let control = SolverControl::default();
        let cancel = control.clone();
        let seen = Rc::new(Cell::new(false));
        let observed = seen.clone();
        let mut calls = 0;
        let error = with_solver_observer(
            &control,
            move |p| {
                if p.stage == stage && p.completed_steps == steps {
                    observed.set(true);
                    cancel.request_cancel();
                }
            },
            || {
                let fit = GridFit::prepare(
                    &case.matrix,
                    &case.seed,
                    case.anchor,
                    &case.order(Order::Reverse),
                )?;
                fit.attempt_rounded_beam(1 << 22, 16, 1e-8, |v| {
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
        assert!(
            seen.get() && error.contains("cancel"),
            "{stage:?}/{steps}: {error}"
        );
        assert!(calls <= 18);
        assert_eq!(observe(&fixture, &case, Order::Reverse, true), baseline);
    }
    let changed =
        Rebuilt::prepare_bounded(&renumber(&Recipe::ThreeLayers.request(100, 1e-10), 113)).unwrap();
    let (changed, _) = prepare(changed.canonical).unwrap();
    let changed_direction = rounded(
        &Factor::prepare_shifted(&changed.directions, changed.value)
            .unwrap()
            .final_direction(&changed.seed, 4)
            .unwrap(),
    );
    assert_eq!(changed.value.to_bits(), root);
    assert_eq!(bits(&changed.seed), frozen);
    assert_eq!(
        observe(
            &changed,
            &Case::new(&changed, &changed_direction),
            Order::Reverse,
            true
        ),
        baseline
    );
    assert_eq!(fixture.value.to_bits(), root);
    assert_eq!(bits(&fixture.seed), frozen);
    println!(
        "material rounded beam real_controls sign_binary=2 final_faults=2 lost_final=1 cancellations=5 exact_healthy_replays=8 fresh_numbering_replays=1 partial_candidates=0"
    );
}

#[test]
fn modal_material_rounded_beam_healthy_readback_and_last_publication_cancel() {
    let input = renumber(&Recipe::ThreeLayers.request(80, 1.0), 7);
    for chart in [Chart::Original, Chart::Inward] {
        let baseline = run_beam(&input, chart);
        assert_eq!(baseline.stage, Stage::Accepted, "{baseline:?}");
        assert!(baseline.readback.unwrap() <= 1e-8);
        let changed = renumber(&Recipe::ThreeLayers.request(80, 1.0), 113);
        let replay = run_beam(&changed, chart);
        assert_eq!(replay.stage, Stage::Accepted, "{replay:?}");
        assert_eq!(replay.root, baseline.root);
        assert_eq!(replay.seed, baseline.seed);
        assert_eq!(replay.factors, baseline.factors);
        assert_eq!(replay.certificates, baseline.certificates);
        // Independent original-numbered assembly may change summation order.
        assert!(replay.readback.unwrap() <= 1e-8);
        assert!((replay.readback.unwrap() / baseline.readback.unwrap() - 1.0).abs() < 1e-12);
        let original = baseline.output.as_ref().unwrap();
        let reordered = replay.output.as_ref().unwrap();
        for (i, node) in original.input.nodes.iter().enumerate() {
            let j = reordered
                .input
                .nodes
                .iter()
                .position(|n| n.x == node.x && n.y == node.y)
                .unwrap();
            assert_eq!(
                bits(&original.modes[0].shape[3 * i..3 * i + 3]),
                bits(&reordered.modes[0].shape[3 * j..3 * j + 3])
            );
        }
        println!(
            "material rounded beam healthy physical={chart:?} original_readback={:e} changed_numbering_readback={:e} factors={:?} certificates={:?}",
            baseline.readback.unwrap(),
            replay.readback.unwrap(),
            baseline.factors,
            baseline.certificates
        );
        for (stage, steps) in [
            (SolverStage::ResultNodes, 81),
            (SolverStage::ResultTotals, 2),
        ] {
            let control = SolverControl::default();
            let cancel = control.clone();
            let mut record = Record::new();
            let error = with_solver_observer(
                &control,
                move |p| {
                    if p.stage == stage && p.completed_steps == steps {
                        cancel.request_cancel();
                    }
                },
                || attempt(&mut record, &changed, chart),
            )
            .unwrap_err();
            assert!(error.contains("cancel"), "{stage:?}: {error}");
            assert!(record.output.is_none());
            let replay = run_beam(&changed, chart);
            assert_eq!(
                serde_json::to_vec(&replay.output).unwrap(),
                serde_json::to_vec(&run_beam(&changed, chart).output).unwrap()
            );
            assert_eq!(replay.stage, Stage::Accepted);
        }
    }
    println!(
        "material rounded beam healthy_readback original_numbered_outputs=4 restoration_cancellations=2 publication_cancellations=2 fresh_healthy_replays=4 difficult_input_publications=0"
    );
}
