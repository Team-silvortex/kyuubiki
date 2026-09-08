use super::*;
use crate::solver_control::{SolverControl, with_solver_observer};

fn matrix(size: usize, wide: bool) -> SparseMatrix {
    let mut matrix = SparseMatrix::with_uniform_row_capacity(size, 3);
    for row in 0..size {
        add_at(&mut matrix, row, row, 4.0);
        if row + 1 < size {
            add_at(&mut matrix, row, row + 1, -1.0);
            add_at(&mut matrix, row + 1, row, -1.0);
        }
    }
    if wide {
        for col in 2..size {
            add_at(&mut matrix, 0, col, -0.0001);
            add_at(&mut matrix, col, 0, -0.0001);
        }
    }
    matrix
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
            assert!(
                result.is_err(),
                "raw operation swallowed cancellation at {stage:?}"
            );
            result
        },
    );
    assert!(result.is_err(), "ignored cancellation at {stage:?}");
    assert!(control.was_interrupted());
    let point = control.last_checkpoint().unwrap();
    assert_eq!(point.stage, stage);
    assert_eq!(point.completed_steps, steps as u64);
}

#[test]
fn compressed_matvec_observes_completed_rows() {
    let matrix = matrix(160, false)
        .compress(SpdPreconditioner::Jacobi)
        .unwrap();
    interrupted(SolverStage::SparseMatvec, 64, || {
        matrix.multiply_vector_into(&[1.0; 160], &mut [0.0; 160])
    });
}

#[test]
fn compressed_matvec_observes_the_interior_of_a_wide_row() {
    let matrix = matrix(4097, true)
        .compress(SpdPreconditioner::Jacobi)
        .unwrap();
    interrupted(SolverStage::SparseMatvecRow, 1024, || {
        matrix.multiply_vector_into(&[1.0; 4097], &mut [0.0; 4097])
    });
}

#[test]
fn residual_norm_observes_completed_rows() {
    let matrix = matrix(160, false);
    interrupted(SolverStage::SparseResidual, 64, || {
        sparse_residual_norm(&matrix, &[1.0; 160], &[0.5; 160])
    });
}

#[test]
fn residual_vector_observes_the_interior_of_a_wide_row() {
    let matrix = matrix(4097, true);
    interrupted(SolverStage::SparseResidualRow, 1024, || {
        sparse_residual_vector(&matrix, &[1.0; 4097], &[0.5; 4097])
    });
}

#[test]
fn relative_validation_observes_completed_rows() {
    let matrix = matrix(160, false);
    interrupted(SolverStage::ResidualValidate, 64, || {
        sparse_relative_residual(&matrix, &[1.0; 160], &[0.5; 160], 1e-8)
    });
}

#[test]
fn relative_validation_observes_the_interior_of_a_wide_row() {
    let matrix = matrix(4097, true);
    interrupted(SolverStage::ResidualValidateRow, 1024, || {
        sparse_relative_residual(&matrix, &[1.0; 4097], &[0.5; 4097], 1e-8)
    });
}

fn reference_matvec(matrix: &CompressedSparseMatrix, vector: &[f64], result: &mut [f64]) {
    for (row, result_value) in result.iter_mut().enumerate() {
        let start = matrix.row_offsets[row];
        let end = matrix.row_offsets[row + 1];
        let mut sum = 0.0;
        for (&column, &value) in matrix.columns[start..end]
            .iter()
            .zip(&matrix.values[start..end])
        {
            sum += value * vector[column];
        }
        *result_value = sum;
    }
}

#[test]
fn wide_product_retains_entry_order_and_reuses_partly_written_output() {
    let matrix = matrix(4097, true)
        .compress(SpdPreconditioner::Jacobi)
        .unwrap();
    let vector: Vec<_> = (0..4097)
        .map(|i| if i % 2 == 0 { 1e10 } else { -1e-9 })
        .collect();
    let mut reference = vec![0.0; 4097];
    reference_matvec(&matrix, &vector, &mut reference);
    let mut result = vec![f64::NAN; 4097];
    interrupted(SolverStage::SparseMatvecRow, 1024, || {
        matrix.multiply_vector_into(&vector, &mut result)
    });
    assert!(
        result[0].is_nan(),
        "an unfinished row must not be published"
    );
    interrupted(SolverStage::SparseMatvec, 64, || {
        matrix.multiply_vector_into(&vector, &mut result)
    });
    assert_eq!(result[..64], reference[..64]);
    assert!(result[64].is_nan());
    matrix.multiply_vector_into(&vector, &mut result).unwrap();
    assert_eq!(
        result.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
        reference.iter().map(|v| v.to_bits()).collect::<Vec<_>>()
    );
}

#[test]
fn compression_records_the_actual_row_bound_for_both_scaling_paths() {
    for size in [0, 49, 1024, 1025] {
        let matrix = matrix(size, true);
        for compressed in [
            matrix.compress(SpdPreconditioner::Jacobi).unwrap(),
            matrix
                .compress_scaled(&vec![0.5; size], SpdPreconditioner::Jacobi)
                .unwrap(),
        ] {
            assert_eq!(compressed.max_row_entries, size);
            let mut actual = vec![0.0; size];
            let mut expected = vec![0.0; size];
            reference_matvec(&compressed, &vec![1.0; size], &mut expected);
            compressed
                .multiply_vector_into(&vec![1.0; size], &mut actual)
                .unwrap();
            assert_eq!(actual, expected);
            if size > 1024 {
                interrupted(SolverStage::SparseMatvecRow, 1024, || {
                    compressed.multiply_vector_into(&vec![1.0; size], &mut actual)
                });
            }
        }
    }
}

#[test]
fn residuals_keep_the_uninterrupted_arithmetic_and_stable_norm() {
    let matrix = matrix(4097, true);
    let solution: Vec<_> = (0..4097)
        .map(|i| if i % 2 == 0 { 1e100 } else { -1e-100 })
        .collect();
    let rhs = vec![0.5; 4097];
    let reference: Vec<_> = matrix
        .rows
        .iter()
        .zip(&rhs)
        .map(|(row, expected)| expected - row.iter().map(|(c, v)| v * solution[*c]).sum::<f64>())
        .collect();
    let expected_norm = stable_l2_norm(reference.iter().copied());
    for stage in [SolverStage::SparseResidual, SolverStage::SparseResidualRow] {
        interrupted(
            stage,
            if stage == SolverStage::SparseResidual {
                64
            } else {
                1024
            },
            || sparse_residual_norm(&matrix, &rhs, &solution),
        );
    }
    let actual = sparse_residual_vector(&matrix, &rhs, &solution).unwrap();
    assert_eq!(
        actual.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
        reference.iter().map(|v| v.to_bits()).collect::<Vec<_>>()
    );
    assert_eq!(
        sparse_residual_norm(&matrix, &rhs, &solution)
            .unwrap()
            .to_bits(),
        expected_norm.to_bits()
    );
}

#[test]
fn short_empty_and_second_validation_passes_observe_cancellation() {
    for size in [0, 49] {
        let matrix = matrix(size, false);
        let compressed = matrix.compress(SpdPreconditioner::Jacobi).unwrap();
        let rhs = vec![1.0; size];
        let solution = vec![0.0; size];
        interrupted(SolverStage::SparseMatvec, size, || {
            compressed.multiply_vector_into(&rhs, &mut vec![0.0; size])
        });
        interrupted(SolverStage::SparseResidual, size, || {
            sparse_residual_norm(&matrix, &rhs, &solution)
        });
        interrupted(SolverStage::ResidualValidate, size * 2, || {
            sparse_relative_residual(&matrix, &rhs, &solution, 1e-8)
        });
    }
    let matrix = matrix(160, false);
    interrupted(SolverStage::ResidualValidate, 192, || {
        sparse_relative_residual(&matrix, &[1.0; 160], &[0.5; 160], 1e-8)
    });
}

#[test]
fn reusable_backends_do_not_export_a_cancelled_validation() {
    for (size, non_chain) in [(160, false), (160, true), (1100, true)] {
        let mut matrix = matrix(size, false);
        if non_chain {
            add_at(&mut matrix, 0, 17, 0.1);
            add_at(&mut matrix, 17, 0, 0.1);
        }
        let prepared = PreparedSpdSolver::factor(matrix).unwrap();
        let rhs = vec![1.0; size];
        let baseline = prepared.solve(&rhs).unwrap();
        for stage in [SolverStage::SparseResidual, SolverStage::ResidualValidate] {
            interrupted(stage, 64, || prepared.solve(&rhs));
            assert_eq!(prepared.solve(&rhs).unwrap(), baseline);
        }
        if size > 1024 {
            interrupted(SolverStage::SparseMatvec, 64, || prepared.solve(&rhs));
            assert_eq!(prepared.solve(&rhs).unwrap(), baseline);
        }
    }
}

#[test]
fn modal_products_propagate_cancellation_without_poisoning_the_operator() {
    let matrix = matrix(160, false);
    let operator =
        crate::modal_sparse::SparseMassNormalizedOperator::new(&matrix, &[2.0; 160]).unwrap();
    let rhs = vec![1.0; 160];
    let baseline = operator.apply(&rhs).unwrap();
    interrupted(SolverStage::SparseMatvec, 64, || operator.apply(&rhs));
    assert_eq!(operator.apply(&rhs).unwrap(), baseline);
}

#[test]
fn buckling_subspace_propagates_a_cancelled_product() {
    let mut stiffness = SparseMatrix::new(160);
    let mut geometric = SparseMatrix::new(160);
    for row in 0..160 {
        add_at(&mut stiffness, row, row, if row == 0 { 2.0 } else { 5.0 });
    }
    add_at(&mut geometric, 0, 0, 1.0);
    add_at(&mut geometric, 1, 1, 2.0);
    let run = || crate::buckling_sparse::sparse_generalized_eigenpairs(&stiffness, &geometric, 1);
    let baseline = run().unwrap();
    interrupted(SolverStage::SparseMatvec, 64, run);
    let after = run().unwrap();
    assert_eq!(after[0].eigenvalue, baseline[0].eigenvalue);
    assert_eq!(after[0].vector, baseline[0].vector);
}

#[test]
#[ignore = "release-mode paired microbenchmark; run explicitly with --ignored --nocapture"]
fn paired_matvec_control_overhead() {
    use std::hint::black_box;
    use std::time::Instant;
    let size = 1_000_000;
    let matrix = matrix(size, false)
        .compress(SpdPreconditioner::Jacobi)
        .unwrap();
    let input = vec![1.0; size];
    let mut output = vec![0.0; size];
    let mut reference = output.clone();
    reference_matvec(&matrix, &input, &mut reference);
    let control = SolverControl::default();
    for _ in 0..4 {
        matrix.multiply_vector_into(&input, &mut output).unwrap();
    }
    let mut samples = [Vec::new(), Vec::new(), Vec::new()];
    for round in 0..9 {
        for offset in 0..3 {
            let mode = (round + offset) % 3;
            let start = Instant::now();
            for _ in 0..20 {
                match mode {
                    0 => reference_matvec(black_box(&matrix), black_box(&input), &mut output),
                    1 => matrix
                        .multiply_vector_into(black_box(&input), &mut output)
                        .unwrap(),
                    _ => crate::solver_control::with_solver_control(&control, || {
                        matrix.multiply_vector_into(black_box(&input), &mut output)
                    })
                    .unwrap(),
                }
                black_box(&output);
            }
            samples[mode].push(start.elapsed().as_secs_f64() * 1000.0 / 20.0);
            assert_eq!(output, reference);
        }
    }
    for sample in &mut samples {
        sample.sort_by(f64::total_cmp);
    }
    eprintln!(
        "paired_matvec rows={size} nnz={} reps=20 samples=9 baseline_ms={:.6} unscoped_ms={:.6} controlled_ms={:.6}",
        matrix.non_zero_count(),
        samples[0][4],
        samples[1][4],
        samples[2][4]
    );
}
