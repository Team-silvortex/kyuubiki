use super::*;
use crate::solver_control::{SolverControl, with_solver_observer};

fn identity() -> Vec<Vec<Wide>> {
    (0..3)
        .map(|i| (0..3).map(|j| Wide::from(f64::from(i == j))).collect())
        .collect()
}

fn fresh_fit() -> GridFit {
    GridFit::prepare_givens(&identity(), &[1.5; 3], 2, &[0, 1], false).unwrap()
}

#[test]
fn triangular_grid_givens_faults_final_certificates_and_cancel_replay() {
    let fit = fresh_fit();
    for fail_at in 1..=MAX_CERTIFICATES {
        let mut calls = 0;
        let error = fit
            .attempt_wide_beam(4, 1e-8, |_| {
                calls += 1;
                if calls == fail_at {
                    Err("givens actual operator fault".into())
                } else {
                    Ok((1.0, vec![0.0; 3]))
                }
            })
            .unwrap_err();
        assert_eq!(calls, fail_at);
        assert_eq!(error, "givens actual operator fault");
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
    for initially_accepted in [true, false] {
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
        let mut calls = 0;
        let replay = fit
            .attempt_wide_beam(4, 1e-8, |_| {
                calls += 1;
                Ok((f64::from(calls == 1), vec![0.0; 3]))
            })
            .unwrap();
        let Attempt::Accepted(replay) = replay else {
            panic!("fresh replay failed");
        };
        assert_eq!(calls, MAX_CERTIFICATES);
        assert_eq!(replay[2].to_bits(), 1.5_f64.to_bits());
        assert_eq!(fit.seed, [1.5; 3]);
    }
    for (stage, steps) in [
        (SolverStage::DenseFactor, 2),
        (SolverStage::ModalVectorUpdate, 3),
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
            || GridFit::prepare_givens(&identity(), &[1.5; 3], 2, &[0, 1], false),
        )
        .err()
        .unwrap();
        assert!(error.contains("cancel"), "{error}");
        assert_eq!(fresh_fit().seed, [1.5; 3]);
    }
    println!(
        "wide givens controls faults=6 malformed_receipts=30 lost_final_certificates=2 cancellations=6 healthy_replays=6 no_partial_candidate=1"
    );
}
