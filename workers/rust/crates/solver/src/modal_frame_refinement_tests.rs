use super::*;
use crate::linear_algebra::{SparseMatrix, add_at};
use crate::modal_sparse::reduce_sparse_modal_system;
use crate::solver_control::{SolverControl, with_solver_observer};
use std::{cell::Cell, rc::Rc};

pub(super) fn diagonal(values: &[f64]) -> ReducedSparseModalSystem {
    let mut matrix = SparseMatrix::new(values.len());
    for (index, &value) in values.iter().enumerate() {
        add_at(&mut matrix, index, index, value);
    }
    reduce_sparse_modal_system(&matrix, &vec![1.0; values.len()], &[]).unwrap()
}

#[test]
fn accurate_dense_modes_do_not_prepare_an_inverse_or_change_their_basis() {
    let system = diagonal(&[1.0, 2.0, 3.0]);
    let original = vec![(1.0, vec![1.0, 0.0, 0.0]), (2.0, vec![0.0, 1.0, 0.0])];
    let mut pairs = original.clone();
    let preparations = Rc::new(Cell::new(0));
    let observed = preparations.clone();
    with_solver_observer(
        &SolverControl::default(),
        move |point| {
            if point.stage == SolverStage::LinearPrepare {
                observed.set(observed.get() + 1);
            }
        },
        || refine_dense_modes(&system, &mut pairs, 1e-8),
    )
    .unwrap();
    assert_eq!(preparations.get(), 0);
    assert_eq!(pairs, original);
}

#[test]
fn dense_refinement_retains_repeated_low_modes_without_duplicate_directions() {
    let system = diagonal(&[1.0, 1.0, 1e6]);
    let mut pairs = vec![
        (1.0, vec![1.0, 0.0, 1e-10]),
        (1.0, vec![0.0, 1.0, 1e-10]),
        (1e6, vec![-1e-10, -1e-10, 1.0]),
    ];
    refine_dense_modes(&system, &mut pairs, 1e-8).unwrap();
    for (index, (value, vector)) in pairs.iter().enumerate() {
        assert!((value / [1.0, 1.0, 1e6][index] - 1.0).abs() < 1e-10);
        assert!(
            relative_residual(*value, vector, &system.operator.apply(vector).unwrap()).unwrap()
                < 1e-8
        );
        for (_, other) in &pairs[..index] {
            assert!(dot(vector, other).abs() < 1e-12);
        }
    }
}

#[test]
fn dense_refinement_cannot_bypass_its_budget() {
    // Two contaminating directions cannot both be removed by one smoothing candidate.
    let system = diagonal(&[1.0, 2.0, 3.0]);
    let seed = vec![(1.0, vec![1.0, 1e-3, 1e-3])];
    for budget in [0, 1] {
        let error = refine_with_budget(&system, &mut seed.clone(), 1e-8, budget).unwrap_err();
        assert!(error.contains(&format!("within {budget} steps")), "{error}");
    }
}

#[test]
fn dense_refinement_rejects_invalid_vectors_values_and_dependent_directions() {
    for tolerance in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(refine_dense_modes(&diagonal(&[1.0]), &mut [(1.0, vec![1.0])], tolerance).is_err());
    }
    for (value, vector, applied) in [
        (f64::NAN, vec![1.0], vec![1.0]),
        (0.0, vec![1.0], vec![1.0]),
        (1.0, vec![], vec![]),
        (1.0, vec![0.0], vec![0.0]),
        (1.0, vec![1.0], vec![]),
        (1.0, vec![f64::NAN], vec![1.0]),
        (1.0, vec![1.0], vec![f64::INFINITY]),
    ] {
        assert!(relative_residual(value, &vector, &applied).is_err());
    }
    assert!(orthogonalize(&mut [1.0, 0.0], &[(1.0, vec![1.0, 0.0])]).is_err());
}

#[test]
fn dense_refinement_orthogonalization_observes_cancellation_before_scope_exit() {
    let previous = [(1.0, vec![1.0, 0.0, 0.0]), (1.0, vec![0.0, 1.0, 0.0])];
    let control = SolverControl::default();
    let cancel = control.clone();
    let error = with_solver_observer(
        &control,
        move |point| {
            if point.stage == SolverStage::ModalValidation && point.completed_steps == 2 {
                cancel.request_cancel();
            }
        },
        || {
            let result = orthogonalize(&mut [0.0, 0.0, 1.0], &previous);
            assert!(result.is_err());
            result
        },
    )
    .unwrap_err();
    assert!(error.contains("cancel"), "{error}");
    let mut replay = [0.0, 0.0, 1.0];
    orthogonalize(&mut replay, &previous).unwrap();
    assert_eq!(replay, [0.0, 0.0, 1.0]);
}
