use crate::CompositeDielectricLossSpec;
use kyuubiki_protocol::{
    SolveElectrostaticPlaneQuad2dRequest, SolveHeatPlaneQuad2dRequest, SolveHeatPlaneQuad2dResult,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub const COMPOSITE_ELECTROTHERMAL_FEEDBACK_SCHEMA_VERSION: &str =
    "kyuubiki.composite-electrothermal-feedback-convergence/v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompositeElectrothermalFeedbackSpec {
    pub max_iterations: usize,
    pub relaxation_factor: f64,
    pub temperature_residual_tolerance_c: f64,
    pub loss_relative_change_tolerance: f64,
    pub conductivity_relative_change_tolerance: f64,
    pub relative_permittivity_temperature_coefficient_1_k: f64,
    pub loss_tangent_temperature_coefficient_1_k: f64,
    pub thermal_conductivity_models: Vec<CompositeThermalConductivityFeedbackModel>,
    pub parameter_source: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompositeThermalConductivityFeedbackModel {
    pub element_id: String,
    pub reference_temperature_c: f64,
    pub temperature_coefficient_1_k: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompositeThermalConductivityFeedbackIteration {
    pub element_id: String,
    pub coupling_temperature_c: f64,
    pub measured_mean_temperature_c: f64,
    pub conductivity_w_mk: f64,
    pub conductivity_relative_change: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompositeElectrothermalFeedbackIteration {
    pub iteration: usize,
    pub coupling_temperature_c: f64,
    pub dielectric_mean_temperature_c: f64,
    pub relative_permittivity: f64,
    pub loss_tangent: f64,
    pub total_loss_w: f64,
    pub total_joule_loss_w: f64,
    pub max_temperature_c: f64,
    pub temperature_residual_c: f64,
    pub loss_relative_change: Option<f64>,
    pub max_conductivity_relative_change: Option<f64>,
    pub thermal_conductivity_updates: Vec<CompositeThermalConductivityFeedbackIteration>,
    pub converged: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompositeElectrothermalFeedbackConvergence {
    pub schema_version: String,
    pub method: String,
    pub status: String,
    pub converged: bool,
    pub iteration_count: usize,
    pub max_iterations: usize,
    pub relaxation_factor: f64,
    pub temperature_residual_tolerance_c: f64,
    pub loss_relative_change_tolerance: f64,
    pub conductivity_relative_change_tolerance: f64,
    pub parameter_source: String,
    pub final_temperature_residual_c: Option<f64>,
    pub final_loss_relative_change: Option<f64>,
    pub final_max_conductivity_relative_change: Option<f64>,
    pub iterations: Vec<CompositeElectrothermalFeedbackIteration>,
}

pub fn temperature_adjusted_composite_loss_spec(
    base: &CompositeDielectricLossSpec,
    feedback: &CompositeElectrothermalFeedbackSpec,
    coupling_temperature_c: f64,
) -> Result<CompositeDielectricLossSpec, String> {
    validate_feedback_spec(feedback)?;
    if !coupling_temperature_c.is_finite() {
        return Err("electrothermal coupling temperature must be finite".to_string());
    }
    let temperature_delta = coupling_temperature_c - base.reference_temperature_c;
    let permittivity_scale =
        1.0 + feedback.relative_permittivity_temperature_coefficient_1_k * temperature_delta;
    let loss_scale = 1.0 + feedback.loss_tangent_temperature_coefficient_1_k * temperature_delta;
    let relative_permittivity = base.relative_permittivity * permittivity_scale;
    let loss_tangent = base.loss_tangent * loss_scale;
    if !relative_permittivity.is_finite() || relative_permittivity <= 0.0 {
        return Err("temperature feedback produced invalid relative permittivity".to_string());
    }
    if !loss_tangent.is_finite() || loss_tangent < 0.0 {
        return Err("temperature feedback produced invalid loss tangent".to_string());
    }
    Ok(CompositeDielectricLossSpec {
        source_element_id: base.source_element_id.clone(),
        frequency_hz: base.frequency_hz,
        relative_permittivity,
        loss_tangent,
        reference_temperature_c: base.reference_temperature_c,
    })
}

pub fn apply_composite_dielectric_permittivity(
    seed: &SolveElectrostaticPlaneQuad2dRequest,
    spec: &CompositeDielectricLossSpec,
) -> Result<SolveElectrostaticPlaneQuad2dRequest, String> {
    let mut request = seed.clone();
    let element = request
        .elements
        .iter_mut()
        .find(|element| element.id == spec.source_element_id)
        .ok_or_else(|| {
            format!(
                "electrostatic feedback model is missing element {}",
                spec.source_element_id
            )
        })?;
    element.permittivity = spec.relative_permittivity;
    Ok(request)
}

pub fn temperature_adjusted_composite_heat_request(
    base: &SolveHeatPlaneQuad2dRequest,
    feedback: &CompositeElectrothermalFeedbackSpec,
    coupling_temperatures_c: &[(String, f64)],
) -> Result<SolveHeatPlaneQuad2dRequest, String> {
    validate_feedback_spec(feedback)?;
    if coupling_temperatures_c.len() != feedback.thermal_conductivity_models.len() {
        return Err("thermal conductivity feedback state count does not match model".to_string());
    }
    let mut request = base.clone();
    for model in &feedback.thermal_conductivity_models {
        let coupling_temperature_c = coupling_temperatures_c
            .iter()
            .find(|(element_id, _)| element_id == &model.element_id)
            .map(|(_, temperature)| *temperature)
            .ok_or_else(|| {
                format!(
                    "thermal conductivity feedback is missing state for {}",
                    model.element_id
                )
            })?;
        if !coupling_temperature_c.is_finite() {
            return Err("thermal conductivity coupling temperature must be finite".to_string());
        }
        let element = request
            .elements
            .iter_mut()
            .find(|element| element.id == model.element_id)
            .ok_or_else(|| {
                format!(
                    "thermal conductivity feedback model is missing element {}",
                    model.element_id
                )
            })?;
        let scale = 1.0
            + model.temperature_coefficient_1_k
                * (coupling_temperature_c - model.reference_temperature_c);
        let conductivity = element.conductivity * scale;
        if !conductivity.is_finite() || conductivity <= 0.0 {
            return Err(format!(
                "temperature feedback produced invalid conductivity for {}",
                model.element_id
            ));
        }
        element.conductivity = conductivity;
    }
    Ok(request)
}

pub fn composite_dielectric_mean_temperature(
    heat: &SolveHeatPlaneQuad2dResult,
    element_id: &str,
) -> Result<f64, String> {
    composite_heat_element_mean_temperature(heat, element_id)
}

pub fn composite_heat_element_mean_temperature(
    heat: &SolveHeatPlaneQuad2dResult,
    element_id: &str,
) -> Result<f64, String> {
    let element = heat
        .input
        .elements
        .iter()
        .find(|element| element.id == element_id)
        .ok_or_else(|| format!("heat feedback result is missing element {element_id}"))?;
    let indices = [
        element.node_i,
        element.node_j,
        element.node_k,
        element.node_l,
    ];
    let temperatures = indices
        .into_iter()
        .map(|index| {
            heat.nodes
                .iter()
                .find(|node| node.index == index)
                .map(|node| node.temperature)
                .ok_or_else(|| format!("heat feedback element references unknown node {index}"))
        })
        .collect::<Result<Vec<_>, String>>()?;
    if temperatures.iter().any(|value| !value.is_finite()) {
        return Err("heat feedback temperatures must be finite".to_string());
    }
    Ok(temperatures.iter().sum::<f64>() / temperatures.len() as f64)
}

pub fn assess_composite_electrothermal_feedback(
    feedback: &CompositeElectrothermalFeedbackSpec,
    iterations: Vec<CompositeElectrothermalFeedbackIteration>,
) -> Result<CompositeElectrothermalFeedbackConvergence, String> {
    validate_feedback_spec(feedback)?;
    if iterations.len() > feedback.max_iterations
        || iterations.iter().enumerate().any(|(index, sample)| {
            sample.iteration != index + 1
                || !valid_iteration(feedback, sample)
                || sample.converged
                    != composite_feedback_iteration_converged(
                        feedback,
                        sample.temperature_residual_c,
                        sample.loss_relative_change,
                        sample.max_conductivity_relative_change,
                    )
                || (sample.converged && index + 1 != iterations.len())
        })
    {
        return Err("electrothermal feedback iteration trace is invalid".to_string());
    }
    let final_sample = iterations.last();
    let converged = final_sample.is_some_and(|sample| sample.converged);
    Ok(CompositeElectrothermalFeedbackConvergence {
        schema_version: COMPOSITE_ELECTROTHERMAL_FEEDBACK_SCHEMA_VERSION.to_string(),
        method: "relaxed_temperature_dependent_dielectric_and_conductivity_fixed_point".to_string(),
        status: if iterations.is_empty() {
            "missing"
        } else if converged {
            "pass"
        } else {
            "fail"
        }
        .to_string(),
        converged,
        iteration_count: iterations.len(),
        max_iterations: feedback.max_iterations,
        relaxation_factor: feedback.relaxation_factor,
        temperature_residual_tolerance_c: feedback.temperature_residual_tolerance_c,
        loss_relative_change_tolerance: feedback.loss_relative_change_tolerance,
        conductivity_relative_change_tolerance: feedback.conductivity_relative_change_tolerance,
        parameter_source: feedback.parameter_source.clone(),
        final_temperature_residual_c: final_sample.map(|sample| sample.temperature_residual_c),
        final_loss_relative_change: final_sample.and_then(|sample| sample.loss_relative_change),
        final_max_conductivity_relative_change: final_sample
            .and_then(|sample| sample.max_conductivity_relative_change),
        iterations,
    })
}

pub fn composite_feedback_iteration_converged(
    feedback: &CompositeElectrothermalFeedbackSpec,
    temperature_residual_c: f64,
    loss_relative_change: Option<f64>,
    conductivity_relative_change: Option<f64>,
) -> bool {
    temperature_residual_c <= feedback.temperature_residual_tolerance_c
        && loss_relative_change
            .is_some_and(|change| change <= feedback.loss_relative_change_tolerance)
        && conductivity_relative_change
            .is_some_and(|change| change <= feedback.conductivity_relative_change_tolerance)
}

pub fn composite_feedback_relative_change(current: f64, previous: f64) -> f64 {
    if previous.abs() <= f64::EPSILON {
        (current - previous).abs()
    } else {
        (current - previous).abs() / previous.abs()
    }
}

fn validate_feedback_spec(feedback: &CompositeElectrothermalFeedbackSpec) -> Result<(), String> {
    if feedback.max_iterations < 2 || feedback.max_iterations > 100 {
        return Err("electrothermal feedback max_iterations must be within 2..=100".to_string());
    }
    if !feedback.relaxation_factor.is_finite()
        || feedback.relaxation_factor <= 0.0
        || feedback.relaxation_factor > 1.0
    {
        return Err("electrothermal feedback relaxation must be within (0, 1]".to_string());
    }
    if !feedback.temperature_residual_tolerance_c.is_finite()
        || feedback.temperature_residual_tolerance_c <= 0.0
        || !feedback.loss_relative_change_tolerance.is_finite()
        || feedback.loss_relative_change_tolerance <= 0.0
        || !feedback.conductivity_relative_change_tolerance.is_finite()
        || feedback.conductivity_relative_change_tolerance <= 0.0
    {
        return Err("electrothermal feedback tolerances must be finite and positive".to_string());
    }
    if !feedback
        .relative_permittivity_temperature_coefficient_1_k
        .is_finite()
        || !feedback
            .loss_tangent_temperature_coefficient_1_k
            .is_finite()
        || feedback.parameter_source.trim().is_empty()
    {
        return Err("electrothermal feedback material sensitivity is invalid".to_string());
    }
    let mut element_ids = HashSet::new();
    if feedback.thermal_conductivity_models.is_empty()
        || feedback.thermal_conductivity_models.iter().any(|model| {
            model.element_id.trim().is_empty()
                || !element_ids.insert(model.element_id.as_str())
                || !model.reference_temperature_c.is_finite()
                || !model.temperature_coefficient_1_k.is_finite()
        })
    {
        return Err("thermal conductivity feedback models are invalid".to_string());
    }
    Ok(())
}

fn valid_iteration(
    feedback: &CompositeElectrothermalFeedbackSpec,
    sample: &CompositeElectrothermalFeedbackIteration,
) -> bool {
    let changes = sample
        .thermal_conductivity_updates
        .iter()
        .filter_map(|update| update.conductivity_relative_change)
        .collect::<Vec<_>>();
    let expected_max_change = (changes.len() == feedback.thermal_conductivity_models.len())
        .then(|| changes.into_iter().fold(0.0_f64, f64::max));
    sample.iteration > 0
        && sample.coupling_temperature_c.is_finite()
        && sample.dielectric_mean_temperature_c.is_finite()
        && sample.relative_permittivity.is_finite()
        && sample.relative_permittivity > 0.0
        && sample.loss_tangent.is_finite()
        && sample.loss_tangent >= 0.0
        && sample.total_loss_w.is_finite()
        && sample.total_loss_w >= 0.0
        && sample.total_joule_loss_w.is_finite()
        && sample.total_joule_loss_w >= 0.0
        && sample.max_temperature_c.is_finite()
        && sample.temperature_residual_c.is_finite()
        && sample.temperature_residual_c >= 0.0
        && sample
            .loss_relative_change
            .is_none_or(|value| value.is_finite() && value >= 0.0)
        && sample
            .max_conductivity_relative_change
            .is_none_or(|value| value.is_finite() && value >= 0.0)
        && sample.thermal_conductivity_updates.len() == feedback.thermal_conductivity_models.len()
        && feedback.thermal_conductivity_models.iter().all(|model| {
            sample
                .thermal_conductivity_updates
                .iter()
                .find(|update| update.element_id == model.element_id)
                .is_some_and(valid_conductivity_update)
        })
        && option_f64_matches(sample.max_conductivity_relative_change, expected_max_change)
        && if sample.iteration == 1 {
            sample.loss_relative_change.is_none()
                && sample.max_conductivity_relative_change.is_none()
        } else {
            sample.loss_relative_change.is_some()
                && sample.max_conductivity_relative_change.is_some()
        }
}

fn valid_conductivity_update(update: &CompositeThermalConductivityFeedbackIteration) -> bool {
    !update.element_id.trim().is_empty()
        && update.coupling_temperature_c.is_finite()
        && update.measured_mean_temperature_c.is_finite()
        && update.conductivity_w_mk.is_finite()
        && update.conductivity_w_mk > 0.0
        && update
            .conductivity_relative_change
            .is_none_or(|value| value.is_finite() && value >= 0.0)
}

fn option_f64_matches(left: Option<f64>, right: Option<f64>) -> bool {
    match (left, right) {
        (Some(left), Some(right)) => (left - right).abs() <= 1.0e-12 * right.abs().max(1.0),
        (None, None) => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CompositeElectrothermalFeedbackIteration, CompositeElectrothermalFeedbackSpec,
        CompositeThermalConductivityFeedbackIteration, CompositeThermalConductivityFeedbackModel,
        assess_composite_electrothermal_feedback, temperature_adjusted_composite_heat_request,
        temperature_adjusted_composite_loss_spec,
    };
    use crate::CompositeDielectricLossSpec;
    use kyuubiki_protocol::SolveHeatPlaneQuad2dRequest;
    use serde_json::json;

    #[test]
    fn adjusts_dielectric_parameters_from_temperature() {
        let adjusted = temperature_adjusted_composite_loss_spec(&loss_spec(), &feedback(), 45.0)
            .expect("adjusted spec");

        assert!((adjusted.relative_permittivity - 3.4034).abs() < 1.0e-12);
        assert!((adjusted.loss_tangent - 0.00808).abs() < 1.0e-12);
    }

    #[test]
    fn adjusts_thermal_conductivity_from_element_temperature() {
        let request: SolveHeatPlaneQuad2dRequest = serde_json::from_value(json!({
            "nodes": [],
            "elements": [{
                "id": "dielectric_core",
                "node_i": 0,
                "node_j": 1,
                "node_k": 2,
                "node_l": 3,
                "thickness": 0.001,
                "conductivity": 0.25
            }]
        }))
        .expect("heat request");
        let adjusted = temperature_adjusted_composite_heat_request(
            &request,
            &feedback(),
            &[("dielectric_core".to_string(), 45.0)],
        )
        .expect("adjusted heat request");

        assert!((adjusted.elements[0].conductivity - 0.245).abs() < 1.0e-12);
    }

    #[test]
    fn convergence_requires_a_complete_final_sample() {
        let first = iteration();
        let mut final_sample = iteration();
        final_sample.iteration = 2;
        final_sample.temperature_residual_c = 0.0;
        final_sample.loss_relative_change = Some(0.0);
        final_sample.max_conductivity_relative_change = Some(0.0);
        final_sample.thermal_conductivity_updates[0].measured_mean_temperature_c = 35.0;
        final_sample.thermal_conductivity_updates[0].conductivity_relative_change = Some(0.0);
        final_sample.converged = true;
        let passed = assess_composite_electrothermal_feedback(
            &feedback(),
            vec![first, final_sample.clone()],
        )
        .expect("assessment");
        final_sample.iteration = 3;
        let invalid = assess_composite_electrothermal_feedback(&feedback(), vec![final_sample]);

        assert_eq!(passed.status, "pass");
        assert!(passed.converged);
        assert!(invalid.is_err());
    }

    fn feedback() -> CompositeElectrothermalFeedbackSpec {
        CompositeElectrothermalFeedbackSpec {
            max_iterations: 12,
            relaxation_factor: 0.8,
            temperature_residual_tolerance_c: 1.0e-7,
            loss_relative_change_tolerance: 1.0e-9,
            conductivity_relative_change_tolerance: 1.0e-9,
            relative_permittivity_temperature_coefficient_1_k: 1.0e-4,
            loss_tangent_temperature_coefficient_1_k: 1.0e-3,
            thermal_conductivity_models: vec![CompositeThermalConductivityFeedbackModel {
                element_id: "dielectric_core".to_string(),
                reference_temperature_c: 35.0,
                temperature_coefficient_1_k: -2.0e-3,
            }],
            parameter_source: "screening_sensitivity_not_material_card".to_string(),
        }
    }

    fn loss_spec() -> CompositeDielectricLossSpec {
        CompositeDielectricLossSpec {
            source_element_id: "dielectric_core".to_string(),
            frequency_hz: 10.0e6,
            relative_permittivity: 3.4,
            loss_tangent: 0.008,
            reference_temperature_c: 35.0,
        }
    }

    fn iteration() -> CompositeElectrothermalFeedbackIteration {
        CompositeElectrothermalFeedbackIteration {
            iteration: 1,
            coupling_temperature_c: 35.0,
            dielectric_mean_temperature_c: 35.0,
            relative_permittivity: 3.4,
            loss_tangent: 0.008,
            total_loss_w: 1.0e-4,
            total_joule_loss_w: 2.0e-4,
            max_temperature_c: 35.1,
            temperature_residual_c: 1.0,
            loss_relative_change: None,
            max_conductivity_relative_change: None,
            thermal_conductivity_updates: vec![CompositeThermalConductivityFeedbackIteration {
                element_id: "dielectric_core".to_string(),
                coupling_temperature_c: 35.0,
                measured_mean_temperature_c: 36.0,
                conductivity_w_mk: 0.25,
                conductivity_relative_change: None,
            }],
            converged: false,
        }
    }
}
