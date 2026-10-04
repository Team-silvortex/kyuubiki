use super::*;
use crate::solver_control::{SolverControl, with_solver_observer};
use std::{cell::Cell, rc::Rc};

pub(super) fn shuffled(size: usize, seed: u64) -> Vec<usize> {
    let mut permutation: Vec<_> = (0..size).collect();
    let mut state = seed;
    for i in (1..size).rev() {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        permutation.swap(i, ((state >> 32) as usize) % (i + 1));
    }
    permutation
}

fn identity() -> Vec<Vec<Wide>> {
    (0..3)
        .map(|i| (0..3).map(|j| Wide::from(f64::from(i == j))).collect())
        .collect()
}

fn seed() -> [f64; 3] {
    [3.0_f64.sqrt().recip(); 3]
}

#[test]
fn triangular_grid_portfolio_enforces_aggregate_not_reset_budgets() {
    let plan = Plan::new(256).unwrap();
    assert_eq!(plan.payload_bytes, 5_832_704);
    assert_eq!(plan.component_visits, 1_008_762_880);
    assert!(plan.payload_bytes < 8 * 1024 * 1024);
    assert!(plan.component_visits < MAX_TOTAL_VISITS);
    for bad in [0, 1, 257, usize::MAX] {
        assert!(Plan::new(bad).is_err());
    }
    let mut budget = Budget::new(Plan::new(3).unwrap());
    for count in 1..=MAX_ATTEMPTS {
        budget.start_attempt().unwrap();
        assert_eq!(budget.usage.attempts, count);
    }
    let visits = budget.usage.component_visits;
    assert_eq!(visits, budget.plan.component_visits);
    for _ in 0..2 {
        assert!(
            budget
                .start_attempt()
                .unwrap_err()
                .contains("attempt budget")
        );
        assert_eq!(budget.usage.component_visits, visits);
    }
    for count in 1..=MAX_TOTAL_CERTIFICATES {
        budget.certify().unwrap();
        assert_eq!(budget.usage.certificates, count);
    }
    for _ in 0..2 {
        assert!(budget.certify().unwrap_err().contains("certificate budget"));
        assert_eq!(budget.usage.certificates, MAX_TOTAL_CERTIFICATES);
    }
    let factors = Rc::new(Cell::new(0));
    let observed = factors.clone();
    let mut calls = 0;
    let original = seed();
    let error = with_solver_observer(
        &SolverControl::default(),
        move |p| {
            if p.stage == SolverStage::DenseFactor && p.completed_steps == 0 {
                observed.set(observed.get() + 1);
            }
        },
        || {
            search_unit_shape(&identity(), &original, &[1.0; 3], 1e-8, |_| {
                calls += 1;
                // Inject declining receipts to exercise all four passes of every fit.
                Ok((1.0 / (calls + 1) as f64, vec![0.0; 3]))
            })
        },
    )
    .unwrap_err();
    assert!(error.contains("attempts=3, certificates=18"), "{error}");
    assert_eq!(calls, MAX_TOTAL_CERTIFICATES - 1);
    assert_eq!(factors.get(), MAX_ATTEMPTS);
    assert_eq!(original, seed());
    let mut calls = 0;
    let accepted = search_unit_shape(&identity(), &original, &[1.0; 3], 1e-8, |_| {
        calls += 1;
        let relative = if calls < 17 {
            1.0 / (calls + 1) as f64
        } else {
            0.0
        };
        Ok((relative, vec![0.0; 3]))
    })
    .unwrap();
    assert_eq!(accepted.order, Order::GridNorm);
    assert_eq!(accepted.shape, original);
    assert_eq!(accepted.usage.attempts, MAX_ATTEMPTS);
    assert_eq!(accepted.usage.certificates, MAX_TOTAL_CERTIFICATES);
    assert_eq!(calls, MAX_TOTAL_CERTIFICATES);
}

#[test]
fn triangular_grid_portfolio_faults_do_not_become_numerical_retries() {
    for error in [
        "physical operator fault",
        "cancel requested",
        "unchanged residual gate: injected operator error",
        "unchanged norm gate: injected operator error",
    ] {
        let mut calls = 0;
        let result = search_unit_shape(&identity(), &seed(), &[1.0; 3], 1e-8, |_| {
            calls += 1;
            Err(error.to_string())
        });
        assert_eq!(result.unwrap_err(), error);
        assert_eq!(calls, 1);
    }
    for certificate in [
        (f64::NAN, vec![0.0; 3]),
        (-1.0, vec![0.0; 3]),
        (0.0, vec![0.0; 2]),
        (0.0, vec![f64::INFINITY; 3]),
    ] {
        let mut calls = 0;
        let error = search_unit_shape(&identity(), &seed(), &[1.0; 3], 1e-8, |_| {
            calls += 1;
            Ok(certificate.clone())
        })
        .unwrap_err();
        assert!(error.contains("finite matching certificate"), "{error}");
        assert_eq!(calls, 1);
    }
    let replay = search_unit_shape(&identity(), &seed(), &[1.0; 3], 1e-8, |_| {
        Ok((0.0, vec![0.0; 3]))
    })
    .unwrap();
    assert_eq!(replay.shape, seed());
    assert_eq!(replay.usage.attempts, 1);
    assert_eq!(replay.usage.certificates, 3);
}

#[test]
fn triangular_grid_portfolio_fresh_final_certificate_cannot_retry_or_publish() {
    for final_certificate in [
        Ok((1.0, vec![0.0; 3])),
        Ok((f64::NAN, vec![0.0; 3])),
        Ok((0.0, vec![0.0; 2])),
        Err("final operator fault".to_string()),
    ] {
        let mut calls = 0;
        let result = search_unit_shape(&identity(), &seed(), &[1.0; 3], 1e-8, |_| {
            calls += 1;
            if calls < 3 {
                Ok((0.0, vec![0.0; 3]))
            } else {
                final_certificate.clone()
            }
        });
        assert!(result.is_err());
        assert_eq!(calls, 3);
    }
}

#[test]
fn triangular_grid_portfolio_keeps_the_norm_gate_and_frozen_anchor() {
    let original = seed();
    let unit = original[0].next_up() - original[0];
    for steps in [1, 1 << 21] {
        let mut target = original;
        target[0] += unit * steps as f64;
        let mut calls = 0;
        let result = search_unit_shape(&identity(), &original, &[1.0, 1.0, 2.0], 1e-8, |v| {
            calls += 1;
            let residual: Vec<_> = v.iter().zip(target).map(|(&v, t)| v - t).collect();
            Ok((vector_norm(residual.iter().copied())? / unit, residual))
        });
        if steps == 1 {
            let accepted = result.unwrap();
            assert_eq!(accepted.shape, target);
            assert_eq!(accepted.anchor, 2);
            assert_eq!(accepted.shape[2].to_bits(), original[2].to_bits());
            assert_eq!(accepted.usage.certificates, 4);
        } else {
            let error = result.unwrap_err();
            assert!(error.contains("norm_rejections=3"), "{error}");
            assert_eq!(calls, 9);
        }
    }
    assert_eq!(original, seed());
}

#[test]
fn triangular_grid_portfolio_preflight_rejects_invalid_inputs_without_operator_calls() {
    for bad_mass in [vec![1.0; 2], vec![0.0; 3], vec![f64::NAN; 3], vec![-1.0; 3]] {
        assert!(
            search_unit_shape(&identity(), &seed(), &bad_mass, 1e-8, |_| unreachable!()).is_err()
        );
    }
    for bad_seed in [
        vec![1.5; 3],
        vec![0.0; 3],
        vec![f64::INFINITY; 3],
        vec![0.5; 4],
    ] {
        assert!(
            search_unit_shape(&identity(), &bad_seed, &[1.0; 3], 1e-8, |_| unreachable!()).is_err()
        );
    }
    let ragged = vec![vec![Wide::from(1.0); 2]; 3];
    assert!(search_unit_shape(&ragged, &seed(), &[1.0; 3], 1e-8, |_| unreachable!()).is_err());
    let mut invalid = identity();
    invalid[0][0].low = f64::NAN;
    assert!(search_unit_shape(&invalid, &seed(), &[1.0; 3], 1e-8, |_| unreachable!()).is_err());
    let singular = vec![vec![Wide::from(1.0); 3]; 3];
    assert!(search_unit_shape(&singular, &seed(), &[1.0; 3], 1e-8, |_| unreachable!()).is_err());
    for tolerance in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(
            search_unit_shape(
                &identity(),
                &seed(),
                &[1.0; 3],
                tolerance,
                |_| unreachable!()
            )
            .is_err()
        );
    }
}

#[test]
fn triangular_grid_portfolio_cancellation_stops_preparation_retry_and_publication() {
    for (stage, steps) in [
        (SolverStage::ModalVectorUpdate, 3),
        (SolverStage::DenseFactor, 0),
        (SolverStage::DenseSubstitution, 0),
        (SolverStage::ModalRoundoffPrepare, 2),
        (SolverStage::ModalRoundoffValidate, 3),
        (SolverStage::ModalRoundoffValidate, 4),
    ] {
        let control = SolverControl::default();
        let cancel = control.clone();
        let mut calls = 0;
        let result = with_solver_observer(
            &control,
            move |p| {
                if p.stage == stage && p.completed_steps == steps {
                    cancel.request_cancel();
                }
            },
            || {
                search_unit_shape(&identity(), &seed(), &[1.0; 3], 1e-8, |_| {
                    calls += 1;
                    let relative = if matches!(stage, SolverStage::ModalRoundoffValidate) {
                        0.0
                    } else {
                        1.0
                    };
                    Ok((relative, vec![0.0; 3]))
                })
            },
        );
        let error = result.unwrap_err();
        assert!(error.contains("cancel"), "{stage:?}/{steps}: {error}");
        assert!(control.was_interrupted());
        if stage == SolverStage::ModalRoundoffPrepare {
            assert_eq!(calls, 3);
        }
        let replay = search_unit_shape(&identity(), &seed(), &[1.0; 3], 1e-8, |_| {
            Ok((0.0, vec![0.0; 3]))
        })
        .unwrap();
        assert_eq!(replay.shape, seed());
    }
}

#[test]
fn triangular_grid_portfolio_beam_renumbering_and_signs_keep_independent_certificates() {
    use super::super::{hybrid_tests::check_published, triangular_grid_robustness_tests::Fixture};
    for step in [1.0, 1e14, 1e-10] {
        let fixture = Fixture::new(step);
        let size = fixture.shape.len();
        for (layout, permutation) in [
            ("identity", (0..size).collect::<Vec<_>>()),
            ("reversed", (0..size).rev().collect()),
            (
                "family",
                (0..size).step_by(2).chain((1..size).step_by(2)).collect(),
            ),
        ] {
            let matrix: Vec<Vec<_>> = permutation
                .iter()
                .map(|&i| {
                    permutation
                        .iter()
                        .map(|&j| fixture.directions[i][j])
                        .collect()
                })
                .collect();
            let mass: Vec<_> = permutation
                .iter()
                .map(|&i| fixture.system.mass[i])
                .collect();
            for sign in [1.0, -1.0] {
                let seed: Vec<_> = permutation
                    .iter()
                    .map(|&i| sign * fixture.shape[i])
                    .collect();
                let restore = |v: &[f64]| {
                    let mut original = vec![0.0; size];
                    for (&i, &a) in permutation.iter().zip(v) {
                        original[i] = a;
                    }
                    original
                };
                let mut calls = 0;
                let result = search_unit_shape(&matrix, &seed, &mass, 1e-8, |v| {
                    calls += 1;
                    let (relative, residual) = fixture.checked(&restore(v))?;
                    Ok((relative, permutation.iter().map(|&i| residual[i]).collect()))
                })
                .unwrap();
                let expected = if layout == "reversed" {
                    Order::Natural
                } else {
                    Order::Reverse
                };
                assert_eq!(result.order, expected, "{step:e}/{layout}/{sign}");
                assert_eq!(
                    result.usage.attempts,
                    if layout == "reversed" { 2 } else { 1 }
                );
                assert_eq!(calls, if layout == "reversed" { 7 } else { 4 });
                assert_eq!(calls, result.usage.certificates);
                assert!(result.usage.component_visits <= Plan::new(size).unwrap().component_visits);
                assert_eq!(
                    result.shape[result.anchor].to_bits(),
                    seed[result.anchor].to_bits()
                );
                assert_eq!(permutation[result.anchor], 252);
                let original = restore(&result.shape);
                let relative = fixture.checked(&original).unwrap().0;
                assert!(relative <= 1e-8);
                check_published(128, step, fixture.value, &original);
                println!(
                    "grid portfolio step={step:e} layout={layout} sign={sign} order={expected:?} calls={calls} relative={relative:e}"
                );
            }
        }
    }
}

#[test]
fn triangular_grid_portfolio_shuffled_numbering_and_mesh_sizes_retain_failure_boundaries() {
    use super::super::{hybrid_tests::check_published, triangular_grid_robustness_tests::Fixture};
    for segments in [80, 100, 128] {
        for step in [1.0, 1e14, 1e-10] {
            let fixture = Fixture::with_segments(segments, step);
            let size = fixture.shape.len();
            let mut accepted = 0;
            let mut rejected = 0;
            let initial = fixture.checked(&fixture.shape).unwrap().0;
            for shuffle in [7, 19, 41, 113] {
                let permutation = shuffled(size, shuffle);
                let mut sorted = permutation.clone();
                sorted.sort_unstable();
                assert_eq!(sorted, (0..size).collect::<Vec<_>>());
                let matrix: Vec<Vec<_>> = permutation
                    .iter()
                    .map(|&i| {
                        permutation
                            .iter()
                            .map(|&j| fixture.directions[i][j])
                            .collect()
                    })
                    .collect();
                let mass: Vec<_> = permutation
                    .iter()
                    .map(|&i| fixture.system.mass[i])
                    .collect();
                for sign in [1.0, -1.0] {
                    let seed: Vec<_> = permutation
                        .iter()
                        .map(|&i| sign * fixture.shape[i])
                        .collect();
                    let restore = |v: &[f64]| {
                        let mut original = vec![0.0; size];
                        for (&i, &a) in permutation.iter().zip(v) {
                            original[i] = a;
                        }
                        original
                    };
                    let mut calls = 0;
                    let outcome = search_unit_shape(&matrix, &seed, &mass, 1e-8, |v| {
                        calls += 1;
                        let (relative, residual) = fixture.checked(&restore(v))?;
                        Ok((relative, permutation.iter().map(|&i| residual[i]).collect()))
                    });
                    assert!(calls <= MAX_TOTAL_CERTIFICATES);
                    let expected_pass = segments != 128
                        || (step == 1.0 && shuffle == 113)
                        || (step == 1e14 && shuffle == 7)
                        || (step == 1e-10 && matches!(shuffle, 7 | 41));
                    assert_eq!(
                        outcome.is_ok(),
                        expected_pass,
                        "{segments}/{step:e}/{shuffle}/{sign}"
                    );
                    match outcome {
                        Ok(result) => {
                            accepted += 1;
                            assert_eq!(result.usage.certificates, calls);
                            assert_eq!(
                                result.shape[result.anchor].to_bits(),
                                seed[result.anchor].to_bits()
                            );
                            let original = restore(&result.shape);
                            let relative = fixture.checked(&original).unwrap().0;
                            assert!(relative <= 1e-8);
                            check_published(segments, step, fixture.value, &original);
                            println!(
                                "shuffled portfolio segments={segments} step={step:e} shuffle={shuffle} sign={sign} initial={initial:e} accepted order={:?} calls={calls} relative={relative:e}",
                                result.order
                            );
                        }
                        Err(error) => {
                            rejected += 1;
                            assert!(error.contains("exhausted its bounded policies"), "{error}");
                            assert!(error.contains("attempts=3"), "{error}");
                            assert!(initial > 1e-8);
                            println!(
                                "shuffled portfolio segments={segments} step={step:e} shuffle={shuffle} sign={sign} initial={initial:e} rejected calls={calls} error={error}"
                            );
                        }
                    }
                    assert_eq!(
                        seed,
                        permutation
                            .iter()
                            .map(|&i| sign * fixture.shape[i])
                            .collect::<Vec<_>>()
                    );
                }
            }
            println!(
                "shuffled portfolio total segments={segments} step={step:e} accepted={accepted} rejected={rejected}"
            );
            assert_eq!(accepted + rejected, 8);
        }
    }
}
