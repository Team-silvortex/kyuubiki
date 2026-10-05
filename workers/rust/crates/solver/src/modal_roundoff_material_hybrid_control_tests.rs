use super::*;

#[test]
fn modal_material_hybrid_real_numbering_cancel_and_failed_request_replay() {
    let input = renumber(&Recipe::UnequalLengths.request(128, 1e-10), 7);
    let changed = renumber(&Recipe::UnequalLengths.request(128, 1e-10), 113);
    let original = run_hybrid(&input);
    let baseline = run_hybrid(&changed);
    for record in [&original, &baseline] {
        assert_eq!(record.stage, Stage::Accepted, "{record:?}");
        assert_eq!(record.factors, [1, 1]);
        assert_eq!(record.certificates, [4, 23]);
        assert!(record.readback.unwrap() <= 1e-8);
    }
    assert_eq!(baseline.root, original.root);
    assert_eq!(baseline.seed, original.seed);
    assert!((baseline.readback.unwrap() / original.readback.unwrap() - 1.0).abs() < 1e-12);
    for (i, node) in original
        .output
        .as_ref()
        .unwrap()
        .input
        .nodes
        .iter()
        .enumerate()
    {
        let output = baseline.output.as_ref().unwrap();
        let j = output
            .input
            .nodes
            .iter()
            .position(|n| n.x == node.x && n.y == node.y)
            .unwrap();
        assert_eq!(
            bits(&original.output.as_ref().unwrap().modes[0].shape[3 * i..3 * i + 3]),
            bits(&output.modes[0].shape[3 * j..3 * j + 3])
        );
    }
    let expected = serde_json::to_vec(&baseline.output).unwrap();
    let frozen = serde_json::to_vec(&changed).unwrap();
    for (stage, steps) in [
        (SolverStage::ModalIteration, 4),
        (SolverStage::ModalRoundoffSearch, 22),
        (SolverStage::ModalRoundoffValidate, 7),
        (SolverStage::ResultNodes, 129),
        (SolverStage::ResultTotals, 2),
    ] {
        let control = SolverControl::default();
        let cancel = control.clone();
        let seen = Rc::new(Cell::new(false));
        let observed = seen.clone();
        let armed = Rc::new(Cell::new(false));
        let ready = armed.clone();
        let mut record = Record::new();
        let error = with_solver_observer(
            &control,
            move |p| {
                if p.stage == SolverStage::ModalRoundoffValidate && p.completed_steps == 5 {
                    ready.set(true);
                }
                if ready.get() && p.stage == stage && p.completed_steps == steps {
                    observed.set(true);
                    cancel.request_cancel();
                }
            },
            || attempt_ablation(&mut record, &changed, false, Tail::Hybrid),
        )
        .unwrap_err();
        assert!(seen.get() && error.contains("cancel"), "{stage:?}: {error}");
        assert!(record.output.is_none());
        assert_eq!(
            record.stage,
            if matches!(stage, SolverStage::ResultNodes | SolverStage::ResultTotals) {
                Stage::Readback
            } else {
                Stage::Physical
            }
        );
        assert_eq!(
            record.certificates[1],
            match stage {
                SolverStage::ModalIteration => 4,
                SolverStage::ModalRoundoffSearch => 22,
                _ => 23,
            }
        );
        assert_eq!(serde_json::to_vec(&changed).unwrap(), frozen);
        let replay = run_hybrid(&changed);
        assert_eq!(replay.stage, Stage::Accepted);
        assert_eq!(serde_json::to_vec(&replay.output).unwrap(), expected);
    }
    for (members, scale) in [(100, 1e-10), (128, 1.0), (128, 1e14), (128, 1e-10)] {
        let failed = renumber(&Recipe::ThreeLayers.request(members, scale), 113);
        let rejected = run_hybrid(&failed);
        assert_eq!(rejected.stage, Stage::Internal);
        assert_eq!(rejected.factors[1], 0);
        assert_eq!(rejected.certificates[1], 0);
        assert!(rejected.output.is_none() && rejected.readback.is_none());
        let replay = run_hybrid(&changed);
        assert_eq!(serde_json::to_vec(&replay.output).unwrap(), expected);
    }
    println!(
        "material hybrid real_readback numbered_outputs=2 original={:e} changed={:e} cancellations=5 failed_requests=4 fresh_json_replays=9 hard_publications=0",
        original.readback.unwrap(),
        baseline.readback.unwrap()
    );
}

fn physical_fixture(input: &SolveModalFrame2dRequest) -> (Prepared, Case, Order) {
    let layout = Rebuilt::prepare_bounded(input).unwrap();
    let (fixture, _) = prepare(layout.canonical).unwrap();
    let direction = rounded(
        &Factor::prepare_shifted(&fixture.directions, fixture.value)
            .unwrap()
            .final_direction(&fixture.seed, 4)
            .unwrap(),
    );
    let internal = search_canonical_direction_ranked(&fixture.directions, &direction, 1e-8, |v| {
        fixture.checked(v)
    })
    .unwrap();
    let case = physical_case(&fixture, &internal.shape, Chart::Original).unwrap();
    (fixture, case, internal.order)
}

fn observe(input: &SolveModalFrame2dRequest, sign: f64) -> (Vec<u64>, usize) {
    let (fixture, mut case, order) = physical_fixture(input);
    for value in &mut case.seed {
        *value *= sign;
    }
    let mut calls = 0;
    let (actual, factors) = measured(|| {
        let fit = GridFit::prepare(&case.matrix, &case.seed, case.anchor, &case.order(order))?;
        fit.attempt_unit_hybrid(1 << 22, 16, 1e-8, |v| {
            calls += 1;
            physical_checked(&fixture, &case, v)
        })
    });
    assert_eq!(factors, 1);
    let Attempt::Accepted(shape) = actual.unwrap() else {
        panic!("fresh healthy physical hybrid rejected");
    };
    assert_eq!(
        shape[case.anchor].to_bits(),
        case.seed[case.anchor].to_bits()
    );
    assert!(physical_checked(&fixture, &case, &shape).unwrap().0 <= 1e-8);
    (
        bits(&shape.iter().map(|v| sign * v).collect::<Vec<_>>()),
        calls,
    )
}

#[test]
fn modal_material_hybrid_real_final_fault_sign_cancel_and_cold_replay() {
    let input = renumber(&Recipe::UnequalLengths.request(128, 1e-10), 7);
    let baseline = observe(&input, 1.0);
    assert_eq!(baseline.1, 23);
    assert_eq!(observe(&input, -1.0), baseline);
    let (fixture, case, order) = physical_fixture(&input);
    let root = fixture.value.to_bits();
    let seed = bits(&fixture.seed);
    for fail_at in [4, 5, 22, 23] {
        let mut calls = 0;
        let (actual, factors) = measured(|| {
            let fit = GridFit::prepare(&case.matrix, &case.seed, case.anchor, &case.order(order))?;
            fit.attempt_unit_hybrid(1 << 22, 16, 1e-8, |v| {
                calls += 1;
                if calls == fail_at {
                    return Err("real hybrid actual operator fault".into());
                }
                physical_checked(&fixture, &case, v)
            })
        });
        assert_eq!(factors, 1);
        assert_eq!(calls, fail_at);
        assert_eq!(actual.unwrap_err(), "real hybrid actual operator fault");
        assert_eq!(observe(&input, 1.0), baseline);
    }
    for malformed in [true, false] {
        let mut calls = 0;
        let fit =
            GridFit::prepare(&case.matrix, &case.seed, case.anchor, &case.order(order)).unwrap();
        let error = fit
            .attempt_unit_hybrid(1 << 22, 16, 1e-8, |v| {
                calls += 1;
                let mut receipt = physical_checked(&fixture, &case, v)?;
                if calls == 23 {
                    receipt.0 = if malformed { f64::NAN } else { 1.0 };
                }
                Ok(receipt)
            })
            .unwrap_err();
        assert_eq!(calls, 23);
        assert!(error.contains(if malformed {
            "finite matching"
        } else {
            "final physical certificate"
        }));
        assert_eq!(observe(&input, 1.0), baseline);
    }
    let control = SolverControl::default();
    let cancel = control.clone();
    let calls = Rc::new(Cell::new(0));
    let current = calls.clone();
    let seen = Rc::new(Cell::new(false));
    let observed = seen.clone();
    let error = with_solver_observer(
        &control,
        move |p| {
            if current.get() == 23 && p.stage == SolverStage::SparseMatvec && p.completed_steps == 0
            {
                observed.set(true);
                cancel.request_cancel();
            }
        },
        || {
            let fit = GridFit::prepare(&case.matrix, &case.seed, case.anchor, &case.order(order))?;
            fit.attempt_unit_hybrid(1 << 22, 16, 1e-8, |v| {
                calls.set(calls.get() + 1);
                physical_checked(&fixture, &case, v)
            })
        },
    )
    .unwrap_err();
    assert!(seen.get() && error.contains("cancel"));
    assert_eq!(calls.get(), 23);
    assert_eq!(observe(&input, 1.0), baseline);
    assert_eq!(fixture.value.to_bits(), root);
    assert_eq!(bits(&fixture.seed), seed);
    println!(
        "material hybrid real_controls sign=1 operator_faults=4 malformed_final=1 lost_final_gate=1 final_product_cancel=1 fresh_exact_replays=7 physical_factors=1 partial_candidates=0"
    );
}
