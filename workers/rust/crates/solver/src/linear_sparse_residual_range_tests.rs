use super::*;
use crate::linear_algebra::{PreparedSpdSolver, add_at, validate_spd_solution};
use crate::linear_solver_profile::SpdSolveProfile;
use crate::solver_control::{SolverControl, with_solver_observer};
use std::cell::Cell;

fn diagonal(values: &[f64]) -> SparseMatrix {
    let mut matrix = SparseMatrix::new(values.len());
    for (index, &value) in values.iter().enumerate() {
        add_at(&mut matrix, index, index, value);
    }
    matrix
}

fn profile(solution: Vec<f64>) -> SpdSolveProfile {
    SpdSolveProfile {
        solution,
        iterations: 0,
        matrix_non_zero_count: 0,
        residual_norm: 0.0,
        stages: Vec::new(),
    }
}

#[test]
fn finite_wrong_solution_cannot_pass_with_an_overflowing_error_denominator() {
    let unit = 2.0_f64.powi(1023);
    let matrix = diagonal(&[unit]);
    let validation = sparse_relative_residual(&matrix, &[1.5 * unit], &[1.0], 1e-8).unwrap();
    assert!(
        (validation.relative - 0.2).abs() < 1e-14,
        "{}",
        validation.relative
    );
    assert!(validate_spd_solution(&matrix, &[1.5 * unit], profile(vec![1.0])).is_err());
}

#[test]
fn overflowing_large_equation_does_not_hide_a_wrong_small_equation() {
    let unit = 2.0_f64.powi(1023);
    let matrix = diagonal(&[unit, 1.0]);
    let validation = sparse_relative_residual(&matrix, &[unit, 1.0], &[1.0, 2.0], 1e-8).unwrap();
    assert_eq!(validation.row, 1);
    assert!((validation.relative - 1.0 / 3.0).abs() < 1e-14);
    assert!(validate_spd_solution(&matrix, &[unit, 1.0], profile(vec![1.0, 2.0])).is_err());
}

#[test]
fn cancelling_overflowing_products_preserve_finite_residuals() {
    let unit = 2.0_f64.powi(1023);
    let mut matrix = diagonal(&[unit; 2]);
    add_at(&mut matrix, 0, 1, -0.5 * unit);
    add_at(&mut matrix, 1, 0, -0.5 * unit);
    let rhs = [1.5 * unit; 2];
    assert_eq!(
        sparse_residual_vector(&matrix, &rhs, &[3.0; 2]).unwrap(),
        vec![0.0; 2]
    );
    assert_eq!(sparse_residual_norm(&matrix, &rhs, &[3.0; 2]).unwrap(), 0.0);
    assert!(validate_spd_solution(&matrix, &rhs, profile(vec![3.0; 2])).is_ok());
    let prepared = PreparedSpdSolver::factor(matrix).unwrap();
    assert_eq!(prepared.solve(&rhs).unwrap(), vec![3.0; 2]);
    assert!(prepared.solve(&[f64::NAN; 2]).is_err());
    assert_eq!(prepared.solve(&[0.75 * unit; 2]).unwrap(), vec![1.5; 2]);
    assert_eq!(prepared.solve(&rhs).unwrap(), vec![3.0; 2]);
}

#[test]
fn overflow_cancellation_retains_the_product_rounding_tail() {
    let unit = 2.0_f64.powi(1023);
    let delta = 2.0_f64.powi(-27);
    let mut matrix = SparseMatrix::new(2);
    add_at(&mut matrix, 0, 0, (1.0 + delta) * unit);
    add_at(&mut matrix, 0, 1, -unit);
    let solution = [2.0 * (1.0 - delta), 2.0];
    // (1+d)(1-d) = 1-d^2. Rounded product heads cancel, but the tail does not.
    let residual = 2.0_f64.powi(970);
    assert_eq!(
        sparse_residual_vector(&matrix, &[0.0; 2], &solution).unwrap(),
        vec![residual, 0.0]
    );
    assert_eq!(
        sparse_residual_norm(&matrix, &[0.0; 2], &solution).unwrap(),
        residual
    );
    let validation = sparse_relative_residual(&matrix, &[0.0; 2], &solution, 1e-8).unwrap();
    assert_eq!(validation.relative, 2.0_f64.powi(-55));
}

#[test]
fn sparse_validation_does_not_silently_zip_away_missing_equations() {
    let matrix = diagonal(&[1.0; 2]);
    for (rhs, solution) in [
        (vec![1.0], vec![1.0; 2]),
        (vec![1.0; 3], vec![1.0; 2]),
        (vec![1.0; 2], vec![1.0]),
        (vec![1.0; 2], vec![1.0; 3]),
    ] {
        assert!(sparse_relative_residual(&matrix, &rhs, &solution, 1e-8).is_err());
        assert!(sparse_residual_norm(&matrix, &rhs, &solution).is_err());
        assert!(sparse_residual_vector(&matrix, &rhs, &solution).is_err());
    }
}

#[test]
fn overflow_fallback_restores_signed_finite_residuals_not_only_relative_errors() {
    let unit = 2.0_f64.powi(1023);
    let mut matrix = diagonal(&[unit; 2]);
    add_at(&mut matrix, 0, 1, -0.5 * unit);
    add_at(&mut matrix, 1, 0, -0.5 * unit);
    let rhs = [1.25 * unit, 1.5 * unit];
    assert_eq!(
        sparse_residual_vector(&matrix, &rhs, &[3.0; 2]).unwrap(),
        vec![-0.25 * unit, 0.0]
    );
    assert_eq!(
        sparse_residual_norm(&matrix, &rhs, &[3.0; 2]).unwrap(),
        0.25 * unit
    );
    let validation = sparse_relative_residual(&matrix, &rhs, &[3.0; 2], 1e-8).unwrap();
    assert!((validation.relative - 1.0 / 23.0).abs() < 1e-14);
    assert!(validate_spd_solution(&matrix, &rhs, profile(vec![3.0; 2])).is_err());
}

#[test]
fn nonfinite_solution_cannot_hide_in_an_empty_equation() {
    let matrix = diagonal(&[1.0, 0.0]);
    for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(validate_spd_solution(&matrix, &[1.0, 0.0], profile(vec![1.0, invalid])).is_err());
    }
}

#[test]
fn complete_solution_scan_can_be_cancelled_before_residual_work() {
    let matrix = diagonal(&vec![1.0; 4097]);
    let rhs = vec![1.0; matrix.size()];
    let control = SolverControl::default();
    let cancel = control.clone();
    let result = with_solver_observer(
        &control,
        move |point| {
            if point.stage == SolverStage::ResidualValidate && point.completed_steps > 0 {
                cancel.request_cancel();
            }
            assert_ne!(point.stage, SolverStage::SparseResidual);
        },
        || validate_spd_solution(&matrix, &rhs, profile(rhs.clone())),
    );
    assert!(result.unwrap_err().contains("cancelled"));
    assert!(control.was_interrupted());
    assert!(validate_spd_solution(&matrix, &rhs, profile(rhs.clone())).is_ok());
}

#[test]
fn invalid_operands_tolerance_and_matrix_columns_fail_explicitly() {
    for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let matrix = diagonal(&[invalid]);
        assert!(sparse_relative_residual(&matrix, &[1.0], &[1.0], 1e-8).is_err());
        assert!(sparse_residual_norm(&matrix, &[1.0], &[1.0]).is_err());
        assert!(sparse_residual_vector(&matrix, &[1.0], &[1.0]).is_err());
    }
    let matrix = diagonal(&[1.0]);
    for tolerance in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(sparse_relative_residual(&matrix, &[1.0], &[1.0], tolerance).is_err());
    }
    let matrix = diagonal(&[1e100]);
    let validation =
        sparse_relative_residual(&matrix, &[1e100], &[2.0], f64::MIN_POSITIVE).unwrap();
    assert!((validation.relative - 1.0 / 3.0).abs() < 1e-14);
    let mut matrix = diagonal(&[1.0]);
    matrix.rows[0].push((2, 1.0));
    assert!(
        sparse_residual_norm(&matrix, &[1.0], &[1.0])
            .unwrap_err()
            .contains("column")
    );
    assert!(
        sparse_relative_residual(&matrix, &[1.0], &[1.0], 1e-8)
            .err()
            .unwrap()
            .contains("column")
    );
}

#[test]
fn genuinely_unrepresentable_or_lossy_residuals_are_not_published_as_zero() {
    let unit = 2.0_f64.powi(1023);
    let matrix = diagonal(&[unit]);
    assert!(validate_spd_solution(&matrix, &[0.0], profile(vec![3.0])).is_err());
    let mut matrix = SparseMatrix::new(3);
    for (column, coefficient) in [unit, -unit, 2.0_f64.powi(-1000)].into_iter().enumerate() {
        add_at(&mut matrix, 0, column, coefficient);
    }
    let error = sparse_residual_vector(&matrix, &[0.0; 3], &[3.0, 3.0, 1.0]).unwrap_err();
    assert!(error.contains("lose numerical range"), "{error}");
}

#[test]
fn scaled_sparse_rows_match_manufactured_integer_equations_across_units() {
    for unit in [
        f64::from_bits(1),
        2.0_f64.powi(-1000),
        1.0,
        2.0_f64.powi(1000),
    ] {
        let solution = [2.0, -1.0, 3.0, -2.0];
        for coefficients in [[1.0, -2.0, 3.0, -4.0], [-3.0, 2.0, -1.0, 4.0]] {
            let exact = coefficients
                .iter()
                .zip(solution)
                .map(|(a, x)| a * x)
                .sum::<f64>();
            let row = coefficients
                .iter()
                .enumerate()
                .map(|(i, a)| (i, a * unit))
                .collect::<Vec<_>>();
            for offset in [-2.0, 0.0, 2.0] {
                let rhs = (exact + offset) * unit;
                let scaled =
                    range::scaled_row(&row, rhs, &solution, SolverStage::ResidualValidateRow)
                        .unwrap();
                assert_eq!(scaled.residual / unit, offset);
                let expected_scale = (exact + offset).abs()
                    + coefficients
                        .iter()
                        .zip(solution)
                        .map(|(a, x)| (a * x).abs())
                        .sum::<f64>();
                assert!((scaled.relative - offset.abs() / expected_scale).abs() < 1e-14);
            }
        }
    }
}

#[test]
fn wide_overflow_fallback_cancels_in_its_second_pass_and_replays() {
    let size = 4097;
    let unit = 2.0_f64.powi(1023);
    let mut matrix = SparseMatrix::new(size);
    for column in 0..4096 {
        add_at(
            &mut matrix,
            0,
            column,
            if column < 2048 { unit } else { -unit },
        );
    }
    add_at(&mut matrix, 0, 4096, 1.0);
    let mut rhs = vec![0.0; size];
    rhs[0] = 3.0;
    let solution = vec![3.0; size];
    for stage in [
        SolverStage::SparseResidualRow,
        SolverStage::ResidualValidateRow,
    ] {
        let control = SolverControl::default();
        let cancel = control.clone();
        let starts = Cell::new(0);
        let result = with_solver_observer(
            &control,
            move |point| {
                if point.stage == stage {
                    if point.completed_steps == 0 {
                        starts.set(starts.get() + 1);
                    }
                    if starts.get() >= 2 && point.completed_steps > size as u64 {
                        cancel.request_cancel();
                    }
                }
            },
            || {
                if stage == SolverStage::SparseResidualRow {
                    sparse_residual_norm(&matrix, &rhs, &solution)
                } else {
                    sparse_relative_residual(&matrix, &rhs, &solution, 1e-8)
                        .map(|value| value.relative)
                }
            },
        );
        assert!(result.unwrap_err().contains("cancelled"));
        assert!(control.was_interrupted());
    }
    assert_eq!(sparse_residual_norm(&matrix, &rhs, &solution).unwrap(), 0.0);
    assert_eq!(
        sparse_relative_residual(&matrix, &rhs, &solution, 1e-8)
            .unwrap()
            .relative,
        0.0
    );
}
