use super::{PreparedTridiagonal, maximum_backward_error};
use crate::solver_control::{SolverControl, SolverStage, with_solver_observer};

#[test]
fn finite_large_row_cannot_hide_a_wrong_weak_equation() {
    for large in [1e20, 2.0_f64.powi(400)] {
        for sign in [-1.0, 1.0] {
            let error = maximum_backward_error(
                &[1.0; 2],
                &[0.0],
                &[0.0],
                &[sign * large, sign],
                &[sign * large, 2.0 * sign],
                1e-10,
            )
            .unwrap();
            assert!((error - 1.0 / 3.0).abs() < 1e-14, "{error}");
        }
    }
}

#[test]
fn underflowed_equation_products_are_not_zero_error() {
    let quantum = f64::from_bits(1);
    for sign in [-1.0, 1.0] {
        let error =
            maximum_backward_error(&[0.5], &[], &[], &[0.0], &[sign * quantum], 1e-10).unwrap();
        assert_eq!(error, 0.5);
    }
}

#[test]
fn two_half_quantum_products_are_checked_before_rounding() {
    let quantum = f64::from_bits(1);
    for sign in [-1.0, 1.0] {
        let error = maximum_backward_error(
            &[0.5; 2],
            &[0.5],
            &[0.5],
            &[sign * quantum; 2],
            &[sign * quantum; 2],
            1e-10,
        )
        .unwrap();
        assert_eq!(error, 0.0);
    }
}

#[test]
fn unrepresentable_weak_response_is_rejected_before_clean_factor_replay() {
    let quantum = f64::from_bits(1);
    let factor = PreparedTridiagonal::factor(&[1.0, 4.0], &[0.0], &[0.0]).unwrap();
    for sign in [-1.0, 1.0] {
        let error = factor.solve(&[sign, sign * quantum]).unwrap_err();
        assert!(error.contains("backward-error"), "{error}");
        assert_eq!(
            factor.solve(&[sign, sign * 4.0 * quantum]).unwrap(),
            vec![sign, sign * quantum]
        );
        assert_eq!(factor.solve(&[sign, 4.0 * sign]).unwrap(), vec![sign; 2]);
    }
}

#[test]
fn prepared_weak_equation_validation_is_independent_of_other_loads() {
    let mut factor = PreparedTridiagonal::factor(&[1.0; 2], &[0.0], &[0.0]).unwrap();
    // Deliberately corrupt a saved original equation, not the prepared factors.
    factor.diagonal[1] = 2.0;
    let error = factor.solve(&[1e100, 1.0]).unwrap_err();
    assert!(error.contains("backward-error"), "{error}");
    factor.diagonal[1] = 1.0;
    assert_eq!(factor.solve(&[1e100, 1.0]).unwrap(), vec![1e100, 1.0]);
}

#[test]
fn tridiagonal_validation_is_row_scale_invariant() {
    for scale in [
        f64::from_bits(2),
        2.0_f64.powi(-500),
        1.0,
        2.0_f64.powi(500),
        2.0_f64.powi(1023),
    ] {
        let error = maximum_backward_error(
            &[scale; 2],
            &[0.0],
            &[0.0],
            &[scale, scale * 0.5],
            &[1.0; 2],
            1e-10,
        )
        .unwrap();
        assert!(
            (error - 1.0 / 3.0).abs() < 1e-14,
            "scale={scale}, error={error}"
        );
    }
}

#[test]
fn tridiagonal_validation_cancellation_does_not_publish_or_poison_a_factor() {
    let size = 513;
    let diagonal = vec![1.0; size];
    let offdiagonal = vec![0.0; size - 1];
    let factor = PreparedTridiagonal::factor(&diagonal, &offdiagonal, &offdiagonal).unwrap();
    let mut rhs = vec![1.0; size];
    rhs[size - 1] = f64::from_bits(1);
    for steps in [0, 256, 512, size] {
        let control = SolverControl::default();
        let cancel = control.clone();
        let result = with_solver_observer(
            &control,
            move |point| {
                assert_ne!(point.stage, SolverStage::DenseFactor);
                if point.stage == SolverStage::SparseResidual
                    && point.completed_steps == steps as u64
                {
                    cancel.request_cancel();
                }
            },
            || factor.solve(&rhs),
        );
        assert!(result.unwrap_err().contains("cancelled"));
        assert!(control.was_interrupted());
        assert_eq!(factor.solve(&rhs).unwrap(), rhs);
    }
}

#[test]
fn pivoted_and_equilibrated_paths_cannot_hide_an_unrepresentable_weak_response() {
    let quantum = f64::from_bits(1);
    for unit in [1.0, 2.0_f64.powi(501)] {
        let factor =
            PreparedTridiagonal::factor(&[0.0, 1.0, 4.0], &[1.0, 0.0], &[unit, 0.0]).unwrap();
        assert!(factor.pivoted.is_some());
        assert_eq!(factor.equilibration.is_some(), unit != 1.0);
        let error = factor.solve(&[unit, 2.0, quantum]).unwrap_err();
        let expected = if unit == 1.0 {
            "backward-error"
        } else {
            "range loss"
        };
        assert!(error.contains(expected), "{error}");
        assert_eq!(
            factor.solve(&[unit, 2.0, 4.0 * quantum]).unwrap(),
            vec![1.0, 1.0, quantum]
        );
    }
}

#[test]
fn tridiagonal_validation_rejects_invalid_dimensions_operands_and_tolerance() {
    for (diagonal, lower, upper, rhs, solution) in [
        (vec![1.0; 2], vec![], vec![0.0], vec![1.0; 2], vec![1.0; 2]),
        (vec![1.0; 2], vec![0.0], vec![], vec![1.0; 2], vec![1.0; 2]),
        (vec![1.0], vec![], vec![], vec![], vec![1.0]),
        (vec![1.0], vec![], vec![], vec![1.0], vec![]),
        (vec![1.0], vec![], vec![], vec![1.0], vec![1.0; 2]),
    ] {
        assert!(maximum_backward_error(&diagonal, &lower, &upper, &rhs, &solution, 1e-10).is_err());
    }
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        for index in 0..6 {
            let mut operands = [1.0, 0.0, 0.0, 1.0, 1.0, 1.0];
            operands[index] = bad;
            assert!(
                maximum_backward_error(
                    &[operands[0], 1.0],
                    &[operands[1]],
                    &[operands[2]],
                    &[operands[3], 1.0],
                    &[operands[4], operands[5]],
                    1e-10,
                )
                .is_err()
            );
        }
    }
    for bad in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(maximum_backward_error(&[1.0], &[], &[], &[1.0], &[1.0], bad).is_err());
    }
    assert_eq!(
        maximum_backward_error(&[], &[], &[], &[], &[], 1e-10).unwrap(),
        0.0
    );
}

#[test]
fn subnormal_roundoff_is_local_and_cannot_cover_a_multi_quantum_error() {
    let quantum = f64::from_bits(1);
    for load in [0.0, 1.0, 1e100] {
        // A rounded q response to 4q / 3 is within half a quantum.
        let error = maximum_backward_error(
            &[1.0, 3.0],
            &[0.0],
            &[0.0],
            &[load, 4.0 * quantum],
            &[load, quantum],
            1e-10,
        )
        .unwrap();
        assert_eq!(error, 0.0);
        let error = maximum_backward_error(
            &[1.0, 3.0],
            &[0.0],
            &[0.0],
            &[load, 8.0 * quantum],
            &[load, quantum],
            1e-10,
        )
        .unwrap();
        assert!((error - 3.5 / 11.0).abs() < 1e-14, "{error}");
    }
}

#[test]
fn long_chain_preserves_quantized_tail_and_closed_form_near_field() {
    let size = 1026;
    let factor = PreparedTridiagonal::factor(
        &vec![4.0; size],
        &vec![-1.0; size - 1],
        &vec![-1.0; size - 1],
    )
    .unwrap();
    for sign in [-1.0, 1.0] {
        let mut rhs = vec![0.0; size];
        rhs[size - 1] = sign;
        let solution = with_solver_observer(
            &SolverControl::default(),
            |point| {
                assert_ne!(point.stage, SolverStage::DenseFactor);
            },
            || factor.solve(&rhs),
        )
        .unwrap();
        assert!(
            solution
                .iter()
                .all(|value| value.is_finite() && value * sign >= 0.0)
        );
        assert!(solution.contains(&0.0));
        assert!(solution.iter().any(|value| value.is_subnormal()));
        // Green's function of 4u_i - u_(i-1) - u_(i+1): the remote-boundary
        // correction is below f64 range at this length, not a relaxed tolerance.
        let ratio = 1.0 / (2.0 + 3.0_f64.sqrt());
        for distance in 0..80 {
            let expected = sign * ratio.powi(distance as i32 + 1);
            assert!((solution[size - 1 - distance] / expected - 1.0).abs() < 1e-12);
        }
    }
}
