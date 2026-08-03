use kyuubiki_headless_sdk::{
    CompositeCurrentConductionFeedbackSpec, CompositeDielectricLossSpec,
    CompositeElectrothermalFeedbackSpec, CompositeThermalAlgebraicSample,
    CompositeThermalExpansionFeedbackSpec, composite_electrostatic_mesh_convergence_for_dielectric,
    composite_electrostatic_refinement_requests_for_dielectric,
    composite_heat_cross_validation_for_regional_loads,
    composite_heat_mesh_convergence_for_regional_loads,
    composite_heat_refinement_requests_for_regional_loads, composite_thermal_algebraic_series,
    composite_thermal_algebraic_validation, composite_thermal_constraint_sensitivity,
    composite_thermal_interface_graded_mesh_convergence,
    composite_thermal_interface_graded_refinement_requests,
    composite_thermal_interface_graded_stress_recovery,
    composite_thermal_interface_grading_assessment, composite_thermal_mesh_convergence,
    composite_thermal_recovered_stress_statistics, composite_thermal_refinement_requests,
    composite_thermal_regularized_mesh_convergence,
    composite_thermal_regularized_refinement_requests, composite_thermal_stress_recovery,
    project_composite_heat_to_thermal, project_composite_temperature_dependent_expansion,
};
use kyuubiki_protocol::{
    SolveCompositeThermoElectricPanelRequest, SolveElectricConductionPlaneQuad2dRequest,
    SolveElectrostaticPlaneQuad2dRequest, SolveHeatPlaneQuad2dRequest,
    SolveThermalPlaneQuad2dRequest,
};
use kyuubiki_solver::{
    SpdSolveOptions, ThermalPlaneQuadProfile, profile_thermal_plane_quad_2d_with_options,
    solve_electrostatic_plane_quad_2d, solve_heat_plane_quad_2d,
};
use serde_json::Value;

use crate::composite_runtime_feedback::solve_composite_electrothermal_feedback;

pub fn solve_composite_thermo_electric_panel(
    request: &SolveCompositeThermoElectricPanelRequest,
) -> Result<Value, String> {
    let electrostatic_request: SolveElectrostaticPlaneQuad2dRequest =
        decode(&request.electrostatic_model, "electrostatic_model")?;
    let electric_conduction_request: SolveElectricConductionPlaneQuad2dRequest = decode(
        &request.electric_conduction_model,
        "electric_conduction_model",
    )?;
    let heat_seed: SolveHeatPlaneQuad2dRequest = decode(&request.heat_model, "heat_model")?;
    let thermal_seed: SolveThermalPlaneQuad2dRequest =
        decode(&request.thermal_model, "thermal_model")?;
    let loss_spec: CompositeDielectricLossSpec =
        decode(&request.electrothermal_loss, "electrothermal_loss")?;
    let feedback_spec: CompositeElectrothermalFeedbackSpec =
        decode(&request.electrothermal_feedback, "electrothermal_feedback")?;
    let current_feedback_spec: CompositeCurrentConductionFeedbackSpec = decode(
        &request.electric_conduction_feedback,
        "electric_conduction_feedback",
    )?;
    let expansion_spec: CompositeThermalExpansionFeedbackSpec = decode(
        &request.thermal_expansion_feedback,
        "thermal_expansion_feedback",
    )?;
    let coupled = solve_composite_electrothermal_feedback(
        &electrostatic_request,
        &electric_conduction_request,
        &heat_seed,
        &loss_spec,
        &current_feedback_spec,
        &feedback_spec,
    )?;
    let electrostatic = coupled.electrostatic;
    let electric_conduction = coupled.electric_conduction;
    let heat = coupled.heat;
    let electrothermal_loss_projection = coupled.loss_projection;
    let joule_heating_projection = coupled.joule_heating_projection;
    let electrothermal_feedback_convergence = coupled.feedback_convergence;
    let dielectric_relative_permittivity =
        composite_dielectric_relative_permittivity(&electrostatic.input)?;
    let mut mesh_fields = vec![(1, electrostatic.max_electric_field)];
    for (level, refined_request) in
        composite_electrostatic_refinement_requests_for_dielectric(dielectric_relative_permittivity)
            .into_iter()
            .filter(|(level, _)| *level > 1)
    {
        let result = solve_electrostatic_plane_quad_2d(&refined_request).map_err(|error| {
            format!("composite electrostatic mesh level {level} solve failed: {error}")
        })?;
        mesh_fields.push((level, result.max_electric_field));
    }
    let electrostatic_mesh_convergence = composite_electrostatic_mesh_convergence_for_dielectric(
        dielectric_relative_permittivity,
        &mesh_fields,
    );
    let heat_conductivities = composite_heat_conductivities(&heat.input)?;
    let regional_heat_loads_w = [
        joule_heating_projection.total_joule_loss_w,
        electrothermal_loss_projection.total_loss_w,
        0.0,
    ];
    let heat_cross_validation = composite_heat_cross_validation_for_regional_loads(
        heat_conductivities,
        regional_heat_loads_w,
        Some(heat.max_temperature),
    );
    let mut heat_mesh_temperatures = vec![(1, heat.max_temperature)];
    for (level, refined_request) in composite_heat_refinement_requests_for_regional_loads(
        heat_conductivities,
        regional_heat_loads_w,
    )?
    .into_iter()
    .filter(|(level, _)| *level > 1)
    {
        let result = solve_heat_plane_quad_2d(&refined_request)
            .map_err(|error| format!("composite heat mesh level {level} solve failed: {error}"))?;
        heat_mesh_temperatures.push((level, result.max_temperature));
    }
    let heat_mesh_convergence = composite_heat_mesh_convergence_for_regional_loads(
        heat_conductivities,
        regional_heat_loads_w,
        &heat_mesh_temperatures,
    );
    let (thermal_request, heat_to_thermal_projection) =
        project_composite_heat_to_thermal(&heat, &thermal_seed, loss_spec.reference_temperature_c)?;
    let (thermal_request, thermal_expansion_projection) =
        project_composite_temperature_dependent_expansion(
            &heat,
            &thermal_request,
            &expansion_spec,
        )?;
    let thermal_profile =
        profile_thermal_plane_quad_2d_with_options(&thermal_request, SpdSolveOptions::default())
            .map_err(|error| format!("composite thermal solve failed: {error}"))?;
    let mut thermal_algebraic_samples = vec![thermal_algebraic_sample(1, &thermal_profile)];
    let thermal = thermal_profile.result;
    let mut thermal_stress_statistics =
        vec![(1, composite_thermal_recovered_stress_statistics(&thermal)?)];
    let mut thermal_mesh_samples = vec![(
        1,
        thermal.max_displacement,
        thermal.total_strain_energy,
        thermal.max_stress,
    )];
    for (level, refined_request) in composite_thermal_refinement_requests(&thermal_request)?
        .into_iter()
        .filter(|(level, _)| *level > 1)
    {
        let profile = profile_thermal_plane_quad_2d_with_options(
            &refined_request,
            SpdSolveOptions::default(),
        )
        .map_err(|error| format!("composite thermal mesh level {level} solve failed: {error}"))?;
        thermal_algebraic_samples.push(thermal_algebraic_sample(level, &profile));
        let result = profile.result;
        thermal_mesh_samples.push((
            level,
            result.max_displacement,
            result.total_strain_energy,
            result.max_stress,
        ));
        thermal_stress_statistics.push((
            level,
            composite_thermal_recovered_stress_statistics(&result)?,
        ));
    }
    let mut thermal_mesh_convergence = composite_thermal_mesh_convergence(&thermal_mesh_samples);
    let thermal_stress_recovery = composite_thermal_stress_recovery(&thermal_stress_statistics);
    let mut regularized_mesh_samples = Vec::new();
    let mut regularized_algebraic_samples = Vec::new();
    for (level, refined_request) in
        composite_thermal_regularized_refinement_requests(&thermal_request)?
    {
        let profile = profile_thermal_plane_quad_2d_with_options(
            &refined_request,
            SpdSolveOptions::default(),
        )
        .map_err(|error| {
            format!("composite regularized thermal mesh level {level} solve failed: {error}")
        })?;
        regularized_algebraic_samples.push(thermal_algebraic_sample(level, &profile));
        let result = profile.result;
        regularized_mesh_samples.push((
            level,
            result.max_displacement,
            result.total_strain_energy,
            result.max_stress,
        ));
    }
    let thermal_constraint_regularized_mesh_convergence =
        composite_thermal_regularized_mesh_convergence(&regularized_mesh_samples);
    let thermal_constraint_sensitivity = composite_thermal_constraint_sensitivity(
        &thermal_mesh_convergence,
        &thermal_constraint_regularized_mesh_convergence,
    );
    let mut graded_mesh_samples = vec![thermal_mesh_samples[0]];
    let mut graded_stress_statistics = vec![thermal_stress_statistics[0].clone()];
    let mut graded_algebraic_samples = vec![thermal_algebraic_samples[0].clone()];
    for (level, refined_request) in
        composite_thermal_interface_graded_refinement_requests(&thermal_request)?
            .into_iter()
            .filter(|(level, _)| *level > 1)
    {
        let profile = profile_thermal_plane_quad_2d_with_options(
            &refined_request,
            SpdSolveOptions::default(),
        )
        .map_err(|error| {
            format!("composite interface-graded thermal mesh level {level} failed: {error}")
        })?;
        graded_algebraic_samples.push(thermal_algebraic_sample(level, &profile));
        let result = profile.result;
        graded_mesh_samples.push((
            level,
            result.max_displacement,
            result.total_strain_energy,
            result.max_stress,
        ));
        graded_stress_statistics.push((
            level,
            composite_thermal_recovered_stress_statistics(&result)?,
        ));
    }
    let thermal_interface_graded_mesh_convergence =
        composite_thermal_interface_graded_mesh_convergence(&graded_mesh_samples);
    let thermal_interface_graded_stress_recovery =
        composite_thermal_interface_graded_stress_recovery(&graded_stress_statistics);
    let thermal_interface_grading_assessment = composite_thermal_interface_grading_assessment(
        &thermal_mesh_convergence,
        &thermal_stress_recovery,
        thermal_interface_graded_mesh_convergence,
        thermal_interface_graded_stress_recovery,
    );
    let thermal_algebraic_validation = composite_thermal_algebraic_validation(vec![
        composite_thermal_algebraic_series("uniform_full_edge_clamp", thermal_algebraic_samples),
        composite_thermal_algebraic_series(
            "uniform_regularized_restraint",
            regularized_algebraic_samples,
        ),
        composite_thermal_algebraic_series(
            "interface_graded_full_edge_clamp",
            graded_algebraic_samples,
        ),
    ]);
    thermal_mesh_convergence.algebraic_validation = thermal_algebraic_validation;

    Ok(serde_json::json!({
        "schema_version": "kyuubiki.composite-thermo-electric-panel-result/v1",
        "research": request.research.clone(),
        "electrostatic": electrostatic,
        "electric_conduction": electric_conduction,
        "electrostatic_mesh_convergence": electrostatic_mesh_convergence,
        "electrothermal_loss_projection": electrothermal_loss_projection,
        "joule_heating_projection": joule_heating_projection,
        "electrothermal_feedback_convergence": electrothermal_feedback_convergence,
        "heat": heat,
        "heat_cross_validation": heat_cross_validation,
        "heat_mesh_convergence": heat_mesh_convergence,
        "heat_to_thermal_projection": heat_to_thermal_projection,
        "thermal_expansion_projection": thermal_expansion_projection,
        "thermal": thermal,
        "thermal_mesh_convergence": thermal_mesh_convergence,
        "thermal_stress_recovery": thermal_stress_recovery,
        "thermal_constraint_regularized_mesh_convergence": thermal_constraint_regularized_mesh_convergence,
        "thermal_constraint_sensitivity": thermal_constraint_sensitivity,
        "thermal_interface_grading_assessment": thermal_interface_grading_assessment,
    }))
}

fn decode<T: serde::de::DeserializeOwned>(payload: &Value, field: &str) -> Result<T, String> {
    serde_json::from_value(payload.clone())
        .map_err(|error| format!("invalid composite {field}: {error}"))
}

fn thermal_algebraic_sample(
    level: usize,
    profile: &ThermalPlaneQuadProfile,
) -> CompositeThermalAlgebraicSample {
    CompositeThermalAlgebraicSample::new(
        level,
        profile.result.nodes.len(),
        profile.result.elements.len(),
        profile.solver_iterations,
        profile.solver_matrix_non_zero_count,
        profile.solver_residual_norm,
        profile.solver_rhs_norm,
    )
}

fn composite_dielectric_relative_permittivity(
    request: &SolveElectrostaticPlaneQuad2dRequest,
) -> Result<f64, String> {
    let value = request
        .elements
        .iter()
        .find(|element| element.id == "dielectric_core")
        .or_else(|| request.elements.get(request.elements.len() / 2))
        .map(|element| element.permittivity)
        .ok_or_else(|| "composite electrostatic model has no dielectric element".to_string())?;
    if !value.is_finite() || value <= 0.0 {
        return Err(
            "composite electrostatic model dielectric permittivity must be finite and positive"
                .to_string(),
        );
    }
    Ok(value)
}

fn composite_heat_conductivities(
    request: &SolveHeatPlaneQuad2dRequest,
) -> Result<[f64; 3], String> {
    let conductivity = |id: &str, fallback: usize| {
        request
            .elements
            .iter()
            .find(|element| element.id == id)
            .or_else(|| request.elements.get(fallback))
            .map(|element| element.conductivity)
    };
    let values = [
        conductivity("conductor_left", 0),
        conductivity("dielectric_core", request.elements.len() / 2),
        conductivity("substrate_right", request.elements.len().saturating_sub(1)),
    ];
    let [Some(conductor), Some(dielectric), Some(substrate)] = values else {
        return Err("composite heat model must contain three material regions".to_string());
    };
    let conductivities = [conductor, dielectric, substrate];
    if conductivities
        .iter()
        .any(|value| !value.is_finite() || *value <= 0.0)
    {
        return Err("composite heat model conductivities must be finite and positive".to_string());
    }
    Ok(conductivities)
}
