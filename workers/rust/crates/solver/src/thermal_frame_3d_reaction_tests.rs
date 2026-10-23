use super::residual_vector;
use crate::frame_3d_math::normalized_direction;
use crate::linear_algebra::{SparseMatrix, add_at};
use crate::solver_control::{SolverControl, SolverStage, with_solver_observer};
use crate::thermal_frame_3d_constraint_basis::reaction_coefficients;

fn directions() -> [[f64; 3]; 3] {
    [[1.0, 0.0, 0.0], [-1.0, 1.0, 0.0], [1.0, 0.0, 1.0]]
        .map(|direction| normalized_direction(direction).unwrap())
}

fn close(actual: f64, expected: f64) {
    assert!(actual.is_finite());
    assert!(
        (actual / expected - 1.0).abs() < 2.0e-14,
        "{actual:e} != {expected:e}"
    );
}

#[test]
fn thermal_frame_reactions_recover_finite_original_rows_after_intermediate_overflow() {
    for sign in [-1.0, 1.0] {
        let scale = 2.0_f64.powi(1022);
        let mut matrix = SparseMatrix::new(3);
        for (column, value) in [(0, scale), (1, scale), (2, -scale)] {
            add_at(&mut matrix, 0, column, value);
        }
        let residual =
            residual_vector(&matrix, &[sign * scale, 0.0, 0.0], &[2.0 * sign; 3]).unwrap();
        assert_eq!(residual[0], sign * scale);
    }
}

#[test]
fn thermal_frame_reactions_recover_aggregate_subnormal_products_with_the_original_sign() {
    let quantum = f64::from_bits(1);
    for sign in [-1.0, 1.0] {
        let mut matrix = SparseMatrix::new(4);
        for column in 0..4 {
            add_at(&mut matrix, 0, column, 0.25);
        }
        let residual = residual_vector(&matrix, &[0.0; 4], &[sign * quantum; 4]).unwrap();
        assert_eq!(residual[0], sign * quantum);
        let residual = residual_vector(
            &matrix,
            &[sign * quantum, 0.0, 0.0, 0.0],
            &[sign * quantum; 4],
        )
        .unwrap();
        assert_eq!(residual[0], 0.0);
    }
}

#[test]
fn thermal_frame_reactions_recover_finite_coefficients_after_intermediate_overflow() {
    let directions = directions();
    for sign in [-1.0, 1.0] {
        let residual = [
            sign * 1.2e308,
            sign * directions[1][1] * 1.0e308,
            sign * directions[2][2] * 1.0e308,
        ];
        let coefficients = reaction_coefficients(&directions, residual).unwrap();
        for (actual, expected) in coefficients.into_iter().zip([1.2e308, 1.0e308, 1.0e308]) {
            close(actual, sign * expected);
        }
    }
}

#[test]
fn thermal_frame_reactions_recover_normal_product_tails_in_zero_or_small_residuals() {
    let delta = 2.0_f64.powi(-27);
    for sign in [-1.0, 1.0] {
        let mut matrix = SparseMatrix::new(1);
        add_at(&mut matrix, 0, 0, 1.0 + delta);
        let residual = residual_vector(&matrix, &[sign], &[sign * (1.0 - delta)]).unwrap();
        assert_eq!(residual[0], -sign * delta * delta);
        let residual = residual_vector(
            &matrix,
            &[sign * (1.0 - 2.0 * delta)],
            &[sign * (1.0 - delta)],
        )
        .unwrap();
        assert_eq!(residual[0], sign * (2.0 * delta - delta * delta));
    }
}

#[test]
fn thermal_frame_reactions_reject_erased_or_rounded_range_then_replay() {
    let quantum = f64::from_bits(1);
    let mut matrix = SparseMatrix::new(1);
    add_at(&mut matrix, 0, 0, 0.25);
    assert!(residual_vector(&matrix, &[0.0], &[quantum]).is_err());
    let mut overflow = SparseMatrix::new(1);
    add_at(&mut overflow, 0, 0, f64::MAX);
    assert!(residual_vector(&overflow, &[0.0], &[2.0]).is_err());
    assert_eq!(
        residual_vector(&matrix, &[0.0], &[4.0 * quantum]).unwrap(),
        [quantum]
    );

    let direction = normalized_direction([1.0, 1.0, 0.0]).unwrap();
    assert!(reaction_coefficients(&[direction], [quantum, quantum, 0.0]).is_err());
    assert!(reaction_coefficients(&[[1.0, 0.0, 0.0]], [quantum, f64::MAX, 0.0]).is_ok());
}

#[test]
fn thermal_frame_reactions_keep_independent_coordinate_scales_and_exact_subnormal_coefficients() {
    let directions = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    let quantum = f64::from_bits(1);
    for sign in [-1.0, 1.0] {
        let residual = [sign * quantum, sign * 2.0_f64.powi(1000), -sign * quantum];
        assert_eq!(
            reaction_coefficients(&directions, residual).unwrap(),
            residual
        );
    }
}

#[test]
fn thermal_frame_reactions_preserve_signed_scale_and_support_order() {
    let original = directions();
    for power in [-600, 0, 600] {
        let scale = 2.0_f64.powi(power);
        let expected = [2.0 * scale, -3.0 * scale, 4.0 * scale];
        let residual = std::array::from_fn(|axis| {
            (0..3)
                .map(|index| original[index][axis] * expected[index])
                .sum::<f64>()
        });
        for order in [[0, 1, 2], [2, 0, 1], [1, 2, 0]] {
            for sign in [-1.0, 1.0] {
                let directions = order.map(|index| original[index].map(|value| sign * value));
                let coefficients = reaction_coefficients(&directions, residual).unwrap();
                for (slot, &index) in order.iter().enumerate() {
                    close(coefficients[slot], sign * expected[index]);
                }
            }
        }
    }
}

#[test]
fn thermal_frame_reactions_reject_invalid_complete_inputs_without_panicking() {
    let matrix = SparseMatrix::new(1);
    for (rhs, solution) in [
        (vec![], vec![0.0]),
        (vec![0.0; 2], vec![0.0]),
        (vec![0.0], vec![]),
        (vec![f64::INFINITY], vec![0.0]),
    ] {
        assert!(
            std::panic::catch_unwind(|| residual_vector(&matrix, &rhs, &solution))
                .unwrap()
                .is_err()
        );
    }
    for residual in [[f64::NAN, 0.0, 0.0], [0.0, f64::INFINITY, 0.0]] {
        assert!(reaction_coefficients(&[[1.0, 0.0, 0.0]], residual).is_err());
    }
    assert!(reaction_coefficients(&[[1.0, 0.0, 0.0]; 4], [0.0; 3]).is_err());
    assert_eq!(
        residual_vector(&SparseMatrix::new(0), &[], &[]).unwrap(),
        Vec::<f64>::new()
    );
}

#[test]
fn thermal_frame_reactions_cancel_wide_residual_and_publication_then_replay() {
    let size = 2049;
    let mut matrix = SparseMatrix::new(size);
    for column in 0..size {
        add_at(&mut matrix, 0, column, 0.25);
    }
    let solution = vec![4.0; size];
    let force = vec![0.0; size];
    for (stage, completed) in [
        (SolverStage::SparseResidualRow, 1024),
        (SolverStage::SparseResidual, size as u64),
        (SolverStage::ResidualValidate, size as u64),
    ] {
        let control = SolverControl::default();
        let cancel = control.clone();
        let result = with_solver_observer(
            &control,
            move |point| {
                if point.stage == stage && point.completed_steps == completed {
                    cancel.request_cancel();
                }
            },
            || residual_vector(&matrix, &force, &solution),
        );
        assert!(result.unwrap_err().contains("cancelled"));
        assert!(control.was_interrupted());
        assert_eq!(
            residual_vector(&matrix, &force, &solution).unwrap()[0],
            size as f64
        );
    }
}

#[test]
fn thermal_frame_reactions_cancel_coefficient_recovery_then_replay() {
    let directions = directions();
    let residual = [2.0, -3.0, 4.0];
    let baseline = reaction_coefficients(&directions, residual).unwrap();
    for (stage, completed) in [
        (SolverStage::DenseFactor, 0),
        (SolverStage::DenseFactor, 3),
        (SolverStage::DenseSubstitution, 0),
        (SolverStage::DenseSubstitution, 6),
        (SolverStage::DenseSubstitution, 1),
    ] {
        let control = SolverControl::default();
        let cancel = control.clone();
        let result = with_solver_observer(
            &control,
            move |point| {
                if point.stage == stage && point.completed_steps == completed {
                    cancel.request_cancel();
                }
            },
            || reaction_coefficients(&directions, residual),
        );
        assert!(result.unwrap_err().contains("cancelled"));
        assert!(control.was_interrupted());
        assert_eq!(
            reaction_coefficients(&directions, residual).unwrap(),
            baseline
        );
    }
}
