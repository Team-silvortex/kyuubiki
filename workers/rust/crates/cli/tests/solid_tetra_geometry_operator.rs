use kyuubiki_engine::run_solve_operator;
use kyuubiki_headless_sdk::{
    HeadlessExecutionBatch, build_execution_plan, engine_solver_headless_bridge_manifest,
};
use serde_json::{Value, json};

fn model(offset: f64) -> Value {
    let nodes: Vec<_> = [
        [0.0; 3],
        [1.0, 0.25, 0.0],
        [0.25, 1.25, 0.25],
        [0.0, 0.25, 1.5],
    ]
    .into_iter()
    .enumerate()
    .map(|(i, [x, y, z])| {
        json!({
            "id":format!("n{i}"), "x":x + offset, "y":y - offset, "z":z + offset,
            "fix_x":i != 3, "fix_y":i != 3, "fix_z":i != 3,
            "load_x":if i == 3 { 3.0 } else { 0.0 },
            "load_y":if i == 3 { -2.0 } else { 0.0 },
            "load_z":if i == 3 { 1.0 } else { 0.0 }
        })
    })
    .collect();
    json!({
        "nodes":nodes,
        "elements":[{"id":"bulk", "node_a":0, "node_b":1, "node_c":2, "node_d":3,
            "youngs_modulus":1200.0, "poisson_ratio":0.25}]
    })
}

fn planned_solve(model: Value) -> Result<Value, String> {
    let batch: HeadlessExecutionBatch = serde_json::from_value(json!({
        "schema_version":"kyuubiki.headless-execution-batch/v1",
        "exported_at":"2026-09-27T00:00:00Z", "language":"rust", "workflow_id":"solid-geometry",
        "steps":[{"index":1, "action":"solve_solid_tetra_3d", "risk":"normal", "payload":{"model":model}}]
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
    assert_eq!(route.engine_operator_id, "solve.solid_tetra_3d");
    run_solve_operator(
        &route.engine_operator_id,
        plan.steps[0].payload["model"].clone(),
    )
}

fn close(actual: &Value, expected: &Value) {
    let actual = actual
        .as_f64()
        .expect("successful numerical output must not be null");
    let expected = expected.as_f64().unwrap();
    assert!(
        actual.is_finite() && (actual - expected).abs() <= 2e-11 * expected.abs().max(1e-9),
        "{actual:e} != {expected:e}"
    );
}

#[test]
fn solid_geometry_translation_survives_headless_plan_and_engine_dispatch() {
    let base = planned_solve(model(0.0)).unwrap();
    for offset in [2f64.powi(40), -2f64.powi(40)] {
        let result = planned_solve(model(offset)).unwrap();
        for key in [
            "total_volume",
            "total_strain_energy",
            "max_von_mises_stress",
            "max_displacement",
        ] {
            close(&result[key], &base[key]);
        }
        let nodes = result["nodes"].as_array().unwrap();
        assert_eq!(nodes.len(), 4);
        for (index, node) in nodes.iter().enumerate() {
            for key in ["ux", "uy", "uz", "reaction_x", "reaction_y", "reaction_z"] {
                close(&node[key], &base["nodes"][index][key]);
            }
        }
        for key in [
            "force_balance_relative_error",
            "free_residual_relative_error",
        ] {
            assert!(result["equilibrium"][key].as_f64().unwrap() < 1e-12);
        }
    }
}

#[test]
fn solid_invalid_coefficients_fail_in_engine_without_poisoning_next_request() {
    let mut invalid = model(0.0);
    invalid["elements"][0]["youngs_modulus"] = json!(f64::MAX);
    invalid["elements"][0]["poisson_ratio"] = json!(0.49);
    let error = planned_solve(invalid).unwrap_err();
    assert!(
        error.contains("bulk") && error.contains("coefficients are not representable"),
        "{error}"
    );
    let result = planned_solve(model(0.0)).unwrap();
    let energy = result["total_strain_energy"].as_f64().unwrap();
    assert!(energy.is_finite() && energy > 0.0);
}
