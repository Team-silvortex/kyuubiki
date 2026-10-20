use super::{PreparedTridiagonal, solve_tridiagonal};
use crate::solver_control::{SolverControl, SolverStage, with_solver_observer};

fn close(actual: &[f64], expected: &[f64]) {
    assert_eq!(actual.len(), expected.len());
    for (actual, expected) in actual.iter().zip(expected) {
        assert!(actual.is_finite() && (actual - expected).abs() < 1e-12);
    }
}

#[test]
fn finite_solution_survives_overflow_in_eliminated_rhs() {
    let unit = 2.0_f64.powi(1023);
    let factor = PreparedTridiagonal::factor(&[unit; 2], &[-0.5 * unit], &[-0.5 * unit]).unwrap();
    for sign in [-1.0, 1.0] {
        let solution = factor.solve(&[sign * 1.5 * unit; 2]).unwrap();
        close(&solution, &[sign * 3.0; 2]);
    }
}

#[test]
fn finite_solution_survives_overflow_in_a_schur_pivot() {
    for unit in [1.0, 2.0_f64.powi(1023)] {
        let solution = solve_tridiagonal(&[unit, -unit], &[unit], &[unit], &[unit, 0.0]).unwrap();
        close(&solution, &[0.5; 2]);
    }
}

#[test]
fn subnormal_coefficients_do_not_round_the_eliminated_equation() {
    let unit = f64::from_bits(2);
    let solution =
        solve_tridiagonal(&[unit; 2], &[-0.5 * unit], &[-0.5 * unit], &[1.5 * unit; 2]).unwrap();
    close(&solution, &[3.0; 2]);
}

#[test]
fn rowwise_scaling_keeps_mixed_units_and_pivoted_rhs_reuse() {
    let large = 2.0_f64.powi(1023);
    let small = f64::from_bits(4);
    let solution = solve_tridiagonal(
        &[large, small],
        &[-0.5 * small],
        &[-0.5 * large],
        &[1.5 * large, 1.5 * small],
    )
    .unwrap();
    close(&solution, &[3.0; 2]);

    let factor = PreparedTridiagonal::factor(&[0.0, small], &[small], &[large]).unwrap();
    assert!(factor.pivoted.is_some() && factor.equilibration.is_some());
    for sign in [-1.0, 1.0] {
        close(
            &factor
                .solve(&[sign * 0.5 * large, sign * 1.5 * small])
                .unwrap(),
            &[sign, sign * 0.5],
        );
    }
}

#[test]
fn scaling_does_not_discard_coefficients_or_rhs_and_failed_rhs_can_replay() {
    let large = 2.0_f64.powi(1023);
    let tiny = f64::from_bits(1);
    let error = PreparedTridiagonal::factor(&[large, 1.0], &[0.0], &[tiny])
        .err()
        .expect("a nonzero coefficient must not silently disappear");
    assert!(
        error.contains("coefficient") && error.contains("range loss"),
        "{error}"
    );

    let factor = PreparedTridiagonal::factor(&[large, 1.0], &[0.0], &[0.0]).unwrap();
    for rhs in [[tiny, 1.0], [3.0 * tiny, 1.0]] {
        let error = factor.solve(&rhs).unwrap_err();
        assert!(
            error.contains("right-hand side") && error.contains("range loss"),
            "{error}"
        );
    }
    close(&factor.solve(&[large, 2.0]).unwrap(), &[1.0, 2.0]);

    // Scaling can lose bits even when the result remains nonzero.
    let unit = 2.0_f64.powi(501);
    let rounded = 3.0 * 2.0_f64.powi(-574);
    assert!(PreparedTridiagonal::factor(&[unit, 1.0], &[0.0], &[rounded]).is_err());
    let factor = PreparedTridiagonal::factor(&[unit], &[], &[]).unwrap();
    assert!(factor.solve(&[rounded]).unwrap_err().contains("range loss"));
}

#[test]
fn scaled_factors_still_validate_against_original_equations() {
    let large = 2.0_f64.powi(1023);
    let mut factor =
        PreparedTridiagonal::factor(&[large; 2], &[-0.5 * large], &[-0.5 * large]).unwrap();
    factor.diagonal[0] *= 0.5;
    let error = factor.solve(&[1.5 * large; 2]).unwrap_err();
    assert!(error.contains("backward-error"), "{error}");
}

#[test]
fn scaling_preserves_rank_loss_and_nonfinite_rejection() {
    for unit in [2.0_f64.powi(1023), f64::from_bits(2)] {
        let error = PreparedTridiagonal::factor(&[unit; 2], &[unit], &[unit])
            .err()
            .expect("scaling must not regularize rank loss");
        assert!(error.contains("singular"), "{error}");
    }
    let factor = PreparedTridiagonal::factor(&[f64::MIN_POSITIVE], &[], &[]).unwrap();
    for rhs in [f64::NAN, f64::INFINITY, f64::MAX] {
        assert!(factor.solve(&[rhs]).is_err());
    }
}

#[test]
fn normal_range_keeps_the_existing_factor_path() {
    for unit in [2.0_f64.powi(-500), 1.0, 2.0_f64.powi(500)] {
        let factor =
            PreparedTridiagonal::factor(&[unit; 2], &[-0.5 * unit], &[-0.5 * unit]).unwrap();
        assert!(factor.equilibration.is_none());
        close(&factor.solve(&[1.5 * unit; 2]).unwrap(), &[3.0; 2]);
    }
}

#[test]
fn large_equilibration_and_rhs_scaling_cancel_without_mutating_reusable_factors() {
    let size = 2048;
    let diagonal = (0..size)
        .map(|i| {
            if i % 2 == 0 {
                2.0_f64.powi(1023)
            } else {
                f64::from_bits(2)
            }
        })
        .collect::<Vec<_>>();
    let offdiagonal = vec![0.0; size - 1];
    let factor = PreparedTridiagonal::factor(&diagonal, &offdiagonal, &offdiagonal).unwrap();
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
                    factor.solve(&diagonal)
                }
            },
        );
        assert!(result.unwrap_err().contains("cancelled"));
        assert!(control.was_interrupted());
    }
    let solution = with_solver_observer(
        &SolverControl::default(),
        |point| assert_ne!(point.stage, SolverStage::DenseFactor),
        || factor.solve(&diagonal),
    )
    .unwrap();
    close(&solution, &vec![1.0; size]);
}
