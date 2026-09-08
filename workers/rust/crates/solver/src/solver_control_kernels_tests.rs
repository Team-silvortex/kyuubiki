use crate::chain_tridiagonal::PreparedTridiagonal;
use crate::linear_algebra::{PreparedSpdSolver, SparseMatrix, add_at, solve_spd_system};
use crate::linear_banded::SymmetricBandCholesky;
use crate::linear_dense::DenseLu;
use crate::solver_control::{SolverControl, SolverStage, with_solver_observer};

fn cancel_at<T>(stage: SolverStage, step: u64, solve: impl FnOnce() -> Result<T, String>) {
    let control = SolverControl::default();
    let cancel = control.clone();
    let result = with_solver_observer(
        &control,
        move |point| {
            if point.stage == stage && point.completed_steps >= step {
                cancel.request_cancel();
            }
        },
        solve,
    );
    assert!(
        result.is_err(),
        "numerical work ignored cancellation at {stage:?}"
    );
    assert!(
        control.was_interrupted(),
        "error was not a cooperative interruption"
    );
    let point = control.last_checkpoint().unwrap();
    assert_eq!(point.stage, stage);
    assert!(point.completed_steps >= step);
}

fn sparse_fixture(size: usize) -> SparseMatrix {
    let mut matrix = SparseMatrix::with_uniform_row_capacity(size, 5);
    for row in 0..size {
        add_at(&mut matrix, row, row, 5.0);
        for offset in [1, 17] {
            if row + offset < size {
                add_at(&mut matrix, row, row + offset, -1.0);
                add_at(&mut matrix, row + offset, row, -1.0);
            }
        }
    }
    matrix
}

#[test]
fn dense_factor_stops_after_completed_pivots() {
    let matrix = vec![
        vec![4.0, 1.0, 0.5],
        vec![1.0, 3.0, 0.25],
        vec![0.5, 0.25, 2.0],
    ];
    cancel_at(SolverStage::DenseFactor, 2, || {
        DenseLu::factor(matrix.clone())
    });
    assert!(
        DenseLu::factor(matrix)
            .unwrap()
            .solve(&[1.0, 2.0, 3.0])
            .is_ok()
    );
}

#[test]
fn sparse_cancellation_does_not_fall_through_regularization() {
    let matrix = sparse_fixture(1100);
    let rhs = vec![1.0; 1100];
    cancel_at(SolverStage::SparseIteration, 1, || {
        solve_spd_system(&matrix, &rhs)
    });
    assert!(solve_spd_system(&matrix, &rhs).is_ok());
}

#[test]
fn prepared_sparse_cancellation_preserves_the_reusable_matrix() {
    let prepared = PreparedSpdSolver::factor(sparse_fixture(1100)).unwrap();
    let rhs = vec![1.0; 1100];
    let before = prepared.solve(&rhs).unwrap();
    cancel_at(SolverStage::SparseIteration, 1, || prepared.solve(&rhs));
    assert_eq!(prepared.solve(&rhs).unwrap(), before);
}

#[test]
fn banded_factor_can_stop_after_completed_rows() {
    cancel_at(SolverStage::BandedFactor, 2, || {
        SymmetricBandCholesky::try_factor(&sparse_fixture(24), 1000)
    });
}

#[test]
fn tridiagonal_factor_can_stop_before_finishing_the_chain() {
    let diagonal = vec![4.0; 600];
    let off_diagonal = vec![-1.0; 599];
    cancel_at(SolverStage::TridiagonalFactor, 256, || {
        PreparedTridiagonal::factor(&diagonal, &off_diagonal, &off_diagonal)
    });
}

#[test]
fn reusable_factor_substitutions_stop_without_mutating_the_factors() {
    let dense = DenseLu::factor(vec![
        vec![4.0, 1.0, 0.5],
        vec![1.0, 3.0, 0.25],
        vec![0.5, 0.25, 2.0],
    ])
    .unwrap();
    let before = dense.solve(&[1.0, 2.0, 3.0]).unwrap();
    cancel_at(SolverStage::DenseSubstitution, 2, || {
        dense.solve(&[1.0, 2.0, 3.0])
    });
    assert_eq!(dense.solve(&[1.0, 2.0, 3.0]).unwrap(), before);

    let banded = SymmetricBandCholesky::try_factor(&sparse_fixture(24), 1000)
        .unwrap()
        .unwrap();
    let rhs = vec![1.0; 24];
    let before = banded.solve(&rhs).unwrap();
    cancel_at(SolverStage::BandedSubstitution, 2, || banded.solve(&rhs));
    assert_eq!(banded.solve(&rhs).unwrap(), before);

    let tridiagonal =
        PreparedTridiagonal::factor(&vec![4.0; 600], &vec![-1.0; 599], &vec![-1.0; 599]).unwrap();
    let rhs = vec![1.0; 600];
    let before = tridiagonal.solve(&rhs).unwrap();
    cancel_at(SolverStage::TridiagonalSubstitution, 256, || {
        tridiagonal.solve(&rhs)
    });
    assert_eq!(tridiagonal.solve(&rhs).unwrap(), before);
}
