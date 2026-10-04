use super::*;
use crate::solver_control::{SolverControl, with_solver_observer};
use std::{cell::RefCell, rc::Rc};

fn diagonal() -> Vec<Vec<f64>> {
    (0..4)
        .map(|i| {
            (0..4)
                .map(|j| if i == j { (i + 1) as f64 } else { 0.0 })
                .collect()
        })
        .collect()
}

#[test]
fn hybrid_switch_shares_one_checked_product_budget_and_preserves_private_seeds() {
    let seed = [1.0; 4];
    let gram = BlockFit::prepare_automatic(diagonal(), &seed).unwrap();
    let qr = BlockFit::prepare_qr_automatic(diagonal(), &seed).unwrap();
    let visits = Rc::new(RefCell::new(Vec::new()));
    let observed = visits.clone();
    let error = with_solver_observer(
        &SolverControl::default(),
        move |p| {
            if p.stage == SolverStage::ModalRoundoffSearch {
                observed.borrow_mut().push(p.completed_steps);
            }
        },
        || gram.correct_hybrid(&qr, &seed, 1e-8, |_| Ok((1.0, vec![1.0; 4]))),
    )
    .unwrap_err();
    assert!(error.contains("unchanged residual gate"));
    assert_eq!(seed, [1.0; 4]);
    let visits = visits.borrow();
    assert!(visits.len() > 8 && visits.len() <= 80);
    assert!(
        visits
            .iter()
            .copied()
            .eq((0..visits.len()).map(|i| i as u64))
    );
    assert_eq!(
        gram.correct_hybrid(&qr, &seed, 1e-8, |_| Ok((0.0, vec![0.0; 4])))
            .unwrap(),
        seed
    );
}

#[test]
fn hybrid_switch_cancellation_replays_with_cached_factors_without_partial_results() {
    let seed = [1.0; 4];
    let gram = BlockFit::prepare_automatic(diagonal(), &seed).unwrap();
    let qr = BlockFit::prepare_qr_automatic(diagonal(), &seed).unwrap();
    let control = SolverControl::default();
    let cancel = control.clone();
    let switched = std::cell::Cell::new(false);
    let error = with_solver_observer(
        &control,
        move |p| {
            if p.stage == SolverStage::ModalIteration && p.completed_steps == 1 {
                switched.set(true);
            }
            if switched.get() && p.stage == SolverStage::ModalRoundoffSearch {
                cancel.request_cancel();
            }
        },
        || gram.correct_hybrid(&qr, &seed, 1e-8, |_| Ok((1.0, vec![1.0; 4]))),
    )
    .unwrap_err();
    assert!(error.contains("cancel"));
    assert_eq!(
        control.last_checkpoint().unwrap().stage,
        SolverStage::ModalRoundoffSearch
    );
    assert_eq!(seed, [1.0; 4]);
    assert_eq!(
        gram.correct_hybrid(&qr, &seed, 1e-8, |_| Ok((0.0, vec![0.0; 4])))
            .unwrap(),
        seed
    );
}

#[test]
fn hybrid_grid_rechecks_final_certificates_cancels_each_phase_and_replays() {
    let seed = [1.0; 4];
    let mut calls = 0;
    let error = grid::correct(&diagonal(), &seed, 1e-8, |_| {
        calls += 1;
        Ok((if calls == 1 { 0.0 } else { 1.0 }, vec![0.0; 4]))
    })
    .unwrap_err();
    assert!(error.contains("unchanged residual gate"));
    assert_eq!(calls, 2);
    for stage in [
        SolverStage::ModalRoundoffPrepare,
        SolverStage::ModalRoundoffSearch,
        SolverStage::ModalRoundoffValidate,
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
            || grid::correct(&diagonal(), &seed, 1e-8, |_| Ok((0.0, vec![0.0; 4]))),
        )
        .unwrap_err();
        assert!(error.contains("cancel"));
        assert_eq!(seed, [1.0; 4]);
        assert_eq!(
            grid::correct(&diagonal(), &seed, 1e-8, |_| Ok((0.0, vec![0.0; 4]))).unwrap(),
            seed
        );
    }
    for tolerance in [0.0, -1.0, f64::INFINITY, f64::NAN] {
        assert!(grid::correct(&diagonal(), &seed, tolerance, |_| unreachable!()).is_err());
    }
    assert!(grid::correct(&[vec![1.0; 257]; 1], &seed, 1e-8, |_| unreachable!()).is_err());
    assert!(grid::correct(&diagonal(), &seed, 1e-8, |_| Ok((0.0, vec![0.0; 3]))).is_err());
    assert!(grid::correct(&diagonal(), &seed, 1e-8, |_| Ok((f64::NAN, vec![0.0; 4]))).is_err());
}
