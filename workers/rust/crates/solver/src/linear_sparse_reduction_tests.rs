use super::{reduce_sparse_system, reduce_sparse_system_with_prescribed};
use crate::linear_algebra::{SparseMatrix, add_at};
use crate::solver_control::{SolverControl, SolverStage, with_solver_observer};

fn scalar_row(diagonal: f64, coefficients: &[f64]) -> SparseMatrix {
    let mut matrix = SparseMatrix::new(coefficients.len() + 1);
    add_at(&mut matrix, 0, 0, diagonal);
    for (index, &value) in coefficients.iter().enumerate() {
        add_at(&mut matrix, 0, index + 1, value);
        add_at(&mut matrix, index + 1, 0, value);
        add_at(&mut matrix, index + 1, index + 1, 1.0);
    }
    matrix
}

#[test]
fn sparse_prescribed_range_combines_four_quarter_quantum_contributions() {
    let matrix = scalar_row(1.0, &[-0.25; 4]);
    for value in [1, 3, 7, 511]
        .into_iter()
        .flat_map(|bits| [-f64::from_bits(bits), f64::from_bits(bits)])
    {
        let prescribed = (1..=4).map(|index| (index, value)).collect::<Vec<_>>();
        let (reduced, rhs, free) =
            reduce_sparse_system_with_prescribed(&matrix, &[0.0; 5], &prescribed).unwrap();
        assert_eq!(free, [0]);
        assert_eq!(reduced.row_entries(0), [(0, 1.0)]);
        assert_eq!(rhs, [value]);
    }
}

#[test]
fn sparse_prescribed_range_rejects_erased_and_rounded_reduced_loads() {
    let matrix = scalar_row(0.5, &[-0.5]);
    for bits in [1, 3, 7] {
        let error =
            reduce_sparse_system_with_prescribed(&matrix, &[0.0; 2], &[(1, f64::from_bits(bits))])
                .unwrap_err();
        assert!(error.contains("range"), "{error}");
    }
    for bits in [2, 4, 8] {
        let (_, rhs, _) =
            reduce_sparse_system_with_prescribed(&matrix, &[0.0; 2], &[(1, f64::from_bits(bits))])
                .unwrap();
        assert_eq!(rhs, [f64::from_bits(bits / 2)]);
    }
}

#[test]
fn sparse_prescribed_range_recovers_overflow_cancellation_without_erasing_the_load() {
    let matrix = scalar_row(1.0, &[2.0, -2.0]);
    let large = 2.0_f64.powi(1023);
    let (_, rhs, _) =
        reduce_sparse_system_with_prescribed(&matrix, &[1.0, 0.0, 0.0], &[(1, large), (2, large)])
            .unwrap();
    assert_eq!(rhs, [1.0]);
}

#[test]
fn sparse_reduction_rejects_dimension_mismatch_instead_of_panicking_or_ignoring_rows() {
    let matrix = scalar_row(1.0, &[-1.0]);
    for rhs in [vec![], vec![0.0], vec![0.0; 3]] {
        assert!(reduce_sparse_system(&matrix, &rhs, &[]).is_err());
        assert!(reduce_sparse_system_with_prescribed(&matrix, &rhs, &[]).is_err());
    }
}

#[test]
fn sparse_reduction_rejects_invalid_or_conflicting_prescribed_data() {
    let matrix = scalar_row(1.0, &[-1.0]);
    assert!(reduce_sparse_system(&matrix, &[0.0; 2], &[2]).is_err());
    for prescribed in [
        vec![(2, 0.0)],
        vec![(0, f64::NAN)],
        vec![(0, f64::INFINITY)],
        vec![(0, 0.0), (0, f64::from_bits(1))],
    ] {
        assert!(reduce_sparse_system_with_prescribed(&matrix, &[0.0; 2], &prescribed).is_err());
    }
}

#[test]
fn sparse_reduction_validates_values_even_when_their_rows_are_constrained() {
    let mut matrix = scalar_row(1.0, &[-1.0]);
    matrix.rows[1][1].1 = f64::NAN;
    assert!(reduce_sparse_system(&matrix, &[0.0; 2], &[1]).is_err());
    assert!(reduce_sparse_system_with_prescribed(&matrix, &[0.0; 2], &[(1, 0.0)]).is_err());
    let matrix = scalar_row(1.0, &[-1.0]);
    assert!(reduce_sparse_system(&matrix, &[0.0, f64::INFINITY], &[1]).is_err());
    assert!(
        reduce_sparse_system_with_prescribed(&matrix, &[0.0, f64::INFINITY], &[(1, 0.0)]).is_err()
    );
}

#[test]
fn sparse_reduction_rejects_bad_columns_order_duplicates_and_nonfinite_coefficients() {
    for entries in [
        vec![(2, 1.0)],
        vec![(0, 1.0), (0, 2.0)],
        vec![(1, 1.0), (0, 2.0)],
        vec![(0, f64::INFINITY)],
    ] {
        let mut matrix = SparseMatrix::new(2);
        matrix.rows[1] = entries;
        assert!(reduce_sparse_system(&matrix, &[0.0; 2], &[1]).is_err());
        assert!(reduce_sparse_system_with_prescribed(&matrix, &[0.0; 2], &[(1, 0.0)]).is_err());
    }
}

#[test]
fn sparse_reduction_keeps_identical_duplicates_empty_and_all_fixed_systems() {
    let empty = SparseMatrix::new(0);
    for result in [
        reduce_sparse_system(&empty, &[], &[]),
        reduce_sparse_system_with_prescribed(&empty, &[], &[]),
    ] {
        let (matrix, rhs, free) = result.unwrap();
        assert_eq!(matrix.size(), 0);
        assert!(rhs.is_empty() && free.is_empty());
    }
    let matrix = scalar_row(1.0, &[-1.0]);
    let (homogeneous, rhs, free) = reduce_sparse_system(&matrix, &[0.0, 1.0], &[0, 0]).unwrap();
    let (reference, expected, expected_free) =
        reduce_sparse_system_with_prescribed(&matrix, &[0.0, 1.0], &[(0, 0.0), (0, -0.0)]).unwrap();
    assert_eq!(rhs, expected);
    assert_eq!(free, expected_free);
    assert_eq!(homogeneous.row_entries(0), reference.row_entries(0));
    let (empty, rhs, free) =
        reduce_sparse_system_with_prescribed(&matrix, &[0.0; 2], &[(1, 2.0), (0, 1.0), (0, 1.0)])
            .unwrap();
    assert_eq!(empty.size(), 0);
    assert!(rhs.is_empty() && free.is_empty());
}

#[test]
fn sparse_prescribed_range_preserves_normal_rhs_bits_and_symmetric_coefficients() {
    let mut matrix = SparseMatrix::new(4);
    for (row, col, value) in [
        (0, 0, 1.0),
        (1, 0, 0.3),
        (1, 1, 2.0),
        (1, 2, 0.25),
        (1, 3, -0.7),
        (2, 1, 0.25),
        (2, 2, 3.0),
        (3, 3, 1.0),
    ] {
        add_at(&mut matrix, row, col, value);
    }
    let snapshot = matrix.rows.clone();
    let (reduced, rhs, free) = with_solver_observer(
        &SolverControl::default(),
        |point| assert_ne!(point.stage, SolverStage::ConstraintReduceRow),
        || {
            reduce_sparse_system_with_prescribed(
                &matrix,
                &[0.0, 1.0, 2.0, 0.0],
                &[(3, 1.1), (0, 0.7)],
            )
        },
    )
    .unwrap();
    assert_eq!(free, [1, 2]);
    assert_eq!(
        rhs[0].to_bits(),
        ((1.0_f64 - 0.3 * 0.7) - (-0.7 * 1.1)).to_bits()
    );
    assert_eq!(rhs[1], 2.0);
    assert_eq!(reduced.row_entries(0), [(0, 2.0), (1, 0.25)]);
    assert_eq!(reduced.row_entries(1), [(0, 0.25), (1, 3.0)]);
    assert_eq!(matrix.rows, snapshot);
}

#[test]
fn sparse_prescribed_range_rejects_true_overflow_and_alignment_loss_then_reuses_inputs() {
    let matrix = scalar_row(1.0, &[2.0]);
    let error =
        reduce_sparse_system_with_prescribed(&matrix, &[0.0; 2], &[(1, 2.0_f64.powi(1023))])
            .unwrap_err();
    assert!(error.contains("representable range"), "{error}");
    assert_eq!(
        reduce_sparse_system_with_prescribed(&matrix, &[0.0; 2], &[(1, 0.5)])
            .unwrap()
            .1,
        [-1.0]
    );
    let matrix = scalar_row(1.0, &[0.5]);
    let error =
        reduce_sparse_system_with_prescribed(&matrix, &[1.0, 0.0], &[(1, f64::from_bits(1))])
            .unwrap_err();
    assert!(error.contains("normalization"), "{error}");
    assert_eq!(
        reduce_sparse_system_with_prescribed(&matrix, &[1.0, 0.0], &[(1, 1.0)])
            .unwrap()
            .1,
        [0.5]
    );
}

#[test]
fn sparse_prescribed_range_cancels_wide_row_and_recovery_phases_then_replays() {
    let size = 2048;
    let matrix = scalar_row(1.0, &vec![-1.0 / size as f64; size]);
    let force = vec![0.0; size + 1];
    let quantum = f64::from_bits(1);
    let prescribed = (1..=size).map(|index| (index, quantum)).collect::<Vec<_>>();
    for (phase, step) in [
        (1, 0),
        (1, 1024),
        (2, 1024),
        (3, 0),
        (3, 64),
        (3, 2048),
        (3, 4096),
    ] {
        let control = SolverControl::default();
        let cancel = control.clone();
        let entered = std::cell::Cell::new(0);
        let result = with_solver_observer(
            &control,
            move |point| {
                if point.stage == SolverStage::ConstraintReduceRow {
                    if point.completed_steps == 0 {
                        entered.set(entered.get() + 1);
                    }
                    if entered.get() == phase && point.completed_steps == step {
                        cancel.request_cancel();
                    }
                }
            },
            || reduce_sparse_system_with_prescribed(&matrix, &force, &prescribed),
        );
        assert!(result.unwrap_err().contains("cancelled"));
        assert!(control.was_interrupted());
        let (reduced, rhs, free) =
            reduce_sparse_system_with_prescribed(&matrix, &force, &prescribed).unwrap();
        assert_eq!(reduced.row_entries(0), [(0, 1.0)]);
        assert_eq!(rhs, [quantum]);
        assert_eq!(free, [0]);
    }
}

#[test]
fn sparse_reduction_cancels_validation_homogeneous_wide_rows_and_final_publication() {
    let matrix = scalar_row(1.0, &vec![-0.25; 2048]);
    let rhs = vec![0.0; 2049];
    for (stage, step, prescribed) in [
        (SolverStage::SparseValidateRhs, 1024, false),
        (SolverStage::SparseValidateMatrixRow, 1024, false),
        (SolverStage::ConstraintReduceRow, 1024, false),
        (SolverStage::ConstraintReduce, 2049, false),
        (SolverStage::ConstraintReduce, 2049, true),
    ] {
        let control = SolverControl::default();
        let cancel = control.clone();
        let result = with_solver_observer(
            &control,
            move |point| {
                if point.stage == stage && point.completed_steps == step {
                    cancel.request_cancel();
                }
            },
            || {
                if prescribed {
                    reduce_sparse_system_with_prescribed(&matrix, &rhs, &[])
                } else {
                    reduce_sparse_system(&matrix, &rhs, &[])
                }
            },
        );
        assert!(result.unwrap_err().contains("cancelled"));
        let (reduced, load, free) = reduce_sparse_system(&matrix, &rhs, &[]).unwrap();
        assert_eq!(free.len(), 2049);
        assert_eq!(load, rhs);
        assert_eq!(reduced.rows, matrix.rows);
    }
}
