use super::*;
use crate::linear_algebra::add_at;
use crate::solver_control::{SolverControl, SolverStage, with_solver_observer};
use std::{cell::Cell, rc::Rc};

fn operator(entries: &[(usize, usize, f64)], mass: &[f64]) -> SparseMassNormalizedOperator {
    let mut stiffness = SparseMatrix::new(mass.len());
    for &(row, column, value) in entries {
        add_at(&mut stiffness, row, column, value);
    }
    SparseMassNormalizedOperator::new(&stiffness, mass).unwrap()
}

fn symmetric(scale: f64, size: usize) -> SparseMassNormalizedOperator {
    let mut entries: Vec<_> = (0..size).map(|i| (i, i, 4.0 * scale)).collect();
    for i in 1..size {
        entries.extend([(i - 1, i, -scale), (i, i - 1, -scale)]);
    }
    operator(&entries, &vec![scale; size])
}

#[test]
fn dense_normalization_retains_representable_weak_couplings_symmetrically() {
    let entries = [
        (0, 0, 1e200),
        (1, 1, 1e-200),
        (0, 1, -1e-300),
        (1, 0, -1e-300),
    ];
    let op = operator(&entries, &[1e200, 1e-200]);
    let dense = op.dense_fallback_matrix().unwrap();
    let sparse = op.normalized_stiffness().unwrap();
    for (row, values) in dense.iter().enumerate() {
        for &(column, value) in sparse.row_entries(row) {
            assert_eq!(values[column].to_bits(), value.to_bits());
        }
        assert!((values[1 - row] / -1e-300 - 1.0).abs() < 1e-14);
    }
    assert_eq!(dense[0][1].to_bits(), dense[1][0].to_bits());
}

#[test]
fn dense_normalization_cannot_turn_unrepresentable_couplings_into_independent_blocks() {
    for mass in [[1e200, 1e200], [1e-320, 1e-320]] {
        let op = operator(
            &[(0, 0, 1.0), (1, 1, 1.0), (0, 1, 1e-300), (1, 0, 1e-300)],
            &mass,
        );
        let error = op
            .dense_fallback_matrix()
            .map(|_| ())
            .expect_err("unrepresentable entries must be rejected");
        assert!(error.contains("normalized stiffness"), "{error}");
    }
}

#[test]
fn dense_normalization_matches_a_discrete_reference_at_extreme_common_scales() {
    for scale in [1.0, 1e-200, 1e200, 1e-318] {
        let op = symmetric(scale, 6);
        let dense = op.dense_fallback_matrix().unwrap();
        for (row, values) in dense.iter().enumerate() {
            for (column, &value) in values.iter().enumerate() {
                let expected = if row == column {
                    4.0
                } else if row.abs_diff(column) == 1 {
                    -1.0
                } else {
                    0.0
                };
                assert!((value - expected).abs() < 1e-14);
                assert_eq!(value.to_bits(), dense[column][row].to_bits());
            }
        }
    }
}

#[test]
fn dense_normalization_does_not_replay_sparse_matvec_per_column() {
    let op = symmetric(1.0, 129);
    let matvecs = Rc::new(Cell::new(0));
    let visited = Rc::new(Cell::new(0));
    let (observed, rows) = (matvecs.clone(), visited.clone());
    let dense = with_solver_observer(
        &SolverControl::default(),
        move |point| {
            if point.stage == SolverStage::SparseMatvec && point.completed_steps == 0 {
                observed.set(observed.get() + 1);
            }
            if point.stage == SolverStage::SparseMatrixScale && point.completed_steps > 0 {
                rows.set(point.completed_steps);
            }
        },
        || op.dense_fallback_matrix(),
    )
    .unwrap();
    assert_eq!(
        matvecs.get(),
        0,
        "dense assembly must visit stored entries directly"
    );
    assert_eq!(visited.get(), 129);
    assert_eq!(dense.len(), 129);
}

#[test]
fn dense_normalization_rejects_oversized_fallback_before_allocating_the_matrix() {
    let op = symmetric(1.0, 4097);
    let error = op
        .dense_fallback_matrix()
        .map(|_| ())
        .expect_err("oversized fallback must be rejected");
    assert!(error.contains("at most 4096"), "{error}");
}

#[test]
fn dense_normalization_row_cancellation_does_not_poison_sparse_or_dense_replay() {
    let op = symmetric(1.0, 129);
    let control = SolverControl::default();
    let cancel = control.clone();
    let error = with_solver_observer(
        &control,
        move |point| {
            if point.stage == SolverStage::SparseMatrixScale && point.completed_steps == 64 {
                cancel.request_cancel();
            }
        },
        || {
            let result = op.dense_fallback_matrix();
            assert!(
                result.is_err(),
                "normalization must stop before returning a matrix"
            );
            result
        },
    )
    .unwrap_err();
    assert!(error.contains("cancel"), "{error}");
    let dense = op.dense_fallback_matrix().unwrap();
    let sparse = op.normalized_stiffness().unwrap();
    for (row, values) in dense.iter().enumerate() {
        for &(column, value) in sparse.row_entries(row) {
            assert_eq!(values[column].to_bits(), value.to_bits());
        }
    }
}

#[test]
fn dense_normalization_wide_row_cancellation_is_not_deferred_to_the_next_row() {
    let mut entries: Vec<_> = (0..130).map(|i| (i, i, 256.0)).collect();
    for i in 1..130 {
        entries.extend([(0, i, -1.0), (i, 0, -1.0)]);
    }
    let op = operator(&entries, &[1.0; 130]);
    let control = SolverControl::default();
    let cancel = control.clone();
    let error = with_solver_observer(
        &control,
        move |point| {
            if point.stage == SolverStage::SparseMatrixScaleRow && point.completed_steps == 64 {
                cancel.request_cancel();
            }
        },
        || {
            let result = op.dense_fallback_matrix();
            assert!(result.is_err());
            result
        },
    )
    .unwrap_err();
    assert!(error.contains("cancel"));
    let dense = op.dense_fallback_matrix().unwrap();
    assert!(dense[0][1..].iter().all(|&value| value == -1.0));
}

#[test]
fn direct_normalization_matches_column_baseline_and_records_bounded_debug_timings() {
    use std::{hint::black_box, time::Instant};

    for size in [128, 512, 1024] {
        let op = symmetric(1.0, size);
        let mut timings = [Vec::new(), Vec::new()];
        for round in 0..3 {
            let mut matrices = [Vec::new(), Vec::new()];
            for path in if round % 2 == 0 { [0, 1] } else { [1, 0] } {
                let started = Instant::now();
                let matrix = if path == 0 {
                    let mut dense = vec![vec![0.0; size]; size];
                    for column in 0..size {
                        let mut basis = vec![0.0; size];
                        basis[column] = 1.0;
                        let applied = op.apply(&basis).unwrap();
                        for row in 0..size {
                            dense[row][column] = applied[row];
                        }
                    }
                    dense
                } else {
                    op.dense_fallback_matrix().unwrap()
                };
                black_box(&matrix);
                timings[path].push(started.elapsed());
                matrices[path] = matrix;
            }
            assert!(
                matrices[0]
                    .iter()
                    .flatten()
                    .zip(matrices[1].iter().flatten())
                    .all(|(left, right)| left.to_bits() == right.to_bits())
            );
        }
        timings.iter_mut().for_each(|values| values.sort());
        println!(
            "modal normalization debug microbenchmark: dofs={size}, entries={}, column_median_us={}, direct_median_us={}, samples=3",
            op.stiffness.values.len(),
            timings[0][1].as_micros(),
            timings[1][1].as_micros()
        );
    }
}
