use super::{PreparedTridiagonal, maximum_backward_error, solve_tridiagonal};
use crate::linear_dense::DenseLu;
use crate::solver_control::{SolverControl, SolverStage, with_solver_observer};

fn rhs(diagonal: &[f64], lower: &[f64], upper: &[f64], x: &[f64]) -> Vec<f64> {
    (0..x.len())
        .map(|i| {
            diagonal[i] * x[i]
                + if i > 0 { lower[i - 1] * x[i - 1] } else { 0.0 }
                + if i + 1 < x.len() {
                    upper[i] * x[i + 1]
                } else {
                    0.0
                }
        })
        .collect()
}

fn close(actual: &[f64], expected: &[f64]) {
    for (a, b) in actual.iter().zip(expected) {
        assert!(
            a.is_finite() && (a - b).abs() <= 1e-10,
            "{actual:?} != {expected:?}"
        );
    }
}

#[test]
fn invertible_zero_leading_pivot_does_not_mean_rank_loss() {
    for scale in [2.0_f64.powi(-500), 1.0, 2.0_f64.powi(500)] {
        let solved = solve_tridiagonal(
            &[0.0, scale],
            &[scale],
            &[scale],
            &[2.0 * scale, 3.0 * scale],
        )
        .unwrap();
        close(&solved, &[1.0, 2.0]);
    }
}

#[test]
fn invertible_zero_interior_schur_pivot_can_use_the_next_row() {
    let diagonal = [1.0, 1.0, 1.0];
    let lower = [1.0, 1.0];
    let upper = [1.0, 1.0];
    let expected = [1.0, -2.0, 3.0];
    let solved = solve_tridiagonal(
        &diagonal,
        &lower,
        &upper,
        &rhs(&diagonal, &lower, &upper, &expected),
    )
    .unwrap();
    close(&solved, &expected);
}

#[test]
fn repeated_adjacent_pivots_preserve_second_superdiagonal_and_rhs_reuse() {
    let diagonal = [0.0, 0.0, 0.0, 0.0, 2.0];
    let lower = [2.0, 3.0, -4.0, 5.0];
    let upper = [1.0, -2.0, 3.0, 4.0];
    let factor = PreparedTridiagonal::factor(&diagonal, &lower, &upper).unwrap();
    for expected in [
        [1.0, -2.0, 3.0, 4.0, -5.0],
        [-2.0, 1.0, -4.0, 3.0, 5.0],
        [0.0; 5],
    ] {
        let solved = factor
            .solve(&rhs(&diagonal, &lower, &upper, &expected))
            .unwrap();
        close(&solved, &expected);
    }
}

#[test]
fn actual_rank_loss_still_fails() {
    for (diagonal, lower, upper) in [
        (vec![0.0, 0.0], vec![1.0], vec![0.0]),
        (vec![1.0, 1.0 + f64::EPSILON], vec![1.0], vec![1.0]),
    ] {
        let error = PreparedTridiagonal::factor(&diagonal, &lower, &upper)
            .err()
            .expect("rank loss must not be regularized into a successful solve");
        assert!(error.contains("singular"), "{error}");
    }
}

#[test]
fn positive_chain_keeps_the_existing_no_pivot_factors() {
    let factor = PreparedTridiagonal::factor(&[2.0, 2.0, 2.0], &[-1.0; 2], &[-1.0; 2]).unwrap();
    assert!(factor.pivoted.is_none());
    close(&factor.solve(&[0.0, 0.0, 4.0]).unwrap(), &[1.0, 2.0, 3.0]);
}

#[test]
fn pivoted_solution_is_checked_against_original_coefficients() {
    let mut factor = PreparedTridiagonal::factor(&[0.0, 1.0], &[1.0], &[1.0]).unwrap();
    assert!(factor.pivoted.is_some());
    // Simulate an incorrect solution relative to the saved original operator.
    factor.diagonal[0] = 4.0;
    assert!(
        factor
            .solve(&[2.0, 3.0])
            .unwrap_err()
            .contains("backward-error"),
        "pivoting must not bypass residual validation"
    );
}

#[test]
fn deterministic_small_systems_match_dense_lu_and_manufactured_solutions() {
    let mut seed = 0x5a17_u64;
    let mut next = || {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        ((seed >> 32) % 7) as f64 - 3.0
    };
    let mut checked = 0;
    for size in 2..=10 {
        for sample in 0..64 {
            let mut diagonal = (0..size).map(|_| next()).collect::<Vec<_>>();
            let lower = (1..size).map(|_| next()).collect::<Vec<_>>();
            let upper = if sample % 2 == 0 {
                lower.clone()
            } else {
                (1..size).map(|_| next()).collect()
            };
            diagonal[0] = 0.0;
            let mut dense = vec![vec![0.0; size]; size];
            for row in 0..size {
                dense[row][row] = diagonal[row];
                if row > 0 {
                    dense[row][row - 1] = lower[row - 1];
                }
                if row + 1 < size {
                    dense[row][row + 1] = upper[row];
                }
            }
            let Ok(reference) = DenseLu::factor(dense) else {
                continue;
            };
            let expected = (0..size).map(|i| (i % 7) as f64 - 3.0).collect::<Vec<_>>();
            let load = rhs(&diagonal, &lower, &upper, &expected);
            let actual = solve_tridiagonal(&diagonal, &lower, &upper, &load).unwrap();
            close(&actual, &expected);
            close(&actual, &reference.solve(&load).unwrap());
            checked += 1;
        }
    }
    assert!(
        checked >= 300,
        "insufficient nonsingular controls: {checked}"
    );
}

#[test]
fn pivoted_large_forest_cancels_and_reuses_factors_without_dense_fallback() {
    let size = 4096;
    let diagonal = (0..size).map(|i| (i % 2) as f64).collect::<Vec<_>>();
    let offdiagonal = (1..size).map(|i| (i % 2) as f64).collect::<Vec<_>>();
    let expected = (0..size).map(|i| (1 + i % 2) as f64).collect::<Vec<_>>();
    let load = rhs(&diagonal, &offdiagonal, &offdiagonal, &expected);
    let factor = PreparedTridiagonal::factor(&diagonal, &offdiagonal, &offdiagonal).unwrap();
    assert!(factor.pivoted.is_some());
    assert!(factor.factored_diagonal.is_empty() && factor.elimination_factors.is_empty());

    for stage in [
        SolverStage::TridiagonalFactor,
        SolverStage::TridiagonalSubstitution,
    ] {
        let control = SolverControl::default();
        let cancel = control.clone();
        let result = with_solver_observer(
            &control,
            move |point| {
                assert_ne!(point.stage, SolverStage::DenseFactor);
                if point.stage == stage && point.completed_steps >= 256 {
                    cancel.request_cancel();
                }
            },
            || {
                if stage == SolverStage::TridiagonalFactor {
                    PreparedTridiagonal::factor(&diagonal, &offdiagonal, &offdiagonal)
                        .map(|_| Vec::new())
                } else {
                    factor.solve(&load)
                }
            },
        );
        assert!(result.unwrap_err().contains("cancelled"));
        assert!(control.was_interrupted());
    }
    let solved = with_solver_observer(
        &SolverControl::default(),
        |point| assert_ne!(point.stage, SolverStage::DenseFactor),
        || factor.solve(&load),
    )
    .unwrap();
    close(&solved, &expected);
}

#[test]
fn overflowing_error_denominator_cannot_validate_a_wrong_solution() {
    let unit = 2.0_f64.powi(1023);
    let error = maximum_backward_error(&[unit], &[], &[], &[1.5 * unit], &[1.0], 1e-10).unwrap();
    assert!((error - 0.2).abs() < 1e-14, "{error}");
}

#[test]
fn overflowing_large_row_cannot_hide_an_incorrect_small_equation() {
    let unit = 2.0_f64.powi(1023);
    let error = maximum_backward_error(
        &[unit, 1.0],
        &[0.0],
        &[0.0],
        &[unit, 1.0],
        &[1.0, 2.0],
        1e-10,
    )
    .unwrap();
    assert!((error - 1.0 / 3.0).abs() < 1e-14, "{error}");
}

#[test]
fn nonfinite_unscaled_products_are_not_folded_into_zero_error() {
    let unit = 2.0_f64.powi(1023);
    let error = maximum_backward_error(
        &[unit, unit],
        &[unit],
        &[-unit],
        &[0.0, 0.0],
        &[2.0, 1.0],
        1e-10,
    )
    .unwrap();
    assert!((error - 1.0).abs() < 1e-14, "{error}");
}

#[test]
fn scaled_validation_preserves_cancellation_and_subnormal_equations() {
    let unit = 2.0_f64.powi(1023);
    let error = maximum_backward_error(
        &[unit, unit],
        &[-0.5 * unit],
        &[-unit],
        &[unit, 0.5 * unit],
        &[3.0, 2.0],
        1e-10,
    )
    .unwrap();
    assert_eq!(error, 0.0);

    let tiny = f64::from_bits(1);
    let diagonal = [unit, 1e-300, 1e300, tiny];
    let solution = [1.0, 1e300, 1e-300, 1.0];
    let mut load = [unit, 1.0, 1.0, tiny];
    let error =
        maximum_backward_error(&diagonal, &[0.0; 3], &[0.0; 3], &load, &solution, 1e-10).unwrap();
    assert!(error <= 4.0 * f64::EPSILON, "{error}");
    load[3] = 2.0 * tiny;
    let error =
        maximum_backward_error(&diagonal, &[0.0; 3], &[0.0; 3], &load, &solution, 1e-10).unwrap();
    assert!((error - 1.0 / 3.0).abs() < 1e-14, "{error}");
}

#[test]
fn original_equation_validation_is_cancellable_in_normal_and_scaled_passes() {
    for value in [1.0, 2.0_f64.powi(1023)] {
        let diagonal = vec![value; 600];
        let offdiagonal = vec![0.0; 599];
        let solution = vec![1.0; 600];
        let control = SolverControl::default();
        let cancel = control.clone();
        let result = with_solver_observer(
            &control,
            move |point| {
                if point.stage == SolverStage::SparseResidual && point.completed_steps >= 256 {
                    cancel.request_cancel();
                }
            },
            || {
                maximum_backward_error(
                    &diagonal,
                    &offdiagonal,
                    &offdiagonal,
                    &diagonal,
                    &solution,
                    1e-10,
                )
            },
        );
        assert!(result.unwrap_err().contains("cancelled"));
        assert!(control.was_interrupted());
        assert_eq!(
            maximum_backward_error(
                &diagonal,
                &offdiagonal,
                &offdiagonal,
                &diagonal,
                &solution,
                1e-10,
            )
            .unwrap(),
            0.0,
        );
    }
}
