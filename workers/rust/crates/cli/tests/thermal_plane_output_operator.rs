use kyuubiki_engine::run_solve_operator;
use kyuubiki_headless_sdk::{
    HeadlessExecutionBatch, build_execution_plan, engine_solver_headless_bridge_manifest,
};
use serde_json::{Value, json};

fn model(quad: bool, temperature: f64, expansion: f64) -> Value {
    let points = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    let nodes: Vec<_> = points[..if quad { 4 } else { 3 }]
        .iter()
        .enumerate()
        .map(|(i, [x, y])| {
            json!({"id":format!("n{i}"), "x":x, "y":y, "fix_x":true, "fix_y":true,
            "load_x":0.0, "load_y":0.0, "temperature_delta":temperature})
        })
        .collect();
    let mut element = json!({"id":"thermal-bulk", "node_i":0, "node_j":1, "node_k":2,
        "thickness":1.0, "youngs_modulus":1200.0, "poisson_ratio":0.25, "thermal_expansion":expansion});
    if quad {
        element["node_l"] = json!(3);
    }
    json!({"nodes":nodes, "elements":[element]})
}

fn planned_solve(quad: bool, model: Value) -> Result<Value, String> {
    let action = if quad {
        "solve_thermal_plane_quad_2d"
    } else {
        "solve_thermal_plane_triangle_2d"
    };
    let batch: HeadlessExecutionBatch = serde_json::from_value(json!({
        "schema_version":"kyuubiki.headless-execution-batch/v1", "language":"rust",
        "exported_at":"2026-09-27T00:00:00Z", "workflow_id":"thermal-output",
        "steps":[{"index":1, "action":action, "risk":"normal", "payload":{"model":model}}]
    }))
    .unwrap();
    let plan = build_execution_plan(&batch);
    assert!(plan.ok, "{:?}", plan.validation);
    let manifest = engine_solver_headless_bridge_manifest();
    let route = manifest
        .routes
        .iter()
        .find(|route| route.action == plan.steps[0].action)
        .unwrap();
    run_solve_operator(
        &route.engine_operator_id,
        plan.steps[0].payload["model"].clone(),
    )
}

#[test]
fn headless_thermal_temperature_means_remain_numeric_and_constant() {
    for quad in [false, true] {
        for temperature in [f64::MAX, -f64::MAX, f64::from_bits(1)] {
            let result = planned_solve(quad, model(quad, temperature, 0.0)).unwrap();
            assert_eq!(
                result["elements"][0]["average_temperature_delta"].as_f64(),
                Some(temperature)
            );
            assert_eq!(result["total_strain_energy"].as_f64(), Some(0.0));
            assert_eq!(result["max_stress"].as_f64(), Some(0.0));
        }
    }
}

#[test]
fn headless_thermal_energy_overflow_returns_an_error_and_normal_patch_replays() {
    for quad in [false, true] {
        let error = planned_solve(quad, model(quad, 1.0, 1e155)).unwrap_err();
        assert!(
            error.contains("thermal-bulk") && error.contains("representable"),
            "{error}"
        );
        let result = planned_solve(quad, model(quad, 50.0, 1e-5)).unwrap();
        assert!((result["elements"][0]["stress_x"].as_f64().unwrap() + 0.8).abs() < 1e-12);
        let energy = if quad { 0.0004 } else { 0.0002 };
        assert!((result["total_strain_energy"].as_f64().unwrap() / energy - 1.0).abs() < 1e-12);
    }
}

#[test]
fn headless_thermal_scaled_volume_retains_finite_energy() {
    for quad in [false, true] {
        let mut input = model(quad, 50.0, 1e-5);
        for node in input["nodes"].as_array_mut().unwrap() {
            node["x"] = json!(node["x"].as_f64().unwrap() * 1e100);
            node["y"] = json!(node["y"].as_f64().unwrap() * 1e100);
        }
        input["elements"][0]["youngs_modulus"] = json!(1.2e203);
        input["elements"][0]["thickness"] = json!(1e-200);
        let result = planned_solve(quad, input).unwrap();
        let expected = if quad { 4e196 } else { 2e196 };
        assert!((result["total_strain_energy"].as_f64().unwrap() / expected - 1.0).abs() < 1e-12);
    }
}
