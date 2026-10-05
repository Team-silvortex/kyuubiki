use super::*;

const ENTER: usize = 9001;

fn run_reduced(input: &SolveModalFrame2dRequest) -> Record {
    let frozen = serde_json::to_vec(input).unwrap();
    let mut record = Record::new();
    let result = (|| {
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
        checkpoint(SolverStage::ModalRoundoffPrepare, ENTER)?;
        let (result, factors) = measured(|| {
            let fit = PairFit::prepare_reduced(
                &case.matrix,
                &case.seed,
                case.anchor,
                &case.order(Order::GridNorm),
                64,
            )?;
            fit.attempt(1 << 22, 1e-8, |v| {
                record.certificates[0] += 1;
                let (relative, residual) = fixture.checked(&case.restore(v))?;
                Ok((
                    relative,
                    case.permutation.iter().map(|&i| residual[i]).collect(),
                ))
            })
        });
        record.factors[0] = factors;
        assert!(factors <= 2 && record.certificates[0] <= 66);
        let Attempt::Accepted(shape) = result? else {
            return Err("bounded lattice internal proposal numerically rejected".into());
        };
        assert_eq!(
            shape[case.anchor].to_bits(),
            case.seed[case.anchor].to_bits()
        );
        assert_eq!(fixture.value.to_bits(), record.root.unwrap());
        assert_eq!(bits(&fixture.seed), record.seed);
        record.order = Some(Order::GridNorm);
        finish_hybrid(
            &mut record,
            fixture,
            layout,
            &case.restore(&shape),
            Order::GridNorm,
        )
    })();
    if let Err(error) = result {
        assert!(record.output.is_none());
        record.error = Some(error);
        record.readback = None;
    }
    assert_eq!(serde_json::to_vec(input).unwrap(), frozen);
    record
}

#[test]
fn modal_material_lattice_real_readback_numbering_late_cancel_and_replay() {
    let base = Recipe::ThreeLayers.request(100, 1e-10);
    let original = run_reduced(&renumber(&base, 7));
    let input = renumber(&base, 113);
    let healthy = run_reduced(&input);
    assert_eq!(original.stage, Stage::Accepted);
    assert_eq!(healthy.stage, Stage::Accepted);
    assert_eq!(healthy.root, original.root);
    assert_eq!(healthy.seed, original.seed);
    assert_eq!(healthy.order, original.order);
    assert_eq!(healthy.factors, original.factors);
    assert_eq!(healthy.certificates, original.certificates);
    assert_eq!(healthy.factors, [2, 1]);
    assert_eq!(healthy.best.to_bits(), original.best.to_bits());
    for (i, node) in original
        .output
        .as_ref()
        .unwrap()
        .input
        .nodes
        .iter()
        .enumerate()
    {
        let output = healthy.output.as_ref().unwrap();
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
    assert_eq!(healthy.certificates[0], 66);
    assert!(healthy.readback.unwrap() <= 1e-8 && original.readback.unwrap() <= 1e-8);
    let snapshot = serde_json::to_vec(&healthy.output).unwrap();
    for (stage, step, expected) in [
        (SolverStage::ModalRoundoffPrepare, 64, Stage::Internal),
        (SolverStage::DenseSubstitution, 0, Stage::Internal),
        (SolverStage::ModalRoundoffSearch, 65, Stage::Internal),
        (SolverStage::ModalRoundoffValidate, 9, Stage::Internal),
        (SolverStage::ModalRoundoffValidate, 7, Stage::Physical),
        (SolverStage::ResultNodes, 101, Stage::Readback),
        (SolverStage::ResultTotals, 2, Stage::Readback),
    ] {
        let control = SolverControl::default();
        let cancel = control.clone();
        let seen = Rc::new(Cell::new(false));
        let observed = seen.clone();
        let armed = Rc::new(Cell::new(false));
        let arm = armed.clone();
        let mut result = Record::new();
        let error = with_solver_observer(
            &control,
            move |p| {
                if p.stage == SolverStage::ModalRoundoffPrepare && p.completed_steps == ENTER as u64
                {
                    arm.set(true);
                }
                if arm.get() && p.stage == stage && p.completed_steps == step {
                    observed.set(true);
                    cancel.request_cancel();
                }
            },
            || {
                result = run_reduced(&input);
                result.error.clone().map_or(Ok(()), Err)
            },
        )
        .unwrap_err();
        assert!(seen.get(), "{stage:?} was not reached");
        assert!(error.contains("cancel"));
        assert!(result.error.unwrap().contains("cancel"));
        assert_eq!(result.stage, expected);
        assert!(result.output.is_none() && result.readback.is_none());
        let fresh = run_reduced(&input);
        assert_eq!(fresh.stage, Stage::Accepted);
        assert_eq!(serde_json::to_vec(&fresh.output).unwrap(), snapshot);
        assert_eq!(fresh.root, healthy.root);
        assert_eq!(fresh.seed, healthy.seed);
        assert_eq!(fresh.factors, healthy.factors);
        assert_eq!(fresh.certificates, healthy.certificates);
        assert_eq!(
            fresh.readback.map(f64::to_bits),
            healthy.readback.map(f64::to_bits)
        );
    }
    for scale in [1.0, 1e14, 1e-10] {
        let failed = run_reduced(&renumber(&Recipe::ThreeLayers.request(128, scale), 113));
        assert_eq!(failed.stage, Stage::Internal);
        assert_eq!(failed.factors, [2, 0]);
        assert_eq!(failed.certificates, [66, 0]);
        assert!(failed.output.is_none() && failed.readback.is_none());
        let fresh = run_reduced(&input);
        assert_eq!(fresh.stage, Stage::Accepted);
        assert_eq!(serde_json::to_vec(&fresh.output).unwrap(), snapshot);
    }
    println!(
        "material lattice real_readback numbered_outputs=2 cancellations=7 failed_requests=3 fresh_json_replays=10 partial_outputs=0 factors={:?} certificates={:?} readbacks={:?}",
        healthy.factors,
        healthy.certificates,
        [original.readback, healthy.readback]
    );
}

fn prepared(sign: f64) -> (Prepared, Case, PairFit) {
    let input = renumber(&Recipe::ThreeLayers.request(100, 1e-10), 113);
    let layout = Rebuilt::prepare_bounded(&input).unwrap();
    let (fixture, refined) = prepare(layout.canonical).unwrap();
    assert!(!refined);
    let direction = rounded(
        &Factor::prepare_shifted(&fixture.directions, fixture.value)
            .unwrap()
            .final_direction(&fixture.seed, 4)
            .unwrap(),
    )
    .into_iter()
    .map(|v| sign * v)
    .collect::<Vec<_>>();
    let case = Case::new(&fixture, &direction);
    let fit = PairFit::prepare_reduced(
        &case.matrix,
        &case.seed,
        case.anchor,
        &case.order(Order::GridNorm),
        64,
    )
    .unwrap();
    (fixture, case, fit)
}

fn checked(fixture: &Prepared, case: &Case, v: &[f64]) -> Result<(f64, Vec<f64>), String> {
    let (relative, residual) = fixture.checked(&case.restore(v))?;
    Ok((
        relative,
        case.permutation.iter().map(|&i| residual[i]).collect(),
    ))
}

fn healthy_bits(sign: f64) -> Vec<u64> {
    let (fixture, case, fit) = prepared(sign);
    let root = fixture.value.to_bits();
    let seed = bits(&fixture.seed);
    let mut calls = 0;
    let Attempt::Accepted(shape) = fit
        .attempt(1 << 22, 1e-8, |v| {
            calls += 1;
            checked(&fixture, &case, v)
        })
        .unwrap()
    else {
        panic!("actual lattice candidate rejected")
    };
    assert_eq!(calls, 66);
    assert_eq!(fixture.value.to_bits(), root);
    assert_eq!(bits(&fixture.seed), seed);
    bits(&shape.into_iter().map(|v| sign * v).collect::<Vec<_>>())
}

#[test]
fn modal_material_lattice_real_final_product_fault_sign_and_cold_replay() {
    let baseline = healthy_bits(1.0);
    assert_eq!(healthy_bits(-1.0), baseline);
    for fail_at in [1, 2, 65, 66] {
        let (fixture, case, fit) = prepared(1.0);
        let mut calls = 0;
        let result = fit.attempt(1 << 22, 1e-8, |v| {
            calls += 1;
            if calls == fail_at {
                Err("actual lattice final product fault".into())
            } else {
                checked(&fixture, &case, v)
            }
        });
        assert_eq!(result.unwrap_err(), "actual lattice final product fault");
        assert_eq!(calls, fail_at);
        assert_eq!(healthy_bits(1.0), baseline);
    }
    for kind in 0..6 {
        let (fixture, case, fit) = prepared(1.0);
        let mut calls = 0;
        let result = fit.attempt(1 << 22, 1e-8, |v| {
            calls += 1;
            let mut receipt = checked(&fixture, &case, v)?;
            if calls == 66 {
                match kind {
                    0 => receipt.0 = f64::NAN,
                    1 => receipt.0 = -1.0,
                    2 => {
                        receipt.1.pop();
                    }
                    3 => receipt.1[0] = f64::INFINITY,
                    4 => receipt.1[0] = 1e51,
                    _ => receipt.0 = 1.0,
                }
            }
            Ok(receipt)
        });
        if kind == 5 {
            assert!(matches!(
                result.unwrap(),
                Attempt::Rejected(Rejection::Residual(1.0))
            ));
        } else {
            assert!(result.unwrap_err().contains("finite matching certificate"));
        }
        assert_eq!(calls, 66);
        assert_eq!(healthy_bits(1.0), baseline);
    }
    let (fixture, case, fit) = prepared(1.0);
    let control = SolverControl::default();
    let cancel = control.clone();
    let calls = Rc::new(Cell::new(0));
    let observed = calls.clone();
    let result = with_solver_observer(
        &control,
        move |p| {
            if observed.get() == 66
                && p.stage == SolverStage::SparseMatvec
                && p.completed_steps == 0
            {
                cancel.request_cancel();
            }
        },
        || {
            fit.attempt(1 << 22, 1e-8, |v| {
                calls.set(calls.get() + 1);
                checked(&fixture, &case, v)
            })
        },
    );
    assert!(result.unwrap_err().contains("cancel"));
    assert_eq!(calls.get(), 66);
    assert_eq!(healthy_bits(1.0), baseline);
    println!(
        "material lattice real_controls signed_directions=2 operator_faults=4 malformed_final_receipts=5 lost_final_gate=1 final_product_cancel=1 fresh_exact_replays=11 partial_candidates=0"
    );
}
