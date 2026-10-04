use super::*;
use crate::solver_control::{SolverControl, with_solver_observer};
use std::{cell::RefCell, rc::Rc};

fn identity(size: usize) -> Vec<Vec<Wide>> {
    (0..size)
        .map(|i| (0..size).map(|j| Wide::from(f64::from(i == j))).collect())
        .collect()
}

#[test]
fn triangular_grid_quantized_backsolve_matches_an_independent_integer_minimum() {
    for order in [[0, 1, 2], [2, 0, 1], [1, 2, 0]] {
        for power in [-200, 0, 200] {
            let scale = 2.0_f64.powi(power);
            let original = [[1.0, 0.0, 0.0], [4.0, 1.0, 0.0]];
            let columns: Vec<Vec<_>> = original
                .iter()
                .map(|r| order.iter().map(|&i| r[i] * scale).collect())
                .collect();
            let rhs = order.map(|i| [2.21 * scale, 0.49 * scale, 0.0][i]);
            let factor = QrFit::factor(&columns).unwrap();
            let continuous = factor.solve(&rhs).unwrap();
            assert!((continuous[0] - 0.25).abs() < 1e-14);
            assert!((continuous[1] - 0.49).abs() < 1e-14);
            assert_eq!(
                continuous.iter().map(|v| v.round()).collect::<Vec<_>>(),
                [0.0, 0.0]
            );
            let quantized = factor.quantized_solve(&rhs, 4).unwrap();
            assert_eq!(quantized, [2.0, 0.0]);
            let square = |a: f64, b: f64| (a + 4.0 * b - 2.21).powi(2) + (b - 0.49).powi(2);
            let selected = square(quantized[0], quantized[1]);
            assert!(selected < square(0.0, 0.0));
            for a in -4..=4 {
                for b in -4..=4 {
                    assert!(selected <= square(a as f64, b as f64));
                }
            }
        }
    }
    for powers in [[0, 0], [-400, 400], [400, -400]] {
        let scales = powers.map(|p| 2.0_f64.powi(p));
        let factor = QrFit::factor(&[
            vec![3.0 * scales[0], 0.0, 0.0],
            vec![0.0, 5.0 * scales[1], 0.0],
        ])
        .unwrap();
        let rhs = [6.75 * scales[0], -8.75 * scales[1], 0.0];
        assert_eq!(factor.quantized_solve(&rhs, 4).unwrap(), [2.0, -2.0]);
        assert_eq!(factor.quantized_solve(&rhs, 1).unwrap(), [1.0, -1.0]);
        for radius in [0, MAX_GRID_RADIUS + 1, usize::MAX] {
            assert!(factor.quantized_solve(&rhs, radius).is_err());
        }
        assert!(factor.quantized_solve(&rhs[..2], 4).is_err());
        assert!(factor.quantized_solve(&[f64::NAN; 3], 4).is_err());
    }
}

#[test]
fn triangular_grid_coupled_rounding_has_a_closed_physical_coordinate_check() {
    let matrix: Vec<Vec<_>> = [[1.0, 4.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
        .iter()
        .map(|r| r.iter().map(|&v| Wide::from(v)).collect())
        .collect();
    for sign in [1.0, -1.0] {
        let seed = [sign * 1.5_f64; 3];
        let unit = seed[0].next_up() - seed[0];
        let initial_norm = 2.21_f64.hypot(0.49);
        let checked = |v: &[f64]| {
            let d: Vec<_> = v.iter().zip(seed).map(|(&v, s)| (v - s) / unit).collect();
            let closed = vec![
                (d[0] + 4.0 * d[1] - 2.21) * unit,
                (d[1] - 0.49) * unit,
                d[2] * unit,
            ];
            Ok((
                super::super::vector_norm(closed.iter().copied())? / (initial_norm * unit),
                closed,
            ))
        };
        let fit = GridFit::prepare(&matrix, &seed, 2, &[0, 1]).unwrap();
        let error = fit
            .correct(4, Policy::Independent, 0.25, checked)
            .unwrap_err();
        assert!(error.contains("relative=1e0"), "{error}");
        let accepted = fit.correct(4, Policy::InBacksolve, 0.25, checked).unwrap();
        assert_eq!(accepted, [seed[0].next_up().next_up(), seed[1], seed[2]]);
        assert!(checked(&accepted).unwrap().0 <= 0.25);
        assert_eq!(seed, [sign * 1.5; 3]);
    }
}

#[test]
fn triangular_grid_input_order_range_and_resource_limits_fail_before_certification() {
    for size in [2, 3, 128, 256] {
        let plan = Plan::new(size).unwrap();
        assert!(plan.payload_bytes <= 8 * 1024 * 1024);
        assert!(plan.component_visits <= 350_000_000);
    }
    let maximum = Plan::new(256).unwrap();
    assert_eq!(maximum.payload_bytes, 3_670_016);
    assert_eq!(maximum.component_visits, 335_544_320);
    for size in [0, 1, 257, usize::MAX] {
        assert!(Plan::new(size).is_err());
    }
    let matrix = identity(3);
    let seed = [1.5; 3];
    for order in [
        vec![],
        vec![0],
        vec![0, 0],
        vec![1, 2],
        vec![0, 3],
        vec![0, 1, 2],
    ] {
        assert!(GridFit::prepare(&matrix, &seed, 2, &order).is_err());
    }
    assert!(GridFit::prepare(&matrix, &seed, 3, &[0, 1]).is_err());
    assert!(GridFit::prepare(&matrix, &seed[..2], 2, &[0, 1]).is_err());
    for bad in [f64::NAN, f64::INFINITY, 1e51, 1e-51, 0.0] {
        assert!(GridFit::prepare(&matrix, &[bad; 3], 2, &[0, 1]).is_err());
    }
    for bad in [
        Wide {
            high: f64::NAN,
            low: 0.0,
        },
        Wide {
            high: 1.0,
            low: f64::INFINITY,
        },
        Wide {
            high: 0.0,
            low: 1.0,
        },
        Wide::from(1e51),
        Wide::from(1e-51),
    ] {
        let mut matrix = matrix.clone();
        matrix[0][0] = bad;
        assert!(GridFit::prepare(&matrix, &seed, 2, &[0, 1]).is_err());
    }
    let ragged = vec![vec![Wide::from(1.0); 2]; 3];
    assert!(GridFit::prepare(&ragged, &seed, 2, &[0, 1]).is_err());
    let duplicate = vec![vec![Wide::from(1.0); 3]; 3];
    assert!(GridFit::prepare(&duplicate, &seed, 2, &[0, 1]).is_err());
    let fit = GridFit::prepare(&matrix, &seed, 2, &[0, 1]).unwrap();
    for policy in [Policy::Independent, Policy::InBacksolve] {
        for radius in [0, MAX_GRID_RADIUS + 1, usize::MAX] {
            assert!(
                fit.correct(radius, policy, 1e-8, |_| unreachable!())
                    .is_err()
            );
        }
        for tolerance in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert!(
                fit.correct(4, policy, tolerance, |_| unreachable!())
                    .is_err()
            );
        }
    }
}

#[test]
fn triangular_grid_cached_factors_share_one_check_counter_and_refresh_the_residual() {
    let fit = GridFit::prepare(&identity(3), &[1.5; 3], 2, &[0, 1]).unwrap();
    for policy in [Policy::Independent, Policy::InBacksolve] {
        let visits = Rc::new(RefCell::new(Vec::new()));
        let observed = visits.clone();
        let factors = Rc::new(RefCell::new(0));
        let observed_factors = factors.clone();
        let mut calls = 0;
        let error = with_solver_observer(
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
                fit.correct(4, policy, 1e-8, |_| {
                    calls += 1;
                    Ok((1.0 / (calls + 1) as f64, vec![0.0; 3]))
                })
            },
        )
        .unwrap_err();
        assert!(error.contains("unchanged residual gate"));
        assert_eq!(calls, MAX_CERTIFICATES);
        assert_eq!(
            visits.borrow().as_slice(),
            (0..MAX_CERTIFICATES as u64).collect::<Vec<_>>()
        );
        assert_eq!(*factors.borrow(), 0);
        let unit = 1.5_f64.next_up() - 1.5;
        let mut seen = Vec::new();
        let error = fit
            .correct(4, policy, 1e-8, |v| {
                seen.push(v.to_vec());
                if seen.len() == 1 {
                    Ok((1.0, vec![-unit, 0.0, 0.0]))
                } else {
                    Ok((0.5, vec![0.0; 3]))
                }
            })
            .unwrap_err();
        assert!(error.contains("relative=5e-1"), "{error}");
        assert_eq!(seen.len(), 4);
        assert_eq!(seen[0], [1.5; 3]);
        assert!(
            seen[1..]
                .iter()
                .all(|v| v == &[1.5_f64.next_up(), 1.5, 1.5])
        );
    }
}

#[test]
fn triangular_grid_final_recheck_errors_and_worse_proposals_cannot_publish() {
    let seed = [1.5; 3];
    let fit = GridFit::prepare(&identity(3), &seed, 2, &[0, 1]).unwrap();
    for policy in [Policy::Independent, Policy::InBacksolve] {
        for (relative, residual) in [
            (1.0, vec![0.0; 3]),
            (f64::NAN, vec![0.0; 3]),
            (-1.0, vec![0.0; 3]),
            (0.0, vec![0.0; 2]),
            (0.0, vec![f64::INFINITY; 3]),
            (0.0, vec![1e51; 3]),
        ] {
            let mut calls = 0;
            let error = fit
                .correct(4, policy, 1e-8, |_| {
                    calls += 1;
                    if calls == 1 {
                        Ok((0.0, vec![0.0; 3]))
                    } else {
                        Ok((relative, residual.clone()))
                    }
                })
                .unwrap_err();
            assert!(
                error.contains("unchanged residual gate")
                    || error.contains("finite matching certificate"),
                "{error}"
            );
            assert_eq!(calls, 2);
        }
        let mut calls = 0;
        let error = fit
            .correct(4, policy, 1e-8, |_| {
                calls += 1;
                Ok((if calls == 2 { 0.0 } else { 1.0 }, vec![0.0; 3]))
            })
            .unwrap_err();
        assert!(error.contains("unchanged residual gate"));
        assert_eq!(calls, 3);
        let unit = 1.5_f64.next_up() - 1.5;
        let mut seen = Vec::new();
        let error = fit
            .correct(4, policy, 1e-8, |v| {
                seen.push(v.to_vec());
                Ok((if v == seed { 1.0 } else { 2.0 }, vec![-unit, 0.0, 0.0]))
            })
            .unwrap_err();
        assert!(error.contains("relative=1e0"), "{error}");
        assert_eq!(
            seen,
            [
                seed.to_vec(),
                vec![1.5_f64.next_up(), 1.5, 1.5],
                seed.to_vec()
            ]
        );
        for fail_at in [1, 2] {
            let mut calls = 0;
            assert_eq!(
                fit.correct(4, policy, 1e-8, |_| {
                    calls += 1;
                    if calls == fail_at {
                        Err("actual operator failure".into())
                    } else {
                        Ok((1.0, vec![0.0; 3]))
                    }
                })
                .unwrap_err(),
                "actual operator failure"
            );
            assert_eq!(calls, fail_at);
        }
        let mut calls = 0;
        assert!(
            fit.correct(4, policy, 1e-8, |_| {
                calls += 1;
                Ok((f64::NAN, vec![0.0; 3]))
            })
            .unwrap_err()
            .contains("finite matching certificate")
        );
        assert_eq!(calls, 1);
        assert_eq!(
            fit.correct(4, policy, 1e-8, |_| Ok((0.0, vec![0.0; 3])))
                .unwrap(),
            seed
        );
        assert_eq!(fit.seed, seed);
    }
}

#[test]
fn triangular_grid_cancelled_preparation_and_backsolve_replay_without_partial_shapes() {
    let matrix = identity(3);
    let seed = [1.5; 3];
    for stage in [
        SolverStage::ModalRoundoffPrepare,
        SolverStage::ModalVectorUpdate,
        SolverStage::ModalVectorScan,
        SolverStage::DenseFactor,
        SolverStage::ModalVectorDot,
    ] {
        let control = SolverControl::default();
        let cancel = control.clone();
        let error = with_solver_observer(
            &control,
            move |p| {
                if p.stage == stage {
                    cancel.request_cancel();
                }
            },
            || GridFit::prepare(&matrix, &seed, 2, &[0, 1]).map(|_| ()),
        )
        .unwrap_err();
        assert!(error.contains("cancel"), "{stage:?}: {error}");
        assert_eq!(control.last_checkpoint().unwrap().stage, stage);
        assert!(GridFit::prepare(&matrix, &seed, 2, &[0, 1]).is_ok());
        assert_eq!(seed, [1.5; 3]);
    }
    let fit = GridFit::prepare(&matrix, &seed, 2, &[0, 1]).unwrap();
    let unit = seed[0].next_up() - seed[0];
    let checked = |v: &[f64]| {
        let r = vec![v[0] - seed[0].next_up(), v[1] - seed[1], 0.0];
        Ok((super::super::vector_norm(r.iter().copied())? / unit, r))
    };
    for policy in [Policy::Independent, Policy::InBacksolve] {
        for (stage, step) in [
            (SolverStage::ModalIteration, 0),
            (SolverStage::DenseSubstitution, 0),
            (SolverStage::DenseSubstitution, 3),
            (SolverStage::ModalVectorDot, 0),
            (SolverStage::ModalVectorUpdate, 0),
            (SolverStage::ModalRoundoffSearch, 0),
            (SolverStage::ModalRoundoffValidate, 0),
        ] {
            let control = SolverControl::default();
            let cancel = control.clone();
            let error = with_solver_observer(
                &control,
                move |p| {
                    if p.stage == stage && (step == 0 || p.completed_steps == step) {
                        cancel.request_cancel();
                    }
                },
                || fit.correct(4, policy, 1e-8, checked),
            )
            .unwrap_err();
            assert!(error.contains("cancel"), "{stage:?}/{step}: {error}");
            assert_eq!(control.last_checkpoint().unwrap().stage, stage);
            assert_eq!(
                fit.correct(4, policy, 1e-8, checked).unwrap(),
                [seed[0].next_up(), seed[1], seed[2]]
            );
            assert_eq!(seed, [1.5; 3]);
        }
    }
}
