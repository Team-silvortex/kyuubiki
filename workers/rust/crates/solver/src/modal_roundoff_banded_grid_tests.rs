use super::*;
use crate::solver_control::{SolverControl, with_solver_observer};

fn matrix(size: usize) -> Vec<Vec<Wide>> {
    (0..size)
        .map(|i| {
            (0..size)
                .map(|j| {
                    Wide::from(match i.abs_diff(j) {
                        0 => 2.0,
                        1 => -1.0,
                        2 => 0.125,
                        3 => -0.0625,
                        _ => 0.0,
                    })
                })
                .collect()
        })
        .collect()
}

fn direction(size: usize) -> Vec<Wide> {
    (0..size)
        .map(|i| Wide::from((i + 1) as f64 / (2 * size) as f64))
        .collect()
}

fn objective(rows: &[[Wide; 7]], shape: &[f64]) -> Wide {
    (0..shape.len()).fold(Wide::default(), |sum, i| {
        sum.add(row_cost(rows, i, |j| shape[j], &mut Usage::default()).unwrap())
    })
}

#[test]
fn modal_banded_grid_exhaustive_frontier_and_covariance() {
    let mut cases = 0;
    for size in 2..=8 {
        let matrix = matrix(size);
        let direction = direction(size);
        let anchor = size - 1;
        let baseline = propose(&matrix, &direction, anchor).unwrap();
        let rows = rows(&matrix, size).unwrap();
        let choices = domains(&direction, anchor).unwrap();
        let mut minimum = None;
        for mut key in 0..3_usize.pow((size - 1) as u32) {
            let mut shape = Vec::with_capacity(size);
            for options in &choices[..anchor] {
                shape.push(options[key % 3]);
                key /= 3;
            }
            shape.push(choices[anchor][0]);
            let score = objective(&rows, &shape);
            if minimum.is_none_or(|old| cmp(score, old).is_lt()) {
                minimum = Some(score);
            }
        }
        assert_eq!(cmp(baseline.score, minimum.unwrap()), Ordering::Equal);
        assert_eq!(
            cmp(baseline.score, objective(&rows, &baseline.shape)),
            Ordering::Equal
        );
        for scale in [-1.0, 0.5, 2.0] {
            let changed = direction
                .iter()
                .map(|v| v.mul(Wide::from(scale)))
                .collect::<Vec<_>>();
            let actual = propose(&matrix, &changed, anchor).unwrap();
            assert_eq!(
                actual.shape.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
                baseline
                    .shape
                    .iter()
                    .map(|v| (v * scale).to_bits())
                    .collect::<Vec<_>>()
            );
            cases += 1;
        }
        let plan = Plan::new(size).unwrap();
        assert!(baseline.usage.peak_states <= STATES);
        assert!(baseline.usage.nodes <= plan.nodes);
        assert!(baseline.usage.terms <= plan.terms);
        assert!(baseline.usage.trials <= plan.trials);
    }
    assert!(Plan::new(256).unwrap().payload_bytes <= 4 * 1024 * 1024);
    assert!(Plan::new(1).is_err() && Plan::new(257).is_err());
    let plan = Plan::for_passes(256, 4).unwrap();
    assert!(plan.payload_bytes <= 4 * 1024 * 1024 && plan.terms <= 16_000_000);
    assert!(Plan::for_passes(8, 0).is_err() && Plan::for_passes(8, 5).is_err());
    println!("banded grid exhaustive sizes=7 covariance_cases={cases} maximum_frontier=729");
}

#[test]
fn modal_banded_grid_four_pass_budget_and_second_pass_cancel() {
    let matrix = matrix(8);
    let direction = direction(8);
    let baseline = propose_passes(&matrix, &direction, 7, 4).unwrap();
    let plan = Plan::for_passes(8, 4).unwrap();
    assert!(baseline.usage.trials <= plan.trials);
    assert!(baseline.usage.terms <= plan.terms);
    assert!(baseline.usage.nodes <= plan.nodes);
    assert!(
        cmp(
            baseline.score,
            propose(&matrix, &direction, 7).unwrap().score
        )
        .is_le()
    );
    let control = SolverControl::default();
    let cancel = control.clone();
    let preparations = std::cell::Cell::new(0);
    let mut calls = 0;
    let outcome = with_solver_observer(
        &control,
        move |p| {
            if p.stage == SolverStage::ModalRoundoffPrepare && p.completed_steps == 0 {
                preparations.set(preparations.get() + 1);
                if preparations.get() == 2 {
                    cancel.request_cancel();
                }
            }
        },
        || {
            checked_passes(&matrix, &direction, 7, 4, |_| {
                calls += 1;
                Ok((0.0, vec![0.0; 8]))
            })
        },
    );
    assert!(outcome.unwrap_err().contains("cancel"));
    assert_eq!(calls, 0);
    assert_eq!(
        propose_passes(&matrix, &direction, 7, 4).unwrap().shape,
        baseline.shape
    );
    println!("banded grid four_pass_cumulative_budget=1 later_pass_cancel=1 healthy_replay=1");
}

fn healthy() -> Proposal {
    checked(&matrix(8), &direction(8), 7, |_| Ok((0.0, vec![0.0; 8]))).unwrap()
}

#[test]
fn modal_banded_grid_known_null_direction_and_binary_matrix_scaling() {
    let mut cases = 0;
    for size in 2_usize..=8 {
        for scale in [2.0_f64.powi(-80), 1.0, 2.0_f64.powi(80)] {
            let matrix: Vec<Vec<_>> = (0..size)
                .map(|i| {
                    (0..size)
                        .map(|j| {
                            Wide::from(
                                scale
                                    * if i == j {
                                        if i == 0 || i == size - 1 { 1.0 } else { 2.0 }
                                    } else if i.abs_diff(j) == 1 {
                                        -1.0
                                    } else {
                                        0.0
                                    },
                            )
                        })
                        .collect()
                })
                .collect();
            let mut direction = vec![Wide::from(0.375_f64.next_up()); size];
            direction[size - 1] = Wide::from(0.375);
            let actual = propose_passes(&matrix, &direction, size - 1, 4).unwrap();
            assert_eq!(actual.shape, vec![0.375; size]);
            assert_eq!(actual.score.rounded(), 0.0);
            cases += 1;
        }
    }
    println!("banded grid known_null_controls={cases} binary_matrix_scale_preserved=1");
}

#[test]
fn modal_banded_grid_fault_receipts_cancellation_and_replay() {
    let baseline = healthy();
    for fail_at in 1..=2 {
        for malformed in [
            (f64::NAN, vec![0.0; 8]),
            (-1.0, vec![0.0; 8]),
            (0.0, vec![0.0; 7]),
            (0.0, vec![f64::INFINITY; 8]),
            (0.0, vec![1e51; 8]),
        ] {
            let mut calls = 0;
            let error = checked(&matrix(8), &direction(8), 7, |_| {
                calls += 1;
                Ok(if calls == fail_at {
                    malformed.clone()
                } else {
                    (0.0, vec![0.0; 8])
                })
            })
            .unwrap_err();
            assert!(error.contains("finite matching certificate"), "{error}");
            assert_eq!(calls, fail_at);
        }
        let mut calls = 0;
        let error = checked(&matrix(8), &direction(8), 7, |_| {
            calls += 1;
            if calls == fail_at {
                Err("banded actual operator fault".into())
            } else {
                Ok((0.0, vec![0.0; 8]))
            }
        })
        .unwrap_err();
        assert_eq!(error, "banded actual operator fault");
        assert_eq!(calls, fail_at);
    }
    let mut calls = 0;
    assert!(
        checked(&matrix(8), &direction(8), 7, |_| {
            calls += 1;
            Ok((f64::from(calls == 2), vec![0.0; 8]))
        })
        .unwrap_err()
        .contains("unchanged residual gate")
    );
    assert_eq!(calls, 2);
    for (stage, steps) in [
        (SolverStage::ModalRoundoffPrepare, 0),
        (SolverStage::ModalRoundoffPrepare, 16),
        (SolverStage::ModalRoundoffSearch, 4),
        (SolverStage::ModalVectorScan, 128),
        (SolverStage::ModalVectorDot, 1),
        (SolverStage::ModalVectorUpdate, 8),
        (SolverStage::ModalRoundoffValidate, 1),
        (SolverStage::ModalRoundoffValidate, 2),
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
                checked(&matrix(8), &direction(8), 7, |_| {
                    calls += 1;
                    Ok((0.0, vec![0.0; 8]))
                })
            },
        )
        .unwrap_err();
        assert!(error.contains("cancel"), "{stage:?}: {error}");
        assert_eq!(
            calls,
            match stage {
                SolverStage::ModalRoundoffValidate => steps as usize,
                _ => 0,
            }
        );
        assert_eq!(healthy().shape, baseline.shape);
    }
    println!(
        "banded grid controls faults=2 malformed_receipts=10 lost_final=1 cancellations=8 healthy_replays=8 no_partial_candidate=1"
    );
}

#[test]
fn modal_banded_grid_malformed_inputs_and_retained_fill() {
    let valid = matrix(8);
    let direction = direction(8);
    let mut cases = 0;
    for changed in [
        vec![],
        vec![vec![Wide::from(1.0); 7]; 8],
        vec![vec![Wide::default(); 8]; 8],
        {
            let mut m = valid.clone();
            m[0][7] = Wide::from(1e-120);
            m
        },
        {
            let mut m = valid.clone();
            m[0][0].high = f64::INFINITY;
            m
        },
        {
            let mut m = valid.clone();
            m[0][0].low = 3.0;
            m
        },
        {
            let mut m = valid.clone();
            m[0][1] = Wide::from(1e-120);
            m
        },
    ] {
        let mut calls = 0;
        assert!(
            checked(&changed, &direction, 7, |_| {
                calls += 1;
                Ok((0.0, vec![0.0; 8]))
            })
            .is_err()
        );
        assert_eq!(calls, 0);
        cases += 1;
    }
    assert!(propose(&valid, &direction, 8).is_err());
    let mut bad = direction.clone();
    bad[0].high = f64::NAN;
    assert!(propose(&valid, &bad, 7).is_err());
    assert!(propose(&valid, &[Wide::default(); 8], 7).is_err());
    let mut bad = direction.clone();
    bad[0] = Wide::from(1e-110);
    assert!(propose(&valid, &bad, 7).is_err());
    assert_eq!(
        healthy().shape,
        propose(&valid, &direction, 7).unwrap().shape
    );
    println!(
        "banded grid malformed_input_cases={} retained_fill_not_dropped=1",
        cases + 4
    );
}
