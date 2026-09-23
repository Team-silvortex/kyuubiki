use kyuubiki_headless_sdk::{
    CompositeElectrothermalFeedbackIteration, CompositeElectrothermalFeedbackSpec,
    CompositeThermalConductivityFeedbackIteration, CompositeThermalConductivityFeedbackModel,
    assess_composite_electrothermal_feedback, composite_feedback_iteration_converged,
    composite_feedback_relative_change, composite_heat_element_mean_temperature,
};
use kyuubiki_protocol::SolveHeatPlaneQuad2dResult;
use serde_json::json;

fn feedback() -> CompositeElectrothermalFeedbackSpec {
    CompositeElectrothermalFeedbackSpec {
        max_iterations: 12,
        relaxation_factor: 1.0,
        temperature_residual_tolerance_c: 1.0e-7,
        loss_relative_change_tolerance: 1.0e-9,
        conductivity_relative_change_tolerance: 1.0e-9,
        relative_permittivity_temperature_coefficient_1_k: 0.0,
        loss_tangent_temperature_coefficient_1_k: 0.0,
        thermal_conductivity_models: vec![CompositeThermalConductivityFeedbackModel {
            element_id: "core".into(),
            reference_temperature_c: 20.0,
            temperature_coefficient_1_k: 0.0,
        }],
        parameter_source: "bounded-feedback-regression".into(),
    }
}

fn steady_trace() -> Vec<CompositeElectrothermalFeedbackIteration> {
    (1..=2)
        .map(|iteration| CompositeElectrothermalFeedbackIteration {
            iteration,
            coupling_temperature_c: 20.0,
            dielectric_mean_temperature_c: 20.0,
            relative_permittivity: 3.4,
            loss_tangent: 0.01,
            total_loss_w: 1.0e-20,
            total_joule_loss_w: 0.0,
            max_temperature_c: 20.0,
            temperature_residual_c: 0.0,
            loss_relative_change: (iteration > 1).then_some(0.0),
            max_conductivity_relative_change: (iteration > 1).then_some(0.0),
            thermal_conductivity_updates: vec![CompositeThermalConductivityFeedbackIteration {
                element_id: "core".into(),
                coupling_temperature_c: 20.0,
                measured_mean_temperature_c: 20.0,
                conductivity_w_mk: 0.25,
                conductivity_relative_change: (iteration > 1).then_some(0.0),
            }],
            converged: iteration > 1,
        })
        .collect()
}

#[test]
fn relative_change_is_scale_independent_without_an_absolute_power_floor() {
    for previous in [1.0e-300, 1.0e-20, 1.0, 1.0e300, f64::from_bits(1)] {
        assert_eq!(
            composite_feedback_relative_change(previous * 2.0, previous),
            1.0
        );
        assert_eq!(composite_feedback_relative_change(previous, previous), 0.0);
    }
}

#[test]
fn zero_transitions_are_explicit_and_never_fake_stability() {
    assert_eq!(composite_feedback_relative_change(0.0, 0.0), 0.0);
    for positive in [f64::from_bits(1), 1.0e-20, 1.0, 1.0e300] {
        assert_eq!(composite_feedback_relative_change(positive, 0.0), 1.0);
        assert_eq!(composite_feedback_relative_change(0.0, positive), 1.0);
    }
}

#[test]
fn invalid_changes_or_tolerances_cannot_satisfy_convergence() {
    let spec = feedback();
    for invalid in [f64::NEG_INFINITY, -1.0, f64::NAN, f64::INFINITY] {
        assert!(!composite_feedback_iteration_converged(
            &spec,
            invalid,
            Some(0.0),
            Some(0.0)
        ));
        assert!(!composite_feedback_iteration_converged(
            &spec,
            0.0,
            Some(invalid),
            Some(0.0)
        ));
        assert!(!composite_feedback_iteration_converged(
            &spec,
            0.0,
            Some(0.0),
            Some(invalid)
        ));
        assert!(!composite_feedback_relative_change(invalid, 1.0).is_finite());
        assert!(!composite_feedback_relative_change(1.0, invalid).is_finite());
    }
    for tolerance in [f64::INFINITY, f64::NAN, 0.0, -1.0] {
        let mut invalid = spec.clone();
        invalid.loss_relative_change_tolerance = tolerance;
        assert!(!composite_feedback_iteration_converged(
            &invalid,
            0.0,
            Some(0.0),
            Some(0.0)
        ));
    }
}

#[test]
fn a_truthful_steady_trace_passes_but_forged_temperature_residual_does_not() {
    let mut trace = steady_trace();
    assert!(
        assess_composite_electrothermal_feedback(&feedback(), trace.clone())
            .unwrap()
            .converged
    );
    trace[1].dielectric_mean_temperature_c = 21.0;
    trace[1].max_temperature_c = 21.0;
    assert!(assess_composite_electrothermal_feedback(&feedback(), trace).is_err());
}

#[test]
fn forged_regional_temperature_residual_is_rejected() {
    let mut trace = steady_trace();
    trace[1].thermal_conductivity_updates[0].measured_mean_temperature_c = 21.0;
    trace[1].max_temperature_c = 21.0;
    assert!(assess_composite_electrothermal_feedback(&feedback(), trace).is_err());
}

#[test]
fn forged_loss_change_is_recomputed_from_both_heating_mechanisms() {
    for joule in [false, true] {
        let mut trace = steady_trace();
        if joule {
            trace[1].total_joule_loss_w = 1.0e-20;
        } else {
            trace[1].total_loss_w *= 2.0;
        }
        assert!(assess_composite_electrothermal_feedback(&feedback(), trace).is_err());
    }
}

#[test]
fn forged_conductivity_change_is_recomputed_by_region_id() {
    let mut trace = steady_trace();
    trace[1].thermal_conductivity_updates[0].conductivity_w_mk *= 2.0;
    assert!(assess_composite_electrothermal_feedback(&feedback(), trace).is_err());
}

#[test]
fn finite_loss_components_cannot_overflow_the_trace_total() {
    let mut trace = steady_trace();
    for sample in &mut trace {
        sample.total_loss_w = f64::MAX;
        sample.total_joule_loss_w = f64::MAX;
    }
    assert!(assess_composite_electrothermal_feedback(&feedback(), trace).is_err());
}

#[test]
fn missing_trace_and_honest_nonconvergence_are_not_success() {
    let missing = assess_composite_electrothermal_feedback(&feedback(), vec![]).unwrap();
    assert_eq!(missing.status, "missing");
    let mut trace = steady_trace();
    trace[1].total_loss_w *= 2.0;
    trace[1].loss_relative_change = Some(1.0);
    trace[1].converged = false;
    let result = assess_composite_electrothermal_feedback(&feedback(), trace).unwrap();
    assert_eq!(result.status, "fail");
    assert!(!result.converged);
}

#[test]
fn near_zero_changes_cannot_hide_behind_an_absolute_comparison_tolerance() {
    let mut spec = feedback();
    spec.loss_relative_change_tolerance = 1.0e-15;
    let mut trace = steady_trace();
    trace[1].total_loss_w *= 1.0 + 1.0e-13;
    assert!(assess_composite_electrothermal_feedback(&spec, trace).is_err());
}

#[test]
fn rounding_tolerance_cannot_turn_a_failed_threshold_into_a_pass() {
    let mut spec = feedback();
    spec.temperature_residual_tolerance_c = 1.0;
    let mut trace = steady_trace();
    trace[1].dielectric_mean_temperature_c = 21.0 + 1.0e-13;
    trace[1].max_temperature_c = trace[1].dielectric_mean_temperature_c;
    trace[1].temperature_residual_c = 1.0;
    assert!(assess_composite_electrothermal_feedback(&spec, trace).is_err());
}

#[test]
fn reordered_regions_are_matched_by_identity_and_duplicates_are_rejected() {
    let mut spec = feedback();
    let mut second = spec.thermal_conductivity_models[0].clone();
    second.element_id = "shell".into();
    spec.thermal_conductivity_models.push(second);
    let mut trace = steady_trace();
    for sample in &mut trace {
        let mut second = sample.thermal_conductivity_updates[0].clone();
        second.element_id = "shell".into();
        second.conductivity_w_mk = 10.0;
        sample.thermal_conductivity_updates.push(second);
    }
    trace[1].thermal_conductivity_updates.reverse();
    assert!(
        assess_composite_electrothermal_feedback(&spec, trace.clone())
            .unwrap()
            .converged
    );
    trace[1].thermal_conductivity_updates[1] = trace[1].thermal_conductivity_updates[0].clone();
    assert!(assess_composite_electrothermal_feedback(&spec, trace).is_err());
}

#[test]
fn the_first_sample_cannot_claim_a_previous_conductivity_measurement() {
    let mut trace = steady_trace();
    trace[0].thermal_conductivity_updates[0].conductivity_relative_change = Some(0.0);
    assert!(assess_composite_electrothermal_feedback(&feedback(), trace).is_err());
}

#[test]
fn heat_feedback_mean_remains_finite_when_the_naive_sum_overflows() {
    let mut heat: SolveHeatPlaneQuad2dResult = serde_json::from_value(json!({
        "input": {"nodes": [], "elements": [{"id": "core", "node_i": 0,
            "node_j": 1, "node_k": 2, "node_l": 3, "conductivity": 1.0, "thickness": 1.0}]},
        "nodes": (0..4).map(|index| json!({"index": index, "id": format!("n{index}"),
            "x": 0.0, "y": 0.0, "temperature": 1.0, "heat_load": 0.0})).collect::<Vec<_>>(),
        "elements": [], "max_temperature": 1.0, "max_heat_flux": 0.0,
        "total_abs_heat_flow_rate": 0.0
    }))
    .unwrap();
    for temperature in [f64::MAX, -f64::MAX, f64::from_bits(1), 20.0] {
        for node in &mut heat.nodes {
            node.temperature = temperature;
        }
        assert_eq!(
            composite_heat_element_mean_temperature(&heat, "core").unwrap(),
            temperature
        );
    }
}
