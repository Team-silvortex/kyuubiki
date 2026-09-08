use super::*;
use std::panic::{AssertUnwindSafe, catch_unwind};

#[test]
fn pre_cancelled_control_never_evaluates_the_operation() {
    let control = SolverControl::default();
    control.request_cancel();
    let result: Result<(), String> = with_solver_control(&control, || panic!("must not run"));
    assert!(result.unwrap_err().contains("cancelled"));
    assert!(control.was_interrupted());
}

#[test]
fn cancellation_cannot_be_swallowed_into_partial_success() {
    let control = SolverControl::default();
    let cancel = control.clone();
    let result: Result<Vec<f64>, String> = with_solver_observer(
        &control,
        move |_| {
            cancel.request_cancel();
        },
        || {
            let _ = checkpoint(SolverStage::DenseFactor, 2);
            Ok(vec![123.0])
        },
    );
    assert!(result.unwrap_err().contains("dense_factor after 2 steps"));
}

#[test]
fn nested_scopes_cannot_mask_parent_cancellation() {
    let parent = SolverControl::default();
    let child = SolverControl::default();
    let result: Result<(), String> = with_solver_control(&parent, || {
        parent.request_cancel();
        with_solver_control(&child, || Ok(()))
    });
    assert!(result.is_err());
    assert!(child.was_interrupted());
    assert!(checkpoint(SolverStage::LinearPrepare, 0).is_ok());
}

#[test]
fn unwind_restores_the_previous_scope_and_a_fresh_call() {
    let outer = SolverControl::default();
    let inner = SolverControl::default();
    let result: Result<(), String> = with_solver_control(&outer, || {
        let panic = catch_unwind(AssertUnwindSafe(|| {
            let _: Result<(), String> = with_solver_control(&inner, || {
                inner.request_cancel();
                panic!("test unwind");
            });
        }));
        assert!(panic.is_err());
        checkpoint(SolverStage::LinearPrepare, 1)
    });
    result.unwrap();
    assert!(!outer.was_interrupted());
    assert_eq!(outer.last_checkpoint().unwrap().completed_steps, 1);
    checkpoint(SolverStage::LinearPrepare, 2).unwrap();
    assert_eq!(outer.last_checkpoint().unwrap().completed_steps, 1);
}

#[test]
fn control_does_not_leak_into_unrelated_threads() {
    let control = SolverControl::default();
    let result: Result<(), String> = with_solver_control(&control, || {
        control.request_cancel();
        std::thread::spawn(|| checkpoint(SolverStage::SparseIteration, 3))
            .join()
            .unwrap()?;
        checkpoint(SolverStage::SparseIteration, 4)
    });
    assert!(result.is_err());
}

#[test]
fn cancellation_request_does_not_relabel_an_unrelated_error() {
    let control = SolverControl::default();
    let result: Result<(), String> = with_solver_control(&control, || {
        control.request_cancel();
        Err("numerical singularity".into())
    });
    assert_eq!(result.unwrap_err(), "numerical singularity");
    assert!(!control.was_interrupted());
}
