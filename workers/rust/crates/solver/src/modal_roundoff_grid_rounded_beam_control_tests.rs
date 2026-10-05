use super::*;
use crate::solver_control::{SolverControl, with_solver_observer};

fn fresh_fit() -> GridFit {
    let identity: Vec<_> = (0..3)
        .map(|i| (0..3).map(|j| Wide::from(f64::from(i == j))).collect())
        .collect();
    GridFit::prepare(&identity, &[0.2, 0.4, 0.6], 2, &[0, 1]).unwrap()
}

#[test]
fn triangular_grid_rounded_beam_faults_final_certificates_and_cancel_replay() {
    let fit = fresh_fit();
    for fail_at in 1..=6 {
        let mut calls = 0;
        let error = fit
            .attempt_rounded_beam(4, 4, 1e-8, |_| {
                calls += 1;
                if calls == fail_at {
                    Err("rounded beam actual operator fault".into())
                } else {
                    Ok((f64::from(calls == 1), vec![0.0; 3]))
                }
            })
            .unwrap_err();
        assert_eq!(calls, fail_at);
        assert_eq!(error, "rounded beam actual operator fault");
        for malformed in [
            (f64::NAN, vec![0.0; 3]),
            (-1.0, vec![0.0; 3]),
            (0.0, vec![0.0; 2]),
            (0.0, vec![f64::INFINITY; 3]),
            (0.0, vec![1e51; 3]),
        ] {
            let mut calls = 0;
            let error = fit
                .attempt_rounded_beam(4, 4, 1e-8, |_| {
                    calls += 1;
                    Ok(if calls == fail_at {
                        malformed.clone()
                    } else {
                        (f64::from(calls == 1), vec![0.0; 3])
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
            .attempt_rounded_beam(4, 4, 1e-8, |_| {
                calls += 1;
                Ok((
                    f64::from(if initially_accepted {
                        calls > 1
                    } else {
                        calls == 1 || calls == 6
                    }),
                    vec![0.0; 3],
                ))
            })
            .unwrap();
        assert!(matches!(
            outcome,
            Attempt::Rejected(Rejection::Residual(1.0))
        ));
        assert_eq!(calls, if initially_accepted { 2 } else { 6 });
    }
    for (stage, steps) in [
        (SolverStage::DenseSubstitution, 0),
        (SolverStage::DenseSubstitution, 4),
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
                fit.attempt_rounded_beam(4, 4, 1e-8, |_| {
                    calls += 1;
                    Ok((f64::from(calls == 1), vec![0.0; 3]))
                })
            },
        )
        .unwrap_err();
        assert!(error.contains("cancel"), "{stage:?}: {error}");
        assert!(calls <= 6);
        let mut calls = 0;
        let Attempt::Accepted(replay) = fresh_fit()
            .attempt_rounded_beam(4, 4, 1e-8, |_| {
                calls += 1;
                Ok((f64::from(calls == 1), vec![0.0; 3]))
            })
            .unwrap()
        else {
            panic!("fresh replay failed");
        };
        assert_eq!(calls, 6);
        assert_eq!(replay, [0.2, 0.4, 0.6]);
        assert_eq!(fit.seed, replay);
    }
    for width in [0, 65, usize::MAX] {
        assert!(
            fit.attempt_rounded_beam(4, width, 1e-8, |_| unreachable!())
                .is_err()
        );
    }
    for radius in [0, MAX_GRID_RADIUS + 1, usize::MAX] {
        assert!(
            fit.attempt_rounded_beam(radius, 4, 1e-8, |_| unreachable!())
                .is_err()
        );
    }
    for tolerance in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(
            fit.attempt_rounded_beam(4, 4, tolerance, |_| unreachable!())
                .is_err()
        );
    }
    let mut invalid_norm = fresh_fit();
    invalid_norm.seed.fill(0.0);
    assert!(
        invalid_norm
            .attempt_rounded_beam(4, 4, 1e-8, |_| unreachable!())
            .is_err()
    );
    let matrix: Vec<_> = (0..3)
        .map(|i| (0..3).map(|j| Wide::from(f64::from(i == j))).collect())
        .collect();
    let preselected = GridFit::prepare_wide(&matrix, &fit.seed, 2, &[0, 1], false).unwrap();
    assert!(
        preselected
            .attempt_rounded_beam(4, 4, 1e-8, |_| unreachable!())
            .is_err()
    );
    for size in [0, 1, 257, usize::MAX] {
        assert!(Plan::rounded_beam(size, 64).is_err());
    }
    let bounds = Plan::rounded_beam(256, 64).unwrap();
    assert_eq!(bounds.payload_bytes, 4_734_976);
    assert_eq!(bounds.component_visits, 603_979_776);
    println!(
        "rounded beam grid controls faults=6 malformed_receipts=30 lost_final_certificates=2 cancellations=5 healthy_replays=5 no_partial_candidate=1"
    );
}
