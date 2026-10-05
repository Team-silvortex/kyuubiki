use super::*;
use crate::solver_control::{SolverControl, with_solver_observer};
use std::{cell::Cell, rc::Rc};

fn fresh() -> Result<GridFit, String> {
    let matrix: Vec<_> = (0..3)
        .map(|i| (0..3).map(|j| Wide::from(f64::from(i == j))).collect())
        .collect();
    GridFit::prepare(&matrix, &[1.0 / 3.0, 2.0 / 3.0, 2.0 / 3.0], 2, &[0, 1])
}

fn healthy(call: usize) -> (f64, Vec<f64>) {
    (f64::from(call <= 4), vec![0.0; 3])
}

#[test]
fn modal_hybrid_grid_legacy_retention_and_shared_factor_budget() {
    let fit = fresh().unwrap();
    let mut calls = 0;
    let actual = fit
        .attempt_unit_hybrid(4, 4, 1e-8, |_| {
            calls += 1;
            Ok((0.0, vec![0.0; 3]))
        })
        .unwrap();
    let Attempt::Accepted(shape) = actual else {
        panic!("legacy seed rejected");
    };
    assert_eq!(shape, fit.seed);
    assert_eq!(calls, 3);
    let mut calls = 0;
    let actual = fit
        .attempt_unit_hybrid(4, 4, 1e-8, |_| {
            calls += 1;
            Ok(healthy(calls))
        })
        .unwrap();
    let Attempt::Accepted(shape) = actual else {
        panic!("supplemental beam rejected");
    };
    assert_eq!(shape, fit.seed);
    assert_eq!(calls, 10);
    let matrix: Vec<_> = (0..6)
        .map(|i| (0..6).map(|j| Wide::from(f64::from(i == j))).collect())
        .collect();
    let seed = [1.0 / 6.0_f64.sqrt(); 6];
    let fit = GridFit::prepare(&matrix, &seed, 5, &[0, 1, 2, 3, 4]).unwrap();
    let mut calls = 0;
    let actual = fit
        .attempt_unit_hybrid(4, 64, 1e-8, |_| {
            calls += 1;
            let error = match calls {
                1..=5 => 1.0 / calls as f64,
                6 => 0.2,
                7 => 1.0,
                _ => 0.0,
            };
            Ok((error, vec![0.0; 6]))
        })
        .unwrap();
    assert!(matches!(actual, Attempt::Accepted(_)));
    assert_eq!(calls, 73);
    for size in [0, 1, 257, usize::MAX] {
        assert!(Plan::rounded_hybrid(size, 64).is_err());
    }
    for width in [0, 65, usize::MAX] {
        assert!(
            fit.attempt_unit_hybrid(4, width, 1e-8, |_| unreachable!())
                .is_err()
        );
    }
    for radius in [0, MAX_GRID_RADIUS + 1, usize::MAX] {
        assert!(
            fit.attempt_unit_hybrid(radius, 4, 1e-8, |_| unreachable!())
                .is_err()
        );
    }
    for tolerance in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(
            fit.attempt_unit_hybrid(4, 4, tolerance, |_| unreachable!())
                .is_err()
        );
    }
    let bounds = Plan::rounded_hybrid(256, 64).unwrap();
    assert_eq!(bounds.payload_bytes, 4_739_072);
    assert_eq!(bounds.component_visits, 608_403_456);
    println!(
        "hybrid grid retention legacy_seed=1 shared_factor=1 width64_receipt_cap=73 reached_receipts=73 payload_bytes={} component_visits={}",
        bounds.payload_bytes, bounds.component_visits
    );
}

#[test]
fn modal_hybrid_grid_faults_stale_gates_cancel_and_replay() {
    for fail_at in 1..=10 {
        let mut calls = 0;
        let error = fresh()
            .unwrap()
            .attempt_unit_hybrid(4, 4, 1e-8, |_| {
                calls += 1;
                if calls == fail_at {
                    return Err("hybrid actual operator fault".into());
                }
                Ok(healthy(calls))
            })
            .unwrap_err();
        assert_eq!(calls, fail_at);
        assert_eq!(error, "hybrid actual operator fault");
        for kind in 0..5 {
            let mut calls = 0;
            let error = fresh()
                .unwrap()
                .attempt_unit_hybrid(4, 4, 1e-8, |_| {
                    calls += 1;
                    let mut receipt = healthy(calls);
                    if calls == fail_at {
                        match kind {
                            0 => receipt.0 = f64::NAN,
                            1 => receipt.0 = -1.0,
                            2 => receipt.1.pop().map(|_| ()).unwrap(),
                            3 => receipt.1[0] = f64::INFINITY,
                            _ => receipt.1[0] = 1e51,
                        }
                    }
                    Ok(receipt)
                })
                .unwrap_err();
            assert_eq!(calls, fail_at);
            assert!(error.contains("finite matching certificate"), "{error}");
        }
    }
    for last in [2, 3, 9, 10] {
        let mut calls = 0;
        let actual = fresh().unwrap().attempt_unit_hybrid(4, 4, 1e-8, |_| {
            calls += 1;
            let pass = if last <= 3 { true } else { calls > 4 };
            Ok((f64::from(!pass || calls == last), vec![0.0; 3]))
        });
        assert_eq!(calls, last);
        match last {
            9 => assert!(matches!(actual.unwrap(), Attempt::Rejected(_))),
            _ => assert!(actual.unwrap_err().contains("final physical certificate")),
        }
    }
    for (stage, steps) in [
        (SolverStage::DenseFactor, 2),
        (SolverStage::ModalIteration, PASSES as u64),
        (SolverStage::ModalRoundoffSearch, 8),
        (SolverStage::ModalRoundoffSearch, 9),
        (SolverStage::ModalRoundoffValidate, 7),
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
            || {
                let fit = fresh()?;
                let mut calls = 0;
                fit.attempt_unit_hybrid(4, 4, 1e-8, |_| {
                    calls += 1;
                    Ok(healthy(calls))
                })
            },
        )
        .unwrap_err();
        assert!(seen.get() && error.contains("cancel"), "{stage:?}: {error}");
        let mut calls = 0;
        let actual = fresh()
            .unwrap()
            .attempt_unit_hybrid(4, 4, 1e-8, |_| {
                calls += 1;
                Ok(healthy(calls))
            })
            .unwrap();
        assert!(matches!(actual, Attempt::Accepted(_)));
        assert_eq!(calls, 10);
    }
    println!(
        "hybrid grid controls faults=10 malformed_receipts=50 lost_final_gates=4 cancellations=5 fresh_replays=5 partial_candidates=0"
    );
}

#[test]
fn modal_hybrid_grid_analytic_null_and_norm_rejection_no_continuation() {
    let base = 1.0 / 3.0_f64.sqrt();
    for sign in [-1.0, 1.0] {
        for scale in [0.0625, 1.0, 16.0] {
            let matrix: Vec<Vec<_>> = [[1.0, -1.0, 0.0], [-1.0, 2.0, -1.0], [0.0, -1.0, 1.0]]
                .iter()
                .map(|row| row.iter().map(|a| Wide::from(scale * a)).collect())
                .collect();
            let seed = [sign * base.next_up(), sign * base, sign * base.next_down()];
            let fit = GridFit::prepare(&matrix, &seed, 1, &[0, 2]).unwrap();
            let actual = fit
                .attempt_unit_hybrid(4, 4, 1e-20, |v| {
                    let residual: Vec<_> = matrix
                        .iter()
                        .map(|row| {
                            row.iter()
                                .zip(v)
                                .fold(Wide::default(), |sum, (a, b)| {
                                    sum.add(a.mul(Wide::from(*b)))
                                })
                                .rounded()
                        })
                        .collect();
                    Ok((
                        super::super::super::vector_norm(residual.iter().copied())?,
                        residual,
                    ))
                })
                .unwrap();
            let Attempt::Accepted(shape) = actual else {
                panic!("known analytic null rejected");
            };
            assert_eq!(shape, [sign * base; 3]);
        }
    }
    let fit = fresh().unwrap();
    let mut calls = 0;
    let actual = fit
        .attempt_unit_hybrid(MAX_GRID_RADIUS, 4, 1e-8, |_| {
            calls += 1;
            Ok((f64::from(calls == 1), vec![-1e-8; 3]))
        })
        .unwrap();
    assert!(matches!(actual, Attempt::Rejected(Rejection::UnitNorm(_))));
    assert_eq!(calls, 3);
    let mut bad = fresh().unwrap();
    bad.seed.fill(1.0);
    assert!(
        bad.attempt_unit_hybrid(4, 4, 1e-8, |_| unreachable!())
            .is_err()
    );
    let matrix: Vec<_> = (0..3)
        .map(|i| (0..3).map(|j| Wide::from(f64::from(i == j))).collect())
        .collect();
    let wide = GridFit::prepare_wide(&matrix, &fit.seed, 2, &[0, 1], false).unwrap();
    assert!(
        wide.attempt_unit_hybrid(4, 4, 1e-8, |_| unreachable!())
            .is_err()
    );
    println!(
        "hybrid grid analytic sign_binary_nulls=6 norm_rejections_without_beam=1 incompatible_factors_declined=1"
    );
}
