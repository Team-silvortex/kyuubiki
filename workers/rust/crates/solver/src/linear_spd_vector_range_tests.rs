use super::*;
use crate::linear_algebra::{SparseMatrix, add_at, stable_l2_norm};
use crate::linear_solver_profile::SpdSolveOptions;
use crate::linear_spd::control_tests::interrupted;
use crate::linear_spd::{recompute_residual, solve_spd_compressed};
use crate::solver_control::{SolverControl, with_solver_observer};

#[test]
fn vector_admission_does_not_ignore_nonfinite_components() {
    for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        for index in [0, 1024, 2048] {
            let mut values = vec![2.0; 2049];
            values[index] = invalid;
            assert!(rhs_scale(&values).is_err(), "scale accepted {invalid}");
            assert!(l2_norm(&values).is_err(), "norm accepted {invalid}");
        }
    }
}

#[test]
fn normalization_rejects_invalid_scales_and_erased_nonzero_loads() {
    for scale in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(normalize_rhs(&[1.0], scale).is_err(), "scale={scale}");
    }
    for (value, scale) in [
        (f64::NAN, 1.0),
        (f64::INFINITY, 1.0),
        (1e300, 1e-300),
        (f64::from_bits(1), 2.0),
        (-f64::from_bits(1), 2.0),
    ] {
        assert!(
            normalize_rhs(&[value], scale).is_err(),
            "normalization accepted {value}/{scale}"
        );
    }
    let q = f64::from_bits(1);
    assert_eq!(normalize_rhs(&[q, -q, 0.0], q).unwrap(), [1.0, -1.0, 0.0]);
    assert_eq!(normalize_rhs(&[2.0 * q, -2.0 * q], 2.0).unwrap(), [q, -q]);
}

#[test]
fn solution_rescaling_rejects_nonfinite_outputs_but_preserves_finite_quantization() {
    for scale in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(rescale_solution(vec![1.0], scale).is_err(), "scale={scale}");
    }
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, f64::MAX] {
        assert!(rescale_solution(vec![value], 2.0).is_err());
    }
    // Physical row-equilibrium validation, not this conversion, judges quantization.
    let q = f64::from_bits(1);
    assert_eq!(rescale_solution(vec![q, -q], 1.0).unwrap(), [q, -q]);
    assert_eq!(rescale_solution(vec![q, -q], 0.25).unwrap(), [0.0, -0.0]);
}

fn diagonal_system(second_diagonal: f64) -> SparseMatrix {
    let mut matrix = SparseMatrix::new(2);
    add_at(&mut matrix, 0, 0, 1.0);
    add_at(&mut matrix, 1, 1, second_diagonal);
    matrix
}

#[test]
fn invalid_rhs_cannot_take_the_zero_load_shortcut_and_replay_stays_clean() {
    let matrix = diagonal_system(2.0);
    let options = SpdSolveOptions::default();
    let compressed = matrix.compress(options.preconditioner).unwrap();
    for rhs in [[f64::NAN, 0.0], [0.0, f64::NAN], [f64::INFINITY, 1.0]] {
        assert!(solve_spd_compressed(&compressed, &rhs, &matrix, &options).is_err());
        let profile = solve_spd_compressed(&compressed, &[1.0, 2.0], &matrix, &options).unwrap();
        assert_eq!(profile.solution, [1.0, 1.0]);
    }
    let zero = solve_spd_compressed(&compressed, &[0.0, -0.0], &matrix, &options).unwrap();
    assert_eq!(zero.iterations, 0);
    assert_eq!(zero.solution, [0.0, 0.0]);
}

#[test]
fn pcg_reports_small_fresh_residuals_instead_of_squared_underflow_zero() {
    let options = SpdSolveOptions::default();
    let mut nonzero_controls = 0;
    for diagonal in [3.0, 7.0, 11.0, 13.0, 17.0] {
        let matrix = diagonal_system(diagonal);
        let compressed = matrix.compress(options.preconditioner).unwrap();
        for load in [1e-200, -1e-200, 1.7e-200, -1.7e-200] {
            let rhs = [1.0, load];
            let profile = solve_spd_compressed(&compressed, &rhs, &matrix, &options).unwrap();
            assert!(
                profile.iterations > 0,
                "must exercise PCG, not dense fallback"
            );
            let expected = stable_l2_norm([
                rhs[0] - profile.solution[0],
                rhs[1] - diagonal * profile.solution[1],
            ]);
            if expected > 0.0 {
                nonzero_controls += 1;
                assert_eq!(expected * expected, 0.0, "must exercise squared underflow");
                assert_eq!(profile.residual_norm, expected);
            }
        }
    }
    assert!(
        nonzero_controls > 0,
        "must retain a nonzero residual control"
    );
}

#[test]
fn residual_norm_recovers_squared_underflow_subnormal_rounding_and_overflow() {
    for scale in [
        f64::from_bits(1),
        2.0_f64.powi(-1000),
        2.0_f64.powi(-540),
        2.0_f64.powi(-537),
        2.0_f64.powi(-530),
        2.0_f64.powi(-512),
        1.0,
        2.0_f64.powi(500),
        2.0_f64.powi(512),
        2.0_f64.powi(1000),
    ] {
        let values = [3.0 * scale, -4.0 * scale];
        let squared = values.iter().map(|v| v * v).sum();
        let norm = norm_from_squared_sum(squared, &values).unwrap();
        assert_eq!(norm, 5.0 * scale, "scale={scale:e}");
    }
    assert_eq!(norm_from_squared_sum(0.0, &[0.0, -0.0]).unwrap(), 0.0);
    assert!(norm_from_squared_sum(f64::NAN, &[f64::NAN, 2.0]).is_err());
    assert!(norm_from_squared_sum(f64::INFINITY, &[f64::INFINITY]).is_err());
    assert!(norm_from_squared_sum(f64::INFINITY, &[f64::MAX, f64::MAX]).is_err());
}

#[test]
fn stable_norm_fallback_is_cancellable_and_the_normal_path_needs_no_second_scan() {
    for value in [1e-200, 1e-161, 1e200] {
        let values = vec![value; 2049];
        let squared = values.iter().map(|v| v * v).sum();
        for steps in [0, 1024, 2049] {
            interrupted(SolverStage::PcgNorm, steps, || {
                norm_from_squared_sum(squared, &values)
            });
            assert_eq!(
                norm_from_squared_sum(squared, &values).unwrap(),
                stable_l2_norm(values.iter().copied())
            );
        }
    }
    let normal = [3.0, 4.0];
    with_solver_observer(
        &SolverControl::default(),
        |point| assert_ne!(point.stage, SolverStage::PcgNorm),
        || {
            assert_eq!(
                norm_from_squared_sum(25.0, &normal)?.to_bits(),
                5.0_f64.to_bits()
            );
            Ok::<(), String>(())
        },
    )
    .unwrap();
}

#[test]
fn fresh_residual_uses_the_same_range_safe_norm_as_the_recursive_path() {
    let matrix = diagonal_system(1.0);
    let options = SpdSolveOptions::default();
    let compressed = matrix.compress(options.preconditioner).unwrap();
    for scale in [f64::from_bits(1), 1e-200, 1e200] {
        let rhs = [3.0 * scale, 4.0 * scale];
        let mut residual = [f64::NAN; 2];
        let norm = recompute_residual(
            &compressed,
            &rhs,
            1.0,
            &[0.0; 2],
            &mut residual,
            &mut [0.0; 2],
        )
        .unwrap();
        assert_eq!(residual, rhs);
        assert_eq!(norm, stable_l2_norm(rhs));
        assert_eq!(
            norm,
            norm_from_squared_sum(dot(&rhs, &rhs).unwrap(), &rhs).unwrap()
        );
    }
}
