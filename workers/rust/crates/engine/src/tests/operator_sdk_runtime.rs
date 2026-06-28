use crate::{
    EngineSolveRequest, solve,
    workflow_executor::{run_export_operator, run_extract_operator, run_transform_operator},
};
use kyuubiki_protocol::{
    AnalysisResult, HeatPlaneNodeInput, HeatPlaneQuadElementInput, SolveHeatPlaneQuad2dRequest,
    SolveThermalPlaneQuad2dRequest,
};

#[test]
fn runs_extract_operator_through_sdk_registry() {
    let summary = run_extract_operator(
        "extract.field_statistics",
        serde_json::json!({
            "nodes": [
                { "id": "n0", "temperature": 10.0 },
                { "id": "n1", "temperature": 14.0 },
                { "id": "n2", "temperature": 22.0 }
            ]
        }),
        serde_json::json!({
            "source": "nodes",
            "field": "temperature",
            "output_prefix": "temperature"
        }),
    )
    .expect("extract.field_statistics should succeed");

    assert_eq!(summary["temperature_count"].as_u64(), Some(3));
    assert_eq!(summary["temperature_min"].as_f64(), Some(10.0));
    assert_eq!(summary["temperature_max"].as_f64(), Some(22.0));
}

#[test]
fn runs_electrostatic_peak_extract_operator_through_sdk_registry() {
    let summary = run_extract_operator(
        "extract.electrostatic_peak_field",
        serde_json::json!({
            "input": {
                "nodes": [],
                "elements": []
            },
            "nodes": [
                { "index": 0, "id": "n0", "x": 0.0, "y": 0.0, "potential": 12.0, "charge_density": 0.0 },
                { "index": 1, "id": "n1", "x": 1.0, "y": 0.0, "potential": 0.0, "charge_density": 0.0 },
                { "index": 2, "id": "n2", "x": 1.0, "y": 1.0, "potential": 4.0, "charge_density": 0.0 },
                { "index": 3, "id": "n3", "x": 0.0, "y": 1.0, "potential": 8.0, "charge_density": 0.0 }
            ],
            "elements": [
                {
                    "index": 0,
                    "id": "eq0",
                    "node_i": 0,
                    "node_j": 1,
                    "node_k": 2,
                    "node_l": 3,
                    "area": 1.0,
                    "average_potential": 6.0,
                    "potential_gradient_x": -8.0,
                    "potential_gradient_y": -2.0,
                    "electric_field_x": 8.0,
                    "electric_field_y": 2.0,
                    "electric_field_magnitude": 8.2462112512,
                    "electric_flux_density_x": 4.0,
                    "electric_flux_density_y": 1.0,
                    "electric_flux_density_magnitude": 4.1231056256,
                    "stored_energy": 1.5
                },
                {
                    "index": 1,
                    "id": "eq1",
                    "node_i": 0,
                    "node_j": 1,
                    "node_k": 2,
                    "node_l": 3,
                    "area": 1.0,
                    "average_potential": 6.0,
                    "potential_gradient_x": -10.0,
                    "potential_gradient_y": -4.0,
                    "electric_field_x": 10.0,
                    "electric_field_y": 4.0,
                    "electric_field_magnitude": 10.7703296143,
                    "electric_flux_density_x": 5.0,
                    "electric_flux_density_y": 2.0,
                    "electric_flux_density_magnitude": 5.3851648071,
                    "stored_energy": 3.5
                }
            ],
            "max_potential": 12.0,
            "max_electric_field": 10.7703296143,
            "max_flux_density": 5.3851648071,
            "total_stored_energy": 5.0
        }),
        serde_json::Value::Null,
    )
    .expect("extract.electrostatic_peak_field should succeed");

    assert_eq!(summary["peak_element_id"].as_str(), Some("eq1"));
    assert_eq!(summary["max_potential"].as_f64(), Some(12.0));
    assert_eq!(summary["max_electric_field"].as_f64(), Some(10.7703296143));
    assert_eq!(summary["max_flux_density"].as_f64(), Some(5.3851648071));
    assert_eq!(summary["peak_average_potential"].as_f64(), Some(6.0));
    assert_eq!(summary["electrostatic_field_peak_x"].as_f64(), Some(10.0));
    assert_eq!(summary["electrostatic_field_peak_y"].as_f64(), Some(4.0));
    assert_eq!(summary["peak_flux_density_x"].as_f64(), Some(5.0));
    assert_eq!(summary["peak_flux_density_y"].as_f64(), Some(2.0));
    assert_eq!(summary["peak_stored_energy"].as_f64(), Some(3.5));
    assert_eq!(summary["total_stored_energy"].as_f64(), Some(5.0));
    assert_eq!(
        summary["peak_potential_gradient_magnitude"].as_f64(),
        Some(10.770329614269007)
    );
    assert_eq!(
        summary["electrostatic_peak_average_potential"].as_f64(),
        Some(6.0)
    );
    assert_eq!(
        summary["electrostatic_field_peak_element_id"].as_str(),
        Some("eq1")
    );
}

#[test]
fn runs_magnetostatic_peak_extract_operator_through_sdk_registry() {
    let summary = run_extract_operator(
        "extract.magnetostatic_peak_field",
        serde_json::json!({
            "input": {
                "nodes": [],
                "elements": []
            },
            "nodes": [],
            "elements": [
                {
                    "index": 0,
                    "id": "mq0",
                    "node_i": 0,
                    "node_j": 1,
                    "node_k": 2,
                    "node_l": 3,
                    "area": 1.0,
                    "average_vector_potential": 2.0,
                    "vector_potential_gradient_x": 3.0,
                    "vector_potential_gradient_y": 4.0,
                    "magnetic_field_strength_x": 3.0,
                    "magnetic_field_strength_y": 4.0,
                    "magnetic_field_strength_magnitude": 5.0,
                    "magnetic_flux_density_x": 6.0,
                    "magnetic_flux_density_y": 8.0,
                    "magnetic_flux_density_magnitude": 10.0,
                    "stored_energy": 2.5
                },
                {
                    "index": 1,
                    "id": "mq1",
                    "node_i": 0,
                    "node_j": 1,
                    "node_k": 2,
                    "node_l": 3,
                    "area": 1.0,
                    "average_vector_potential": 4.0,
                    "vector_potential_gradient_x": 5.0,
                    "vector_potential_gradient_y": 12.0,
                    "magnetic_field_strength_x": 5.0,
                    "magnetic_field_strength_y": 12.0,
                    "magnetic_field_strength_magnitude": 13.0,
                    "magnetic_flux_density_x": 8.0,
                    "magnetic_flux_density_y": 15.0,
                    "magnetic_flux_density_magnitude": 17.0,
                    "stored_energy": 7.0
                }
            ],
            "max_vector_potential": 5.0,
            "max_magnetic_field_strength": 13.0,
            "max_flux_density": 17.0,
            "total_stored_energy": 9.5
        }),
        serde_json::Value::Null,
    )
    .expect("extract.magnetostatic_peak_field should succeed");

    assert_eq!(summary["peak_element_id"].as_str(), Some("mq1"));
    assert_eq!(summary["peak_magnetic_field_strength"].as_f64(), Some(13.0));
    assert_eq!(summary["peak_flux_density"].as_f64(), Some(17.0));
    assert_eq!(summary["peak_average_vector_potential"].as_f64(), Some(4.0));
    assert_eq!(
        summary["magnetostatic_field_peak_element_id"].as_str(),
        Some("mq1")
    );
    assert_eq!(
        summary["magnetostatic_flux_peak_element_id"].as_str(),
        Some("mq1")
    );
    assert_eq!(
        summary["magnetostatic_peak_stored_energy"].as_f64(),
        Some(7.0)
    );
    assert_eq!(summary["max_magnetic_field_strength"].as_f64(), Some(13.0));
    assert_eq!(summary["total_stored_energy"].as_f64(), Some(9.5));
}

#[test]
fn runs_heat_peak_flux_extract_operator_through_sdk_registry() {
    let summary = run_extract_operator(
        "extract.heat_peak_flux",
        serde_json::json!({
            "input": {
                "nodes": [],
                "elements": []
            },
            "nodes": [
                { "index": 0, "id": "h0", "x": 0.0, "y": 0.0, "temperature": 100.0, "heat_load": 0.0 },
                { "index": 1, "id": "h1", "x": 1.0, "y": 0.0, "temperature": 60.0, "heat_load": 0.0 },
                { "index": 2, "id": "h2", "x": 1.0, "y": 1.0, "temperature": 20.0, "heat_load": 0.0 },
                { "index": 3, "id": "h3", "x": 0.0, "y": 1.0, "temperature": 40.0, "heat_load": 0.0 }
            ],
            "elements": [
                {
                    "index": 0,
                    "id": "hq0",
                    "node_i": 0,
                    "node_j": 1,
                    "node_k": 2,
                    "node_l": 3,
                    "area": 1.0,
                    "average_temperature": 55.0,
                    "temperature_gradient_x": -15.0,
                    "temperature_gradient_y": -5.0,
                    "heat_flux_x": 30.0,
                    "heat_flux_y": 10.0,
                    "heat_flux_magnitude": 31.6227766017
                },
                {
                    "index": 1,
                    "id": "hq1",
                    "node_i": 0,
                    "node_j": 1,
                    "node_k": 2,
                    "node_l": 3,
                    "area": 1.0,
                    "average_temperature": 55.0,
                    "temperature_gradient_x": -20.0,
                    "temperature_gradient_y": -12.0,
                    "heat_flux_x": 40.0,
                    "heat_flux_y": 24.0,
                    "heat_flux_magnitude": 46.6476151588
                }
            ],
            "max_temperature": 100.0,
            "max_heat_flux": 46.6476151588
        }),
        serde_json::Value::Null,
    )
    .expect("extract.heat_peak_flux should succeed");

    assert_eq!(summary["peak_element_id"].as_str(), Some("hq1"));
    assert_eq!(summary["peak_heat_flux"].as_f64(), Some(46.6476151588));
    assert_eq!(summary["max_temperature"].as_f64(), Some(100.0));
    assert_eq!(summary["max_heat_flux"].as_f64(), Some(46.6476151588));
    assert_eq!(summary["peak_heat_flux_x"].as_f64(), Some(40.0));
    assert_eq!(summary["peak_heat_flux_y"].as_f64(), Some(24.0));
    assert_eq!(summary["peak_average_temperature"].as_f64(), Some(55.0));
    assert_eq!(summary["peak_temperature_gradient_x"].as_f64(), Some(-20.0));
    assert_eq!(summary["peak_temperature_gradient_y"].as_f64(), Some(-12.0));
    assert_eq!(
        summary["peak_temperature_gradient_magnitude"].as_f64(),
        Some(23.323807579381203)
    );
    assert_eq!(
        summary["thermal_peak_average_temperature"].as_f64(),
        Some(55.0)
    );
    assert_eq!(summary["thermal_flux_peak_x"].as_f64(), Some(40.0));
    assert_eq!(summary["thermal_flux_peak_y"].as_f64(), Some(24.0));
    assert_eq!(
        summary["thermal_flux_peak_element_id"].as_str(),
        Some("hq1")
    );
}

#[test]
fn runs_thermo_peak_response_extract_operator_through_sdk_registry() {
    let summary = run_extract_operator(
        "extract.thermo_peak_response",
        serde_json::json!({
            "input": {
                "nodes": [],
                "elements": []
            },
            "nodes": [
                {
                    "index": 0,
                    "id": "t0",
                    "x": 0.0,
                    "y": 0.0,
                    "ux": 0.001,
                    "uy": 0.002,
                    "displacement_magnitude": 0.0022360679775,
                    "temperature_delta": 25.0
                },
                {
                    "index": 1,
                    "id": "t1",
                    "x": 1.0,
                    "y": 0.0,
                    "ux": 0.003,
                    "uy": 0.004,
                    "displacement_magnitude": 0.005,
                    "temperature_delta": 40.0
                }
            ],
            "elements": [
                {
                    "index": 0,
                    "id": "te0",
                    "node_i": 0,
                    "node_j": 1,
                    "node_k": 1,
                    "node_l": 0,
                    "area": 1.0,
                    "average_temperature_delta": 30.0,
                    "thermal_strain": 0.001,
                    "mechanical_strain_x": 0.002,
                    "mechanical_strain_y": 0.001,
                    "total_strain_x": 0.003,
                    "total_strain_y": 0.002,
                    "gamma_xy": 0.0005,
                    "stress_x": 10.0,
                    "stress_y": 8.0,
                    "tau_xy": 2.0,
                    "principal_stress_1": 11.0,
                    "principal_stress_2": 7.0,
                    "max_in_plane_shear": 2.0,
                    "von_mises": 12.0
                },
                {
                    "index": 1,
                    "id": "te1",
                    "node_i": 0,
                    "node_j": 1,
                    "node_k": 1,
                    "node_l": 0,
                    "area": 1.0,
                    "average_temperature_delta": 35.0,
                    "thermal_strain": 0.0012,
                    "mechanical_strain_x": 0.0025,
                    "mechanical_strain_y": 0.0012,
                    "total_strain_x": 0.0037,
                    "total_strain_y": 0.0024,
                    "gamma_xy": 0.0007,
                    "stress_x": 14.0,
                    "stress_y": 9.0,
                    "tau_xy": 3.0,
                    "principal_stress_1": 15.0,
                    "principal_stress_2": 8.0,
                    "max_in_plane_shear": 3.5,
                    "von_mises": 16.5
                }
            ],
            "max_displacement": 0.005,
            "max_stress": 16.5,
            "max_temperature_delta": 40.0
        }),
        serde_json::Value::Null,
    )
    .expect("extract.thermo_peak_response should succeed");

    assert_eq!(summary["peak_node_id"].as_str(), Some("t1"));
    assert_eq!(summary["peak_element_id"].as_str(), Some("te1"));
    assert_eq!(summary["peak_displacement"].as_f64(), Some(0.005));
    assert_eq!(summary["peak_von_mises"].as_f64(), Some(16.5));
    assert_eq!(summary["max_temperature_delta"].as_f64(), Some(40.0));
    assert_eq!(summary["thermo_displacement_peak_x"].as_f64(), Some(0.003));
    assert_eq!(summary["thermo_displacement_peak_y"].as_f64(), Some(0.004));
    assert_eq!(
        summary["thermo_displacement_peak_element_id"].as_str(),
        Some("t1")
    );
    assert_eq!(summary["peak_stress_x"].as_f64(), Some(14.0));
    assert_eq!(summary["peak_stress_y"].as_f64(), Some(9.0));
    assert_eq!(summary["peak_tau_xy"].as_f64(), Some(3.0));
    assert_eq!(summary["peak_thermal_strain"].as_f64(), Some(0.0012));
    assert_eq!(summary["peak_mechanical_strain_x"].as_f64(), Some(0.0025));
    assert_eq!(summary["peak_mechanical_strain_y"].as_f64(), Some(0.0012));
    assert_eq!(summary["peak_total_strain_x"].as_f64(), Some(0.0037));
    assert_eq!(summary["peak_total_strain_y"].as_f64(), Some(0.0024));
    assert_eq!(summary["peak_gamma_xy"].as_f64(), Some(0.0007));
    assert_eq!(summary["peak_principal_stress_1"].as_f64(), Some(15.0));
    assert_eq!(summary["peak_principal_stress_2"].as_f64(), Some(8.0));
    assert_eq!(summary["peak_max_in_plane_shear"].as_f64(), Some(3.5));
    assert_eq!(summary["thermo_peak_thermal_strain"].as_f64(), Some(0.0012));
    assert_eq!(
        summary["thermo_peak_principal_stress_1"].as_f64(),
        Some(15.0)
    );
    assert_eq!(
        summary["thermo_peak_principal_stress_2"].as_f64(),
        Some(8.0)
    );
    assert_eq!(
        summary["thermo_peak_max_in_plane_shear"].as_f64(),
        Some(3.5)
    );
    assert_eq!(
        summary["thermo_stress_peak_element_id"].as_str(),
        Some("te1")
    );
}

#[test]
fn runs_export_operator_through_sdk_registry() {
    let export = run_export_operator(
        "export.summary_json",
        serde_json::json!({
            "max_temperature": 22.0,
            "mean_temperature": 15.3333333333
        }),
        serde_json::Value::Null,
    )
    .expect("export.summary_json should succeed");

    assert_eq!(export["format"].as_str(), Some("json"));
    assert!(
        export["content"]
            .as_str()
            .is_some_and(|content| content.contains("\"max_temperature\": 22.0"))
    );
}

#[test]
fn runs_transform_operator_through_sdk_registry() {
    let summary = run_transform_operator(
        "transform.compare_summary_pair",
        serde_json::json!({
            "left": { "max_stress": 10.0, "max_displacement": 1.5 },
            "right": { "max_stress": 15.0, "max_displacement": 2.0 }
        }),
        serde_json::json!({
            "left_prefix": "baseline",
            "right_prefix": "candidate",
            "delta_prefix": "delta"
        }),
    )
    .expect("transform.compare_summary_pair should succeed");

    assert_eq!(summary["baseline_max_stress"].as_f64(), Some(10.0));
    assert_eq!(summary["candidate_max_stress"].as_f64(), Some(15.0));
    assert_eq!(summary["delta_max_stress"].as_f64(), Some(5.0));
}

#[test]
fn runs_magnetostatic_guard_and_benchmark_transforms_through_sdk_registry() {
    let guard = run_transform_operator(
        "transform.evaluate_magnetostatic_guard",
        serde_json::json!({
            "magnetostatic_field_peak_magnitude": 13.0,
            "total_stored_energy": 9.5
        }),
        serde_json::json!({
            "rules": [
                {
                    "field": "magnetostatic_field_peak_magnitude",
                    "threshold": 12.0,
                    "severity": "block",
                    "label": "H peak"
                },
                { "field": "total_stored_energy", "threshold": 20.0, "severity": "warn" }
            ]
        }),
    )
    .expect("magnetostatic guard should succeed");

    assert_eq!(guard["guard_status"].as_str(), Some("block"));
    assert_eq!(guard["guard_block_count"].as_u64(), Some(1));
    assert_eq!(guard["guard_trigger_count"].as_u64(), Some(1));

    let benchmark = run_transform_operator(
        "transform.benchmark_magnetostatic_pair",
        serde_json::json!({
            "left": {
                "magnetostatic_field_peak_magnitude": 11.0,
                "total_stored_energy": 7.0
            },
            "right": {
                "magnetostatic_field_peak_magnitude": 13.0,
                "total_stored_energy": 9.5
            }
        }),
        serde_json::json!({
            "left_label": "candidate_a",
            "right_label": "candidate_b",
            "criteria": [
                { "field": "magnetostatic_field_peak_magnitude", "goal": "min", "weight": 2.0 },
                { "field": "total_stored_energy", "goal": "min", "weight": 1.0 }
            ]
        }),
    )
    .expect("magnetostatic benchmark should succeed");

    assert_eq!(benchmark["benchmark_winner"].as_str(), Some("candidate_a"));
    assert_eq!(benchmark["candidate_a_score"].as_f64(), Some(3.0));
    assert_eq!(benchmark["candidate_b_score"].as_f64(), Some(0.0));
}

#[test]
fn runs_bridge_operator_through_sdk_registry() {
    let solved = solve(EngineSolveRequest::HeatPlaneQuad2d(
        SolveHeatPlaneQuad2dRequest {
            nodes: vec![
                HeatPlaneNodeInput {
                    id: "h0".to_string(),
                    x: 0.0,
                    y: 0.0,
                    fix_temperature: true,
                    temperature: 100.0,
                    heat_load: 0.0,
                },
                HeatPlaneNodeInput {
                    id: "h1".to_string(),
                    x: 1.0,
                    y: 0.0,
                    fix_temperature: false,
                    temperature: 0.0,
                    heat_load: 0.0,
                },
                HeatPlaneNodeInput {
                    id: "h2".to_string(),
                    x: 1.0,
                    y: 1.0,
                    fix_temperature: true,
                    temperature: 20.0,
                    heat_load: 0.0,
                },
                HeatPlaneNodeInput {
                    id: "h3".to_string(),
                    x: 0.0,
                    y: 1.0,
                    fix_temperature: true,
                    temperature: 20.0,
                    heat_load: 0.0,
                },
            ],
            elements: vec![HeatPlaneQuadElementInput {
                id: "hq0".to_string(),
                node_i: 0,
                node_j: 1,
                node_k: 2,
                node_l: 3,
                thickness: 0.02,
                conductivity: 45.0,
            }],
        },
    ))
    .expect("heat quad should solve");
    let heat_result = match solved {
        AnalysisResult::HeatPlaneQuad2d(result) => result,
        _ => unreachable!("expected heat quad result"),
    };

    let bridged = run_transform_operator(
        "bridge.temperature_field_to_thermo_quad_2d",
        serde_json::to_value(heat_result).expect("heat result should serialize"),
        serde_json::json!({
            "seed_model": {
                "nodes": [
                    { "id": "n0", "x": 0.0, "y": 0.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0, "temperature_delta": 0.0 },
                    { "id": "n1", "x": 1.0, "y": 0.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0, "temperature_delta": 0.0 },
                    { "id": "n2", "x": 1.0, "y": 1.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0, "temperature_delta": 0.0 },
                    { "id": "n3", "x": 0.0, "y": 1.0, "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0, "temperature_delta": 0.0 }
                ],
                "elements": [
                    { "id": "tq0", "node_i": 0, "node_j": 1, "node_k": 2, "node_l": 3, "thickness": 0.02, "youngs_modulus": 70000000000.0, "poisson_ratio": 0.33, "thermal_expansion": 0.000011 }
                ]
            }
        }),
    )
    .expect("bridge should succeed");

    let model: SolveThermalPlaneQuad2dRequest =
        serde_json::from_value(bridged.clone()).expect("bridged model should decode");
    assert_eq!(model.nodes[0].temperature_delta, 100.0);
    assert_eq!(model.nodes[2].temperature_delta, 20.0);
    assert!(bridged.get("__bridge_diagnostics").is_some());
}
