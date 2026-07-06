use crate::linear_algebra::{SparseMatrix, add_at, solve_spd_system};
use crate::linear_dense::solve_linear_system;

fn assert_err_contains<T: std::fmt::Debug>(result: Result<T, String>, expected: &str) {
    let error = result.expect_err("solver should reject invalid numeric input");
    assert!(
        error.contains(expected),
        "expected error to contain {expected:?}, got {error:?}",
    );
}

#[test]
fn dense_solver_rejects_non_finite_matrix_before_pivoting() {
    assert_err_contains(
        solve_linear_system(vec![vec![f64::NAN, 0.0], vec![0.0, 1.0]], vec![1.0, 2.0]),
        "linear system matrix contains non-finite value",
    );
}

#[test]
fn dense_solver_rejects_non_finite_rhs_before_pivoting() {
    assert_err_contains(
        solve_linear_system(
            vec![vec![1.0, 0.0], vec![0.0, 1.0]],
            vec![1.0, f64::INFINITY],
        ),
        "linear system vector contains non-finite value",
    );
}

#[test]
fn sparse_spd_solver_rejects_non_finite_matrix_before_solving() {
    let mut matrix = SparseMatrix::new(2);
    add_at(&mut matrix, 0, 0, 1.0);
    add_at(&mut matrix, 1, 1, f64::NAN);

    assert_err_contains(
        solve_spd_system(&matrix, &[1.0, 2.0]),
        "linear system matrix contains non-finite value",
    );
}

#[test]
fn sparse_spd_solver_rejects_non_finite_rhs_before_solving() {
    let mut matrix = SparseMatrix::new(2);
    add_at(&mut matrix, 0, 0, 1.0);
    add_at(&mut matrix, 1, 1, 1.0);

    assert_err_contains(
        solve_spd_system(&matrix, &[1.0, f64::NEG_INFINITY]),
        "linear system vector contains non-finite value",
    );
}
