use super::*;

fn fresh(input: &SolveModalFrame2dRequest) -> (Prepared, Case) {
    let layout = Rebuilt::prepare_bounded(input).unwrap();
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

fn search(
    case: &Case,
    checked: impl FnMut(&[f64]) -> Result<Receipt, String>,
) -> Result<Attempt, String> {
    let fit = GridFit::prepare(
        &case.matrix,
        &case.seed,
        case.anchor,
        &case.order(Order::GridNorm),
    )?;
    fit.attempt_rounded_coupled(1 << 22, 16, 1e-8, checked)
}

fn observe(fixture: &Prepared, case: &Case) -> (Vec<u64>, [u64; 2], usize) {
    let mut calls = 0;
    let (actual, factors) = measured(|| {
        search(case, |v| {
            calls += 1;
            checked(fixture, case, v)
        })
    });
    assert_eq!(factors, 1);
    let Attempt::Accepted(shape) = actual.unwrap() else {
        panic!("healthy coupled direction rejected");
    };
    assert_eq!(
        shape[case.anchor].to_bits(),
        case.seed[case.anchor].to_bits()
    );
    let final_receipt = checked(fixture, case, &shape).unwrap();
    assert!(final_receipt.internal <= 1e-8);
    (
        bits(&shape),
        [
            final_receipt.internal.to_bits(),
            final_receipt.mapped.to_bits(),
        ],
        calls,
    )
}

fn fresh_observation(input: &SolveModalFrame2dRequest) -> (Vec<u64>, [u64; 2], usize) {
    let (fixture, case) = fresh(input);
    observe(&fixture, &case)
}

#[test]
fn modal_material_coupled_real_final_pair_fault_cancel_and_fresh_replay() {
    let input = renumber(&Recipe::UnequalLengths.request(128, 1e-10), 7);
    let (fixture, case) = fresh(&input);
    let root = fixture.value.to_bits();
    let frozen = bits(&fixture.seed);
    let baseline = observe(&fixture, &case);
    assert_eq!(baseline.2, 18);
    for kind in 0..3 {
        let mut calls = 0;
        let actual = search(&case, |v| {
            calls += 1;
            let mut receipt = checked(&fixture, &case, v)?;
            if calls == 18 {
                match kind {
                    0 => return Err("real coupled final operator fault".into()),
                    1 => receipt.mapped = f64::NAN,
                    _ => receipt.internal = 1.0,
                }
            }
            Ok(receipt)
        });
        assert_eq!(calls, 18);
        match kind {
            0 => assert_eq!(actual.unwrap_err(), "real coupled final operator fault"),
            1 => assert!(actual.unwrap_err().contains("finite nonnegative mapped")),
            _ => assert!(matches!(
                actual.unwrap(),
                Attempt::Rejected(Rejection::Residual(1.0))
            )),
        }
        assert_eq!(fresh_observation(&input), baseline);
    }
    for (stage, steps) in [
        (SolverStage::DenseFactor, 2),
        (SolverStage::DenseSubstitution, 0),
        (SolverStage::ModalRoundoffSearch, 17),
        (SolverStage::ModalRoundoffValidate, 1),
    ] {
        let control = SolverControl::default();
        let cancel = control.clone();
        let seen = Rc::new(Cell::new(false));
        let observed = seen.clone();
        let error = with_solver_observer(
            &control,
            move |p| {
                if p.stage == stage && p.completed_steps == steps {
                    observed.set(true);
                    cancel.request_cancel();
                }
            },
            || search(&case, |v| checked(&fixture, &case, v)),
        )
        .unwrap_err();
        assert!(
            seen.get() && error.contains("cancel"),
            "{stage:?}/{steps}: {error}"
        );
        assert_eq!(fresh_observation(&input), baseline);
    }
    let control = SolverControl::default();
    let cancel = control.clone();
    let calls = Rc::new(Cell::new(0));
    let current = calls.clone();
    let products = Rc::new(Cell::new(0));
    let observed = products.clone();
    let error = with_solver_observer(
        &control,
        move |p| {
            if current.get() == 18 && p.stage == SolverStage::SparseMatvec && p.completed_steps == 0
            {
                observed.set(observed.get() + 1);
                if observed.get() == 2 {
                    cancel.request_cancel();
                }
            }
        },
        || {
            search(&case, |v| {
                calls.set(calls.get() + 1);
                checked(&fixture, &case, v)
            })
        },
    )
    .unwrap_err();
    assert!(error.contains("cancel"));
    assert_eq!(calls.get(), 18);
    assert_eq!(products.get(), 2);
    assert_eq!(fresh_observation(&input), baseline);
    let changed = renumber(&Recipe::UnequalLengths.request(128, 1e-10), 113);
    let (renumbered, changed_case) = fresh(&changed);
    assert_eq!(renumbered.value.to_bits(), root);
    assert_eq!(bits(&renumbered.seed), frozen);
    assert_eq!(observe(&renumbered, &changed_case), baseline);
    assert_eq!(fixture.value.to_bits(), root);
    assert_eq!(bits(&fixture.seed), frozen);
    println!(
        "material coupled real_controls final_faults=2 lost_final_gate=1 cancellations=5 fresh_exact_replays=8 changed_numbering=1 partial_candidates=0"
    );
}

#[test]
fn modal_material_coupled_real_gain_numbering_json_publication_cancel() {
    let input = renumber(&Recipe::UnequalLengths.request(128, 1e-10), 7);
    let changed = renumber(&Recipe::UnequalLengths.request(128, 1e-10), 113);
    let baseline = run_coupled(&input, Order::GridNorm, Chart::Original);
    let reordered = run_coupled(&changed, Order::GridNorm, Chart::Original);
    for record in [&baseline, &reordered] {
        assert_eq!(record.stage, Stage::Accepted, "{record:?}");
        assert_eq!(record.factors, [1, 1]);
        assert_eq!(record.certificates, [18, 18]);
        assert!(record.readback.unwrap() <= 1e-8);
    }
    for request in [&input, &changed] {
        let old = run(request, Chart::Original);
        assert_eq!(old.stage, Stage::Physical);
        assert!(old.output.is_none());
    }
    assert_eq!(reordered.root, baseline.root);
    assert_eq!(reordered.seed, baseline.seed);
    assert!((reordered.readback.unwrap() / baseline.readback.unwrap() - 1.0).abs() < 1e-12);
    let original = baseline.output.as_ref().unwrap();
    let other = reordered.output.as_ref().unwrap();
    for (i, node) in original.input.nodes.iter().enumerate() {
        let j = other
            .input
            .nodes
            .iter()
            .position(|n| n.x == node.x && n.y == node.y)
            .unwrap();
        assert_eq!(
            bits(&original.modes[0].shape[3 * i..3 * i + 3]),
            bits(&other.modes[0].shape[3 * j..3 * j + 3])
        );
    }
    let expected = serde_json::to_vec(&reordered.output).unwrap();
    let frozen = serde_json::to_vec(&changed).unwrap();
    for (stage, steps) in [
        (SolverStage::ModalRoundoffValidate, 2),
        (SolverStage::ResultNodes, 129),
        (SolverStage::ResultTotals, 2),
    ] {
        let control = SolverControl::default();
        let cancel = control.clone();
        let seen = Rc::new(Cell::new(false));
        let observed = seen.clone();
        let mut record = Record::new();
        let error = with_solver_observer(
            &control,
            move |p| {
                if p.stage == stage && p.completed_steps == steps {
                    observed.set(true);
                    cancel.request_cancel();
                }
            },
            || attempt(&mut record, &changed, Order::GridNorm, Chart::Original),
        )
        .unwrap_err();
        assert!(seen.get() && error.contains("cancel"), "{stage:?}: {error}");
        assert!(record.output.is_none());
        assert_eq!(serde_json::to_vec(&changed).unwrap(), frozen);
        let replay = run_coupled(&changed, Order::GridNorm, Chart::Original);
        assert_eq!(replay.stage, Stage::Accepted);
        assert_eq!(serde_json::to_vec(&replay.output).unwrap(), expected);
    }
    for (members, scale) in [(100, 1e-10), (128, 1.0), (128, 1e14), (128, 1e-10)] {
        let failed = renumber(&Recipe::ThreeLayers.request(members, scale), 113);
        let record = run_coupled(&failed, Order::Reverse, Chart::Original);
        assert!(record.output.is_none() && record.readback.is_none());
        assert_eq!(
            record.stage,
            if members == 100 {
                Stage::Physical
            } else {
                Stage::Internal
            }
        );
        let replay = run_coupled(&changed, Order::GridNorm, Chart::Original);
        assert_eq!(replay.stage, Stage::Accepted);
        assert_eq!(serde_json::to_vec(&replay.output).unwrap(), expected);
    }
    println!(
        "material coupled real_gain numbered_outputs=2 original_readback={:e} changed_readback={:e} old_physical_rejections=2 final_norm_cancel=1 restoration_cancel=1 publication_cancel=1 failed_requests=4 fresh_json_replays=7 hard_publications=0",
        baseline.readback.unwrap(),
        reordered.readback.unwrap()
    );
}
