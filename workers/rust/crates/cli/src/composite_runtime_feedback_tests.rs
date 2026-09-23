use super::*;
use kyuubiki_headless_sdk::{
    CompositeCurrentConductionRegionSpec, CompositeThermalConductivityFeedbackModel,
};
use serde_json::json;

struct Models {
    electrostatic: SolveElectrostaticPlaneQuad2dRequest,
    current: SolveElectricConductionPlaneQuad2dRequest,
    heat: SolveHeatPlaneQuad2dRequest,
    loss: CompositeDielectricLossSpec,
    current_feedback: CompositeCurrentConductionFeedbackSpec,
    feedback: CompositeElectrothermalFeedbackSpec,
}

impl Models {
    fn new(thickness: f64) -> Self {
        let coordinates = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
        let nodes = |field: &str| {
            coordinates
                .iter()
                .enumerate()
                .map(|(index, [x, y])| {
                    let mut node = json!({"id": format!("n{index}"), "x": x, "y": y});
                    match field {
                        "electric" => {
                            node["fix_electric_potential"] = json!(true);
                            node["electric_potential_v"] = json!(x);
                        }
                        "heat" => {
                            node["fix_temperature"] = json!(*x == 0.0);
                            node["temperature"] = json!(0.0);
                        }
                        _ => {
                            node["fix_potential"] = json!(true);
                        }
                    }
                    node
                })
                .collect::<Vec<_>>()
        };
        let element = json!({"id": "dielectric_core", "node_i": 0, "node_j": 1,
            "node_k": 2, "node_l": 3, "thickness": thickness, "permittivity": 1.0,
            "electrical_conductivity_s_m": 1.0, "conductivity": 1.0});
        Self {
            electrostatic: serde_json::from_value(
                json!({"nodes": nodes("field"), "elements": [element.clone()]}),
            )
            .unwrap(),
            current: serde_json::from_value(
                json!({"nodes": nodes("electric"), "elements": [element.clone()]}),
            )
            .unwrap(),
            heat: serde_json::from_value(json!({"nodes": nodes("heat"), "elements": [element]}))
                .unwrap(),
            loss: CompositeDielectricLossSpec {
                source_element_id: "dielectric_core".into(),
                frequency_hz: 1.0e6,
                relative_permittivity: 1.0,
                loss_tangent: 0.0,
                reference_temperature_c: 0.0,
            },
            current_feedback: CompositeCurrentConductionFeedbackSpec {
                regions: vec![CompositeCurrentConductionRegionSpec {
                    element_id: "dielectric_core".into(),
                    reference_resistivity_ohm_m: 1.0,
                    reference_temperature_c: 0.0,
                    resistivity_temperature_coefficient_1_k: 1.0,
                }],
                parameter_source: "analytic-fixed-point".into(),
            },
            feedback: CompositeElectrothermalFeedbackSpec {
                max_iterations: 40,
                relaxation_factor: 1.0,
                temperature_residual_tolerance_c: 1.0,
                loss_relative_change_tolerance: 1.0e-9,
                conductivity_relative_change_tolerance: 1.0e-9,
                relative_permittivity_temperature_coefficient_1_k: 0.0,
                loss_tangent_temperature_coefficient_1_k: 0.0,
                thermal_conductivity_models: vec![CompositeThermalConductivityFeedbackModel {
                    element_id: "dielectric_core".into(),
                    reference_temperature_c: 0.0,
                    temperature_coefficient_1_k: 0.0,
                }],
                parameter_source: "analytic-fixed-point".into(),
            },
        }
    }

    fn solve(&self) -> Result<CompositeElectrothermalSolve, String> {
        solve_composite_electrothermal_feedback(
            &self.electrostatic,
            &self.current,
            &self.heat,
            &self.loss,
            &self.current_feedback,
            &self.feedback,
        )
    }
}

#[test]
fn low_power_feedback_matches_the_unit_scale_analytic_fixed_point() {
    // Mean temperature theta = 1 / (4 * (1 + theta)); thickness cancels.
    let expected = (2.0_f64.sqrt() - 1.0) / 2.0;
    let mut counts = Vec::new();
    for thickness in [1.0, 1.0e-20, 1.0e20] {
        let result = Models::new(thickness).solve().unwrap();
        let trace = &result.feedback_convergence;
        let final_sample = trace.iterations.last().unwrap();
        eprintln!(
            "thickness={thickness:e} iterations={} mean_temperature={:.12} normalized_power={:.12}",
            trace.iteration_count,
            final_sample.dielectric_mean_temperature_c,
            result.joule_heating_projection.total_joule_loss_w / thickness
        );
        assert!(trace.converged);
        assert!(
            trace.iteration_count > 2,
            "premature convergence: {trace:?}"
        );
        assert!((final_sample.dielectric_mean_temperature_c - expected).abs() < 1.0e-9);
        assert!(
            (result.joule_heating_projection.total_joule_loss_w / thickness
                - 1.0 / (1.0 + expected))
                .abs()
                < 1.0e-9
        );
        for pair in trace.iterations.windows(2) {
            let previous = pair[0].total_joule_loss_w;
            let expected_change = ((pair[1].total_joule_loss_w - previous) / previous).abs();
            assert!((pair[1].loss_relative_change.unwrap() - expected_change).abs() < 1.0e-15);
        }
        counts.push(trace.iteration_count);
    }
    assert!(
        counts.iter().all(|count| *count == counts[0]),
        "counts={counts:?}"
    );
}

#[test]
fn exhausted_iteration_budget_is_a_failed_convergence_not_a_success() {
    let mut models = Models::new(1.0e-20);
    models.feedback.max_iterations = 2;
    let result = models.solve().unwrap();
    assert!(!result.feedback_convergence.converged);
    assert_eq!(result.feedback_convergence.status, "fail");
    assert_eq!(result.feedback_convergence.iteration_count, 2);
}

#[test]
fn invalid_iteration_budget_returns_an_error_without_panicking() {
    let mut models = Models::new(1.0);
    for budget in [0, 1, 101, usize::MAX] {
        models.feedback.max_iterations = budget;
        let result = std::panic::catch_unwind(|| models.solve());
        assert!(
            result.is_ok(),
            "invalid configuration must not panic before validation"
        );
        assert!(result.unwrap().is_err());
    }
}

#[test]
fn failed_material_feedback_can_be_retried_without_mutating_the_seed() {
    let mut models = Models::new(1.0);
    let original = serde_json::to_value(&models.current).unwrap();
    models.current_feedback.regions[0].resistivity_temperature_coefficient_1_k = -8.0;
    let error = models
        .solve()
        .err()
        .expect("negative resistivity must fail");
    assert!(
        error.contains("iteration 2 failed current feedback") && error.contains("resistivity"),
        "{error}"
    );
    assert_eq!(serde_json::to_value(&models.current).unwrap(), original);
    models.current_feedback.regions[0].resistivity_temperature_coefficient_1_k = 1.0;
    assert!(models.solve().unwrap().feedback_convergence.converged);
}

#[test]
fn dielectric_and_joule_feedback_with_temperature_dependent_conductivity_closes() {
    let mut models = Models::new(1.0e-20);
    for node in &mut models.electrostatic.nodes {
        node.potential = node.x;
    }
    models.loss.loss_tangent = 0.1;
    models
        .feedback
        .relative_permittivity_temperature_coefficient_1_k = 0.01;
    models.feedback.loss_tangent_temperature_coefficient_1_k = 0.02;
    models.feedback.thermal_conductivity_models[0].temperature_coefficient_1_k = 0.1;
    models.feedback.temperature_residual_tolerance_c = 1.0e-9;
    models.feedback.relaxation_factor = 0.75;
    let result = models.solve().unwrap();
    assert!(result.feedback_convergence.converged);
    assert!(result.loss_projection.total_loss_w > 0.0);
    assert!(result.joule_heating_projection.total_joule_loss_w > 0.0);
    assert!(result.heat.input.elements[0].conductivity > 1.0);
    let sample = result.feedback_convergence.iterations.last().unwrap();
    let theta = sample.dielectric_mean_temperature_c;
    let dielectric_power = 2.0
        * std::f64::consts::PI
        * 1.0e6
        * 8.854_187_812_8e-12
        * (1.0 + 0.01 * theta)
        * 0.1
        * (1.0 + 0.02 * theta);
    let expected_mean = (dielectric_power + 1.0 / (1.0 + theta)) / (4.0 * (1.0 + 0.1 * theta));
    assert!((theta - expected_mean).abs() < 1.0e-9);
}

#[test]
fn errors_name_the_actual_failed_stage() {
    let mut field = Models::new(1.0);
    field.electrostatic.elements[0].node_i = usize::MAX;
    assert!(
        field
            .solve()
            .err()
            .unwrap()
            .contains("iteration 1 failed electrostatic solve")
    );
    let mut thermal = Models::new(1.0);
    thermal.heat.elements[0].conductivity = -1.0;
    assert!(
        thermal
            .solve()
            .err()
            .unwrap()
            .contains("iteration 1 failed thermal conductivity feedback")
    );
    let mut mapping = Models::new(1.0);
    mapping.heat.nodes[0].x = -1.0;
    assert!(
        mapping
            .solve()
            .err()
            .unwrap()
            .contains("iteration 1 failed dielectric heat projection")
    );
    let mut current = Models::new(1.0);
    current.current.elements[0].node_i = usize::MAX;
    assert!(
        current
            .solve()
            .err()
            .unwrap()
            .contains("iteration 1 failed current solve")
    );
}
