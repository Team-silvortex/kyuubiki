use super::*;

fn fresh(input: &SolveModalFrame2dRequest) -> (Prepared, PhysicalCase) {
    let layout = Rebuilt::prepare_bounded(input).unwrap();
    let (fixture, _) = prepare(layout.canonical).unwrap();
    let wide = Factor::prepare_shifted(&fixture.directions, fixture.value)
        .unwrap()
        .final_direction(&fixture.seed, 4)
        .unwrap();
    let case = PhysicalCase::new(&fixture, physical_seed(&fixture, &wide), &rounded(&wide));
    (fixture, case)
}

fn search(
    case: &PhysicalCase,
    checked: impl FnMut(&[f64]) -> Result<Receipt, String>,
) -> Result<Outcome, String> {
    let fit = GridFit::prepare(
        &case.matrix,
        &case.seed,
        case.anchor,
        &case.order(Order::Reverse),
    )?;
    fit.attempt_rounded_joint(1 << 22, 16, 1e-8, checked)
}

fn observe(fixture: &Prepared, case: &PhysicalCase) -> (Vec<f64>, [u64; 3], usize) {
    let mut calls = 0;
    let (outcome, factors) = measured(|| {
        search(case, |v| {
            calls += 1;
            case.checked(fixture, v)
        })
    });
    assert_eq!(factors, 1);
    let Outcome::Accepted(shape) = outcome.unwrap() else {
        panic!("healthy joint replay rejected");
    };
    assert_eq!(
        shape[case.anchor].to_bits(),
        case.seed[case.anchor].to_bits()
    );
    let receipt = case.checked(fixture, &shape).unwrap();
    let norm = crate::modal_math::checked_shape_norm(&shape).unwrap();
    assert!(receipt.physical <= 1e-8 && receipt.internal <= 1e-8);
    assert!((norm - 1.0).abs() < 1e-10);
    (
        shape,
        [
            receipt.physical.to_bits(),
            receipt.internal.to_bits(),
            norm.to_bits(),
        ],
        calls,
    )
}

fn fresh_observation(input: &SolveModalFrame2dRequest) -> (Vec<f64>, [u64; 3], usize) {
    let (fixture, case) = fresh(input);
    observe(&fixture, &case)
}

#[test]
fn modal_material_joint_real_fault_cancel_sign_material_numbering_replay() {
    let input = renumber(&Recipe::Jittered.request(80, 1.0), 7);
    let frozen_input = serde_json::to_vec(&input).unwrap();
    let (fixture, case) = fresh(&input);
    let root = fixture.value.to_bits();
    let seed = bits(&fixture.seed);
    let physical = bits(&case.seed);
    let baseline = observe(&fixture, &case);
    assert_eq!(baseline.2, 18);
    let signed = PhysicalCase::new(
        &fixture,
        case.restore(&case.seed).iter().map(|v| -v).collect(),
        &rounded(
            &Factor::prepare_shifted(&fixture.directions, fixture.value)
                .unwrap()
                .final_direction(&fixture.seed, 4)
                .unwrap(),
        ),
    );
    let negative = observe(&fixture, &signed);
    assert_eq!(
        bits(&negative.0.iter().map(|v| -v).collect::<Vec<_>>()),
        bits(&baseline.0)
    );
    assert_eq!(negative.1, baseline.1);
    assert_eq!(negative.2, baseline.2);

    let changed = renumber(&Recipe::Jittered.request(80, 1.0), 113);
    let (renumbered, other) = fresh(&changed);
    assert_eq!(renumbered.value.to_bits(), root);
    assert_eq!(bits(&renumbered.seed), seed);
    assert_eq!(observe(&renumbered, &other), baseline);
    let mut scaled = input.clone();
    for element in &mut scaled.elements {
        element.youngs_modulus *= 16.0;
        element.density *= 4.0;
    }
    let (scaled_fixture, _) = fresh(&scaled);
    assert_eq!(
        scaled_fixture.value.to_bits(),
        (fixture.value * 4.0).to_bits()
    );
    // A fresh material request can change ordinary refinement, even for binary ratios.
    assert_ne!(bits(&scaled_fixture.seed), seed);
    let scaled_record = run_joint(&scaled, Order::Reverse);
    assert_eq!(scaled_record.root, Some(scaled_fixture.value.to_bits()));
    assert_eq!(scaled_record.seed, bits(&scaled_fixture.seed));
    assert_eq!(scaled_record.stage, Stage::Physical);
    assert_eq!(scaled_record.certificates, [18, 18]);
    assert!(scaled_record.output.is_none() && scaled_record.readback.is_none());
    let old_scaled = run(&scaled, Chart::Original);
    assert_eq!(old_scaled.stage, Stage::Accepted, "{old_scaled:?}");
    assert_eq!(old_scaled.root, scaled_record.root);
    assert_eq!(old_scaled.seed, scaled_record.seed);
    println!(
        "material joint binary_material young_ratio=16 density_ratio=4 eigenvalue_ratio=4 seed_bits_equal=0 stage={:?} best={:e} certificates={:?} error={:?}",
        scaled_record.stage, scaled_record.best, scaled_record.certificates, scaled_record.error
    );
    assert_eq!(fresh_observation(&input), baseline);

    for kind in 0..4 {
        let mut calls = 0;
        let result = search(&case, |v| {
            calls += 1;
            let mut receipt = case.checked(&fixture, v)?;
            if calls == 18 {
                match kind {
                    0 => return Err("real joint final operator fault".into()),
                    1 => receipt.internal = f64::NAN,
                    2 => receipt.physical = 1.0,
                    _ => receipt.internal = 1.0,
                }
            }
            Ok(receipt)
        });
        assert_eq!(calls, 18);
        match kind {
            0 => assert_eq!(result.unwrap_err(), "real joint final operator fault"),
            1 => assert!(result.unwrap_err().contains("finite matching internal")),
            _ => assert!(matches!(result.unwrap(), Outcome::Rejected { .. })),
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
            || search(&case, |v| case.checked(&fixture, v)),
        )
        .unwrap_err();
        assert!(
            seen.get() && error.contains("cancel"),
            "{stage:?}/{steps}: {error}"
        );
        assert_eq!(fresh_observation(&input), baseline);
    }
    // Cancel inside the second actual operator product of the final paired receipt.
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
                case.checked(&fixture, v)
            })
        },
    )
    .unwrap_err();
    assert!(error.contains("cancel"), "{error}");
    assert_eq!(calls.get(), 18);
    assert_eq!(products.get(), 2);
    assert_eq!(fresh_observation(&input), baseline);
    assert_eq!(fixture.value.to_bits(), root);
    assert_eq!(bits(&fixture.seed), seed);
    assert_eq!(bits(&case.seed), physical);
    assert_eq!(serde_json::to_vec(&input).unwrap(), frozen_input);
    println!(
        "material joint real_controls sign=1 binary_material=1 numbering=1 final_faults=2 lost_final_gates=2 cancellations=5 fresh_exact_replays=10 partial_candidates=0"
    );
}

#[test]
fn modal_material_joint_real_numbered_readback_publication_failure_replay() {
    let input = renumber(&Recipe::Jittered.request(80, 1.0), 7);
    let baseline = run_joint(&input, Order::Reverse);
    let changed = renumber(&Recipe::Jittered.request(80, 1.0), 113);
    let reordered = run_joint(&changed, Order::Reverse);
    for record in [&baseline, &reordered] {
        assert_eq!(record.stage, Stage::Accepted, "{record:?}");
        assert_eq!(record.factors, [0, 1]);
        assert_eq!(record.certificates, [18, 18]);
        assert!(record.readback.unwrap() <= 1e-8);
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
    let frozen = serde_json::to_vec(&changed).unwrap();
    let expected = serde_json::to_vec(&reordered.output).unwrap();
    for (stage, steps) in [
        (SolverStage::ResultNodes, 81),
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
            || attempt(&mut record, &changed, Order::Reverse),
        )
        .unwrap_err();
        assert!(seen.get() && error.contains("cancel"), "{stage:?}: {error}");
        assert!(record.output.is_none());
        assert_eq!(serde_json::to_vec(&changed).unwrap(), frozen);
        let replay = run_joint(&changed, Order::Reverse);
        assert_eq!(replay.stage, Stage::Accepted);
        assert_eq!(serde_json::to_vec(&replay.output).unwrap(), expected);
    }
    for (members, scale) in [(100, 1e-10), (128, 1.0), (128, 1e14), (128, 1e-10)] {
        let failed = renumber(&Recipe::ThreeLayers.request(members, scale), 113);
        let record = run_joint(&failed, Order::Reverse);
        assert_eq!(record.stage, Stage::Physical, "{record:?}");
        assert!(record.output.is_none() && record.readback.is_none());
        assert_eq!(record.certificates, [18, 18]);
        assert_eq!(
            serde_json::to_vec(&run_joint(&changed, Order::Reverse).output).unwrap(),
            expected
        );
    }
    println!(
        "material joint real_readback outputs=2 original_residual={:e} renumbered_residual={:e} restoration_cancel=1 publication_cancel=1 failed_requests=4 fresh_json_replays=6 hard_publications=0",
        baseline.readback.unwrap(),
        reordered.readback.unwrap()
    );
}
