use crate::SpdPreconditioner;
use crate::linear_algebra::{
    SparseMatrix, add_at, reduce_sparse_system, reduce_sparse_system_with_prescribed,
};
use crate::solver_control::{SolverControl, SolverStage, with_solver_observer};

fn interrupted<T>(stage: SolverStage, steps: u64, operation: impl FnOnce() -> Result<T, String>) {
    let control = SolverControl::default();
    let cancel = control.clone();
    let result = with_solver_observer(
        &control,
        move |point| {
            if point.stage == stage && point.completed_steps >= steps {
                cancel.request_cancel();
            }
        },
        operation,
    );
    assert!(result.is_err(), "ignored cancellation in {stage:?}");
    assert!(control.was_interrupted());
    let point = control.last_checkpoint().unwrap();
    assert_eq!(point.stage, stage);
    assert_eq!(point.completed_steps, steps);
}

fn matrix(n: usize) -> SparseMatrix {
    let mut matrix = SparseMatrix::with_uniform_row_capacity(n, 5);
    for row in 0..n {
        add_at(&mut matrix, row, row, 5.0);
        for offset in [1, 17] {
            if row + offset < n {
                add_at(&mut matrix, row, row + offset, -1.0);
                add_at(&mut matrix, row + offset, row, -1.0);
            }
        }
    }
    matrix
}

fn cancel_preconditioner(kind: SpdPreconditioner, stage: SolverStage) {
    let compressed = matrix(1100).compress(kind).unwrap();
    let rhs = vec![1.0; 1100];
    let mut output = vec![0.0; 1100];
    let mut scratch = output.clone();
    compressed
        .apply_preconditioner_into(kind, &rhs, &mut output, &mut scratch)
        .unwrap();
    let expected = output.clone();
    let expected_scratch = scratch.clone();
    let different_rhs: Vec<_> = (0..1100).map(|i| 2.0 + i as f64 / 1100.0).collect();
    interrupted(stage, 64, || {
        compressed.apply_preconditioner_into(kind, &different_rhs, &mut output, &mut scratch)
    });
    compressed
        .apply_preconditioner_into(kind, &rhs, &mut output, &mut scratch)
        .unwrap();
    assert_eq!(output, expected);
    assert_eq!(scratch, expected_scratch);
}

#[test]
fn jacobi_application_can_be_cancelled_inside_its_sweep() {
    cancel_preconditioner(SpdPreconditioner::Jacobi, SolverStage::PreconditionerJacobi);
}
#[test]
fn sgs_forward_application_can_be_cancelled() {
    cancel_preconditioner(
        SpdPreconditioner::SymmetricGaussSeidel,
        SolverStage::SgsForward,
    );
}
#[test]
fn sgs_backward_application_can_be_cancelled() {
    cancel_preconditioner(
        SpdPreconditioner::SymmetricGaussSeidel,
        SolverStage::SgsBackward,
    );
}
#[test]
fn ic0_forward_application_can_be_cancelled() {
    cancel_preconditioner(
        SpdPreconditioner::IncompleteCholesky,
        SolverStage::Ic0Forward,
    );
}
#[test]
fn ic0_backward_application_can_be_cancelled() {
    cancel_preconditioner(
        SpdPreconditioner::IncompleteCholesky,
        SolverStage::Ic0Backward,
    );
}

fn cancel_reduction(stage: SolverStage) {
    let matrix = matrix(256);
    let rhs = vec![1.0; 256];
    let fixed: Vec<_> = (0..128).collect();
    let prescribed: Vec<_> = fixed.iter().map(|&i| (i, 20.0)).collect();
    interrupted(stage, 64, || reduce_sparse_system(&matrix, &rhs, &fixed));
    interrupted(stage, 64, || {
        reduce_sparse_system_with_prescribed(&matrix, &rhs, &prescribed)
    });
}

#[test]
fn constraint_indexing_can_be_cancelled() {
    cancel_reduction(SolverStage::ConstraintIndex);
}
#[test]
fn free_dof_mapping_can_be_cancelled() {
    cancel_reduction(SolverStage::ConstraintMap);
}
#[test]
fn prescribed_and_homogeneous_reduction_can_be_cancelled() {
    cancel_reduction(SolverStage::ConstraintReduce);
}

#[test]
fn reduced_matrix_and_prescribed_rhs_match_an_independent_dense_reference() {
    let matrix = matrix(256);
    let force: Vec<_> = (0..256).map(|i| i as f64 / 256.0).collect();
    let prescribed: Vec<_> = (0..256)
        .filter(|i| i % 3 == 0)
        .map(|i| (i, 10.0 + i as f64))
        .collect();
    let fixed: Vec<_> = prescribed.iter().map(|&(i, _)| i).collect();
    for stage in [
        SolverStage::ConstraintIndex,
        SolverStage::ConstraintMap,
        SolverStage::ConstraintReduce,
    ] {
        interrupted(stage, 64, || {
            reduce_sparse_system_with_prescribed(&matrix, &force, &prescribed)
        });
    }
    for values in [
        prescribed.clone(),
        fixed.iter().map(|&i| (i, 0.0)).collect(),
    ] {
        let (reduced, rhs, free) =
            reduce_sparse_system_with_prescribed(&matrix, &force, &values).unwrap();
        let expected_free: Vec<_> = (0..256).filter(|i| !fixed.contains(i)).collect();
        assert_eq!(free, expected_free);
        let mut dense = vec![vec![0.0; 256]; 256];
        for row in 0..256 {
            for &(col, value) in matrix.row_entries(row) {
                dense[row][col] = value;
            }
        }
        for (row, &global) in free.iter().enumerate() {
            let expected_rhs = values.iter().fold(force[global], |sum, &(col, value)| {
                sum - dense[global][col] * value
            });
            assert!((rhs[row] - expected_rhs).abs() < 1e-12);
            for (col, &global_col) in free.iter().enumerate() {
                let actual = reduced
                    .row_entries(row)
                    .iter()
                    .find(|(c, _)| *c == col)
                    .map_or(0.0, |(_, v)| *v);
                assert_eq!(actual, dense[global][global_col]);
            }
        }
    }
    let (homogeneous, rhs, free) = reduce_sparse_system(&matrix, &force, &fixed).unwrap();
    let zero: Vec<_> = fixed.iter().map(|&i| (i, 0.0)).collect();
    let (reference, expected, expected_free) =
        reduce_sparse_system_with_prescribed(&matrix, &force, &zero).unwrap();
    assert_eq!(rhs, expected);
    assert_eq!(free, expected_free);
    for row in 0..free.len() {
        assert_eq!(homogeneous.row_entries(row), reference.row_entries(row));
    }
}

#[test]
fn short_and_all_constrained_systems_still_poll_at_boundaries() {
    let matrix = matrix(49);
    let rhs = vec![1.0; 49];
    for stage in [SolverStage::ConstraintMap, SolverStage::ConstraintReduce] {
        interrupted(stage, 49, || reduce_sparse_system(&matrix, &rhs, &[]));
    }
    let fixed: Vec<_> = (0..49).collect();
    interrupted(SolverStage::ConstraintReduce, 0, || {
        reduce_sparse_system(&matrix, &rhs, &fixed)
    });
    let (empty, force, free) = reduce_sparse_system(&matrix, &rhs, &fixed).unwrap();
    assert_eq!(empty.size(), 0);
    assert!(force.is_empty() && free.is_empty());
    for (kind, stage) in [
        (SpdPreconditioner::Jacobi, SolverStage::PreconditionerJacobi),
        (
            SpdPreconditioner::SymmetricGaussSeidel,
            SolverStage::SgsBackward,
        ),
        (
            SpdPreconditioner::IncompleteCholesky,
            SolverStage::Ic0Backward,
        ),
    ] {
        let compressed = matrix.compress(kind).unwrap();
        interrupted(stage, 49, || {
            compressed.apply_preconditioner_into(kind, &rhs, &mut [0.0; 49], &mut [0.0; 49])
        });
    }
}

#[test]
fn later_pcg_preconditioner_cancellation_does_not_fall_back_or_poison_a_new_solve() {
    use crate::SpdSolveOptions;
    use crate::linear_algebra::solve_spd_system_profile_with_options;
    for (kind, stage) in [
        (SpdPreconditioner::Jacobi, SolverStage::PreconditionerJacobi),
        (
            SpdPreconditioner::SymmetricGaussSeidel,
            SolverStage::SgsBackward,
        ),
        (
            SpdPreconditioner::IncompleteCholesky,
            SolverStage::Ic0Backward,
        ),
    ] {
        let matrix = matrix(1100);
        let rhs = vec![1.0; 1100];
        let options = SpdSolveOptions {
            preconditioner: kind,
            progress_interval: None,
        };
        let baseline =
            solve_spd_system_profile_with_options(&matrix, &rhs, options.clone()).unwrap();
        assert!(baseline.iterations > 1);
        let control = SolverControl::default();
        let cancel = control.clone();
        let entered = std::cell::Cell::new(0);
        let result = with_solver_observer(
            &control,
            move |point| {
                if point.stage == stage {
                    if point.completed_steps == 0 {
                        entered.set(entered.get() + 1);
                    }
                    if entered.get() == 2 && point.completed_steps == 64 {
                        cancel.request_cancel();
                    }
                }
            },
            || solve_spd_system_profile_with_options(&matrix, &rhs, options.clone()),
        );
        assert!(result.is_err());
        assert!(control.was_interrupted());
        assert_eq!(control.last_checkpoint().unwrap().stage, stage);
        assert_eq!(control.last_checkpoint().unwrap().completed_steps, 64);
        assert_eq!(
            solve_spd_system_profile_with_options(&matrix, &rhs, options)
                .unwrap()
                .solution,
            baseline.solution
        );
    }
}
