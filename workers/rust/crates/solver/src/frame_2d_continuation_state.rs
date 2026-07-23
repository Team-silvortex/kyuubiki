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
