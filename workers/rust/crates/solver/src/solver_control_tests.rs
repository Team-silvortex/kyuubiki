use super::*;
use std::panic::{AssertUnwindSafe, catch_unwind};

#[test]
fn constraint_row_stage_appends_without_renumbering_existing_stages() {
    assert_eq!(SolverStage::ConstraintReduce as u8, 17);
    assert_eq!(SolverStage::TridiagonalForwardRange as u8, 84);
    assert_eq!(SolverStage::ConstraintReduceRow as u8, 85);
    assert_eq!(
        SolverStage::decode(85),
        Some(SolverStage::ConstraintReduceRow)
    );
    assert_eq!(
        SolverStage::ConstraintReduceRow.as_str(),
        "constraint_reduce_row"
    );
}

#[test]
fn tridiagonal_range_stage_appends_without_renumbering_existing_stages() {
    assert_eq!(SolverStage::TridiagonalSubstitution as u8, 8);
    assert_eq!(SolverStage::BandedSubstitutionRange as u8, 82);
    assert_eq!(SolverStage::TridiagonalSubstitutionRange as u8, 83);
    assert_eq!(
        SolverStage::decode(83),
        Some(SolverStage::TridiagonalSubstitutionRange)
    );
    assert_eq!(SolverStage::decode(0), None);
    assert_eq!(SolverStage::decode(u8::MAX), None);
    assert_eq!(
        SolverStage::TridiagonalSubstitutionRange.as_str(),
        "tridiagonal_substitution_range"
    );
}

#[test]
fn tridiagonal_forward_stage_appends_after_back_substitution_range() {
    assert_eq!(SolverStage::TridiagonalSubstitutionRange as u8, 83);
    assert_eq!(SolverStage::TridiagonalForwardRange as u8, 84);
    assert_eq!(
        SolverStage::decode(84),
        Some(SolverStage::TridiagonalForwardRange)
    );
    assert_eq!(
        SolverStage::TridiagonalForwardRange.as_str(),
        "tridiagonal_forward_range"
    );
}

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
