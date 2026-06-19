use crate::{
    workflow_diagnostics::{
        extract_electrostatic_result_diagnostics, extract_thermal_result_diagnostics,
        extract_thermo_result_diagnostics,
    },
    workflow_executor::run_extract_operator,
};

fn approx_eq(left: Option<f64>, right: f64) {
    let value = left.expect("expected numeric diagnostic value");
    assert!(
        (value - right).abs() < 1.0e-9,
        "left={value}, right={right}"
    );
}

#[test]
fn extracts_thermal_result_diagnostics() {
    let diagnostics = extract_thermal_result_diagnostics(
        serde_json::json!({
            "nodes": [
                { "id": "n0", "temperature": 20.0, "heat_load": 1.0 },
                { "id": "n1", "temperature": 80.0, "heat_load": 3.0 },
                { "id": "n2", "temperature": 50.0, "heat_load": 2.0 }
            ],
            "elements": [
                {
                    "id": "e0",
                    "temperature_gradient_x": 3.0,
                    "temperature_gradient_y": 4.0,
                    "temperature_gradient_magnitude": 2.0,
                    "heat_flux_x": 6.0,
                    "heat_flux_y": 8.0,
                    "heat_flux_magnitude": 10.0
                },
                {
                    "id": "e1",
                    "temperature_gradient_x": 5.0,
                    "temperature_gradient_y": 12.0,
                    "temperature_gradient_magnitude": 13.0,
                    "heat_flux_x": 9.0,
                    "heat_flux_y": 12.0,
                    "heat_flux_magnitude": 15.0
                }
            ]
        }),
        serde_json::json!({}),
    )
    .expect("thermal diagnostics should succeed");

    assert_eq!(diagnostics["diagnostic_domain"].as_str(), Some("thermal"));
    assert_eq!(diagnostics["thermal_temperature_min"].as_f64(), Some(20.0));
    assert_eq!(diagnostics["thermal_temperature_max"].as_f64(), Some(80.0));
    assert_eq!(diagnostics["thermal_temperature_span"].as_f64(), Some(60.0));
    assert_eq!(diagnostics["thermal_heat_load_sum"].as_f64(), Some(6.0));
    assert_eq!(diagnostics["thermal_heat_load_count"].as_u64(), Some(3));
    assert_eq!(diagnostics["thermal_heat_load_mean"].as_f64(), Some(2.0));
    assert_eq!(
        diagnostics["thermal_gradient_peak_magnitude"].as_f64(),
        Some(13.0)
    );
    assert_eq!(diagnostics["thermal_peak_gradient"].as_f64(), Some(13.0));
    assert_eq!(diagnostics["thermal_peak_gradient_id"].as_str(), Some("e1"));
    assert_eq!(diagnostics["thermal_gradient_peak_x"].as_f64(), Some(5.0));
    assert_eq!(diagnostics["thermal_gradient_peak_y"].as_f64(), Some(12.0));
    assert_eq!(
        diagnostics["thermal_flux_peak_magnitude"].as_f64(),
        Some(15.0)
    );
    assert_eq!(diagnostics["thermal_peak_flux"].as_f64(), Some(15.0));
    assert_eq!(diagnostics["thermal_peak_flux_id"].as_str(), Some("e1"));
    assert_eq!(
        diagnostics["thermal_flux_peak_element_id"].as_str(),
        Some("e1")
    );
}

#[test]
fn extracts_thermo_result_diagnostics_with_payload_and_strain_fallbacks() {
    let diagnostics = extract_thermo_result_diagnostics(
        serde_json::json!({
            "max_stress": 220.0,
            "nodes": [
                { "id": "n0", "ux": 3.0, "uy": 4.0, "displacement_magnitude": 5.0, "temperature_delta": 25.0 },
                { "id": "n1", "ux": 5.0, "uy": 12.0, "displacement_magnitude": 13.0, "temperature_delta": 45.0 }
            ],
            "elements": [
                {
                    "id": "e0",
                    "stress_x": 110.0,
                    "thermal_strain_x": 2.0e-4,
                    "mechanical_strain_x": -1.0e-4,
                    "total_strain_x": 1.0e-4
                },
                {
                    "id": "e1",
                    "stress_x": 180.0,
                    "thermal_strain_x": 4.5e-4,
                    "mechanical_strain_x": -3.0e-4,
                    "total_strain_x": 2.0e-4
                }
            ]
        }),
        serde_json::json!({}),
    )
    .expect("thermo diagnostics should succeed");

    assert_eq!(
        diagnostics["diagnostic_domain"].as_str(),
        Some("thermo_mechanical")
    );
    assert_eq!(
        diagnostics["thermo_temperature_delta_min"].as_f64(),
        Some(25.0)
    );
    assert_eq!(
        diagnostics["thermo_temperature_delta_max"].as_f64(),
        Some(45.0)
    );
    assert_eq!(
        diagnostics["thermo_displacement_peak_magnitude"].as_f64(),
        Some(13.0)
    );
    assert_eq!(diagnostics["thermo_peak_displacement"].as_f64(), Some(13.0));
    assert_eq!(
        diagnostics["thermo_peak_displacement_id"].as_str(),
        Some("n1")
    );
    assert_eq!(
        diagnostics["thermo_displacement_peak_x"].as_f64(),
        Some(5.0)
    );
    assert_eq!(
        diagnostics["thermo_displacement_peak_y"].as_f64(),
        Some(12.0)
    );
    assert_eq!(
        diagnostics["thermo_displacement_peak_element_id"].as_str(),
        Some("n1")
    );
    approx_eq(diagnostics["thermo_stress_peak"].as_f64(), 220.0);
    approx_eq(diagnostics["thermo_peak_stress"].as_f64(), 220.0);
    assert_eq!(
        diagnostics["thermo_peak_stress_id"].as_str(),
        Some("max_stress")
    );
    assert_eq!(
        diagnostics["thermo_stress_peak_element_id"].as_str(),
        Some("max_stress")
    );
    approx_eq(diagnostics["thermo_thermal_strain_peak"].as_f64(), 4.5e-4);
    approx_eq(diagnostics["thermo_peak_thermal_strain"].as_f64(), 4.5e-4);
    assert_eq!(
        diagnostics["thermo_peak_thermal_strain_id"].as_str(),
        Some("e1")
    );
    approx_eq(
        diagnostics["thermo_peak_mechanical_strain"].as_f64(),
        -3.0e-4,
    );
    approx_eq(diagnostics["thermo_peak_total_strain"].as_f64(), 2.0e-4);
}

#[test]
fn extracts_thermo_result_diagnostics_from_signed_component_stress() {
    let diagnostics = extract_thermo_result_diagnostics(
        serde_json::json!({
            "nodes": [
                { "id": "n0", "temperature_delta": 10.0 }
            ],
            "elements": [
                { "id": "e0", "stress_x": -80.0 },
                { "id": "e1", "stress_x": -120.0 }
            ]
        }),
        serde_json::json!({}),
    )
    .expect("thermo diagnostics should succeed");

    approx_eq(diagnostics["thermo_peak_stress"].as_f64(), -120.0);
    assert_eq!(diagnostics["thermo_peak_stress_id"].as_str(), Some("e1"));
}

#[test]
fn extracts_electrostatic_result_diagnostics() {
    let diagnostics = extract_electrostatic_result_diagnostics(
        serde_json::json!({
            "nodes": [
                { "id": "n0", "potential": 10.0, "charge_density": 0.2 },
                { "id": "n1", "potential": 4.0, "charge_density": 0.5 },
                { "id": "n2", "potential": 1.0, "charge_density": 0.1 }
            ],
            "elements": [
                {
                    "id": "e0",
                    "electric_field_x": 6.0,
                    "electric_field_y": 8.0,
                    "electric_field_magnitude": 10.0,
                    "energy_density": 2.5
                },
                {
                    "id": "e1",
                    "electric_field_x": 5.0,
                    "electric_field_y": 12.0,
                    "electric_field_magnitude": 13.0,
                    "energy_density": 4.0
                }
            ]
        }),
        serde_json::json!({}),
    )
    .expect("electrostatic diagnostics should succeed");

    assert_eq!(
        diagnostics["diagnostic_domain"].as_str(),
        Some("electrostatic")
    );
    assert_eq!(
        diagnostics["electrostatic_potential_min"].as_f64(),
        Some(1.0)
    );
    assert_eq!(
        diagnostics["electrostatic_potential_max"].as_f64(),
        Some(10.0)
    );
    approx_eq(
        diagnostics["electrostatic_charge_density_sum"].as_f64(),
        0.8,
    );
    assert_eq!(
        diagnostics["electrostatic_field_peak_magnitude"].as_f64(),
        Some(13.0)
    );
    approx_eq(
        diagnostics["electrostatic_energy_density_peak"].as_f64(),
        4.0,
    );
}

#[test]
fn runs_diagnostics_extract_operator_through_workflow_executor() {
    let diagnostics = run_extract_operator(
        "extract.thermo_result_diagnostics",
        serde_json::json!({
            "nodes": [
                { "id": "n0", "ux": 0.0, "uy": 2.0, "displacement_magnitude": 2.0, "temperature_delta": 20.0 },
                { "id": "n1", "ux": 0.0, "uy": 5.0, "displacement_magnitude": 5.0, "temperature_delta": 35.0 }
            ],
            "elements": [
                { "id": "e0", "von_mises": 90.0 },
                { "id": "e1", "von_mises": 150.0 }
            ]
        }),
        serde_json::json!({
            "output_prefix": "candidate"
        }),
    )
    .expect("workflow extract operator should succeed");

    assert_eq!(diagnostics["diagnostic_prefix"].as_str(), Some("candidate"));
    assert_eq!(
        diagnostics["candidate_displacement_peak_magnitude"].as_f64(),
        Some(5.0)
    );
    approx_eq(diagnostics["candidate_stress_peak"].as_f64(), 150.0);
}
