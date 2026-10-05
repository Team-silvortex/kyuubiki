use super::*;
use crate::solver_control::{SolverControl, with_solver_observer};

fn fresh_fit() -> GridFit {
    let identity: Vec<_> = (0..3)
        .map(|i| (0..3).map(|j| Wide::from(f64::from(i == j))).collect())
        .collect();
    GridFit::prepare(&identity, &[1.0 / 3.0, 2.0 / 3.0, 2.0 / 3.0], 2, &[0, 1]).unwrap()
}

fn healthy(calls: usize) -> Receipt {
    Receipt {
        physical: f64::from(calls == 1),
        internal: 0.0,
        residual: vec![0.0; 3],
    }
}

#[test]
fn modal_joint_grid_faults_malformed_both_gates_and_cancellation() {
    for fail_at in 1..=6 {
        let mut calls = 0;
        let error = fresh_fit()
            .attempt_rounded_joint(4, 4, 1e-8, |_| {
                calls += 1;
                if calls == fail_at {
                    Err("joint actual operator fault".into())
                } else {
                    Ok(healthy(calls))
                }
            })
            .unwrap_err();
        assert_eq!(error, "joint actual operator fault");
        assert_eq!(calls, fail_at);
        for kind in 0..8 {
            let mut calls = 0;
            let error = fresh_fit()
                .attempt_rounded_joint(4, 4, 1e-8, |_| {
                    calls += 1;
                    let mut receipt = healthy(calls);
                    if calls == fail_at {
                        match kind {
                            0 => receipt.physical = f64::NAN,
                            1 => receipt.physical = -1.0,
                            2 => receipt.residual.pop().map(|_| ()).unwrap(),
                            3 => receipt.residual[0] = f64::INFINITY,
                            4 => receipt.residual[0] = 1e51,
                            5 => receipt.internal = f64::NAN,
                            6 => receipt.internal = -1.0,
                            _ => receipt.internal = f64::INFINITY,
                        }
                    }
                    Ok(receipt)
                })
                .unwrap_err();
            assert!(error.contains("finite matching"), "{error}");
            assert_eq!(calls, fail_at);
        }
    }
    for initially_passing in [false, true] {
        for fail_physical in [false, true] {
            let end = if initially_passing { 2 } else { 6 };
            let mut calls = 0;
            let outcome = fresh_fit()
                .attempt_rounded_joint(4, 4, 1e-8, |_| {
                    calls += 1;
                    let mut receipt = if initially_passing {
                        healthy(2)
                    } else {
                        healthy(calls)
                    };
                    if calls == end {
                        if fail_physical {
                            receipt.physical = 1.0;
                        } else {
                            receipt.internal = 1.0;
                        }
                    }
                    Ok(receipt)
                })
                .unwrap();
            assert!(matches!(outcome, Outcome::Rejected { .. }));
            assert_eq!(calls, end);
        }
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
        let fit = fresh_fit();
        let error = with_solver_observer(
            &control,
            move |p| {
                if p.stage == stage && p.completed_steps == steps {
                    cancel.request_cancel();
                }
            },
            || {
                fit.attempt_rounded_joint(4, 4, 1e-8, |_| {
                    calls += 1;
                    Ok(healthy(calls))
                })
            },
        )
        .unwrap_err();
        assert!(error.contains("cancel"), "{stage:?}: {error}");
        let mut calls = 0;
        let Outcome::Accepted(shape) = fresh_fit()
            .attempt_rounded_joint(4, 4, 1e-8, |_| {
                calls += 1;
                Ok(healthy(calls))
            })
            .unwrap()
        else {
            panic!("healthy replay rejected");
        };
        assert_eq!(calls, 6);
        assert_eq!(shape, fresh_fit().seed);
    }
    for width in [0, 65, usize::MAX] {
        assert!(
            fresh_fit()
                .attempt_rounded_joint(4, width, 1e-8, |_| unreachable!())
                .is_err()
        );
    }
    for radius in [0, MAX_GRID_RADIUS + 1, usize::MAX] {
        assert!(
            fresh_fit()
                .attempt_rounded_joint(radius, 4, 1e-8, |_| unreachable!())
                .is_err()
        );
    }
    for tolerance in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(
            fresh_fit()
                .attempt_rounded_joint(4, 4, tolerance, |_| unreachable!())
                .is_err()
        );
    }
    let mut fit = fresh_fit();
    fit.seed.fill(1.0);
    assert!(
        fit.attempt_rounded_joint(4, 4, 1e-8, |_| unreachable!())
            .is_err()
    );
    let matrix: Vec<_> = (0..3)
        .map(|i| (0..3).map(|j| Wide::from(f64::from(i == j))).collect())
        .collect();
    let fit = fresh_fit();
    let wide = GridFit::prepare_wide(&matrix, &fit.seed, 2, &[0, 1], false).unwrap();
    assert!(
        wide.attempt_rounded_joint(4, 4, 1e-8, |_| unreachable!())
            .is_err()
    );
    for size in [0, 1, 257, usize::MAX] {
        assert!(Plan::rounded_joint(size, 64).is_err());
    }
    let plan = Plan::rounded_joint(256, 64).unwrap();
    assert_eq!(plan.payload_bytes, 4_751_360);
    assert_eq!(plan.component_visits, 606_142_464);
    println!(
        "joint grid controls faults=6 malformed_receipts=48 lost_final_gates=4 cancellations=5 exact_replays=5 partial_candidates=0"
    );
}

#[test]
fn modal_joint_grid_known_null_shape_and_sign_binary_covariance() {
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
            let mut calls = 0;
            let result = fit
                .attempt_rounded_joint(4_194_304, 16, 1e-18, |v| {
                    calls += 1;
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
                    let relative =
                        super::super::super::vector_norm(residual.iter().copied())? / scale;
                    Ok(Receipt {
                        physical: relative,
                        internal: relative,
                        residual,
                    })
                })
                .unwrap();
            let Outcome::Accepted(shape) = result else {
                panic!("analytic joint rejected: {result:?}");
            };
            assert_eq!(shape, [sign * base; 3]);
            assert!(calls <= 18);
        }
    }
    println!(
        "joint grid analytic sign_binary_controls=6 known_null_shapes=6 unchanged_final_norm_gate=1"
    );
}
