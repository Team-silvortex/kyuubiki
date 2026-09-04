use crate::linear_algebra::{
    SparseMatrix, add_at, solve_spd_system, solve_spd_system_profile_with_options,
    sparse_residual_norm,
};
use crate::linear_dense::{DenseLu, solve_linear_system};
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
fn dense_solver_preserves_uniformly_tiny_physics_scales() {
    let solution = solve_linear_system(
        vec![vec![2.0e-24, -1.0e-24], vec![-1.0e-24, 2.0e-24]],
        vec![1.0e-24, 0.0],
    )
    .expect("a uniformly tiny nonsingular system should solve");

    assert!((solution[0] - 2.0 / 3.0).abs() < 1.0e-12);
    assert!((solution[1] - 1.0 / 3.0).abs() < 1.0e-12);
}

#[test]
fn dense_solver_uses_row_scaled_pivot_selection() {
    let solution = solve_linear_system(
        vec![vec![1.0e-20, 0.0], vec![0.0, 1.0e20]],
        vec![2.0e-20, 3.0e20],
    )
    .expect("mixed but independent physical scales should solve");

    assert!((solution[0] - 2.0).abs() < 1.0e-12);
    assert!((solution[1] - 3.0).abs() < 1.0e-12);
}

#[test]
fn dense_solver_rejects_scale_relative_rank_loss() {
    assert_err_contains(
        solve_linear_system(
            vec![vec![1.0, 1.0], vec![1.0, 1.0 + f64::EPSILON]],
            vec![2.0, 2.0 + f64::EPSILON],
        ),
        "system is singular",
    );
}

#[test]
fn dense_factor_reuses_scaled_pivots_for_multiple_right_hand_sides() {
    let factor = DenseLu::factor(vec![vec![0.0, 2.0], vec![1.0, 3.0]])
        .expect("pivoted dense matrix should factor");

    let first = factor.solve(&[4.0, 7.0]).expect("first RHS should solve");
    let second = factor.solve(&[8.0, 11.0]).expect("second RHS should solve");
    assert!((first[0] - 1.0).abs() < 1.0e-12);
    assert!((first[1] - 2.0).abs() < 1.0e-12);
    assert!((second[0] + 1.0).abs() < 1.0e-12);
    assert!((second[1] - 4.0).abs() < 1.0e-12);
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
fn dense_spd_profile_reports_recomputed_residual() {
    let mut matrix = SparseMatrix::new(2);
    for (row, column, value) in [(0, 0, 0.1), (0, 1, 0.03), (1, 0, 0.03), (1, 1, 0.2)] {
        add_at(&mut matrix, row, column, value);
    }
    let rhs = [0.7, 0.11];
    let profile = solve_spd_system_profile_with_options(&matrix, &rhs, SpdSolveOptions::default())
        .expect("dense SPD system should solve");
    let residual = [
        rhs[0] - 0.1 * profile.solution[0] - 0.03 * profile.solution[1],
        rhs[1] - 0.03 * profile.solution[0] - 0.2 * profile.solution[1],
    ]
    .iter()
    .map(|value| value * value)
    .sum::<f64>()
    .sqrt();

    assert!((profile.residual_norm - residual).abs() <= f64::EPSILON);
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
    let recomputed_residual = profile
        .solution
        .iter()
        .map(|value| (2.0 - 2.0 * value).powi(2))
        .sum::<f64>()
        .sqrt();
    assert!((profile.residual_norm - recomputed_residual).abs() < 1.0e-12);
    assert!(profile.residual_norm < 1.0e-10);
}

#[test]
fn sparse_residual_norm_remains_finite_across_extreme_scales() {
    let mut matrix = SparseMatrix::new(2);
    add_at(&mut matrix, 0, 0, 1.0e200);
    add_at(&mut matrix, 1, 1, 1.0e200);

    let large = sparse_residual_norm(&matrix, &[0.0, 0.0], &[1.0e100, 1.0e100]);
    assert!(large.is_finite());
    assert!((large / 1.0e300 - std::f64::consts::SQRT_2).abs() < 1.0e-12);

    let tiny = sparse_residual_norm(&matrix, &[1.0e-300, 1.0e-300], &[0.0, 0.0]);
    assert!(tiny > 0.0);
    assert!((tiny / 1.0e-300 - std::f64::consts::SQRT_2).abs() < 1.0e-12);
}

#[test]
fn sparse_spd_iterative_path_preserves_extreme_rhs_scale() {
    let size = 1025;
    let mut matrix = SparseMatrix::new(size);
    let rhs = vec![2.0e-200; size];
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
    .expect("iterative solve should be invariant to RHS magnitude");

    assert_eq!(profile.iterations, 1);
    assert!(
        profile
            .solution
            .iter()
            .all(|value| (*value - 1.0e-200).abs() <= 1.0e-212)
    );
    assert!(profile.residual_norm.is_finite());
    assert!(profile.residual_norm < 1.0e-210);
}

#[test]
fn sparse_spd_iterative_path_preserves_extreme_matrix_scale() {
    let size = 1025;
    let mut matrix = SparseMatrix::new(size);
    let rhs = vec![2.0e-200; size];
    for index in 0..size {
        add_at(&mut matrix, index, index, 2.0e-200);
    }

    let profile = solve_spd_system_profile_with_options(
        &matrix,
        &rhs,
        SpdSolveOptions {
            preconditioner: SpdPreconditioner::Jacobi,
            progress_interval: None,
        },
    )
    .expect("iterative solve should preserve uniformly tiny matrix coefficients");

    assert_eq!(profile.iterations, 1);
    assert!(
        profile
            .solution
            .iter()
            .all(|value| (*value - 1.0).abs() < 1.0e-12)
    );
    assert!(profile.residual_norm.is_finite());
    assert!(profile.residual_norm < 1.0e-210);
}

#[test]
fn sparse_spd_rejects_regularized_solution_that_misses_original_equilibrium() {
    let size = 1025;
    let mut matrix = SparseMatrix::new(size);
    let mut rhs = vec![0.0; size];
    for index in 0..size - 1 {
        add_at(&mut matrix, index, index, 1.0);
    }
    rhs[size - 1] = 1.0;

    assert_err_contains(
        solve_spd_system_profile_with_options(
            &matrix,
            &rhs,
            SpdSolveOptions {
                preconditioner: SpdPreconditioner::Jacobi,
                progress_interval: None,
            },
        ),
        "residual validation",
    );
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
