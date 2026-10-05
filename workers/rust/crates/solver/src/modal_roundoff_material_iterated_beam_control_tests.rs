use super::*;

#[test]
fn modal_material_iterated_beam_real_numbering_late_cancel_and_json_replay() {
    let input = renumber(&Recipe::ThreeLayers.request(100, 1e-10), 7);
    let changed = renumber(&Recipe::ThreeLayers.request(100, 1e-10), 113);
    let original = run_extended(&input);
    let baseline = run_extended(&changed);
    for record in [&original, &baseline] {
        assert_eq!(record.stage, Stage::Accepted, "{record:?}");
        assert_eq!(record.factors, [3, 1]);
        assert!(record.readback.unwrap() <= 1e-8);
    }
    assert_eq!(baseline.root, original.root);
    assert_eq!(baseline.seed, original.seed);
    assert_eq!(baseline.order, original.order);
    assert_eq!(baseline.certificates, original.certificates);
    assert_eq!(baseline.best.to_bits(), original.best.to_bits());
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
    let frozen = serde_json::to_vec(&changed).unwrap();
    let expected = serde_json::to_vec(&baseline.output).unwrap();
    for (stage, steps) in [
        (SolverStage::DenseSubstitution, 0),
        (SolverStage::ModalRoundoffSearch, 65),
        (SolverStage::ModalRoundoffValidate, 8),
        (SolverStage::ModalRoundoffValidate, 7),
        (SolverStage::ResultNodes, 101),
        (SolverStage::ResultTotals, 2),
    ] {
        let control = SolverControl::default();
        let cancel = control.clone();
        let armed = Rc::new(Cell::new(false));
        let ready = armed.clone();
        let seen = Rc::new(Cell::new(false));
        let observed = seen.clone();
        let mut record = Record::new();
        let error = with_solver_observer(
            &control,
            move |p| {
                if p.stage == SolverStage::ModalRoundoffPrepare && p.completed_steps == 4 {
                    ready.set(true);
                }
                if ready.get() && p.stage == stage && p.completed_steps == steps {
                    observed.set(true);
                    cancel.request_cancel();
                }
            },
            || attempt_extended(&mut record, &changed),
        )
        .unwrap_err();
        assert!(seen.get() && error.contains("cancel"), "{stage:?}: {error}");
        assert!(record.output.is_none());
        assert_eq!(
            record.stage,
            match stage {
                SolverStage::ResultNodes | SolverStage::ResultTotals => Stage::Readback,
                SolverStage::ModalRoundoffValidate if steps == 7 => Stage::Physical,
                _ => Stage::Internal,
            }
        );
        assert_eq!(serde_json::to_vec(&changed).unwrap(), frozen);
        let replay = run_extended(&changed);
        assert_eq!(serde_json::to_vec(&replay.output).unwrap(), expected);
    }
    for scale in [1.0, 1e14, 1e-10] {
        let failed = renumber(&Recipe::ThreeLayers.request(128, scale), 113);
        let rejected = run_extended(&failed);
        assert_eq!(rejected.stage, Stage::Internal);
        assert_eq!(rejected.factors, [3, 0]);
        assert!(rejected.output.is_none() && rejected.readback.is_none());
        assert_eq!(rejected.certificates[1], 0);
        let replay = run_extended(&changed);
        assert_eq!(serde_json::to_vec(&replay.output).unwrap(), expected);
    }
    println!(
        "material iterated real_readback numbered_outputs=2 original={:e} changed={:e} actual_certificates={:?} cancellations=6 failed_requests=3 fresh_json_replays=9 partial_outputs=0",
        original.readback.unwrap(),
        baseline.readback.unwrap(),
        baseline.certificates
    );
}

fn fresh_case(sign: f64) -> (Prepared, Case) {
    let input = renumber(&Recipe::ThreeLayers.request(100, 1e-10), 113);
    let layout = Rebuilt::prepare_bounded(&input).unwrap();
    let (fixture, refined) = prepare(layout.canonical).unwrap();
    assert!(!refined);
    let direction = rounded(
        &Factor::prepare_shifted(&fixture.directions, fixture.value)
            .unwrap()
            .final_direction(&fixture.seed, 4)
            .unwrap(),
    );
    let mut case = Case::new(&fixture, &direction);
    for v in &mut case.seed {
        *v *= sign;
    }
    (fixture, case)
}

fn receipt(fixture: &Prepared, case: &Case, v: &[f64]) -> Result<(f64, Vec<f64>), String> {
    let (relative, residual) = fixture.checked(&case.restore(v))?;
    Ok((
        relative,
        case.permutation.iter().map(|&i| residual[i]).collect(),
    ))
}

fn observe(sign: f64) -> (Vec<u64>, usize) {
    let (fixture, case) = fresh_case(sign);
    let mut calls = 0;
    let (actual, factors) = measured(|| {
        let fit = GridFit::prepare(
            &case.matrix,
            &case.seed,
            case.anchor,
            &case.order(Order::Reverse),
        )?;
        fit.attempt_iterated_beam(1 << 22, 64, 1, 1e-8, |v| {
            calls += 1;
            receipt(&fixture, &case, v)
        })
    });
    assert_eq!(factors, 1);
    let Attempt::Accepted(shape) = actual.unwrap() else {
        panic!("fresh real direction rejected")
    };
    assert_eq!(
        shape[case.anchor].to_bits(),
        case.seed[case.anchor].to_bits()
    );
    assert!(receipt(&fixture, &case, &shape).unwrap().0 <= 1e-8);
    (
        bits(&shape.iter().map(|v| sign * v).collect::<Vec<_>>()),
        calls,
    )
}

#[test]
fn modal_material_iterated_beam_real_operator_final_fault_and_cold_replay() {
    let expected = observe(1.0);
    assert_eq!(expected.1, 66);
    assert_eq!(observe(-1.0), expected);
    let (fixture, case) = fresh_case(1.0);
    let root = fixture.value.to_bits();
    let seed = bits(&fixture.seed);
    for fail_at in [1, 2, 65, 66] {
        let mut calls = 0;
        let fit = GridFit::prepare(
            &case.matrix,
            &case.seed,
            case.anchor,
            &case.order(Order::Reverse),
        )
        .unwrap();
        let error = fit
            .attempt_iterated_beam(1 << 22, 64, 1, 1e-8, |v| {
                calls += 1;
                if calls == fail_at {
                    return Err("real iterated actual operator fault".into());
                }
                receipt(&fixture, &case, v)
            })
            .unwrap_err();
        assert_eq!(calls, fail_at);
        assert_eq!(error, "real iterated actual operator fault");
        assert_eq!(observe(1.0), expected);
    }
    for kind in 0..6 {
        let mut calls = 0;
        let fit = GridFit::prepare(
            &case.matrix,
            &case.seed,
            case.anchor,
            &case.order(Order::Reverse),
        )
        .unwrap();
        let actual = fit.attempt_iterated_beam(1 << 22, 64, 1, 1e-8, |v| {
            calls += 1;
            let mut value = receipt(&fixture, &case, v)?;
            if calls == 66 {
                match kind {
                    0 => value.0 = f64::NAN,
                    1 => value.0 = -1.0,
                    2 => {
                        value.1.pop();
                    }
                    3 => value.1[0] = f64::INFINITY,
                    4 => value.1[0] = 1e51,
                    _ => value.0 = 1.0,
                }
            }
            Ok(value)
        });
        assert_eq!(calls, 66);
        if kind == 5 {
            assert!(matches!(
                actual.unwrap(),
                Attempt::Rejected(Rejection::Residual(1.0))
            ));
        } else {
            assert!(actual.unwrap_err().contains("finite matching certificate"));
        }
        assert_eq!(observe(1.0), expected);
    }
    let mut calls = 0;
    let control = SolverControl::default();
    let cancel = control.clone();
    let armed = Rc::new(Cell::new(false));
    let seen = Rc::new(Cell::new(false));
    let ready = armed.clone();
    let observed = seen.clone();
    let fit = GridFit::prepare(
        &case.matrix,
        &case.seed,
        case.anchor,
        &case.order(Order::Reverse),
    )
    .unwrap();
    let error = with_solver_observer(
        &control,
        move |p| {
            if ready.get() && p.stage == SolverStage::SparseMatvec && p.completed_steps == 0 {
                observed.set(true);
                cancel.request_cancel();
            }
        },
        || {
            fit.attempt_iterated_beam(1 << 22, 64, 1, 1e-8, |v| {
                calls += 1;
                armed.set(calls == 66);
                receipt(&fixture, &case, v)
            })
        },
    )
    .unwrap_err();
    assert!(seen.get() && error.contains("cancel"));
    assert_eq!(calls, 66);
    assert_eq!(observe(1.0), expected);
    assert_eq!(fixture.value.to_bits(), root);
    assert_eq!(bits(&fixture.seed), seed);
    println!(
        "material iterated real_controls operator_faults=4 malformed_final_receipts=5 lost_final_gate=1 final_product_cancel=1 signed_directions=2 fresh_exact_replays=11 partial_candidates=0"
    );
}
