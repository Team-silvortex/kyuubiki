use crate::linear_algebra::{SparseMatrix, stable_l2_norm};
use crate::solver_control::{SolverStage, checkpoint, checkpoint_chunk};

const EQUILIBRIUM_TOLERANCE: f64 = 1e-8;

fn validate_shape(shape: &[f64]) -> Result<(), String> {
    if shape.is_empty() || !shape.len().is_multiple_of(3) {
        return Err("frame 2d stability shape must contain complete node DOFs".into());
    }
    for (index, value) in shape.iter().enumerate() {
        if !value.is_finite() {
            return Err(format!("frame 2d stability DOF {index} is non-finite"));
        }
        checkpoint_chunk(SolverStage::StabilityRecovery, index + 1, shape.len())?;
    }
    Ok(())
}

fn translation_scale(shape: &[f64]) -> f64 {
    shape.chunks_exact(3).fold(0.0_f64, |scale, node| {
        scale.max(node[0].abs()).max(node[1].abs())
    })
}

// Try equivalent orders before rejecting a representable scaled product.
fn scaled_product_ratio(left: f64, right: f64, divisor: f64) -> f64 {
    let direct = (left / divisor) * right;
    if direct.is_finite() && direct != 0.0 {
        return direct;
    }
    let product_first = (left * right) / divisor;
    if product_first.is_finite() && product_first != 0.0 {
        return product_first;
    }
    let ratio_first = left * (right / divisor);
    if ratio_first.is_finite() && ratio_first != 0.0 {
        return ratio_first;
    }
    if [direct, product_first, ratio_first].contains(&0.0) {
        0.0
    } else {
        direct
    }
}

pub(crate) fn scale_imperfection(mode: &[f64], amplitude: f64) -> Result<Vec<f64>, String> {
    checkpoint(SolverStage::StabilityRecovery, 0)?;
    validate_shape(mode)?;
    let scale = translation_scale(mode);
    if scale == 0.0 {
        return Err("frame 2d p-delta selected mode has no translational imperfection".into());
    }
    let peak = mode
        .chunks_exact(3)
        .map(|node| (node[0] / scale).hypot(node[1] / scale))
        .fold(0.0_f64, f64::max);
    let target = amplitude / peak;
    let mut shape = Vec::with_capacity(mode.len());
    for (index, &value) in mode.iter().enumerate() {
        let value = scaled_product_ratio(value, target, scale);
        if !value.is_finite() {
            return Err(format!(
                "frame 2d p-delta scaled imperfection DOF {index} is non-finite"
            ));
        }
        shape.push(value);
        checkpoint_chunk(
            SolverStage::StabilityRecovery,
            mode.len() + index + 1,
            mode.len() * 2,
        )?;
    }
    if translation_scale(&shape) == 0.0 {
        return Err("frame 2d p-delta scaled translational imperfection underflows".into());
    }
    Ok(shape)
}

pub(crate) fn imperfection_amplification(
    initial: &[f64],
    displacement: &[f64],
) -> Result<f64, String> {
    validate_shape(initial)?;
    validate_shape(displacement)?;
    if initial.len() != displacement.len() {
        return Err("frame 2d stability displacement/imperfection dimensions differ".into());
    }
    let scale = translation_scale(initial);
    if scale == 0.0 {
        return Err("frame 2d stability imperfection has no translation".into());
    }
    let denominator = initial
        .chunks_exact(3)
        .map(|node| (node[0] / scale).powi(2) + (node[1] / scale).powi(2))
        .sum::<f64>();
    let mut projection = 0.0;
    for (index, (&initial, &increment)) in initial.iter().zip(displacement).enumerate() {
        if index % 3 != 2 && initial != 0.0 {
            let weight = (initial / scale) / denominator;
            projection += scaled_product_ratio(weight, increment, scale);
        }
        checkpoint_chunk(
            SolverStage::StabilityRecovery,
            index + 1,
            displacement.len(),
        )?;
    }
    let amplification = 1.0 + projection;
    if !amplification.is_finite() {
        return Err("frame 2d stability imperfection amplification is non-finite".into());
    }
    Ok(amplification)
}

pub(crate) fn max_translation(displacements: &[f64]) -> Result<f64, String> {
    validate_shape(displacements)?;
    let maximum = displacements
        .chunks_exact(3)
        .map(|node| node[0].hypot(node[1]))
        .fold(0.0_f64, f64::max);
    if !maximum.is_finite() {
        return Err("frame 2d stability translation magnitude is non-finite".into());
    }
    Ok(maximum)
}

pub(crate) fn linearized_residual(
    matrix: &SparseMatrix,
    solution: &[f64],
    rhs: &[f64],
) -> Result<f64, String> {
    checkpoint(SolverStage::ResidualValidate, 0)?;
    if matrix.size() != solution.len() || solution.len() != rhs.len() {
        return Err("frame 2d p-delta residual dimensions differ".into());
    }
    let mut residuals = Vec::with_capacity(rhs.len());
    for (row, &expected) in rhs.iter().enumerate() {
        let mut scale = expected.abs();
        if !expected.is_finite() {
            return Err(format!("frame 2d p-delta row {row} load is non-finite"));
        }
        for &(column, coefficient) in matrix.row_entries(row) {
            let term = coefficient * solution[column];
            if !term.is_finite() {
                return Err(format!("frame 2d p-delta row {row} recovery is non-finite"));
            }
            scale = scale.max(term.abs());
        }
        let mut residual = 0.0;
        if scale != 0.0 {
            let mut actual = 0.0;
            let mut magnitude = expected.abs() / scale;
            for &(column, coefficient) in matrix.row_entries(row) {
                let term = (coefficient * solution[column]) / scale;
                actual += term;
                magnitude += term.abs();
            }
            let difference = expected / scale - actual;
            let backward_error = difference.abs() / magnitude;
            if !backward_error.is_finite() || backward_error > EQUILIBRIUM_TOLERANCE {
                return Err(format!(
                    "frame 2d p-delta row {row} failed equilibrium validation ({backward_error:.6e}, residual={:.6e}, equation_scale={:.6e})",
                    difference * scale,
                    magnitude * scale
                ));
            }
            residual = difference * scale;
        }
        residuals.push(residual);
        checkpoint_chunk(SolverStage::ResidualValidate, row + 1, rhs.len())?;
    }
    // Preserve ||Ku-f|| / max(||f||, 1), without squaring dimensional loads.
    let scale = rhs.iter().map(|value| value.abs()).fold(1.0_f64, f64::max);
    let norm = stable_l2_norm(residuals.iter().map(|value| value / scale))
        / stable_l2_norm(rhs.iter().map(|value| value / scale)).max(1.0 / scale);
    if !norm.is_finite() {
        return Err("frame 2d p-delta residual norm is non-finite".into());
    }
    Ok(norm)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::linear_algebra::add_at;

    #[test]
    fn normalization_handles_large_translations_and_extreme_rotational_ratios() {
        let shape = scale_imperfection(&[f64::MAX, f64::MAX, f64::MAX], 1.0).unwrap();
        assert!((shape[0] - 1.0 / 2.0_f64.sqrt()).abs() < 1e-15);
        assert!((max_translation(&shape).unwrap() - 1.0).abs() < 1e-15);
        let shape = scale_imperfection(&[1e-200, 0.0, 1e308], 1e-200).unwrap();
        assert!((shape[2] / 1e308 - 1.0).abs() < 1e-15);
        let shape = scale_imperfection(&[1e200, 0.0, 1e-200], 1e200).unwrap();
        assert!((shape[2] / 1e-200 - 1.0).abs() < 1e-15);
    }

    #[test]
    fn amplification_ignores_rotations_and_orthogonal_motion_across_scales() {
        for scale in [1e-320, 1e-200, 1.0, 1e200] {
            let initial = [scale, 0.0, 1e308];
            let displacement = [2.0 * scale, 1e308, -1e308];
            let amplification = imperfection_amplification(&initial, &displacement).unwrap();
            assert!((amplification - 3.0).abs() < 1e-14);
            assert!(max_translation(&displacement).unwrap().is_finite());
        }
    }

    #[test]
    fn invalid_or_unrepresentable_metrics_are_errors_not_silent_nan() {
        assert!(scale_imperfection(&[0.0, 0.0, 1.0], 1.0).is_err());
        assert!(scale_imperfection(&[1e-200, 0.0, 1e308], 1.0).is_err());
        assert!(max_translation(&[f64::MAX, f64::MAX, 0.0]).is_err());
        assert!(max_translation(&[1.0, 0.0, f64::NAN]).is_err());
        assert!(max_translation(&[1.0]).is_err());
        assert!(imperfection_amplification(&[1.0, 0.0, 0.0], &[]).is_err());
        assert!(imperfection_amplification(&[1e-200, 0.0, 0.0], &[1e308, 0.0, 0.0]).is_err());
    }

    #[test]
    fn residual_validation_cannot_hide_a_weak_equation_beside_a_strong_one() {
        let mut matrix = SparseMatrix::new(2);
        add_at(&mut matrix, 0, 0, 1e200);
        add_at(&mut matrix, 1, 1, 1e-200);
        assert_eq!(
            linearized_residual(&matrix, &[1.0, 1.0], &[1e200, 1e-200]).unwrap(),
            0.0
        );
        let error = linearized_residual(&matrix, &[1.0, 0.0], &[1e200, 1e-200]).unwrap_err();
        assert!(
            error.contains("row 1") && error.contains("equilibrium"),
            "{error}"
        );
    }

    #[test]
    fn residual_norm_handles_overflowing_raw_norms_and_rejects_nonfinite_terms() {
        let mut matrix = SparseMatrix::new(2);
        add_at(&mut matrix, 0, 0, 1e308);
        add_at(&mut matrix, 1, 1, 1e308);
        let norm = linearized_residual(&matrix, &[1.0 + 1e-12; 2], &[1e308; 2]).unwrap();
        assert!(norm > 5e-13 && norm < 2e-12);
        assert!(linearized_residual(&matrix, &[2.0; 2], &[1e308; 2]).is_err());
        assert!(linearized_residual(&matrix, &[f64::NAN; 2], &[1e308; 2]).is_err());
    }

    #[test]
    fn nonlinear_residual_does_not_turn_nonfinite_input_or_scale_overflow_into_convergence() {
        use crate::frame_2d_corotational::normalized_residual;
        assert_eq!(normalized_residual(&[1e308], &[1e308], 2.0), 0.5);
        for (residual, external, load_factor) in [
            (f64::NAN, 1.0, 1.0),
            (0.0, f64::INFINITY, 1.0),
            (0.0, 1.0, f64::NAN),
        ] {
            assert_eq!(
                normalized_residual(&[residual], &[external], load_factor),
                f64::INFINITY
            );
        }
    }
}
