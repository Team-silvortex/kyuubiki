use super::*;
use crate::solver_control::{SolverControl, with_solver_observer};
use std::{cell::Cell, rc::Rc};

fn identity() -> Vec<Vec<Wide>> {
    (0..3)
        .map(|i| (0..3).map(|j| Wide::from(f64::from(i == j))).collect())
        .collect()
}

fn seed() -> [f64; 3] {
    let norm = 14.0_f64.sqrt();
    [1.0 / norm, 2.0 / norm, 3.0 / norm]
}

fn bits(v: &[f64]) -> Vec<u64> {
    v.iter().map(|v| v.to_bits()).collect()
}

#[test]
fn modal_inward_chart_budget_sign_permutation_and_single_fit_are_explicit() {
    let plan = Plan::inward_chart(256, Order::Reverse).unwrap();
    assert_eq!(plan.payload_bytes, 6_946_816);
    assert_eq!(plan.component_visits, 338_198_528);
    assert_eq!(plan.max_attempts, 1);
    assert_eq!(plan.max_certificates, 7);
    for bad in [0, 1, 257, usize::MAX] {
        assert!(Plan::inward_chart(bad, Order::Reverse).is_err());
    }
    for order in [Order::Reverse, Order::Natural, Order::GridNorm] {
        for permutation in [[0, 1, 2], [2, 0, 1], [1, 2, 0]] {
            for sign in [1.0_f64, -1.0] {
                let seed = permutation.map(|i| sign * seed()[i]);
                let frozen = bits(&seed);
                let factors = Rc::new(Cell::new(0));
                let observed = factors.clone();
                let mut calls = 0;
                let actual = with_solver_observer(
                    &SolverControl::default(),
                    move |p| {
                        if p.stage == SolverStage::DenseFactor && p.completed_steps == 0 {
                            observed.set(observed.get() + 1);
                        }
                    },
                    || {
                        search_canonical_unit_shape_inward(
                            &identity(),
                            &seed,
                            &[1.0; 3],
                            1e-8,
                            order,
                            |_| {
                                calls += 1;
                                // Synthetic certificate tests chart mechanics, not modal accuracy.
                                Ok((0.0, vec![0.0; 3]))
                            },
                        )
                    },
                )
                .unwrap();
                assert_eq!(factors.get(), 1);
                assert_eq!(calls, 3);
                assert_eq!(actual.usage.certificates, calls);
                assert_eq!(actual.usage.attempts, 1);
                assert_eq!(actual.order, order);
                assert_eq!(permutation[actual.anchor], 2);
                let expected = if sign > 0.0 {
                    seed[actual.anchor].next_down()
                } else {
                    seed[actual.anchor].next_up()
                };
                for (i, value) in seed.iter().enumerate() {
                    assert_eq!(
                        actual.shape[i].to_bits(),
                        if i == actual.anchor {
                            expected.to_bits()
                        } else {
                            value.to_bits()
                        }
                    );
                }
                let plan = Plan::inward_chart(3, order).unwrap();
                assert_eq!(actual.usage.component_visits, plan.component_visits);
                assert_eq!(bits(&seed), frozen);
            }
        }
        let mut calls = 0;
        let error = search_canonical_unit_shape_inward(
            &identity(),
            &seed(),
            &[1.0; 3],
            1e-8,
            order,
            |_| {
                calls += 1;
                Ok((1.0 / (calls + 1) as f64, vec![0.0; 3]))
            },
        )
        .unwrap_err();
        assert_eq!(calls, 6);
        assert!(error.contains("attempts=1, certificates=6"), "{error}");
        calls = 0;
        let accepted = search_canonical_unit_shape_inward(
            &identity(),
            &seed(),
            &[1.0; 3],
            1e-8,
            order,
            |_| {
                calls += 1;
                Ok((
                    if calls < 6 {
                        1.0 / (calls + 1) as f64
                    } else {
                        0.0
                    },
                    vec![0.0; 3],
                ))
            },
        )
        .unwrap();
        assert_eq!(calls, 7);
        assert_eq!(accepted.usage.certificates, 7);
        assert_eq!(accepted.usage.attempts, 1);
    }
    println!(
        "inward chart synthetic_sign_permutation_controls=18 physical_fit_cap=1 certificate_cap=7 reserved_payload_bytes=6946816 reserved_component_visits=338198528"
    );
}

#[test]
fn modal_inward_chart_preflight_and_malformed_certificates_fail_closed() {
    let matrix = identity();
    let ties = [3.0_f64.sqrt().recip(); 3];
    let error = search_canonical_unit_shape_inward(
        &matrix,
        &ties,
        &[1.0; 3],
        1e-8,
        Order::Reverse,
        |_| unreachable!(),
    )
    .unwrap_err();
    assert!(
        error.contains("uniquely dominant shifted anchor"),
        "{error}"
    );
    let mut near_ties = ties;
    near_ties[2] = near_ties[2].next_up();
    let error = search_canonical_unit_shape_inward(
        &matrix,
        &near_ties,
        &[1.0; 3],
        1e-8,
        Order::Reverse,
        |_| unreachable!(),
    )
    .unwrap_err();
    assert!(
        error.contains("uniquely dominant shifted anchor"),
        "{error}"
    );
    for bad in [f64::NAN, f64::INFINITY, 0.0, 1e-51] {
        let mut invalid = seed();
        invalid[0] = bad;
        assert!(
            search_canonical_unit_shape_inward(
                &matrix,
                &invalid,
                &[1.0; 3],
                1e-8,
                Order::Reverse,
                |_| unreachable!()
            )
            .is_err()
        );
    }
    for bad in [f64::NAN, 0.0, -1.0] {
        assert!(
            search_canonical_unit_shape_inward(
                &matrix,
                &seed(),
                &[1.0, bad, 1.0],
                1e-8,
                Order::Reverse,
                |_| unreachable!()
            )
            .is_err()
        );
    }
    assert!(
        search_canonical_unit_shape_inward(
            &matrix,
            &[1.0, 2.0, 3.0],
            &[1.0; 3],
            1e-8,
            Order::Reverse,
            |_| unreachable!()
        )
        .is_err()
    );
    for receipt in [
        (0.0, vec![0.0; 2]),
        (f64::NAN, vec![0.0; 3]),
        (-1.0, vec![0.0; 3]),
        (0.0, vec![f64::INFINITY; 3]),
    ] {
        let mut calls = 0;
        assert!(
            search_canonical_unit_shape_inward(
                &matrix,
                &seed(),
                &[1.0; 3],
                1e-8,
                Order::Reverse,
                |_| {
                    calls += 1;
                    Ok(receipt.clone())
                }
            )
            .is_err()
        );
        assert_eq!(calls, 1);
    }
    let mut calls = 0;
    let error = search_canonical_unit_shape_inward(
        &matrix,
        &seed(),
        &[1.0; 3],
        1e-8,
        Order::Reverse,
        |_| {
            calls += 1;
            Ok((if calls < 3 { 0.0 } else { 1.0 }, vec![0.0; 3]))
        },
    )
    .unwrap_err();
    assert!(
        error.contains("lost its final physical certificate"),
        "{error}"
    );
    assert_eq!(calls, 3);
    for fault_at in [1, 3] {
        let mut calls = 0;
        let error = search_canonical_unit_shape_inward(
            &matrix,
            &seed(),
            &[1.0; 3],
            1e-8,
            Order::Reverse,
            |_| {
                calls += 1;
                if calls == fault_at {
                    Err("unchanged residual gate: operator fault is not a rejection".into())
                } else {
                    Ok((0.0, vec![0.0; 3]))
                }
            },
        )
        .unwrap_err();
        assert_eq!(
            error,
            "unchanged residual gate: operator fault is not a rejection"
        );
        assert_eq!(calls, fault_at);
    }
}

#[test]
fn modal_inward_chart_preparation_fit_search_and_last_validation_cancel_and_replay() {
    let matrix = identity();
    let seed = seed();
    let original = bits(&seed);
    let baseline =
        search_canonical_unit_shape_inward(&matrix, &seed, &[1.0; 3], 1e-8, Order::Reverse, |_| {
            Ok((0.0, vec![0.0; 3]))
        })
        .unwrap();
    for (stage, steps) in [
        (SolverStage::ModalRoundoffPrepare, 0),
        (SolverStage::DenseFactor, 0),
        (SolverStage::ModalRoundoffSearch, 1),
        (SolverStage::ModalRoundoffValidate, 6),
    ] {
        let control = SolverControl::default();
        let cancel = control.clone();
        let saw = Rc::new(Cell::new(false));
        let observed = saw.clone();
        let error = with_solver_observer(
            &control,
            move |p| {
                if p.stage == stage && p.completed_steps == steps {
                    observed.set(true);
                    cancel.request_cancel();
                }
            },
            || {
                search_canonical_unit_shape_inward(
                    &matrix,
                    &seed,
                    &[1.0; 3],
                    1e-8,
                    Order::Reverse,
                    |_| Ok((0.0, vec![0.0; 3])),
                )
            },
        )
        .unwrap_err();
        assert!(saw.get() && error.contains("cancel"), "{error}");
        let replay = search_canonical_unit_shape_inward(
            &matrix,
            &seed,
            &[1.0; 3],
            1e-8,
            Order::Reverse,
            |_| Ok((0.0, vec![0.0; 3])),
        )
        .unwrap();
        assert_eq!(bits(&replay.shape), bits(&baseline.shape));
        assert_eq!(replay.usage.certificates, baseline.usage.certificates);
        assert_eq!(bits(&seed), original);
    }
    println!("inward chart cancellation_stages=4 exact_replays=4 no_extra_fit=1");
}
