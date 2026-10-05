use super::*;
use crate::solver_control::{SolverControl, with_solver_observer};
use std::{cell::Cell, rc::Rc};

fn identity() -> Vec<Vec<Wide>> {
    (0..3)
        .map(|i| (0..3).map(|j| Wide::from(f64::from(i == j))).collect())
        .collect()
}

#[test]
fn modal_internal_inward_chart_bounded_contract_fault_cancel_and_replay() {
    let plan = Plan::inward_direction(256).unwrap();
    assert_eq!(plan.max_attempts, 2);
    assert_eq!(plan.max_certificates, 13);
    assert_eq!(plan.payload_bytes, 6_946_816);
    assert_eq!(plan.component_visits, 673_742_848);
    for size in [0, 1, 257, usize::MAX] {
        assert!(Plan::inward_direction(size).is_err());
    }
    let seed = [0.2, 0.4, 0.6];
    let matrix = identity();
    // Synthetic certificates check policy mechanics, not physical numerical accuracy.
    let baseline =
        search_canonical_direction_inward(&matrix, &seed, 1e-8, |_| Ok((0.0, vec![0.0; 3])))
            .unwrap();
    assert_eq!(baseline.order, Order::GridNorm);
    assert_eq!(baseline.usage.attempts, 1);
    assert_eq!(baseline.usage.certificates, 3);
    assert_eq!(baseline.shape[2].to_bits(), seed[2].next_down().to_bits());
    for amplitude in [-2.0_f64, -1.0, 0.5, 2.0] {
        for permutation in [[0, 1, 2], [2, 0, 1], [1, 2, 0]] {
            let mapped = permutation.map(|i| amplitude * seed[i]);
            let actual = search_canonical_direction_inward(&matrix, &mapped, 1e-8, |_| {
                Ok((0.0, vec![0.0; 3]))
            })
            .unwrap();
            assert_eq!(permutation[actual.anchor], 2);
            assert_eq!(actual.usage.attempts, 1);
            for (i, &j) in permutation.iter().enumerate() {
                assert_eq!(
                    (actual.shape[i] / amplitude).to_bits(),
                    baseline.shape[j].to_bits()
                );
            }
        }
    }
    let mut calls = 0;
    let error = search_canonical_direction_inward(&matrix, &seed, 1e-8, |_| {
        calls += 1;
        Ok((
            if calls <= 6 {
                1.0 / (calls + 1) as f64
            } else {
                1.0 / (calls - 5) as f64
            },
            vec![0.0; 3],
        ))
    })
    .unwrap_err();
    assert_eq!(calls, 12);
    assert!(error.contains("attempts=2, certificates=12"), "{error}");
    calls = 0;
    let error = search_canonical_direction_inward(&matrix, &seed, 1e-8, |_| {
        calls += 1;
        Ok((if calls < 3 { 0.0 } else { 1.0 }, vec![0.0; 3]))
    })
    .unwrap_err();
    assert!(
        error.contains("lost its final normalized certificate"),
        "{error}"
    );
    assert_eq!(calls, 3);
    for fault_at in [1, 3] {
        let factors = Rc::new(Cell::new(0));
        let observed = factors.clone();
        let mut calls = 0;
        let error = with_solver_observer(
            &SolverControl::default(),
            move |p| {
                if p.stage == SolverStage::DenseFactor && p.completed_steps == 0 {
                    observed.set(observed.get() + 1);
                }
            },
            || {
                search_canonical_direction_inward(&matrix, &seed, 1e-8, |_| {
                    calls += 1;
                    if calls == fault_at {
                        Err("internal chart operator fault".into())
                    } else {
                        Ok((0.0, vec![0.0; 3]))
                    }
                })
            },
        )
        .unwrap_err();
        assert_eq!(error, "internal chart operator fault");
        assert_eq!(calls, fault_at);
        assert_eq!(factors.get(), 1);
    }
    for (stage, step) in [
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
                if p.stage == stage && p.completed_steps == step {
                    observed.set(true);
                    cancel.request_cancel();
                }
            },
            || search_canonical_direction_inward(&matrix, &seed, 1e-8, |_| Ok((0.0, vec![0.0; 3]))),
        )
        .unwrap_err();
        assert!(saw.get() && error.contains("cancel"), "{error}");
        let replay =
            search_canonical_direction_inward(&matrix, &seed, 1e-8, |_| Ok((0.0, vec![0.0; 3])))
                .unwrap();
        assert_eq!(replay.shape, baseline.shape);
        assert_eq!(replay.usage.certificates, baseline.usage.certificates);
    }
    for invalid in [
        [0.4; 3],
        [0.2, 0.4, 0.4_f64.next_up()],
        [0.0, 0.4, 0.6],
        [f64::NAN, 0.4, 0.6],
        [0.1, 0.01, 0.02],
        [3.0, 4.0, 5.0],
    ] {
        assert!(
            search_canonical_direction_inward(&matrix, &invalid, 1e-8, |_| unreachable!()).is_err()
        );
    }
    for certificate in [
        (f64::NAN, vec![0.0; 3]),
        (0.0, vec![0.0; 2]),
        (-1.0, vec![0.0; 3]),
        (0.0, vec![f64::INFINITY; 3]),
    ] {
        let mut calls = 0;
        assert!(
            search_canonical_direction_inward(&matrix, &seed, 1e-8, |_| {
                calls += 1;
                Ok(certificate.clone())
            })
            .is_err()
        );
        assert_eq!(calls, 1);
    }
    println!(
        "internal inward chart attempts_cap=2 certificates_cap=13 faults=2 cancellations=4 exact_replays=4 reserved_payload_bytes=6946816 reserved_component_visits=673742848"
    );
}
