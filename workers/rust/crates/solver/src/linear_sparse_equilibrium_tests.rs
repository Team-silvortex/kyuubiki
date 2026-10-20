use super::*;
use crate::solver_control::{SolverControl, with_solver_observer};
use std::cell::Cell;

fn system(size: usize, strong: f64, weak: f64) -> (SparseMatrix, Vec<f64>) {
    let mut matrix = SparseMatrix::new(size);
    let mut rhs = vec![strong; size];
    for row in 0..size {
        add_at(
            &mut matrix,
            row,
            row,
            if row < 4 { 2.0 * weak } else { strong },
        );
    }
    add_at(&mut matrix, 0, 0, 2.0 * weak);
    for column in 1..4 {
        add_at(&mut matrix, 0, column, -weak);
        add_at(&mut matrix, column, 0, -weak);
    }
    rhs[..4].copy_from_slice(&[weak, 0.0, 0.0, 0.0]);
    (matrix, rhs)
}

fn check(solution: &[f64]) {
    for (index, &value) in solution.iter().enumerate() {
        let expected = match index {
            0 => 0.4,
            1..=3 => 0.2,
            _ => 1.0,
        };
        assert!(
            (value - expected).abs() < 1e-8,
            "row {index}: {value} != {expected}"
        );
    }
}

#[test]
fn finite_global_scale_cannot_mask_a_wrong_weak_equation() {
    let (matrix, rhs) = system(5, 1e20, 1e-20);
    let solution = vec![0.25, 0.0, 0.0, 0.0, 1.0];
    let validation = sparse_relative_residual(&matrix, &rhs, &solution, 1e-8).unwrap();
    assert!(validation.relative > 0.1, "{}", validation.relative);
    assert!(
        validate_spd_solution(
            &matrix,
            &rhs,
            SpdSolveProfile {
                solution,
                iterations: 1,
                matrix_non_zero_count: matrix.non_zero_count(),
                residual_norm: 0.0,
                stages: Vec::new(),
            }
        )
        .is_err()
    );
}

#[test]
fn iterative_solution_resolves_weak_equations_before_returning() {
    for preconditioner in [
        SpdPreconditioner::Jacobi,
        SpdPreconditioner::SymmetricGaussSeidel,
        SpdPreconditioner::IncompleteCholesky,
    ] {
        for (strong, weak) in [
            (1.0, 1.0),
            (1e20, 1e-20),
            (1e-20, 1e20),
            (1e100, 1e60),
            (1e-60, 1e-100),
        ] {
            let (matrix, rhs) = system(1025, strong, weak);
            let options = SpdSolveOptions {
                preconditioner,
                progress_interval: None,
            };
            let profile =
                solve_spd_system_profile_with_options(&matrix, &rhs, options.clone()).unwrap();
            assert!(
                (1..=128).contains(&profile.iterations),
                "{}",
                profile.iterations
            );
            check(&profile.solution);
            let prepared = PreparedSpdSolver::factor_with_options(matrix, options).unwrap();
            check(&prepared.solve(&rhs).unwrap());
            let reversed_rhs: Vec<_> = rhs.iter().map(|value| -value).collect();
            let reversed = prepared.solve(&reversed_rhs).unwrap();
            check(&reversed.iter().map(|value| -value).collect::<Vec<_>>());
        }
    }
}

#[test]
fn extreme_mixed_scale_pcg_resolves_weak_equations_without_underflowing_inner_products() {
    for preconditioner in [
        SpdPreconditioner::Jacobi,
        SpdPreconditioner::SymmetricGaussSeidel,
        SpdPreconditioner::IncompleteCholesky,
    ] {
        for (strong, weak) in [(1e300, 1e-300), (1e-300, 1e300), (1e250, 1e-250)] {
            let (matrix, rhs) = system(1025, strong, weak);
            let options = SpdSolveOptions {
                preconditioner,
                progress_interval: None,
            };
            let profile =
                solve_spd_system_profile_with_options(&matrix, &rhs, options.clone()).unwrap();
            assert!(
                (1..=128).contains(&profile.iterations),
                "{}",
                profile.iterations
            );
            check(&profile.solution);
            let prepared = PreparedSpdSolver::factor_with_options(matrix, options).unwrap();
            check(&prepared.solve(&rhs).unwrap());
            let reversed: Vec<_> = rhs.iter().map(|value| -value).collect();
            check(
                &prepared
                    .solve(&reversed)
                    .unwrap()
                    .iter()
                    .map(|value| -value)
                    .collect::<Vec<_>>(),
            );
            assert_eq!(prepared.solve(&vec![0.0; 1025]).unwrap(), vec![0.0; 1025]);
        }
    }
}

#[test]
fn genuinely_unrepresentable_mixed_scale_displacements_fail_and_replay_cleanly() {
    let (matrix, rhs) = system(1025, 1e300, 1e-300);
    let mut unrepresentable = rhs.clone();
    unrepresentable[0] = 1e300;
    assert!(solve_spd_system(&matrix, &unrepresentable).is_err());
    let prepared = PreparedSpdSolver::factor(matrix).unwrap();
    assert!(prepared.solve(&unrepresentable).is_err());
    check(&prepared.solve(&rhs).unwrap());
}

#[test]
fn equilibrium_candidate_cancellation_cannot_trigger_regularized_fallback() {
    let (matrix, rhs) = system(1025, 1e20, 1e-20);
    let prepared = PreparedSpdSolver::factor(matrix).unwrap();
    let iterations = Cell::new(0);
    let control = SolverControl::default();
    let cancel = control.clone();
    let result = with_solver_observer(
        &control,
        move |point| {
            if point.stage == SolverStage::SparseIteration {
                iterations.set(point.completed_steps);
            }
            if point.stage == SolverStage::ResidualValidate
                && point.completed_steps == 64
                && iterations.get() > 0
            {
                cancel.request_cancel();
            }
            assert_ne!(point.stage, SolverStage::SparseRegularizeCopy);
        },
        || prepared.solve(&rhs),
    );
    assert!(result.unwrap_err().contains("cancelled"));
    assert!(control.was_interrupted());
    check(&prepared.solve(&rhs).unwrap());
}

#[test]
fn compressed_equilibrium_gate_checks_wide_rows_and_replays_after_cancellation() {
    let size = 4097;
    let mut matrix = SparseMatrix::new(size);
    for row in 0..size {
        add_at(&mut matrix, 0, row, 1.0);
        if row > 0 {
            add_at(&mut matrix, row, row, 1.0);
        }
    }
    let compressed = matrix.compress(SpdPreconditioner::Jacobi).unwrap();
    let solution = vec![1.0; size];
    let mut rhs = solution.clone();
    rhs[0] = size as f64;
    for (stage, steps) in [
        (SolverStage::ResidualValidateRow, 1024),
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
            || compressed.satisfies_equilibrium(&rhs, 1.0, &solution),
        );
        assert!(result.unwrap_err().contains("cancelled"));
        assert!(control.was_interrupted());
    }
    assert!(
        compressed
            .satisfies_equilibrium(&rhs, 1.0, &solution)
            .unwrap()
    );
    let mut wrong = solution.clone();
    wrong[size - 1] = 2.0;
    assert!(!compressed.satisfies_equilibrium(&rhs, 1.0, &wrong).unwrap());
}

#[test]
fn compressed_equilibrium_rejects_invalid_inputs_and_erased_loads() {
    let (matrix, rhs) = system(5, 1.0, 1.0);
    let compressed = matrix.compress(SpdPreconditioner::Jacobi).unwrap();
    let solution = [0.4, 0.2, 0.2, 0.2, 1.0];
    for invalid in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(
            compressed
                .satisfies_equilibrium(&rhs, invalid, &solution)
                .is_err()
        );
    }
    assert!(
        compressed
            .satisfies_equilibrium(&rhs[..4], 1.0, &solution)
            .is_err()
    );
    assert!(
        compressed
            .satisfies_equilibrium(&rhs, 1.0, &solution[..4])
            .is_err()
    );
    for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let mut wrong = solution;
        wrong[0] = invalid;
        assert!(compressed.satisfies_equilibrium(&rhs, 1.0, &wrong).is_err());
    }
    let tiny_rhs = [f64::from_bits(1); 5];
    assert!(
        compressed
            .satisfies_equilibrium(&tiny_rhs, 2.0, &solution)
            .is_err()
    );
}

#[test]
fn subnormal_solution_quantization_is_local_and_cannot_hide_resolvable_loads() {
    let quantum = f64::from_bits(1);
    let mut matrix = SparseMatrix::new(2);
    add_at(&mut matrix, 0, 0, 1e8);
    add_at(&mut matrix, 0, 1, -1.0);
    add_at(&mut matrix, 1, 1, 1.0);
    let compressed = matrix.compress(SpdPreconditioner::Jacobi).unwrap();
    let solution = [0.0, quantum];
    let rhs = [0.0, quantum];
    assert_eq!(
        sparse_residual_norm(&matrix, &rhs, &solution).unwrap(),
        quantum
    );
    assert_eq!(
        sparse_relative_residual(&matrix, &rhs, &solution, 1e-8)
            .unwrap()
            .relative,
        0.0
    );
    assert!(
        compressed
            .satisfies_equilibrium(&rhs, 1.0, &solution)
            .unwrap()
    );

    // A resolvable load cannot be dismissed as displacement quantization.
    let loaded = [1e-100, quantum];
    assert!(
        sparse_relative_residual(&matrix, &loaded, &solution, 1e-8)
            .unwrap()
            .relative
            > 0.9
    );
    assert!(
        !compressed
            .satisfies_equilibrium(&loaded, 1.0, &solution)
            .unwrap()
    );
    let resolved = [0.0, 1e-100];
    assert!(
        sparse_relative_residual(&matrix, &resolved, &resolved, 1e-8)
            .unwrap()
            .relative
            > 0.9
    );
    assert!(
        !compressed
            .satisfies_equilibrium(&resolved, 1.0, &resolved)
            .unwrap()
    );
    let mut tiny_matrix = SparseMatrix::new(1);
    add_at(&mut tiny_matrix, 0, 0, quantum);
    assert_eq!(
        sparse_relative_residual(&tiny_matrix, &[quantum], &[0.0], 1e-8)
            .unwrap()
            .relative,
        1.0
    );
}

#[test]
fn range_fallback_keeps_the_same_subnormal_tail_rule_and_raw_residual() {
    let quantum = f64::from_bits(1);
    let unit = 2.0_f64.powi(1023);
    let mut matrix = SparseMatrix::new(3);
    for (row, column, coefficient) in [(0, 0, unit), (1, 1, 1e8), (1, 2, -1.0), (2, 2, 1.0)] {
        add_at(&mut matrix, row, column, coefficient);
    }
    let solution = [1.0, 0.0, quantum];
    let rhs = [unit, 0.0, quantum];
    assert_eq!(
        sparse_residual_norm(&matrix, &rhs, &solution).unwrap(),
        quantum
    );
    assert_eq!(
        sparse_relative_residual(&matrix, &rhs, &solution, 1e-8)
            .unwrap()
            .relative,
        0.0
    );
    let loaded = [unit, 1e-100, quantum];
    assert!(
        sparse_relative_residual(&matrix, &loaded, &solution, 1e-8)
            .unwrap()
            .relative
            > 0.9
    );
}
