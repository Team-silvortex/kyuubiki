use crate::linear_algebra::{
    SparseMatrix, add_at, solve_spd_system, solve_spd_system_profile_with_options,
};
use crate::linear_dense::solve_linear_system;
use crate::linear_solver_profile::{SpdPreconditioner, SpdSolveOptions};

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

#[test]
fn sparse_spd_profile_exposes_iterative_hotspots() {
    let size = 1025;
    let mut matrix = SparseMatrix::new(size);
    let rhs = vec![2.0; size];
    for index in 0..size {
        add_at(&mut matrix, index, index, 2.0);
    }

    let profile = solve_spd_system_profile_with_options(
        &matrix,
        &rhs,
        SpdSolveOptions {
            preconditioner: SpdPreconditioner::Jacobi,
            progress_interval: None,
        },
    )
    .expect("diagonal SPD system should solve");
    let labels = profile
        .stages
        .iter()
        .map(|stage| stage.label)
        .collect::<Vec<_>>();

    assert!(labels.contains(&"solve_spd_matvec"));
    assert!(labels.contains(&"solve_spd_preconditioner"));
    assert!(labels.contains(&"solve_spd_preconditioner_setup"));
    assert!(labels.contains(&"solve_spd_vector_update"));
    assert!(labels.contains(&"solve_spd_dot"));
}

#[test]
fn incomplete_cholesky_solves_block_diagonal_spd_in_one_iteration() {
    let size = 1026;
    let mut matrix = SparseMatrix::new(size);
    let mut rhs = vec![0.0; size];
    for first in (0..size).step_by(2) {
        let second = first + 1;
        add_at(&mut matrix, first, first, 4.0);
        add_at(&mut matrix, first, second, 1.0);
        add_at(&mut matrix, second, first, 1.0);
        add_at(&mut matrix, second, second, 3.0);
        rhs[first] = 1.0;
        rhs[second] = 2.0;
    }

    let profile = solve_spd_system_profile_with_options(
        &matrix,
        &rhs,
        SpdSolveOptions {
            preconditioner: SpdPreconditioner::IncompleteCholesky,
            progress_interval: None,
        },
    )
    .expect("IC(0) should solve an SPD block diagonal system");

    assert_eq!(profile.iterations, 1);
    assert!((profile.solution[0] - (1.0 / 11.0)).abs() < 1.0e-12);
    assert!((profile.solution[1] - (7.0 / 11.0)).abs() < 1.0e-12);
}
