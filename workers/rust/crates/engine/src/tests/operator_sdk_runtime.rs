use crate::{
    solve,
    workflow_executor::{run_export_operator, run_extract_operator, run_transform_operator},
    EngineSolveRequest,
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
    assert!(export["content"]
        .as_str()
        .is_some_and(|content| content.contains("\"max_temperature\": 22.0")));
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
