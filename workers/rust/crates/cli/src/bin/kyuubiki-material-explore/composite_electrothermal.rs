use kyuubiki_headless_sdk::{
    CompositeDielectricLossSpec, CompositeElectrothermalFeedbackConvergence,
    CompositeElectrothermalFeedbackIteration, CompositeElectrothermalFeedbackSpec,
    CompositeElectrothermalLossProjection, CompositeThermalConductivityFeedbackIteration,
    apply_composite_dielectric_permittivity, assess_composite_electrothermal_feedback,
    composite_dielectric_mean_temperature, composite_feedback_iteration_converged,
    composite_feedback_relative_change, composite_heat_element_mean_temperature,
    project_composite_dielectric_loss_to_heat, temperature_adjusted_composite_heat_request,
    temperature_adjusted_composite_loss_spec,
};
use kyuubiki_protocol::{
    SolveElectrostaticPlaneQuad2dRequest, SolveElectrostaticPlaneQuad2dResult,
    SolveHeatPlaneQuad2dRequest, SolveHeatPlaneQuad2dResult,
};
use kyuubiki_solver::{solve_electrostatic_plane_quad_2d, solve_heat_plane_quad_2d};

pub(crate) struct CompositeElectrothermalSolve {
    pub electrostatic: SolveElectrostaticPlaneQuad2dResult,
    pub heat: SolveHeatPlaneQuad2dResult,
    pub loss_projection: CompositeElectrothermalLossProjection,
    pub feedback_convergence: CompositeElectrothermalFeedbackConvergence,
}

pub(crate) fn solve_composite_electrothermal_feedback(
    electrostatic_seed: &SolveElectrostaticPlaneQuad2dRequest,
    heat_seed: &SolveHeatPlaneQuad2dRequest,
    base_loss_spec: &CompositeDielectricLossSpec,
    feedback_spec: &CompositeElectrothermalFeedbackSpec,
) -> Result<CompositeElectrothermalSolve, String> {
    let mut coupling_temperature_c = base_loss_spec.reference_temperature_c;
    let mut previous_loss_w = None;
    let mut conductivity_temperatures_c = feedback_spec
        .thermal_conductivity_models
        .iter()
        .map(|model| (model.element_id.clone(), model.reference_temperature_c))
        .collect::<Vec<_>>();
    let mut previous_conductivities = None::<Vec<(String, f64)>>;
    let mut iterations = Vec::with_capacity(feedback_spec.max_iterations);
    let mut final_result = None;

    for iteration in 1..=feedback_spec.max_iterations {
        let loss_spec = temperature_adjusted_composite_loss_spec(
            base_loss_spec,
            feedback_spec,
            coupling_temperature_c,
        )?;
        let electrostatic_request =
            apply_composite_dielectric_permittivity(electrostatic_seed, &loss_spec)?;
        let electrostatic = solve_electrostatic_plane_quad_2d(&electrostatic_request)
            .map_err(|error| format!("composite electrothermal iteration {iteration} failed electrostatic solve: {error}"))?;
        let adjusted_heat_seed = temperature_adjusted_composite_heat_request(
            heat_seed,
            feedback_spec,
            &conductivity_temperatures_c,
        )?;
        let (heat_request, loss_projection) = project_composite_dielectric_loss_to_heat(
            &electrostatic,
            &adjusted_heat_seed,
            &loss_spec,
        )?;
        let heat = solve_heat_plane_quad_2d(&heat_request).map_err(|error| {
            format!("composite electrothermal iteration {iteration} failed heat solve: {error}")
        })?;
        let dielectric_mean_temperature_c =
            composite_dielectric_mean_temperature(&heat, &base_loss_spec.source_element_id)?;
        let thermal_conductivity_updates = feedback_spec
            .thermal_conductivity_models
            .iter()
            .map(|model| {
                let coupling_temperature_c = conductivity_temperatures_c
                    .iter()
                    .find(|(element_id, _)| element_id == &model.element_id)
                    .map(|(_, temperature)| *temperature)
                    .ok_or_else(|| {
                        format!("missing thermal coupling state for {}", model.element_id)
                    })?;
                let measured_mean_temperature_c =
                    composite_heat_element_mean_temperature(&heat, &model.element_id)?;
                let conductivity_w_mk = heat
                    .input
                    .elements
                    .iter()
                    .find(|element| element.id == model.element_id)
                    .map(|element| element.conductivity)
                    .ok_or_else(|| {
                        format!("missing feedback conductivity for {}", model.element_id)
                    })?;
                let conductivity_relative_change = previous_conductivities
                    .as_ref()
                    .and_then(|previous| {
                        previous
                            .iter()
                            .find(|(element_id, _)| element_id == &model.element_id)
                    })
                    .map(|(_, previous)| {
                        composite_feedback_relative_change(conductivity_w_mk, *previous)
                    });
                Ok(CompositeThermalConductivityFeedbackIteration {
                    element_id: model.element_id.clone(),
                    coupling_temperature_c,
                    measured_mean_temperature_c,
                    conductivity_w_mk,
                    conductivity_relative_change,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        let temperature_residual_c = thermal_conductivity_updates
            .iter()
            .map(|update| {
                (update.measured_mean_temperature_c - update.coupling_temperature_c).abs()
            })
            .fold(
                (dielectric_mean_temperature_c - coupling_temperature_c).abs(),
                f64::max,
            );
        let loss_relative_change = previous_loss_w.map(|previous| {
            composite_feedback_relative_change(loss_projection.total_loss_w, previous)
        });
        let max_conductivity_relative_change = thermal_conductivity_updates
            .iter()
            .map(|update| update.conductivity_relative_change)
            .collect::<Option<Vec<_>>>()
            .map(|changes| changes.into_iter().fold(0.0_f64, f64::max));
        let converged = composite_feedback_iteration_converged(
            feedback_spec,
            temperature_residual_c,
            loss_relative_change,
            max_conductivity_relative_change,
        );
        iterations.push(CompositeElectrothermalFeedbackIteration {
            iteration,
            coupling_temperature_c,
            dielectric_mean_temperature_c,
            relative_permittivity: loss_spec.relative_permittivity,
            loss_tangent: loss_spec.loss_tangent,
            total_loss_w: loss_projection.total_loss_w,
            max_temperature_c: heat.max_temperature,
            temperature_residual_c,
            loss_relative_change,
            max_conductivity_relative_change,
            thermal_conductivity_updates: thermal_conductivity_updates.clone(),
            converged,
        });
        previous_loss_w = Some(loss_projection.total_loss_w);
        previous_conductivities = Some(
            thermal_conductivity_updates
                .iter()
                .map(|update| (update.element_id.clone(), update.conductivity_w_mk))
                .collect(),
        );
        final_result = Some((electrostatic, heat, loss_projection));
        if converged {
            break;
        }
        coupling_temperature_c += feedback_spec.relaxation_factor
            * (dielectric_mean_temperature_c - coupling_temperature_c);
        for state in &mut conductivity_temperatures_c {
            let update = thermal_conductivity_updates
                .iter()
                .find(|update| update.element_id == state.0)
                .ok_or_else(|| format!("missing thermal feedback update for {}", state.0))?;
            state.1 +=
                feedback_spec.relaxation_factor * (update.measured_mean_temperature_c - state.1);
        }
    }

    let feedback_convergence = assess_composite_electrothermal_feedback(feedback_spec, iterations)?;
    let (electrostatic, heat, loss_projection) = final_result
        .ok_or_else(|| "composite electrothermal feedback produced no iteration".to_string())?;
    Ok(CompositeElectrothermalSolve {
        electrostatic,
        heat,
        loss_projection,
        feedback_convergence,
    })
}
