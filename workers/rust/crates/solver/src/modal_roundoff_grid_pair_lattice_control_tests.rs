use super::*;
use crate::solver_control::{SolverControl, with_solver_observer};
use std::{cell::Cell, rc::Rc};

#[test]
fn modal_pair_lattice_unimodular_grid_and_budget_controls() {
    let matrix = vec![
        vec![Wide::from(1.0), Wide::from(3.0), Wide::default()],
        vec![Wide::default(), Wide::from(1.0), Wide::default()],
        vec![Wide::default(), Wide::default(), Wide::from(1.0)],
    ];
    let fit = PairFit::prepare(&matrix, &[0.3; 3], 2, &[0, 1], 64, 4).unwrap();
    assert!(fit.shears > 0);
    assert!(fit.pair_steps <= 4 * PAIR_STEPS);
    let determinant =
        fit.transform[0][0] * fit.transform[1][1] - fit.transform[1][0] * fit.transform[0][1];
    assert_eq!(determinant.abs(), 1);
    assert!(fit.original_offsets(&[4.0, 4.0], 4).unwrap().is_none());
    for decisions in [
        vec![0.0],
        vec![0.5, 0.0],
        vec![f64::NAN, 0.0],
        vec![5.0, 0.0],
    ] {
        assert!(fit.original_offsets(&decisions, 4).is_err());
    }
    for size in [0, 1, 257, usize::MAX] {
        assert!(Plan::pair_lattice(size, 64, 4).is_err());
    }
    for sweeps in [0, 5, usize::MAX] {
        assert!(Plan::pair_lattice(256, 64, sweeps).is_err());
    }
    let plan = Plan::pair_lattice(256, 64, 4).unwrap();
    assert!(plan.payload_bytes <= 8 * 1024 * 1024);
    assert!(plan.component_visits <= 650_000_000);
    println!(
        "pair lattice budget payload_bytes={} component_visits={} determinant={determinant}",
        plan.payload_bytes, plan.component_visits
    );
}

fn fresh(reduced: bool, width: usize) -> PairFit {
    let matrix: Vec<_> = (0..6)
        .map(|i| (0..6).map(|j| Wide::from(f64::from(i == j))).collect())
        .collect();
    if reduced {
        PairFit::prepare_reduced(&matrix, &[0.3; 6], 5, &[0, 1, 2, 3, 4], width).unwrap()
    } else {
        PairFit::prepare(&matrix, &[0.3; 6], 5, &[0, 1, 2, 3, 4], width, 4).unwrap()
    }
}

#[test]
fn modal_pair_lattice_faults_final_gate_cancel_and_frozen_grid() {
    for reduced in [false, true] {
        let fit = fresh(reduced, 4);
        for radius in [0, MAX_GRID_RADIUS + 1, usize::MAX] {
            assert!(fit.attempt(radius, 1e-8, |_| unreachable!()).is_err());
        }
        for tolerance in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert!(fit.attempt(4, tolerance, |_| unreachable!()).is_err());
        }
        let healthy = || {
            let fit = fresh(reduced, 4);
            let mut calls = 0;
            let result = fit
                .attempt(4, 1e-8, |v| {
                    calls += 1;
                    assert_eq!(v[5].to_bits(), fit.seed[5].to_bits());
                    for &i in &fit.order {
                        let step = Wide::from(v[i])
                            .sub(Wide::from(fit.seed[i]))
                            .div(Wide::from(fit.grids[i]))
                            .rounded();
                        assert!(step.fract() == 0.0 && step.abs() <= 4.0);
                    }
                    Ok((
                        if calls == 6 { 0.0 } else { 1.0 / calls as f64 },
                        vec![0.0; 6],
                    ))
                })
                .unwrap();
            assert_eq!(calls, 6);
            let Attempt::Accepted(shape) = result else {
                panic!("controlled final receipt rejected")
            };
            shape.iter().map(|v| v.to_bits()).collect::<Vec<_>>()
        };
        let baseline = healthy();
        for fail_at in 1..=6 {
            let mut calls = 0;
            let error = fit
                .attempt(4, 1e-8, |_| {
                    calls += 1;
                    if calls == fail_at {
                        Err("lattice actual product fault".into())
                    } else {
                        Ok((1.0 / calls as f64, vec![0.0; 6]))
                    }
                })
                .unwrap_err();
            assert_eq!(error, "lattice actual product fault");
            assert_eq!(calls, fail_at);
            for kind in 0..5 {
                let mut calls = 0;
                let error = fit
                    .attempt(4, 1e-8, |_| {
                        calls += 1;
                        let mut receipt = (1.0 / calls as f64, vec![0.0; 6]);
                        if calls == fail_at {
                            match kind {
                                0 => receipt.0 = f64::NAN,
                                1 => receipt.0 = -1.0,
                                2 => {
                                    receipt.1.pop();
                                }
                                3 => receipt.1[0] = f64::INFINITY,
                                _ => receipt.1[0] = 1e51,
                            }
                        }
                        Ok(receipt)
                    })
                    .unwrap_err();
                assert!(error.contains("finite matching certificate"));
                assert_eq!(calls, fail_at);
            }
            assert_eq!(healthy(), baseline);
        }
        let mut calls = 0;
        let result = fit
            .attempt(4, 1e-8, |_| {
                calls += 1;
                Ok((f64::from(calls != 1), vec![0.0; 6]))
            })
            .unwrap();
        assert!(matches!(
            result,
            Attempt::Rejected(Rejection::Residual(1.0))
        ));
        assert_eq!(calls, 2);
        for (stage, step) in [
            (SolverStage::DenseSubstitution, 0),
            (SolverStage::ModalRoundoffSearch, 5),
            (SolverStage::ModalRoundoffValidate, 9),
        ] {
            let control = SolverControl::default();
            let cancel = control.clone();
            let seen = Rc::new(Cell::new(false));
            let observed = seen.clone();
            let mut calls = 0;
            let result = with_solver_observer(
                &control,
                move |p| {
                    if p.stage == stage && p.completed_steps == step {
                        observed.set(true);
                        cancel.request_cancel();
                    }
                },
                || {
                    fit.attempt(4, 1e-8, |_| {
                        calls += 1;
                        Ok((
                            if calls == 6 { 0.0 } else { 1.0 / calls as f64 },
                            vec![0.0; 6],
                        ))
                    })
                },
            );
            assert!(seen.get() && result.unwrap_err().contains("cancel"));
            assert_eq!(healthy(), baseline);
        }
        let fit = fresh(reduced, 64);
        let mut calls = 0;
        assert!(matches!(
            fit.attempt(MAX_GRID_RADIUS, 1e-8, |_| {
                calls += 1;
                Ok((
                    if calls == 66 { 0.0 } else { 1.0 / calls as f64 },
                    vec![0.0; 6],
                ))
            })
            .unwrap(),
            Attempt::Accepted(_)
        ));
        assert_eq!(calls, 66);
    }
    println!(
        "pair lattice controls callback_faults=12 malformed_receipts=60 stale_final_gates=2 cancellations=6 fresh_exact_replays=18 max_actual_receipts=66 partial_candidates=0"
    );
}

#[test]
fn modal_pair_lattice_known_null_sign_scale_and_preflight() {
    let base = 1.0 / 3.0_f64.sqrt();
    for reduced in [false, true] {
        for sign in [-1.0, 1.0] {
            for scale in [0.0625, 1.0, 16.0] {
                let matrix: Vec<Vec<_>> = [[1.0, -1.0, 0.0], [-1.0, 2.0, -1.0], [0.0, -1.0, 1.0]]
                    .iter()
                    .map(|r| r.iter().map(|a| Wide::from(scale * a)).collect())
                    .collect();
                let seed = [sign * base.next_up(), sign * base, sign * base.next_down()];
                let fit = if reduced {
                    PairFit::prepare_reduced(&matrix, &seed, 1, &[0, 2], 64)
                } else {
                    PairFit::prepare(&matrix, &seed, 1, &[0, 2], 64, 4)
                }
                .unwrap();
                let actual = fit
                    .attempt(4, 1e-20, |v| {
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
                    panic!("analytic null rejected")
                };
                assert_eq!(shape, [sign * base; 3]);
            }
        }
    }
    let matrix = vec![vec![Wide::from(1.0); 6]; 6];
    for width in [0, 65, usize::MAX] {
        assert!(PairFit::prepare_reduced(&matrix, &[0.3; 6], 5, &[0, 1, 2, 3, 4], width).is_err());
    }
    assert!(PairFit::prepare_reduced(&matrix, &[0.0; 6], 5, &[0, 1, 2, 3, 4], 64).is_err());
    let plan = Plan::triangular_lattice(256, 64).unwrap();
    assert_eq!(plan.payload_bytes, 8_404_992);
    assert_eq!(plan.component_visits, 3_061_841_920);
    println!(
        "pair lattice analytic signed_binary_nulls=12 invalid_widths=3 unresolved_seed=1 triangular_payload_bytes={} triangular_component_visits={}",
        plan.payload_bytes, plan.component_visits
    );
}
