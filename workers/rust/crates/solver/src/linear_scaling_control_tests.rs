use super::*;
use crate::solver_control::{SolverControl, with_solver_control, with_solver_observer};

fn system(size: usize, negative: bool, wide: bool) -> (SparseMatrix, Vec<f64>) {
    let sign = if negative { -1.0 } else { 1.0 };
    let mut matrix = SparseMatrix::with_uniform_row_capacity(size, 4);
    for row in 0..size {
        add_at(&mut matrix, row, row, sign * (4.0 + (row % 7) as f64));
        if row + 1 < size {
            add_at(&mut matrix, row, row + 1, -0.25);
            add_at(&mut matrix, row + 1, row, -0.25);
        }
    }
    if size > 17 {
        add_at(&mut matrix, 0, 17, 0.01);
        add_at(&mut matrix, 17, 0, 0.01);
    }
    if wide {
        for column in 2..size {
            add_at(&mut matrix, 0, column, 0.0001);
            add_at(&mut matrix, column, 0, 0.0001);
        }
    }
    let rhs = (0..size).map(|i| 1.0 + (i % 11) as f64).collect();
    (matrix, rhs)
}

fn interrupted<T>(stage: SolverStage, steps: usize, operation: impl FnOnce() -> Result<T, String>) {
    let control = SolverControl::default();
    let cancel = control.clone();
    let result = with_solver_observer(
        &control,
        move |point| {
            if point.stage == stage && point.completed_steps == steps as u64 {
                cancel.request_cancel();
            }
        },
        || {
            let result = operation();
            assert!(result.is_err(), "raw sparse operation ignored {stage:?}");
            result
        },
    );
    let error = result.err().expect("cancellation must escape");
    assert!(error.starts_with("solver cancelled"), "{stage:?}: {error}");
    assert!(control.was_interrupted(), "did not reach {stage:?}");
    let point = control.last_checkpoint().unwrap();
    assert_eq!(point.stage, stage);
    assert_eq!(point.completed_steps, steps as u64);
}

fn cancel_public(stage: SolverStage, steps: usize, negative: bool, wide: bool) {
    let (matrix, rhs) = system(2049, negative, wide);
    interrupted(stage, steps, || solve_spd_system_profile(&matrix, &rhs));
}

#[test]
fn sparse_rhs_validation_cancels_inside_the_vector() {
    cancel_public(SolverStage::SparseValidateRhs, 1024, false, false);
}

#[test]
fn sparse_matrix_validation_cancels_between_rows() {
    cancel_public(SolverStage::SparseValidateMatrix, 64, false, false);
}

#[test]
fn sparse_matrix_validation_cancels_inside_a_wide_row() {
    cancel_public(SolverStage::SparseValidateMatrixRow, 1024, false, true);
}

#[test]
fn sparse_diagonal_scaling_cancels_between_rows() {
    cancel_public(SolverStage::SparseDiagonalScale, 64, false, false);
}

#[test]
fn sparse_rhs_scaling_cancels_inside_the_vector() {
    cancel_public(SolverStage::SparseRhsScale, 1024, false, false);
}

#[test]
fn sparse_solution_unscaling_cancels_before_result_acceptance() {
    cancel_public(SolverStage::SparseSolutionUnscale, 1024, false, false);
}

#[test]
fn sparse_diagonal_magnitude_cancels_inside_the_reduction() {
    cancel_public(SolverStage::SparseDiagonalMagnitude, 64, false, false);
}

#[test]
fn sparse_fallback_capacity_scan_cancels_between_rows() {
    cancel_public(SolverStage::SparseCapacityScan, 1024, true, false);
}

#[test]
fn sparse_fallback_matrix_scaling_cancels_between_rows() {
    cancel_public(SolverStage::SparseMatrixScale, 64, true, false);
}

#[test]
fn sparse_fallback_matrix_scaling_cancels_inside_a_wide_row() {
    cancel_public(SolverStage::SparseMatrixScaleRow, 1024, true, true);
}

#[test]
fn sparse_regularization_copy_cancels_between_rows() {
    cancel_public(SolverStage::SparseRegularizeCopy, 64, true, false);
}

#[test]
fn sparse_regularization_copy_cancels_inside_a_wide_row() {
    cancel_public(SolverStage::SparseRegularizeCopyRow, 1024, true, true);
}

#[test]
fn sparse_regularization_diagonal_cancels_between_rows() {
    cancel_public(SolverStage::SparseRegularizeDiagonal, 64, true, false);
}

#[path = "linear_scaling_reference.rs"]
mod reference;

fn profile(solution: Vec<f64>) -> SpdSolveProfile {
    SpdSolveProfile {
        solution,
        iterations: 7,
        matrix_non_zero_count: 123,
        residual_norm: 0.125,
        stages: vec![crate::linear_solver_profile::SpdSolveStage {
            label: "test-stage",
            elapsed_ms: 1.25,
        }],
    }
}

fn same_bits(actual: &[f64], expected: &[f64]) {
    assert_eq!(actual.len(), expected.len());
    for (index, (&actual, &expected)) in actual.iter().zip(expected).enumerate() {
        assert!(
            actual.to_bits() == expected.to_bits() || actual.is_nan() && expected.is_nan(),
            "element {index}: {actual:?} != {expected:?}"
        );
    }
}

fn same_matrix(actual: &SparseMatrix, expected: &SparseMatrix) {
    assert_eq!(actual.size(), expected.size());
    for (actual, expected) in actual.rows.iter().zip(&expected.rows) {
        assert_eq!(actual.len(), expected.len());
        for ((actual_col, actual), (expected_col, expected)) in actual.iter().zip(expected) {
            assert_eq!(actual_col, expected_col);
            same_bits(&[*actual], &[*expected]);
        }
    }
}

#[test]
fn scaling_preserves_original_arithmetic_sparse_entries_and_profile_metadata() {
    for size in [0, 1, 63, 64, 65, 1023, 1024, 1025, 2049] {
        let (mut matrix, _) = system(size, false, size > 1024);
        if size > 1 {
            matrix.rows[size - 1].clear();
            matrix.add_at(1, 1, -matrix.diagonal_value(1));
        }
        let factors = reference::diagonal_scaling(&matrix);
        same_bits(
            &scaling::diagonal_sparse_scaling(&matrix).unwrap(),
            &factors,
        );
        same_bits(
            &[scaling::average_scaled_diagonal_magnitude(&matrix, &factors).unwrap()],
            &[reference::diagonal_magnitude(&matrix, &factors)],
        );
        same_matrix(
            &scaling::scale_sparse_matrix(&matrix, &factors).unwrap(),
            &reference::scale_matrix(&matrix, &factors),
        );
        for epsilon in [0.0, 0.125, -4.0, 1e-300] {
            same_matrix(
                &scaling::regularize_sparse_diagonal(&matrix, epsilon).unwrap(),
                &reference::regularize(&matrix, epsilon),
            );
        }
        let values: Vec<_> = (0..size)
            .map(|i| [0.0, -0.0, 1e100, -1e-100][i % 4])
            .collect();
        let expected = reference::scale_vector(&values, &factors);
        same_bits(
            &scaling::scale_sparse_rhs(&values, &factors).unwrap(),
            &expected,
        );
        let unscaled = scaling::unscale_profile(profile(values), &factors).unwrap();
        same_bits(&unscaled.solution, &expected);
        assert_eq!(unscaled.iterations, 7);
        assert_eq!(unscaled.matrix_non_zero_count, 123);
        assert_eq!(unscaled.residual_norm.to_bits(), 0.125f64.to_bits());
        assert_eq!(unscaled.stages.len(), 1);
        assert_eq!(unscaled.stages[0].label, "test-stage");
        assert_eq!(unscaled.stages[0].elapsed_ms, 1.25);
    }
}

#[test]
fn scaling_polls_empty_short_and_last_blocks_without_publishing_partial_data() {
    for size in [0, 1, 63, 64, 65, 1023, 1024, 1025] {
        let (matrix, rhs) = system(size, false, false);
        let factors = vec![0.5; size];
        for steps in [0, size] {
            interrupted(SolverStage::SparseValidateRhs, steps, || {
                scaling::validate_sparse_system_finite(&matrix, &rhs)
            });
            interrupted(SolverStage::SparseValidateMatrix, steps, || {
                scaling::validate_sparse_system_finite(&matrix, &rhs)
            });
            interrupted(SolverStage::SparseDiagonalScale, steps, || {
                scaling::diagonal_sparse_scaling(&matrix)
            });
            interrupted(SolverStage::SparseRhsScale, steps, || {
                scaling::scale_sparse_rhs(&rhs, &factors)
            });
            interrupted(SolverStage::SparseSolutionUnscale, steps, || {
                scaling::unscale_profile(profile(rhs.clone()), &factors)
            });
            interrupted(SolverStage::SparseDiagonalMagnitude, steps, || {
                scaling::average_scaled_diagonal_magnitude(&matrix, &factors)
            });
            interrupted(SolverStage::SparseCapacityScan, steps, || {
                scaling::scale_sparse_matrix(&matrix, &factors)
            });
            interrupted(SolverStage::SparseMatrixScale, steps, || {
                scaling::scale_sparse_matrix(&matrix, &factors)
            });
            interrupted(SolverStage::SparseRegularizeCopy, steps, || {
                scaling::regularize_sparse_diagonal(&matrix, 0.01)
            });
            interrupted(SolverStage::SparseRegularizeDiagonal, steps, || {
                scaling::regularize_sparse_diagonal(&matrix, 0.01)
            });
        }
        assert_eq!(
            scaling::validate_sparse_system_finite(&matrix, &rhs),
            Ok(())
        );
    }
}

#[test]
fn binary_diagonal_lookup_matches_linear_reference_at_wide_row_edges() {
    const SIZE: usize = 2049;
    for row in [0, 1024, SIZE - 1] {
        let mut matrix = SparseMatrix::new(SIZE);
        matrix.rows[row] = (0..SIZE).map(|column| (column, 0.25)).collect();
        for diagonal in [0.0, -4.0, 1e-300, 1e300, f64::INFINITY, f64::NAN] {
            matrix.rows[row][row].1 = diagonal;
            same_bits(
                &scaling::diagonal_sparse_scaling(&matrix).unwrap(),
                &reference::diagonal_scaling(&matrix),
            );
        }
        matrix.rows[row].remove(row);
        same_bits(
            &scaling::diagonal_sparse_scaling(&matrix).unwrap(),
            &reference::diagonal_scaling(&matrix),
        );
    }
}

#[test]
fn wide_scaling_rows_poll_entry_interior_and_final_short_chunks() {
    for size in [1025, 2049] {
        let (matrix, rhs) = system(size, false, true);
        assert_eq!(matrix.rows[0].len(), size);
        for steps in [0, 1024, size] {
            interrupted(SolverStage::SparseValidateMatrixRow, steps, || {
                scaling::validate_sparse_system_finite(&matrix, &rhs)
            });
            interrupted(SolverStage::SparseMatrixScaleRow, steps, || {
                scaling::scale_sparse_matrix(&matrix, &vec![0.5; size])
            });
            interrupted(SolverStage::SparseRegularizeCopyRow, steps, || {
                scaling::regularize_sparse_diagonal(&matrix, 0.01)
            });
        }
        same_matrix(
            &scaling::regularize_sparse_diagonal(&matrix, 0.01).unwrap(),
            &reference::regularize(&matrix, 0.01),
        );
    }
}

#[test]
fn validation_keeps_nonfinite_rejection_priority_at_vector_and_row_edges() {
    let (matrix, rhs) = system(2049, false, true);
    for index in [0, 63, 1023, 1024, 2048] {
        for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let mut bad_rhs = rhs.clone();
            bad_rhs[index] = invalid;
            let mut bad_matrix = matrix.clone();
            bad_matrix.rows[0][index].1 = invalid;
            for (matrix, rhs) in [
                (&matrix, &bad_rhs),
                (&bad_matrix, &rhs),
                (&bad_matrix, &bad_rhs),
            ] {
                assert_eq!(
                    scaling::validate_sparse_system_finite(matrix, rhs),
                    reference::validate(matrix, rhs)
                );
            }
        }
    }
    let mut bad_rhs = rhs.clone();
    bad_rhs[2048] = f64::NAN;
    interrupted(SolverStage::SparseValidateRhs, 1024, || {
        scaling::validate_sparse_system_finite(&matrix, &bad_rhs)
    });
    let mut bad_matrix = matrix.clone();
    bad_matrix.rows[0][2048].1 = f64::NAN;
    interrupted(SolverStage::SparseValidateMatrixRow, 1024, || {
        scaling::validate_sparse_system_finite(&bad_matrix, &rhs)
    });
    assert_eq!(
        scaling::validate_sparse_system_finite(&matrix, &rhs),
        Ok(())
    );
}

#[test]
fn validation_counts_empty_rows_and_still_rejects_late_invalid_entries() {
    let mut matrix = SparseMatrix::new(2049);
    interrupted(SolverStage::SparseValidateMatrix, 64, || {
        scaling::validate_sparse_system_finite(&matrix, &[])
    });
    matrix.add_at(2048, 2048, f64::INFINITY);
    assert_eq!(
        scaling::validate_sparse_system_finite(&matrix, &[]),
        reference::validate(&matrix, &[])
    );
}

#[test]
fn prepared_scaling_cancellation_does_not_escape_or_poison_a_reused_factor() {
    let (matrix, rhs) = system(2049, false, false);
    for stage in [
        SolverStage::SparseValidateMatrix,
        SolverStage::SparseDiagonalScale,
        SolverStage::SparseDiagonalMagnitude,
    ] {
        interrupted(stage, 64, || PreparedSpdSolver::factor(matrix.clone()));
    }
    let prepared = PreparedSpdSolver::factor(matrix.clone()).unwrap();
    let expected = prepared.solve(&rhs).unwrap();
    for (stage, steps) in [
        (SolverStage::SparseValidateRhs, 1024),
        (SolverStage::SparseValidateMatrix, 64),
        (SolverStage::SparseRhsScale, 1024),
        (SolverStage::SparseSolutionUnscale, 1024),
    ] {
        interrupted(stage, steps, || prepared.solve(&rhs));
        same_bits(&prepared.solve(&rhs).unwrap(), &expected);
    }
    let (bad_matrix, bad_rhs) = system(2049, true, true);
    let bad_prepared = PreparedSpdSolver::factor(bad_matrix).unwrap();
    for (stage, steps) in [
        (SolverStage::SparseCapacityScan, 1024),
        (SolverStage::SparseMatrixScale, 64),
        (SolverStage::SparseMatrixScaleRow, 1024),
        (SolverStage::SparseRegularizeCopy, 64),
        (SolverStage::SparseRegularizeCopyRow, 1024),
        (SolverStage::SparseRegularizeDiagonal, 64),
    ] {
        interrupted(stage, steps, || bad_prepared.solve(&bad_rhs));
    }
    same_bits(&prepared.solve(&rhs).unwrap(), &expected);
}

#[test]
fn public_scaling_cancellation_preserves_all_three_preconditioner_results() {
    let (matrix, rhs) = system(2049, false, false);
    for preconditioner in [
        SpdPreconditioner::Jacobi,
        SpdPreconditioner::SymmetricGaussSeidel,
        SpdPreconditioner::IncompleteCholesky,
    ] {
        let options = SpdSolveOptions {
            preconditioner,
            progress_interval: None,
        };
        let expected =
            solve_spd_system_profile_with_options(&matrix, &rhs, options.clone()).unwrap();
        for (stage, steps) in [
            (SolverStage::SparseValidateRhs, 1024),
            (SolverStage::SparseValidateMatrix, 64),
            (SolverStage::SparseDiagonalScale, 64),
            (SolverStage::SparseRhsScale, 1024),
            (SolverStage::SparseDiagonalMagnitude, 64),
            (SolverStage::SparseSolutionUnscale, 1024),
        ] {
            interrupted(stage, steps, || {
                solve_spd_system_profile_with_options(&matrix, &rhs, options.clone())
            });
            let actual = with_solver_control(&SolverControl::default(), || {
                solve_spd_system_profile_with_options(&matrix, &rhs, options.clone())
            })
            .unwrap();
            same_bits(&actual.solution, &expected.solution);
            same_bits(&[actual.residual_norm], &[expected.residual_norm]);
            assert_eq!(actual.iterations, expected.iterations);
            assert_eq!(actual.matrix_non_zero_count, expected.matrix_non_zero_count);
        }
    }
}

#[path = "linear_scaling_benchmark.rs"]
mod benchmark;
