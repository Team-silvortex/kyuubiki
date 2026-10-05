use super::*;
use crate::solver_control::{SolverControl, with_solver_observer};

fn prepare_fit(unit: bool) -> Result<GridFit, String> {
    let matrix: Vec<_> = (0..3)
        .map(|i| (0..3).map(|j| Wide::from(f64::from(i == j))).collect())
        .collect();
    let seed = if unit {
        [1.0 / 3.0, 2.0 / 3.0, 2.0 / 3.0]
    } else {
        [1.0; 3]
    };
    GridFit::prepare(&matrix, &seed, 2, &[0, 1])
}

fn fresh_fit(unit: bool) -> GridFit {
    prepare_fit(unit).unwrap()
}

fn healthy(calls: usize) -> Receipt {
    Receipt {
        internal: f64::from(calls == 1),
        mapped: 1.0 / calls as f64,
        residual: vec![0.0; 3],
    }
}

#[test]
fn modal_coupled_grid_eligible_selection_and_initial_hint_contract() {
    let fit = fresh_fit(false);
    let mut calls = 0;
    let mut expected = Vec::new();
    let outcome = fit
        .attempt_rounded_coupled(4, 4, 1e-8, |v| {
            calls += 1;
            if calls == 3 {
                expected = v.to_vec();
            }
            let (internal, mapped) = match calls {
                1 => (1.0, 0.0),
                2 => (1e-10, 10.0),
                3 | 6 => (1e-9, 1.0),
                _ => (1e-12, 2.0),
            };
            if calls == 6 {
                assert_eq!(v, expected);
            }
            Ok(Receipt {
                internal,
                mapped,
                residual: vec![0.0; 3],
            })
        })
        .unwrap();
    let Attempt::Accepted(shape) = outcome else {
        panic!("eligible selection rejected");
    };
    assert_eq!(shape, expected);
    assert_eq!(shape[2].to_bits(), fit.seed[2].to_bits());
    assert_eq!(calls, 6);
    let mut calls = 0;
    let outcome = fit
        .attempt_rounded_coupled(4, 4, 1e-8, |v| {
            calls += 1;
            assert_eq!(v, fit.seed);
            Ok(Receipt {
                internal: 0.0,
                mapped: 1.0,
                residual: vec![0.0; 3],
            })
        })
        .unwrap();
    assert!(matches!(outcome, Attempt::Accepted(_)));
    assert_eq!(calls, 2);
    println!(
        "coupled grid selection eligible_mapped_rank=1 unchanged_initial_seed=1 mapped_hint_is_not_publication=1"
    );
}

#[test]
fn modal_coupled_grid_faults_final_receipts_cancel_and_preflight() {
    for fail_at in 1..=6 {
        let mut calls = 0;
        let error = fresh_fit(false)
            .attempt_rounded_coupled(4, 4, 1e-8, |_| {
                calls += 1;
                if calls == fail_at {
                    Err("coupled actual operator fault".into())
                } else {
                    Ok(healthy(calls))
                }
            })
            .unwrap_err();
        assert_eq!(error, "coupled actual operator fault");
        assert_eq!(calls, fail_at);
        for kind in 0..8 {
            let mut calls = 0;
            let error = fresh_fit(false)
                .attempt_rounded_coupled(4, 4, 1e-8, |_| {
                    calls += 1;
                    let mut receipt = healthy(calls);
                    if calls == fail_at {
                        match kind {
                            0 => receipt.internal = f64::NAN,
                            1 => receipt.internal = -1.0,
                            2 => receipt.residual.pop().map(|_| ()).unwrap(),
                            3 => receipt.residual[0] = f64::INFINITY,
                            4 => receipt.residual[0] = 1e51,
                            5 => receipt.mapped = f64::NAN,
                            6 => receipt.mapped = -1.0,
                            _ => receipt.mapped = f64::INFINITY,
                        }
                    }
                    Ok(receipt)
                })
                .unwrap_err();
            assert!(error.contains("finite"), "{error}");
            assert_eq!(calls, fail_at);
        }
    }
    for initially_passing in [false, true] {
        let end = if initially_passing { 2 } else { 6 };
        let mut calls = 0;
        let outcome = fresh_fit(false)
            .attempt_rounded_coupled(4, 4, 1e-8, |_| {
                calls += 1;
                let mut receipt = healthy(if initially_passing { 2 } else { calls });
                if calls == end {
                    receipt.internal = 1.0;
                }
                Ok(receipt)
            })
            .unwrap();
        assert!(matches!(
            outcome,
            Attempt::Rejected(Rejection::Residual(1.0))
        ));
        assert_eq!(calls, end);
    }
    for (stage, steps) in [
        (SolverStage::DenseFactor, 2),
        (SolverStage::DenseSubstitution, 0),
        (SolverStage::DenseSubstitution, 4),
        (SolverStage::ModalVectorUpdate, 2),
        (SolverStage::ModalRoundoffSearch, 5),
        (SolverStage::ModalRoundoffValidate, 1),
    ] {
        let control = SolverControl::default();
        let cancel = control.clone();
        let error = with_solver_observer(
            &control,
            move |p| {
                if p.stage == stage && p.completed_steps == steps {
                    cancel.request_cancel();
                }
            },
            || {
                let fit = prepare_fit(false)?;
                let mut calls = 0;
                fit.attempt_rounded_coupled(4, 4, 1e-8, |_| {
                    calls += 1;
                    Ok(healthy(calls))
                })
            },
        )
        .unwrap_err();
        assert!(error.contains("cancel"), "{stage:?}: {error}");
        let mut calls = 0;
        let fit = fresh_fit(false);
        let outcome = fit
            .attempt_rounded_coupled(4, 4, 1e-8, |_| {
                calls += 1;
                Ok(healthy(calls))
            })
            .unwrap();
        assert!(matches!(outcome, Attempt::Accepted(_)));
        assert_eq!(calls, 6);
    }
    for width in [0, 65, usize::MAX] {
        assert!(
            fresh_fit(false)
                .attempt_rounded_coupled(4, width, 1e-8, |_| unreachable!())
                .is_err()
        );
    }
    for radius in [0, MAX_GRID_RADIUS + 1, usize::MAX] {
        assert!(
            fresh_fit(false)
                .attempt_rounded_coupled(radius, 4, 1e-8, |_| unreachable!())
                .is_err()
        );
    }
    for tolerance in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(
            fresh_fit(false)
                .attempt_rounded_coupled(4, 4, tolerance, |_| unreachable!())
                .is_err()
        );
    }
    let mut fit = fresh_fit(false);
    fit.seed.fill(0.0);
    assert!(
        fit.attempt_rounded_coupled(4, 4, 1e-8, |_| unreachable!())
            .is_err()
    );
    let matrix: Vec<_> = (0..3)
        .map(|i| (0..3).map(|j| Wide::from(f64::from(i == j))).collect())
        .collect();
    let fit = fresh_fit(false);
    let wide = GridFit::prepare_wide(&matrix, &fit.seed, 2, &[0, 1], false).unwrap();
    assert!(
        wide.attempt_rounded_coupled(4, 4, 1e-8, |_| unreachable!())
            .is_err()
    );
    println!(
        "coupled grid controls faults=6 malformed_receipts=48 lost_final_gates=2 cancellations=6 healthy_replays=6 partial_candidates=0"
    );
}

#[test]
fn modal_coupled_grid_known_null_and_physical_unit_gate() {
    let base = 3.0_f64.sqrt().recip();
    let step = base.next_up() - base;
    let seed = [base + 5.0 * step, base - 3.0 * step, base];
    let matrix = [[1.0, -1.0, 0.0], [-1.0, 2.0, -1.0], [0.0, -1.0, 1.0]];
    for sign in [-1.0, 1.0] {
        for exponent in [-80, 0, 80] {
            let scale = 2.0_f64.powi(exponent);
            let matrix: Vec<_> = matrix
                .iter()
                .map(|row| row.iter().map(|v| Wide::from(v * scale)).collect())
                .collect();
            let signed: Vec<_> = seed.iter().map(|v| sign * v).collect();
            let fit = GridFit::prepare(&matrix, &signed, 2, &[0, 1]).unwrap();
            let checked = |v: &[f64]| {
                let residual: Vec<_> = matrix
                    .iter()
                    .map(|row| {
                        row.iter()
                            .zip(v)
                            .fold(Wide::default(), |sum, (&a, &x)| {
                                sum.add(a.mul(Wide::from(x)))
                            })
                            .rounded()
                    })
                    .collect();
                let relative = super::super::super::vector_norm(residual.iter().copied())? / scale;
                Ok((relative, residual))
            };
            let outcome = fit
                .attempt_rounded_coupled(1 << 22, 16, 1e-18, |v| {
                    let (internal, residual) = checked(v)?;
                    Ok(Receipt {
                        internal,
                        mapped: internal,
                        residual,
                    })
                })
                .unwrap();
            let Attempt::Accepted(shape) = outcome else {
                panic!("known internal null rejected");
            };
            assert_eq!(shape, [sign * base; 3]);
            let outcome = fit
                .attempt_rounded_unit_beam(1 << 22, 16, 1e-18, checked)
                .unwrap();
            let Attempt::Accepted(shape) = outcome else {
                panic!("known physical null rejected");
            };
            assert_eq!(shape, [sign * base; 3]);
        }
    }
    assert!(
        fresh_fit(false)
            .attempt_rounded_unit_beam(4, 4, 1e-8, |_| unreachable!())
            .is_err()
    );
    let identity: Vec<_> = (0..3)
        .map(|i| (0..3).map(|j| Wide::from(f64::from(i == j))).collect())
        .collect();
    let fit = GridFit::prepare(&identity, &[base; 3], 2, &[0, 1]).unwrap();
    let mut calls = 0;
    let outcome = fit
        .attempt_rounded_unit_beam(1 << 22, 4, 1e-8, |_| {
            calls += 1;
            Ok((f64::from(calls == 1), vec![-1.0, -1.0, 0.0]))
        })
        .unwrap();
    assert!(
        matches!(outcome, Attempt::Rejected(Rejection::UnitNorm(_))),
        "{outcome:?}"
    );
    assert_eq!(calls, 6);
    println!(
        "coupled grid analytic sign_binary_controls=6 known_internal_nulls=6 known_physical_nulls=6 norm_loss_without_renormalizing=1"
    );
}
