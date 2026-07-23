use crate::frame_2d_corotational::{
    correct_corotational_equilibrium, correct_parameter_continuation_equilibrium,
};
use crate::frame_2d_stability::Frame2dStabilitySystem;
use kyuubiki_protocol::{Frame2dPDeltaContinuationState, SolveFrame2dPDeltaRequest};

pub(crate) struct PreparedContinuationState {
    pub(crate) displacement: Vec<f64>,
    pub(crate) load_factor: f64,
    pub(crate) displacement_increment: Vec<f64>,
    pub(crate) load_increment: f64,
    pub(crate) correction_norm: Option<f64>,
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn prepare_continuation_state(
    request: &SolveFrame2dPDeltaRequest,
    positions: &[(f64, f64)],
    system: &Frame2dStabilitySystem,
    free: &[usize],
    load_scale: f64,
    max_iterations: usize,
    tolerance: f64,
) -> Result<PreparedContinuationState, String> {
    let Some(input) = &request.continuation_state else {
        return Ok(PreparedContinuationState {
            displacement: vec![0.0; positions.len() * 3],
            load_factor: 0.0,
            displacement_increment: vec![0.0; free.len()],
            load_increment: 0.0,
            correction_norm: None,
        });
    };
    validate_continuation_state(input, positions.len() * 3, &system.constrained_dofs)?;
    let displacement_increment = free
        .iter()
        .map(|&dof| input.displacement_increment[dof])
        .collect::<Vec<_>>();
    let fixed_load_correction = correct_corotational_equilibrium(
        positions,
        &request.buckling.frame.elements,
        system,
        &input.displacements,
        input.load_factor,
        max_iterations,
        tolerance,
    )?;
    let fixed_load_correction = fixed_load_correction.filter(|corrected| {
        fixed_correction_retains_identity(
            &input.displacements,
            corrected,
            &displacement_increment,
            free,
        ) || request.imperfection_shape.as_deref().is_some_and(|shape| {
            fixed_correction_retains_target_shape(&input.displacements, corrected, shape)
        })
    });
    let (displacement, load_factor) = match fixed_load_correction {
        Some(displacement) => (displacement, input.load_factor),
        None => correct_parameter_continuation_equilibrium(
            positions,
            &request.buckling.frame.elements,
            system,
            &input.displacements,
            input.load_factor,
            &displacement_increment,
            input.load_factor_increment,
            load_scale,
            max_iterations,
            tolerance,
        )?
        .ok_or_else(|| {
            "frame 2d p-delta continuation state could not be projected to nearby equilibrium"
                .to_string()
        })?,
    };
    let correction_norm = displacement
        .iter()
        .zip(&input.displacements)
        .map(|(corrected, requested)| (corrected - requested).powi(2))
        .sum::<f64>()
        + (load_scale * (load_factor - input.load_factor)).powi(2);
    Ok(PreparedContinuationState {
        displacement,
        load_factor,
        displacement_increment,
        load_increment: input.load_factor_increment,
        correction_norm: Some(correction_norm.sqrt()),
    })
}

fn fixed_correction_retains_identity(
    imported: &[f64],
    corrected: &[f64],
    reduced_increment: &[f64],
    free: &[usize],
) -> bool {
    let imported_norm = free
        .iter()
        .map(|&dof| imported[dof] * imported[dof])
        .sum::<f64>()
        .sqrt();
    if imported_norm <= 1.0e-14 {
        return true;
    }
    let corrected_norm = free
        .iter()
        .map(|&dof| corrected[dof] * corrected[dof])
        .sum::<f64>()
        .sqrt();
    let increment_norm = reduced_increment
        .iter()
        .map(|value| value * value)
        .sum::<f64>()
        .sqrt();
    let imported_projection = free
        .iter()
        .zip(reduced_increment)
        .map(|(&dof, increment)| imported[dof] * increment)
        .sum::<f64>();
    let corrected_projection = free
        .iter()
        .zip(reduced_increment)
        .map(|(&dof, increment)| corrected[dof] * increment)
        .sum::<f64>();
    let projection_scale = imported_norm * increment_norm;
    if projection_scale > 1.0e-14 && imported_projection.abs() > projection_scale * 1.0e-6 {
        return corrected_projection * imported_projection > 0.0
            && corrected_projection.abs() >= imported_projection.abs() * 0.25;
    }
    if corrected_norm < imported_norm * 0.25 {
        return false;
    }
    let overlap = free
        .iter()
        .map(|&dof| imported[dof] * corrected[dof])
        .sum::<f64>()
        .abs()
        / (imported_norm * corrected_norm);
    overlap >= 0.5
}

fn fixed_correction_retains_target_shape(
    imported: &[f64],
    corrected: &[f64],
    target_shape: &[f64],
) -> bool {
    if imported.len() != corrected.len() || imported.len() != target_shape.len() {
        return false;
    }
    let shape_scale = target_shape
        .iter()
        .map(|value| value.abs())
        .fold(0.0_f64, f64::max);
    if shape_scale <= f64::EPSILON {
        return false;
    }
    let support_tolerance = shape_scale * 1.0e-12;
    let (imported_norm, corrected_norm) = imported
        .iter()
        .zip(corrected)
        .zip(target_shape)
        .filter(|((_, _), shape)| shape.abs() > support_tolerance)
        .fold(
            (0.0, 0.0),
            |(imported_norm, corrected_norm), ((imported, corrected), _)| {
                (
                    imported_norm + imported * imported,
                    corrected_norm + corrected * corrected,
                )
            },
        );
    let imported_norm = imported_norm.sqrt();
    let corrected_norm = corrected_norm.sqrt();
    imported_norm > 1.0e-14 && corrected_norm >= imported_norm * 1.0e-3
}

fn validate_continuation_state(
    state: &Frame2dPDeltaContinuationState,
    dof_count: usize,
    constrained_dofs: &[usize],
) -> Result<(), String> {
    if state.displacements.len() != dof_count {
        return Err(format!(
            "frame 2d p-delta continuation displacements must contain exactly {dof_count} values"
        ));
    }
    if state.displacement_increment.len() != dof_count {
        return Err(format!(
            "frame 2d p-delta continuation displacement_increment must contain exactly {dof_count} values"
        ));
    }
    if !state.load_factor.is_finite() || !state.load_factor_increment.is_finite() {
        return Err("frame 2d p-delta continuation load factors must be finite".into());
    }
    if state
        .displacements
        .iter()
        .chain(&state.displacement_increment)
        .any(|value| !value.is_finite())
    {
        return Err("frame 2d p-delta continuation vectors must be finite".into());
    }
    if constrained_dofs.iter().any(|&dof| {
        state.displacements[dof].abs() > 1.0e-12
            || state.displacement_increment[dof].abs() > 1.0e-12
    }) {
        return Err(
            "frame 2d p-delta continuation state must be zero on constrained degrees of freedom"
                .into(),
        );
    }
    Ok(())
}

pub(crate) fn export_continuation_state(
    displacement: &[f64],
    load_factor: f64,
    reduced_displacement_increment: &[f64],
    load_increment: f64,
    free: &[usize],
) -> Frame2dPDeltaContinuationState {
    let mut displacement_increment = vec![0.0; displacement.len()];
    for (&dof, &increment) in free.iter().zip(reduced_displacement_increment) {
        displacement_increment[dof] = increment;
    }
    Frame2dPDeltaContinuationState {
        displacements: displacement.to_vec(),
        load_factor,
        displacement_increment,
        load_factor_increment: load_increment,
    }
}

#[cfg(test)]
mod tests {
    use super::{fixed_correction_retains_identity, fixed_correction_retains_target_shape};

    #[test]
    fn fixed_correction_rejects_collapsed_and_reversed_branch_roots() {
        let free = [0, 1, 2];
        let imported = [1.0, 0.5, 0.0];
        let increment = [0.2, 0.1, 0.0];
        assert!(fixed_correction_retains_identity(
            &imported,
            &[0.8, 0.4, 0.0],
            &increment,
            &free
        ));
        assert!(!fixed_correction_retains_identity(
            &imported,
            &[0.0, 0.0, 2.0],
            &increment,
            &free
        ));
        assert!(!fixed_correction_retains_identity(
            &imported,
            &[-0.8, -0.4, 0.0],
            &increment,
            &free
        ));
    }

    #[test]
    fn near_orthogonal_tangents_fall_back_to_displacement_overlap() {
        let free = [0, 1];
        assert!(fixed_correction_retains_identity(
            &[1.0, 0.0],
            &[0.8, 0.1],
            &[0.0, 1.0],
            &free
        ));
        assert!(!fixed_correction_retains_identity(
            &[1.0, 0.0],
            &[0.0, 1.0],
            &[0.0, 1.0],
            &free
        ));
    }

    #[test]
    fn explicit_target_shape_support_rejects_only_catastrophic_amplitude_collapse() {
        let imported = [0.0, 0.0, 1.0, 0.0];
        let target = [0.0, 0.0, 1.0, 0.0];
        assert!(fixed_correction_retains_target_shape(
            &imported,
            &[0.0, 7.0, 0.8, -3.0],
            &target
        ));
        assert!(!fixed_correction_retains_target_shape(
            &imported,
            &[0.0, 7.0, 0.0001, -3.0],
            &target
        ));
    }
}
