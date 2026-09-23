use kyuubiki_engine::run_solve_operator;
use kyuubiki_headless_sdk::{
    HeadlessExecutionBatch, build_execution_plan, engine_solver_headless_bridge_manifest,
};
use serde_json::{Value, json};

fn model(quad: bool, conductivity: f64) -> Value {
    let mut nodes = json!([
        {"id":"a", "x":0.0, "y":0.0, "fix_temperature":true, "temperature":0.0},
        {"id":"b", "x":1.0, "y":0.0, "fix_temperature":true, "temperature":20.0},
        {"id":"c", "x":1.0, "y":1.0, "fix_temperature":true, "temperature":10.0},
        {"id":"d", "x":0.0, "y":1.0, "fix_temperature":true, "temperature":-10.0}
    ]);
    let mut element = json!({"id":"bulk", "node_i":0, "node_j":1, "node_k":2,
        "conductivity":conductivity, "thickness":0.01});
    if quad {
        element["node_l"] = json!(3);
    } else {
        nodes.as_array_mut().unwrap().pop();
    }
    json!({"nodes":nodes, "elements":[element]})
}

fn planned_solve(quad: bool, model: Value) -> Result<Value, String> {
    let action = if quad {
        "solve_heat_plane_quad_2d"
    } else {
        "solve_heat_plane_triangle_2d"
    };
    let batch: HeadlessExecutionBatch = serde_json::from_value(json!({
        "schema_version":"kyuubiki.headless-execution-batch/v1",
        "exported_at":"2026-09-23T00:00:00Z", "language":"rust", "workflow_id":"heat-output",
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
fn unrepresentable_heat_fields_fail_through_the_headless_plan_and_allow_clean_replay() {
    for quad in [false, true] {
        for overflow in [false, true] {
            let mut input = model(quad, if overflow { 1e308 } else { 1e-200 });
            if !overflow {
                for node in input["nodes"].as_array_mut().unwrap() {
                    node["temperature"] = json!(node["temperature"].as_f64().unwrap() * 1e-200);
                }
            }
            let error = planned_solve(quad, input).unwrap_err();
            assert!(
                error.contains("heat flux") && error.contains("bulk"),
                "{error}"
            );
            let clean = planned_solve(quad, model(quad, 1.0)).unwrap();
            let maximum = clean["max_heat_flux"].as_f64().unwrap();
            assert!((maximum / 20.0_f64.hypot(10.0) - 1.0).abs() < 1e-12);
        }
    }
}

#[test]
fn representable_extreme_heat_fields_remain_numeric_through_the_headless_plan() {
    for quad in [false, true] {
        for conductivity in [1e-200, 1e200] {
            let result = planned_solve(quad, model(quad, conductivity)).unwrap();
            for field in [
                "heat_flux_x",
                "heat_flux_y",
                "heat_flux_magnitude",
                "heat_flow_rate",
            ] {
                assert!(
                    result["elements"][0][field].is_number(),
                    "{field}: {result}"
                );
            }
            let magnitude = result["max_heat_flux"].as_f64().unwrap();
            assert!((magnitude / (20.0_f64.hypot(10.0) * conductivity) - 1.0).abs() < 1e-12);
        }
    }
}
