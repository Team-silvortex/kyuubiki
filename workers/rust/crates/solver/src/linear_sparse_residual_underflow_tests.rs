use super::*;
use crate::linear_algebra::{add_at, validate_spd_solution};
use crate::linear_solver_profile::{SpdPreconditioner, SpdSolveProfile};
use crate::solver_control::{SolverControl, with_solver_observer};

fn diagonal(coefficient: f64) -> SparseMatrix {
    let mut matrix = SparseMatrix::new(1);
    add_at(&mut matrix, 0, 0, coefficient);
    matrix
}

#[test]
fn underflowed_products_cannot_certify_a_nonzero_response_to_zero_load() {
    let matrix = diagonal(1e-200);
    let solution = [1e-200];
    assert!(sparse_relative_residual(&matrix, &[0.0], &solution, 1e-8).is_err());
    assert!(sparse_residual_vector(&matrix, &[0.0], &solution).is_err());
    assert!(sparse_residual_norm(&matrix, &[0.0], &solution).is_err());
}

#[test]
fn partially_rounded_subnormal_products_cannot_erase_equilibrium_error() {
    let quantum = f64::from_bits(1);
    let matrix = diagonal(quantum);
    for value in [0.75, 1.25] {
        let profile = SpdSolveProfile {
            solution: vec![value],
            iterations: 0,
            matrix_non_zero_count: 1,
            residual_norm: 0.0,
            stages: Vec::new(),
        };
        assert!(
            validate_spd_solution(&matrix, &[quantum], profile).is_err(),
            "wrong displacement {value} was accepted"
        );
    }
}

#[test]
fn compressed_candidate_checks_product_underflow_before_acceptance() {
    for (coefficient, rhs, solution) in [
        (1e-200, 0.0, 1e-200),
        (f64::from_bits(1), f64::from_bits(1), 1.25),
    ] {
        let matrix = diagonal(coefficient);
        let compressed = matrix.compress(SpdPreconditioner::Jacobi).unwrap();
        assert!(
            !matches!(
                compressed.satisfies_equilibrium(&[rhs], 1.0, &[solution]),
                Ok(true)
            ),
            "wrong displacement {solution} was accepted"
        );
    }
}

#[test]
fn exact_subnormal_equations_still_pass_with_signed_loads() {
    for coefficient in [f64::from_bits(1), f64::MIN_POSITIVE] {
        let matrix = diagonal(coefficient);
        let compressed = matrix.compress(SpdPreconditioner::Jacobi).unwrap();
        for displacement in [-2.0, -1.0, 0.0, 1.0, 2.0] {
            let rhs = [coefficient * displacement];
            let solution = [displacement];
            assert_eq!(
                sparse_relative_residual(&matrix, &rhs, &solution, 1e-8)
                    .unwrap()
                    .relative,
                0.0
            );
            assert_eq!(
                sparse_residual_vector(&matrix, &rhs, &solution).unwrap(),
                [0.0]
            );
            assert!(
                compressed
                    .satisfies_equilibrium(&rhs, 1.0, &solution)
                    .unwrap()
            );
        }
    }
}

#[test]
fn exact_cancellation_of_unrepresentable_products_does_not_fail() {
    let mut matrix = SparseMatrix::new(2);
    add_at(&mut matrix, 0, 0, f64::from_bits(1));
    add_at(&mut matrix, 0, 1, -f64::from_bits(1));
    add_at(&mut matrix, 1, 1, 1.0);
    let solution = [0.25, 0.25];
    let rhs = [0.0, 0.25];
    assert_eq!(
        sparse_residual_vector(&matrix, &rhs, &solution).unwrap(),
        [0.0; 2]
    );
    assert_eq!(sparse_residual_norm(&matrix, &rhs, &solution).unwrap(), 0.0);
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

fn accumulated_system(size: usize) -> (SparseMatrix, Vec<f64>, Vec<f64>) {
    let mut matrix = SparseMatrix::new(size);
    let solution = vec![1.0 / size as f64; size];
    let mut rhs = solution.clone();
    rhs[0] = 0.0;
    for column in 0..size {
        add_at(&mut matrix, 0, column, f64::from_bits(1));
        if column > 0 {
            add_at(&mut matrix, column, column, 1.0);
        }
    }
    (matrix, rhs, solution)
}

#[test]
fn individually_erased_products_recover_a_representable_signed_residual() {
    let (matrix, rhs, solution) = accumulated_system(4096);
    let compressed = matrix.compress(SpdPreconditioner::Jacobi).unwrap();
    for sign in [1.0, -1.0] {
        let solution: Vec<_> = solution.iter().map(|value| sign * value).collect();
        let rhs: Vec<_> = rhs.iter().map(|value| sign * value).collect();
        let residual = sparse_residual_vector(&matrix, &rhs, &solution).unwrap();
        assert_eq!(residual[0], -sign * f64::from_bits(1));
        assert!(residual[1..].iter().all(|value| *value == 0.0));
        assert_eq!(
            sparse_residual_norm(&matrix, &rhs, &solution).unwrap(),
            f64::from_bits(1)
        );
        let validation = sparse_relative_residual(&matrix, &rhs, &solution, 1e-8).unwrap();
        assert_eq!(validation.row, 0);
        assert_eq!(validation.relative, 1.0);
        assert_eq!(validation.residual, f64::from_bits(1));
        assert!(
            !compressed
                .satisfies_equilibrium(&rhs, 1.0, &solution)
                .unwrap()
        );
    }
}

#[test]
fn underflow_range_passes_are_cancellable_and_replay_cleanly() {
    let (matrix, rhs, solution) = accumulated_system(4096);
    let compressed = matrix.compress(SpdPreconditioner::Jacobi).unwrap();
    for compressed_path in [false, true] {
        let control = SolverControl::default();
        let cancel = control.clone();
        let result = with_solver_observer(
            &control,
            move |point| {
                // This checkpoint occurs only in the range pass, not the direct row pass.
                if point.stage == SolverStage::ResidualValidateRow && point.completed_steps == 5120
                {
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
    }
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
