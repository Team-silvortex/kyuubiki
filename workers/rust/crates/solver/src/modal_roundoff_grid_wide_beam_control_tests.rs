use super::*;
use crate::solver_control::{SolverControl, with_solver_observer};
use std::{cell::RefCell, rc::Rc};

fn identity() -> Vec<Vec<Wide>> {
    (0..3)
        .map(|i| (0..3).map(|j| Wide::from(f64::from(i == j))).collect())
        .collect()
}

fn fit() -> GridFit {
    GridFit::prepare_wide(&identity(), &[1.5; 3], 2, &[0, 1], false).unwrap()
}

#[test]
fn triangular_grid_wide_beam_preflight_budget_and_frozen_seed() {
    let seed = [1.5; 3];
    let rounded = GridFit::prepare(&identity(), &seed, 2, &[0, 1]).unwrap();
    assert!(
        rounded
            .attempt_wide_beam(4, 1e-8, |_| unreachable!())
            .is_err()
    );
    let fit = fit();
    for radius in [0, MAX_GRID_RADIUS + 1, usize::MAX] {
        assert!(
            fit.attempt_wide_beam(radius, 1e-8, |_| unreachable!())
                .is_err()
        );
    }
    for tolerance in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(
            fit.attempt_wide_beam(4, tolerance, |_| unreachable!())
                .is_err()
        );
    }
    assert!(
        fit.correct(4, Policy::Independent, 1e-8, |_| unreachable!())
            .is_err()
    );
    let visits = Rc::new(RefCell::new(Vec::new()));
    let observed = visits.clone();
    let factors = Rc::new(RefCell::new(0));
    let observed_factors = factors.clone();
    let mut seen = Vec::new();
    let rejected = with_solver_observer(
        &SolverControl::default(),
        move |p| {
            if p.stage == SolverStage::ModalRoundoffSearch {
                observed.borrow_mut().push(p.completed_steps);
            }
            if p.stage == SolverStage::DenseFactor {
                *observed_factors.borrow_mut() += 1;
            }
        },
        || {
            fit.attempt_wide_beam(4, 1e-8, |v| {
                seen.push(v.to_vec());
                Ok((if v == seed { 1.0 } else { 2.0 }, vec![0.0; 3]))
            })
        },
    )
    .unwrap();
    assert!(matches!(
        rejected,
        Attempt::Rejected(Rejection::Residual(1.0))
    ));
    assert_eq!(seen.len(), MAX_CERTIFICATES);
    assert_eq!(visits.borrow().as_slice(), [0, 1, 2, 3, 4, 5]);
    assert_eq!(*factors.borrow(), 0);
    assert_eq!(seen.first(), seen.last());
    assert!(seen.iter().all(|v| v[2].to_bits() == seed[2].to_bits()));
    assert_eq!(fit.seed, seed);
}

#[test]
fn triangular_grid_wide_beam_operator_faults_and_fresh_final_certificates_fail_closed() {
    let fit = fit();
    for fail_at in 1..=MAX_CERTIFICATES {
        let mut calls = 0;
        let error = fit
            .attempt_wide_beam(4, 1e-8, |_| {
                calls += 1;
                if calls == fail_at {
                    Err("actual beam operator fault".into())
                } else {
                    Ok((1.0, vec![0.0; 3]))
                }
            })
            .unwrap_err();
        assert_eq!(error, "actual beam operator fault");
        assert_eq!(calls, fail_at);
        for malformed in [
            (f64::NAN, vec![0.0; 3]),
            (-1.0, vec![0.0; 3]),
            (0.0, vec![0.0; 2]),
            (0.0, vec![f64::INFINITY; 3]),
            (0.0, vec![1e51; 3]),
        ] {
            let mut calls = 0;
            let error = fit
                .attempt_wide_beam(4, 1e-8, |_| {
                    calls += 1;
                    Ok(if calls == fail_at {
                        malformed.clone()
                    } else {
                        (1.0, vec![0.0; 3])
                    })
                })
                .unwrap_err();
            assert!(error.contains("finite matching certificate"), "{error}");
            assert_eq!(calls, fail_at);
        }
    }
    for initially_accepted in [false, true] {
        let mut calls = 0;
        let outcome = fit
            .attempt_wide_beam(4, 1e-8, |_| {
                calls += 1;
                let relative = if initially_accepted {
                    f64::from(calls > 1)
                } else {
                    f64::from(calls != 2)
                };
                Ok((relative, vec![0.0; 3]))
            })
            .unwrap();
        assert!(matches!(
            outcome,
            Attempt::Rejected(Rejection::Residual(1.0))
        ));
        assert_eq!(
            calls,
            if initially_accepted {
                2
            } else {
                MAX_CERTIFICATES
            }
        );
    }
}

#[test]
fn triangular_grid_wide_beam_cancellation_and_fresh_replay_never_adopt_partial_state() {
    let fit = fit();
    for (stage, steps) in [
        (SolverStage::DenseSubstitution, 0),
        (SolverStage::ModalVectorUpdate, 2),
        (SolverStage::ModalRoundoffSearch, 5),
        (SolverStage::ModalRoundoffValidate, 1),
    ] {
        let control = SolverControl::default();
        let cancel = control.clone();
        let mut calls = 0;
        let error = with_solver_observer(
            &control,
            move |p| {
                if p.stage == stage && p.completed_steps == steps {
                    cancel.request_cancel();
                }
            },
            || {
                fit.attempt_wide_beam(4, 1e-8, |_| {
                    calls += 1;
                    Ok((f64::from(calls == 1), vec![0.0; 3]))
                })
            },
        )
        .unwrap_err();
        assert!(error.contains("cancel"), "{stage:?}: {error}");
        assert!(calls <= MAX_CERTIFICATES);
        let mut replay_calls = 0;
        let replay = fit
            .attempt_wide_beam(4, 1e-8, |_| {
                replay_calls += 1;
                Ok((f64::from(replay_calls == 1), vec![0.0; 3]))
            })
            .unwrap();
        assert!(matches!(replay, Attempt::Accepted(_)));
        assert_eq!(replay_calls, MAX_CERTIFICATES);
        assert_eq!(fit.seed, [1.5; 3]);
    }
    let control = SolverControl::default();
    let cancel = control.clone();
    let error = with_solver_observer(
        &control,
        move |p| {
            if p.stage == SolverStage::DenseFactor && p.completed_steps == 1 {
                cancel.request_cancel();
            }
        },
        || GridFit::prepare_wide(&identity(), &[1.5; 3], 2, &[0, 1], false),
    )
    .err()
    .unwrap();
    assert!(error.contains("cancel"), "{error}");
    assert_eq!(
        GridFit::prepare_wide(&identity(), &[1.5; 3], 2, &[0, 1], false)
            .unwrap()
            .seed,
        [1.5; 3]
    );
}
