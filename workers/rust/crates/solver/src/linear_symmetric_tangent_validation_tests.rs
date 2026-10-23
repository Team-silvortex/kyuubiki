use super::{
    DENSE_PIVOTED_FALLBACK, SYMMETRIC_BAND_CHOLESKY, solve_refined, solve_symmetric_tangent,
    validate_solution,
};
use crate::linear_algebra::{SparseMatrix, add_at};
use crate::linear_banded::SymmetricBandCholesky;
use crate::linear_dense::DenseLu;
use crate::solver_control::{SolverControl, SolverStage, with_solver_observer};
use std::cell::Cell;
use std::rc::Rc;

fn diagonal(value: f64) -> SparseMatrix {
    let mut matrix = SparseMatrix::new(1);
    add_at(&mut matrix, 0, 0, value);
    matrix
}

#[test]
fn overflowing_tangent_residual_cannot_be_hidden_by_nan_maximum() {
    let high = 2.0_f64.powi(1023);
    assert!(validate_solution(&diagonal(high), &[high], &[2.0], "overflow").is_err());
}

#[test]
fn finite_wrong_tangent_response_cannot_pass_with_an_overflowing_scale() {
    let high = 2.0_f64.powi(1023);
    assert!(validate_solution(&diagonal(high), &[high], &[1.5], "scale").is_err());
}

#[test]
fn underflowed_tangent_products_cannot_certify_a_wrong_nonzero_response() {
    assert!(validate_solution(&diagonal(f64::from_bits(1)), &[0.0], &[0.25], "underflow").is_err());
}

#[test]
fn large_connected_displacement_cannot_mask_a_wrong_weak_equation() {
    let mut matrix = SparseMatrix::new(2);
    for (row, column, value) in [(0, 0, 1.0), (1, 1, 1.0), (0, 1, 1e-30), (1, 0, 1e-30)] {
        add_at(&mut matrix, row, column, value);
    }
    assert!(validate_solution(&matrix, &[0.0, 1e20], &[0.0, 1e20], "weak").is_err());
}

fn matrix(entries: &[&[f64]]) -> SparseMatrix {
    let mut matrix = SparseMatrix::new(entries.len());
    for (row, values) in entries.iter().enumerate() {
        for (column, value) in values.iter().enumerate() {
            add_at(&mut matrix, row, column, *value);
        }
    }
    matrix
}

#[test]
fn valid_extreme_and_subnormal_tangent_equations_remain_accepted() {
    let high = 2.0_f64.powi(1023);
    let coupled = matrix(&[&[high, -high * 0.5], &[-high * 0.5, high]]);
    for sign in [1.0, -1.0, 0.0] {
        validate_solution(&coupled, &[high * sign; 2], &[2.0 * sign; 2], "valid").unwrap();
        for scale in [f64::from_bits(1), 2.0_f64.powi(-1000), 1.0, high] {
            validate_solution(&diagonal(scale), &[scale * sign], &[sign], "units").unwrap();
        }
    }
}

#[test]
fn both_tangent_backends_validate_original_equations_at_extreme_scale() {
    let high = 2.0_f64.powi(1023);
    let positive = matrix(&[&[high, -high * 0.5], &[-high * 0.5, high]]);
    let indefinite = matrix(&[&[high, -high], &[-high, high * 0.5]]);
    for sign in [1.0, -1.0, 1.0] {
        let sparse = solve_symmetric_tangent(&positive, &[sign * high; 2], 0, "banded").unwrap();
        assert_eq!(sparse.method, SYMMETRIC_BAND_CHOLESKY);
        for value in sparse.solution {
            assert!((value - 2.0 * sign).abs() < 1e-12);
        }
        let dense = solve_symmetric_tangent(&indefinite, &[0.0, -sign * high], 2, "dense").unwrap();
        assert_eq!(dense.method, DENSE_PIVOTED_FALLBACK);
        assert_eq!(dense.solution, [2.0 * sign; 2]);
    }
}

#[test]
fn finite_tangent_solution_recovers_after_banded_substitution_overflow() {
    let high = 2.0_f64.powi(1023);
    let target = matrix(&[&[8.0, -7.0], &[-7.0, 8.0]]);
    let band = SymmetricBandCholesky::try_factor(&target, 4)
        .unwrap()
        .unwrap();
    for sign in [1.0, -1.0] {
        let rhs = [sign * high; 2];
        let direct = band.solve(&rhs).expect("range-aware banded substitution");
        let recovered =
            solve_symmetric_tangent(&target, &rhs, 0, "without dense fallback").unwrap();
        assert_eq!(recovered.method, SYMMETRIC_BAND_CHOLESKY);
        assert_eq!(recovered.solution, direct);
        validate_solution(&target, &rhs, &recovered.solution, "original").unwrap();
        for value in recovered.solution {
            assert!((value / high - sign).abs() < 1e-14);
        }
    }
}

#[test]
fn tangent_validation_rejects_missing_equations_and_hidden_nonfinite_components() {
    let mut sparse = SparseMatrix::new(2);
    add_at(&mut sparse, 0, 0, 1.0);
    for (rhs, solution) in [
        (vec![1.0], vec![1.0, 0.0]),
        (vec![1.0, 0.0], vec![1.0]),
        (vec![1.0, 0.0], vec![1.0, 0.0, 1.0]),
    ] {
        assert!(validate_solution(&sparse, &rhs, &solution, "dimensions").is_err());
    }
    for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(
            validate_solution(&sparse, &[1.0, 0.0], &[1.0, invalid], "unused component").is_err()
        );
        assert!(validate_solution(&sparse, &[1.0, invalid], &[1.0, 0.0], "rhs").is_err());
        assert!(validate_solution(&diagonal(invalid), &[0.0], &[0.0], "matrix").is_err());
    }
    validate_solution(&sparse, &[1.0, 0.0], &[1.0, 0.0], "replay").unwrap();
}

#[test]
fn malformed_tangent_columns_fail_before_either_factorization_path() {
    for column in [1, usize::MAX] {
        let mut invalid = diagonal(1.0);
        add_at(&mut invalid, 0, column, 1.0);
        for limit in [0, 1] {
            let error = solve_symmetric_tangent(&invalid, &[1.0], limit, "invalid column")
                .err()
                .expect("invalid columns must be rejected before factorization");
            assert!(error.contains("column indices"));
        }
    }
}

fn cancel_at<T>(stage: SolverStage, step: u64, operation: impl FnOnce() -> Result<T, String>) {
    let control = SolverControl::default();
    let cancel = control.clone();
    let result = with_solver_observer(
        &control,
        move |point| {
            if point.stage == stage && point.completed_steps == step {
                cancel.request_cancel();
            }
        },
        operation,
    );
    assert!(result.is_err());
    assert!(control.was_interrupted());
    let point = control.last_checkpoint().unwrap();
    assert_eq!(point.stage, stage);
    assert_eq!(point.completed_steps, step);
}

#[test]
fn tangent_validation_cancels_during_complete_scan_and_wide_range_recovery() {
    let size = 2049;
    let high = 2.0_f64.powi(1023);
    let mut wide = SparseMatrix::new(size);
    add_at(&mut wide, 0, 0, 1.0);
    for column in 1..size {
        add_at(
            &mut wide,
            0,
            column,
            if column % 2 == 1 { high } else { -high },
        );
    }
    let mut rhs = vec![0.0; size];
    rhs[0] = 1.0;
    let solution = vec![1.0; size];
    for (stage, step) in [
        (SolverStage::ResidualValidate, 1024),
        (SolverStage::ResidualValidateRow, 1024),
        (SolverStage::ResidualValidateRow, 3073),
    ] {
        cancel_at(stage, step, || {
            validate_solution(&wide, &rhs, &solution, "wide")
        });
        validate_solution(&wide, &rhs, &solution, "replay").unwrap();
    }
}

#[test]
fn tangent_refinement_uses_checked_residuals_and_replays_after_cancellation() {
    let target = matrix(&[&[2.0, -1.0], &[-1.0, 2.0]]);
    // A deliberately approximate factor exercises genuine residual corrections.
    let approximation = matrix(&[&[2.0001, -1.0], &[-1.0, 2.0001]]);
    let factor = SymmetricBandCholesky::try_factor(&approximation, 4)
        .unwrap()
        .unwrap();
    cancel_at(SolverStage::SparseResidual, 0, || {
        solve_refined(&target, &[1.0, 1.0], |rhs| factor.solve(rhs), "refinement")
    });
    for sign in [1.0, -1.0, 1.0] {
        let solution =
            solve_refined(&target, &[sign; 2], |rhs| factor.solve(rhs), "replay").unwrap();
        validate_solution(&target, &[sign; 2], &solution, "original").unwrap();
        for value in solution {
            assert!((value - sign).abs() < 1e-9);
        }
    }
}

#[test]
fn dense_tangent_refinement_reuses_factor_and_preserves_cancellation() {
    let target = matrix(&[&[0.0, 1.0], &[1.0, 0.0]]);
    let factor = DenseLu::factor(vec![vec![0.0001, 1.0], vec![1.0, 0.0001]]).unwrap();
    let control = SolverControl::default();
    let cancel = control.clone();
    let visits = Rc::new(Cell::new(0));
    let observed = visits.clone();
    let result = with_solver_observer(
        &control,
        move |point| {
            if point.stage == SolverStage::DenseSubstitution && point.completed_steps == 0 {
                observed.set(observed.get() + 1);
                if observed.get() == 2 {
                    cancel.request_cancel();
                }
            }
        },
        || solve_refined(&target, &[-1.0, 1.0], |rhs| factor.solve(rhs), "dense"),
    );
    assert!(result.is_err());
    assert!(control.was_interrupted());
    assert_eq!(visits.get(), 2, "cancel inside the correction solve");
    for sign in [1.0, -1.0, 1.0] {
        let solution =
            solve_refined(&target, &[-sign, sign], |rhs| factor.solve(rhs), "replay").unwrap();
        validate_solution(&target, &[-sign, sign], &solution, "original").unwrap();
        assert!((solution[0] - sign).abs() < 1e-9);
        assert!((solution[1] + sign).abs() < 1e-9);
    }
}

#[test]
fn tangent_refinement_failure_cannot_bypass_iteration_or_dense_size_limits() {
    let solves = Cell::new(0);
    let error = solve_refined(
        &diagonal(1.0),
        &[1.0],
        |_| {
            solves.set(solves.get() + 1);
            Ok(vec![2.0])
        },
        "stagnating",
    )
    .unwrap_err();
    assert!(error.contains("backward-error validation"));
    assert_eq!(solves.get(), 5, "one solve and at most four corrections");

    let indefinite = matrix(&[&[0.0, 1.0], &[1.0, 0.0]]);
    let dense_visits = Rc::new(Cell::new(0));
    let observed = dense_visits.clone();
    let result = with_solver_observer(
        &SolverControl::default(),
        move |point| {
            if point.stage == SolverStage::DenseFactor {
                observed.set(observed.get() + 1);
            }
        },
        || solve_symmetric_tangent(&indefinite, &[1.0, 2.0], 1, "bounded"),
    );
    assert!(
        result
            .err()
            .unwrap()
            .contains("exceeds the 1-dof dense fallback")
    );
    assert_eq!(dense_visits.get(), 0);
    let replay = solve_symmetric_tangent(&indefinite, &[1.0, 2.0], 2, "replay").unwrap();
    assert_eq!(replay.method, DENSE_PIVOTED_FALLBACK);
    assert_eq!(replay.solution, [2.0, 1.0]);
}

#[test]
fn accepted_tangent_needs_no_correction_vector_and_cancelled_validation_cannot_fallback() {
    let target = matrix(&[&[2.0, -1.0], &[-1.0, 2.0]]);
    let residual_visits = Rc::new(Cell::new(0));
    let observed = residual_visits.clone();
    let result = with_solver_observer(
        &SolverControl::default(),
        move |point| {
            if point.stage == SolverStage::SparseResidual {
                observed.set(observed.get() + 1);
            }
        },
        || solve_symmetric_tangent(&target, &[1.0, 1.0], 2, "fast"),
    )
    .unwrap();
    assert_eq!(result.method, SYMMETRIC_BAND_CHOLESKY);
    assert_eq!(residual_visits.get(), 0);
    cancel_at(SolverStage::ResidualValidate, 0, || {
        solve_symmetric_tangent(&target, &[1.0, 1.0], 2, "cancel")
    });
    let replay = solve_symmetric_tangent(&target, &[1.0, 1.0], 2, "replay").unwrap();
    assert_eq!(replay.solution, result.solution);
}
