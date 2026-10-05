use super::*;
use crate::solver_control::{SolverControl, with_solver_observer};
use std::{cell::Cell, rc::Rc};

fn fresh() -> GridFit {
    let matrix: Vec<_> = (0..6)
        .map(|i| (0..6).map(|j| Wide::from(f64::from(i == j))).collect())
        .collect();
    GridFit::prepare(&matrix, &[0.3; 6], 5, &[0, 1, 2, 3, 4]).unwrap()
}

fn receipt(call: usize, last: usize) -> (f64, Vec<f64>) {
    (
        if call == last { 0.0 } else { 1.0 / call as f64 },
        vec![0.0; 6],
    )
}

fn attempt(
    fit: &GridFit,
    hybrid: bool,
    checked: impl FnMut(&[f64]) -> Result<(f64, Vec<f64>), String>,
) -> Result<Attempt, String> {
    if hybrid {
        fit.attempt_direction_iterated_hybrid(4, 4, 4, 1e-8, checked)
    } else {
        fit.attempt_iterated_beam(4, 4, 4, 1e-8, checked)
    }
}

#[test]
fn modal_iterated_grid_budget_legacy_retention_and_frozen_radius() {
    let fit = fresh();
    for hybrid in [false, true] {
        let mut calls = 0;
        let actual = attempt(&fit, hybrid, |_| {
            calls += 1;
            Ok((0.0, vec![0.0; 6]))
        })
        .unwrap();
        let Attempt::Accepted(shape) = actual else {
            panic!("legacy seed rejected")
        };
        assert_eq!(shape, fit.seed);
        assert_eq!(calls, 2);
        let mut calls = 0;
        let end = if hybrid { 72 } else { 66 };
        let checked = |v: &[f64]| {
            calls += 1;
            assert_eq!(v[5].to_bits(), fit.seed[5].to_bits());
            for &i in &fit.order {
                let distance = Wide::from(v[i])
                    .sub(Wide::from(fit.seed[i]))
                    .div(Wide::from(fit.grids[i]))
                    .rounded();
                assert!(distance.abs() <= MAX_GRID_RADIUS as f64 && distance.fract() == 0.0);
            }
            Ok(receipt(calls, end))
        };
        let actual = if hybrid {
            fit.attempt_direction_iterated_hybrid(MAX_GRID_RADIUS, 16, 4, 1e-8, checked)
        } else {
            fit.attempt_iterated_beam(MAX_GRID_RADIUS, 16, 4, 1e-8, checked)
        }
        .unwrap();
        assert!(matches!(actual, Attempt::Accepted(_)));
        assert_eq!(calls, end);
    }
    for (width, passes) in [
        (0, 1),
        (65, 1),
        (usize::MAX, 1),
        (16, 0),
        (16, 5),
        (16, usize::MAX),
        (17, 4),
        (64, 2),
    ] {
        assert!(Plan::iterated_beam(256, width, passes).is_err());
        assert!(
            fit.attempt_direction_iterated_hybrid(4, width, passes, 1e-8, |_| unreachable!())
                .is_err()
        );
    }
    for size in [0, 1, 257, usize::MAX] {
        assert!(Plan::iterated_beam(size, 16, 4).is_err());
    }
    for radius in [0, MAX_GRID_RADIUS + 1, usize::MAX] {
        assert!(
            fit.attempt_direction_iterated_hybrid(radius, 4, 4, 1e-8, |_| unreachable!())
                .is_err()
        );
    }
    for tolerance in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(
            fit.attempt_direction_iterated_hybrid(4, 4, 4, tolerance, |_| unreachable!())
                .is_err()
        );
    }
    let plan = Plan::iterated_beam(256, 16, 4).unwrap();
    assert_eq!(plan.payload_bytes, 4_747_264);
    assert_eq!(plan.component_visits, 608_403_456);
    println!(
        "iterated grid budget reached_plain_receipts=66 reached_hybrid_receipts=72 total_branches=64 payload_bytes={} component_visits={}",
        plan.payload_bytes, plan.component_visits
    );
}

#[test]
fn modal_iterated_grid_faults_stale_final_cancel_and_replay() {
    for hybrid in [false, true] {
        let end = if hybrid { 24 } else { 18 };
        for fail_at in 1..=end {
            let mut calls = 0;
            let error = attempt(&fresh(), hybrid, |_| {
                calls += 1;
                if calls == fail_at {
                    Err("iterated actual operator fault".into())
                } else {
                    Ok(receipt(calls, end))
                }
            })
            .unwrap_err();
            assert_eq!(error, "iterated actual operator fault");
            assert_eq!(calls, fail_at);
            for kind in 0..5 {
                let mut calls = 0;
                let error = attempt(&fresh(), hybrid, |_| {
                    calls += 1;
                    let mut value = receipt(calls, end);
                    if calls == fail_at {
                        match kind {
                            0 => value.0 = f64::NAN,
                            1 => value.0 = -1.0,
                            2 => {
                                value.1.pop();
                            }
                            3 => value.1[0] = f64::INFINITY,
                            _ => value.1[0] = 1e51,
                        }
                    }
                    Ok(value)
                })
                .unwrap_err();
                assert!(error.contains("finite matching certificate"), "{error}");
                assert_eq!(calls, fail_at);
            }
        }
        let mut calls = 0;
        let actual = attempt(&fresh(), hybrid, |_| {
            calls += 1;
            Ok((f64::from(calls != 1), vec![0.0; 6]))
        });
        if hybrid {
            assert!(actual.unwrap_err().contains("lost a passing legacy"));
        } else {
            assert!(matches!(
                actual.unwrap(),
                Attempt::Rejected(Rejection::Residual(1.0))
            ));
        }
        assert_eq!(calls, 2);
    }
    for (stage, steps) in [
        (SolverStage::ModalRoundoffSearch, 0),
        (SolverStage::ModalRoundoffSearch, 5),
        (SolverStage::ModalRoundoffSearch, 17),
        (SolverStage::ModalIteration, 1),
        (SolverStage::DenseSubstitution, 0),
        (SolverStage::ModalRoundoffValidate, 8),
    ] {
        let fit = fresh();
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
                attempt(&fit, false, |_| {
                    calls += 1;
                    Ok(receipt(calls, 18))
                })
            },
        )
        .unwrap_err();
        assert!(seen.get() && error.contains("cancel"), "{stage:?}: {error}");
        let mut calls = 0;
        assert!(matches!(
            attempt(&fresh(), false, |_| {
                calls += 1;
                Ok(receipt(calls, 18))
            })
            .unwrap(),
            Attempt::Accepted(_)
        ));
        assert_eq!(calls, 18);
    }
    println!(
        "iterated grid controls callback_faults=42 malformed_receipts=210 stale_final_gates=2 cancellations=6 fresh_replays=6 partial_candidates=0"
    );
}

#[test]
fn modal_iterated_grid_known_null_sign_binary_scale_and_preflight() {
    let base = 1.0 / 3.0_f64.sqrt();
    for hybrid in [false, true] {
        for sign in [-1.0, 1.0] {
            for scale in [0.0625, 1.0, 16.0] {
                let matrix: Vec<Vec<_>> = [[1.0, -1.0, 0.0], [-1.0, 2.0, -1.0], [0.0, -1.0, 1.0]]
                    .iter()
                    .map(|r| r.iter().map(|a| Wide::from(scale * a)).collect())
                    .collect();
                let seed = [sign * base.next_up(), sign * base, sign * base.next_down()];
                let fit = GridFit::prepare(&matrix, &seed, 1, &[0, 2]).unwrap();
                let checked = |v: &[f64]| {
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
                };
                let actual = if hybrid {
                    fit.attempt_direction_iterated_hybrid(4, 4, 4, 1e-20, checked)
                } else {
                    fit.attempt_iterated_beam(4, 4, 4, 1e-20, checked)
                }
                .unwrap();
                let Attempt::Accepted(shape) = actual else {
                    panic!("known analytic null rejected")
                };
                assert_eq!(shape, [sign * base; 3]);
            }
        }
    }
    let mut fit = fresh();
    fit.seed.fill(0.0);
    assert!(attempt(&fit, true, |_| unreachable!()).is_err());
    let matrix: Vec<_> = (0..6)
        .map(|i| (0..6).map(|j| Wide::from(f64::from(i == j))).collect())
        .collect();
    let fit = GridFit::prepare_wide(&matrix, &[0.3; 6], 5, &[0, 1, 2, 3, 4], false).unwrap();
    assert!(attempt(&fit, true, |_| unreachable!()).is_err());
    println!("iterated grid analytic signed_binary_nulls=12 invalid_norm=1 incompatible_factor=1");
}
