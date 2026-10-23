use super::*;
use crate::linear_algebra::{
    PreparedSpdSolver, add_at, solve_spd_system_profile_with_options, validate_spd_solution,
};
use crate::linear_solver_profile::{SpdPreconditioner, SpdSolveOptions, SpdSolveProfile};
use crate::solver_control::{SolverControl, with_solver_observer};

fn diagonal(coefficient: f64) -> SparseMatrix {
    let mut matrix = SparseMatrix::new(1);
    add_at(&mut matrix, 0, 0, coefficient);
    matrix
}

#[test]
fn erased_load_plain_validation_cannot_spend_zero_response_roundoff() {
    let matrix = diagonal(4.0);
    for sign in [-1.0, 1.0] {
        let rhs = [sign * f64::from_bits(1)];
        let validation = sparse_relative_residual(&matrix, &rhs, &[0.0], 1e-8).unwrap();
        assert_eq!(validation.relative, 1.0);
        let profile = SpdSolveProfile {
            solution: vec![0.0],
            iterations: 0,
            matrix_non_zero_count: 1,
            residual_norm: 0.0,
            stages: Vec::new(),
        };
        assert!(validate_spd_solution(&matrix, &rhs, profile).is_err());
    }
}

#[test]
fn erased_load_scaled_validation_cannot_spend_zero_response_roundoff() {
    let matrix = diagonal(4.0);
    let quantum = f64::from_bits(1);
    let row = range::scaled_row(
        matrix.row_entries(0),
        quantum,
        &[0.0],
        SolverStage::ResidualValidateRow,
    )
    .unwrap();
    assert_eq!(row.resolved_relative(), 1.0);
}

#[test]
fn erased_load_compressed_candidate_cannot_accept_a_zero_response() {
    let matrix = diagonal(4.0);
    let compressed = matrix.compress(SpdPreconditioner::Jacobi).unwrap();
    assert!(
        !compressed
            .satisfies_equilibrium(&[f64::from_bits(1)], 1.0, &[0.0])
            .unwrap()
    );
}

fn branched_system() -> (SparseMatrix, Vec<f64>) {
    let size = 1025;
    let mut matrix = SparseMatrix::new(size);
    let mut rhs = vec![4.0; size];
    for index in 0..size {
        add_at(&mut matrix, index, index, 4.0);
    }
    for (offset, value) in rhs[1..4].iter_mut().enumerate() {
        let leaf = offset + 1;
        add_at(&mut matrix, 0, leaf, -1.0);
        add_at(&mut matrix, leaf, 0, -1.0);
        *value = 3.0;
    }
    rhs[0] = 1.0;
    (matrix, rhs)
}

#[test]
fn erased_load_iterative_and_prepared_solves_reject_missing_response_then_replay() {
    let (matrix, rhs) = branched_system();
    for preconditioner in [
        SpdPreconditioner::Jacobi,
        SpdPreconditioner::SymmetricGaussSeidel,
        SpdPreconditioner::IncompleteCholesky,
    ] {
        let options = SpdSolveOptions {
            preconditioner,
            progress_interval: None,
        };
        let prepared =
            PreparedSpdSolver::factor_with_options(matrix.clone(), options.clone()).unwrap();
        let mut bad = rhs.clone();
        bad[1024] = f64::from_bits(1);
        assert!(
            solve_spd_system_profile_with_options(&matrix, &bad, options.clone()).is_err(),
            "iterative solve accepted an erased independent load"
        );
        assert!(
            prepared.solve(&bad).is_err(),
            "prepared solve accepted an erased independent load"
        );
        for solution in [
            prepared.solve(&rhs).unwrap(),
            solve_spd_system_profile_with_options(&matrix, &rhs, options)
                .unwrap()
                .solution,
        ] {
            assert!(solution.iter().all(|value| (value - 1.0).abs() < 1e-8));
        }
    }
}

#[test]
fn erased_load_admission_is_row_local_in_plain_and_whole_matrix_range_paths() {
    let unit = 2.0_f64.powi(1023);
    let mut matrix = SparseMatrix::new(3);
    for (row, column, value) in [(0, 0, unit), (0, 1, -unit), (1, 1, 1.0), (2, 2, 4.0)] {
        add_at(&mut matrix, row, column, value);
    }
    let solution = [2.0, 2.0, 0.0];
    let quantum = f64::from_bits(1);
    let rhs = [0.0, 2.0, quantum];
    let validation = sparse_relative_residual(&matrix, &rhs, &solution, 1e-8).unwrap();
    assert_eq!(validation.row, 2);
    assert_eq!(validation.relative, 1.0);
    assert_eq!(validation.residual, quantum);
    assert_eq!(
        sparse_residual_norm(&matrix, &rhs, &solution).unwrap(),
        quantum
    );
    assert!(
        !matrix
            .compress(SpdPreconditioner::Jacobi)
            .unwrap()
            .satisfies_equilibrium(&rhs, 1.0, &solution)
            .unwrap()
    );
    let mut ordinary = SparseMatrix::new(2);
    add_at(&mut ordinary, 0, 0, 4.0);
    add_at(&mut ordinary, 1, 1, 1.0);
    let validation =
        sparse_relative_residual(&ordinary, &[quantum, 1.0], &[0.0, 1.0], 1e-8).unwrap();
    assert_eq!(validation.row, 0);
    assert_eq!(validation.relative, 1.0);
}

#[test]
fn erased_load_guard_keeps_zero_loads_and_valid_quantized_responses() {
    let quantum = f64::from_bits(1);
    for (coefficient, rhs, solution) in [
        (4.0, 0.0, 0.0),
        (4.0, -0.0, -0.0),
        (4.0, 4.0 * quantum, quantum),
        (4.0, -4.0 * quantum, -quantum),
        (3.0, 4.0 * quantum, quantum),
        (3.0, -4.0 * quantum, -quantum),
    ] {
        let matrix = diagonal(coefficient);
        assert_eq!(
            sparse_relative_residual(&matrix, &[rhs], &[solution], 1e-8)
                .unwrap()
                .relative,
            0.0
        );
        assert_eq!(
            range::scaled_row(
                matrix.row_entries(0),
                rhs,
                &[solution],
                SolverStage::ResidualValidateRow
            )
            .unwrap()
            .resolved_relative(),
            0.0
        );
        assert!(
            matrix
                .compress(SpdPreconditioner::Jacobi)
                .unwrap()
                .satisfies_equilibrium(&[rhs], 1.0, &[solution])
                .unwrap()
        );
    }
    let matrix = diagonal(3.0);
    assert!(
        sparse_relative_residual(&matrix, &[8.0 * quantum], &[quantum], 1e-8)
            .unwrap()
            .relative
            > 0.1
    );
    assert!(
        !matrix
            .compress(SpdPreconditioner::Jacobi)
            .unwrap()
            .satisfies_equilibrium(&[8.0 * quantum], 1.0, &[quantum])
            .unwrap()
    );
}

#[test]
fn erased_load_guard_distinguishes_cancelling_terms_from_absent_response() {
    let quantum = f64::from_bits(1);
    let mut matrix = SparseMatrix::new(2);
    for (row, column, value) in [(0, 0, 2.0), (0, 1, 2.0), (1, 1, 1.0)] {
        add_at(&mut matrix, row, column, value);
    }
    let solution = [quantum, -quantum];
    let rhs = [quantum, -quantum];
    assert_eq!(
        sparse_residual_vector(&matrix, &rhs, &solution).unwrap(),
        [quantum, 0.0]
    );
    assert_eq!(
        sparse_relative_residual(&matrix, &rhs, &solution, 1e-8)
            .unwrap()
            .relative,
        0.0
    );
    assert!(
        matrix
            .compress(SpdPreconditioner::Jacobi)
            .unwrap()
            .satisfies_equilibrium(&rhs, 1.0, &solution)
            .unwrap()
    );
}

#[test]
fn erased_load_candidate_guard_uses_the_expected_rhs_scale_and_keeps_normalization_errors() {
    let compressed = diagonal(4.0).compress(SpdPreconditioner::Jacobi).unwrap();
    let quantum = f64::from_bits(1);
    for scale in [1.0, 2.0, 4.0] {
        assert!(
            !compressed
                .satisfies_equilibrium(&[4.0 * quantum], scale, &[0.0])
                .unwrap()
        );
    }
    assert!(
        compressed
            .satisfies_equilibrium(&[quantum], 2.0, &[0.0])
            .is_err()
    );
}

fn wide_case(range_path: bool) -> (SparseMatrix, Vec<f64>, Vec<f64>) {
    let size = 2049;
    let mut matrix = SparseMatrix::new(size);
    let mut rhs = vec![0.0; size];
    let mut solution = vec![0.0; size];
    for column in 0..size {
        let coefficient = if range_path && column < 2 {
            if column == 0 {
                2.0_f64.powi(1023)
            } else {
                -2.0_f64.powi(1023)
            }
        } else {
            4.0
        };
        add_at(&mut matrix, 0, column, coefficient);
        if column > 0 {
            add_at(&mut matrix, column, column, 4.0);
        }
    }
    if range_path {
        solution[0] = 2.0;
        solution[1] = 2.0;
        rhs[1] = 8.0;
        rhs[size - 1] = f64::from_bits(1);
    } else {
        rhs[0] = f64::from_bits(1);
    }
    (matrix, rhs, solution)
}

#[test]
fn erased_load_validation_cancels_wide_plain_and_range_passes_then_replays() {
    for range_path in [false, true] {
        let (matrix, rhs, solution) = wide_case(range_path);
        let compressed = matrix.compress(SpdPreconditioner::Jacobi).unwrap();
        let range_steps = if range_path { 3073 } else { 1024 };
        for compressed_path in [false, true] {
            for (stage, steps) in [
                (SolverStage::ResidualValidateRow, range_steps),
                (SolverStage::ResidualValidate, 64),
            ] {
                let control = SolverControl::default();
                let cancel = control.clone();
                let result = with_solver_observer(
                    &control,
                    move |point| {
                        if point.stage == stage && point.completed_steps == steps {
                            cancel.request_cancel();
                        }
                    },
                    || {
                        if compressed_path {
                            compressed.satisfies_equilibrium(&rhs, 1.0, &solution)
                        } else {
                            sparse_relative_residual(&matrix, &rhs, &solution, 1e-8)
                                .map(|value| value.relative <= 1e-8)
                        }
                    },
                );
                assert!(result.unwrap_err().contains("cancelled"));
                assert!(control.was_interrupted());
                assert!(
                    !compressed
                        .satisfies_equilibrium(&rhs, 1.0, &solution)
                        .unwrap()
                );
                assert_eq!(
                    sparse_relative_residual(&matrix, &rhs, &solution, 1e-8)
                        .unwrap()
                        .relative,
                    1.0
                );
            }
        }
    }
}
