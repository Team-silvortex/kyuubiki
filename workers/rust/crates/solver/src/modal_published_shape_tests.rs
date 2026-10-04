use super::*;
use crate::linear_algebra::{SparseMatrix, add_at};
use crate::solver_control::{SolverControl, with_solver_observer};
use std::{cell::Cell, rc::Rc};

#[path = "modal_published_shape_control_tests.rs"]
mod control;

fn operator(values: &[f64], mass: &[f64]) -> SparseMassNormalizedOperator {
    let mut matrix = SparseMatrix::new(values.len());
    for (i, &value) in values.iter().enumerate() {
        add_at(&mut matrix, i, i, value);
    }
    SparseMassNormalizedOperator::new(&matrix, mass).unwrap()
}

#[test]
fn published_shape_polish_retains_exact_shapes_and_rejects_invalid_inputs() {
    let op = operator(&[2.0, 3.0], &[1.0, 1.0]);
    let mut shape = vec![1.0, 0.0];
    op.polish_published_shape(2.0, &mut shape, &[1.0, 1.0], 1e-8)
        .unwrap();
    assert_eq!(shape, [1.0, 0.0]);
    for tolerance in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(
            op.polish_published_shape(2.0, &mut shape, &[1.0, 1.0], tolerance)
                .is_err()
        );
    }
}

fn coupled(scale: f64, size: usize) -> SparseMassNormalizedOperator {
    let mut matrix = SparseMatrix::new(size);
    for i in 0..size {
        add_at(&mut matrix, i, i, 2.0 * scale);
        add_at(&mut matrix, i, i ^ 1, -scale);
    }
    SparseMassNormalizedOperator::new(&matrix, &vec![scale; size]).unwrap()
}

fn bits(values: &[f64]) -> Vec<u64> {
    values.iter().map(|v| v.to_bits()).collect()
}

#[test]
fn published_shape_uses_original_physical_coordinates_and_mass() {
    let mut matrix = SparseMatrix::new(2);
    for (row, column, value) in [(0, 0, 8.0), (0, 1, -4.0), (1, 0, -4.0), (1, 1, 32.0)] {
        add_at(&mut matrix, row, column, value);
    }
    let op = SparseMassNormalizedOperator::new(&matrix, &[4.0, 16.0]).unwrap();
    assert_eq!(
        op.apply_physical_compensated(&[0.5, -0.25]).unwrap(),
        [2.5, -2.5]
    );
    let mut shape = vec![0.5, -0.25];
    op.polish_published_shape(2.5, &mut shape, &[4.0, 16.0], 1e-14)
        .unwrap();
    assert_eq!(shape, [0.5, -0.25]);
}

#[test]
fn published_shape_subnormal_stiffness_does_not_round_before_mass_weighting() {
    for scale in [1e-318, 1e307] {
        let op = operator(&[scale], &[scale]);
        let mut shape = vec![0.123456789];
        let result = op.apply_physical_compensated(&shape).unwrap()[0];
        let expected = (scale * scale.sqrt().recip()) * shape[0];
        assert!((result / expected - 1.0).abs() < 1e-14);
        op.polish_published_shape(1.0, &mut shape, &[scale], 1e-8)
            .unwrap();
        assert_eq!(shape, [0.123456789]);
    }
}

#[test]
fn published_shape_repairs_small_output_roundoff_without_factorization_at_extreme_scales() {
    for scale in [1.0, 1e200, 1e-200] {
        let op = coupled(scale, 2);
        let mut shape = vec![1.0 + 1e-7, 1.0];
        with_solver_observer(
            &SolverControl::default(),
            |point| {
                assert_ne!(point.stage, SolverStage::DenseFactor);
            },
            || op.polish_published_shape(1.0, &mut shape, &[scale; 2], 1e-8),
        )
        .unwrap();
        assert!((shape[0] - shape[1]).abs() < 1e-14);
        let (relative, _) = op
            .physical_residual(
                1.0,
                &shape,
                &[scale; 2],
                &op.apply_physical_compensated(&shape).unwrap(),
            )
            .unwrap();
        assert!(relative <= 1e-8, "scale={scale:e}, residual={relative:e}");
    }
}

#[test]
fn published_shape_rejects_nonfinite_or_mismatched_data_without_mutation() {
    let op = coupled(1.0, 2);
    for original in [
        vec![],
        vec![1.0],
        vec![0.0, 0.0],
        vec![f64::NAN, 1.0],
        vec![f64::INFINITY, 1.0],
    ] {
        let mut shape = original.clone();
        assert!(
            op.polish_published_shape(1.0, &mut shape, &[1.0; 2], 1e-8)
                .is_err()
        );
        assert_eq!(bits(&shape), bits(&original));
    }
    for mass in [
        vec![],
        vec![1.0],
        vec![0.0, 1.0],
        vec![-1.0, 1.0],
        vec![f64::NAN, 1.0],
        vec![f64::INFINITY, 1.0],
    ] {
        let mut shape = vec![1.0, 1.0];
        assert!(
            op.polish_published_shape(1.0, &mut shape, &mass, 1e-8)
                .is_err()
        );
        assert_eq!(shape, [1.0, 1.0]);
    }
    for value in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        let mut shape = vec![1.0, 1.0];
        assert!(
            op.polish_published_shape(value, &mut shape, &[1.0; 2], 1e-8)
                .is_err()
        );
        assert_eq!(shape, [1.0, 1.0]);
    }
}

#[test]
fn published_shape_budget_exhaustion_is_atomic_and_does_not_poison_replay() {
    let op = coupled(1.0, 2);
    let original = vec![1.0 + 1e-7, 1.0];
    let mut shape = original.clone();
    let error = op
        .polish_shape_with_budget(1.0, &mut shape, &[1.0; 2], 1e-8, 0)
        .unwrap_err();
    assert!(error.contains("within 0 corrections"), "{error}");
    assert_eq!(shape, original);
    op.polish_published_shape(1.0, &mut shape, &[1.0; 2], 1e-8)
        .unwrap();
    assert!((shape[0] - shape[1]).abs() < 1e-14);
}

#[test]
fn published_shape_rejects_a_non_eigenpair_instead_of_publishing_zero() {
    let op = operator(&[2.0, 3.0], &[1.0, 1.0]);
    let mut shape = vec![1.0, 1.0];
    assert!(
        op.polish_published_shape(1.0, &mut shape, &[1.0; 2], 1e-8)
            .is_err()
    );
    assert_eq!(shape, [1.0, 1.0]);
}

#[test]
fn published_shape_product_residual_and_sweeps_cancel_atomically_and_replay() {
    for (stage, after_repair) in [
        (SolverStage::SparseMatvec, false),
        (SolverStage::ResidualValidate, false),
        (SolverStage::ResidualValidate, true),
    ] {
        let op = coupled(1.0, 130);
        let mut shape = vec![1.0; 130];
        shape[0] += 1e-5;
        let original = shape.clone();
        let repairing = Rc::new(Cell::new(false));
        let seen = Rc::new(Cell::new(false));
        let (active, hit) = (repairing.clone(), seen.clone());
        let control = SolverControl::default();
        let cancel = control.clone();
        let error = with_solver_observer(
            &control,
            move |point| {
                if point.stage == SolverStage::ModalValidation {
                    active.set(true);
                }
                if (!after_repair || active.get())
                    && point.stage == stage
                    && point.completed_steps == 64
                {
                    hit.set(true);
                    cancel.request_cancel();
                }
            },
            || {
                let result = op.polish_published_shape(1.0, &mut shape, &[1.0; 130], 1e-8);
                assert!(result.is_err());
                result
            },
        )
        .unwrap_err();
        assert!(seen.get());
        assert!(error.contains("cancel"));
        assert_eq!(shape, original);
        op.polish_published_shape(1.0, &mut shape, &[1.0; 130], 1e-8)
            .unwrap();
    }
}

#[test]
fn published_shape_wide_columns_cancel_before_finishing_a_sweep() {
    let size = 1026;
    let mut matrix = SparseMatrix::new(size);
    add_at(&mut matrix, 0, 0, 1.0 + (size - 1) as f64 * 0.125);
    for row in 1..size {
        add_at(&mut matrix, row, row, 1.125);
        add_at(&mut matrix, row, 0, -0.125);
        add_at(&mut matrix, 0, row, -0.125);
    }
    let mass = vec![1.0; size];
    let op = SparseMassNormalizedOperator::new(&matrix, &mass).unwrap();
    let mut shape = vec![1.0; size];
    shape[0] += 1e-4;
    let original = shape.clone();
    let control = SolverControl::default();
    let cancel = control.clone();
    let seen = Rc::new(Cell::new(false));
    let hit = seen.clone();
    let error = with_solver_observer(
        &control,
        move |point| {
            if point.stage == SolverStage::ResidualValidateRow && point.completed_steps == 1024 {
                hit.set(true);
                cancel.request_cancel();
            }
        },
        || {
            let result = op.polish_published_shape(1.0, &mut shape, &mass, 1e-8);
            assert!(result.is_err());
            result
        },
    )
    .unwrap_err();
    assert!(seen.get());
    assert!(error.contains("cancel"));
    assert_eq!(shape, original);
    op.polish_published_shape(1.0, &mut vec![1.0; size], &mass, 1e-8)
        .unwrap();
}
