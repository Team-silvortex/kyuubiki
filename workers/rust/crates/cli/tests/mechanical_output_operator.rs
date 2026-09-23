use kyuubiki_engine::run_solve_operator;
use kyuubiki_headless_sdk::{
    HeadlessExecutionBatch, build_execution_plan, engine_solver_headless_bridge_manifest,
};
use serde_json::{Value, json};

fn model(quad: bool, modulus: f64, strain: f64) -> Value {
    let nodes: Vec<_> = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]
        .into_iter()
        .enumerate()
        .map(|(i, [x, y])| {
            json!({
                "id":format!("n{i}"), "x":x, "y":y,
                "fix_x":x == 0.0, "fix_y":true, "load_y":0.0,
                "load_x":if x == 1.0 { 0.5 * modulus * strain } else { 0.0 }
            })
        })
        .collect();
    let mut element = json!({"id":"bulk", "node_i":0, "node_j":1, "node_k":2,
        "thickness":1.0, "youngs_modulus":modulus, "poisson_ratio":0.0});
    let elements = if quad {
        element["node_l"] = json!(3);
        vec![element]
    } else {
        let mut second = element.clone();
        second["id"] = json!("upper");
        second["node_j"] = json!(2);
        second["node_k"] = json!(3);
        vec![element, second]
    };
    json!({"nodes":nodes, "elements":elements})
}

fn planned_solve(quad: bool, model: Value) -> Result<Value, String> {
    let action = if quad {
        "solve_plane_quad_2d"
    } else {
        "solve_plane_triangle_2d"
    };
    let batch: HeadlessExecutionBatch = serde_json::from_value(json!({
        "schema_version":"kyuubiki.headless-execution-batch/v1",
        "exported_at":"2026-09-23T00:00:00Z", "language":"rust", "workflow_id":"mechanical-output",
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
fn mechanical_extreme_moduli_keep_analytical_stress_and_energy_through_headless_plan() {
    for quad in [false, true] {
        for modulus in [1e-200, 1.0, 1e200] {
            let result = planned_solve(quad, model(quad, modulus, 1e-3)).unwrap();
            for (key, expected) in [
                ("max_displacement", 1e-3),
                ("max_stress", 1e-3 * modulus),
                ("total_strain_energy", 5e-7 * modulus),
            ] {
                let actual = result[key]
                    .as_f64()
                    .expect("successful output must be numeric");
                assert!(
                    (actual / expected - 1.0).abs() < 1e-10,
                    "{key}: {actual} != {expected}"
                );
            }
        }
    }
}

#[test]
fn mechanical_overflow_is_an_engine_error_and_does_not_poison_replay() {
    for quad in [false, true] {
        // Deliberately outside small-strain validity: test rejection, not physical accuracy.
        let error = planned_solve(quad, model(quad, 1.0, 1e160)).unwrap_err();
        assert!(
            error.contains("bulk") && error.contains("representable"),
            "{error}"
        );
        let result = planned_solve(quad, model(quad, 1.0, 1e-3)).unwrap();
        assert!((result["total_strain_energy"].as_f64().unwrap() / 5e-7 - 1.0).abs() < 1e-10);
    }
}
