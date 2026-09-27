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

#[test]
fn triangle_mixed_orientation_keeps_signed_stress_through_headless_plan() {
    for mask in 0..4 {
        let mut input = model(false, 1200.0, 1e-3);
        for (i, element) in input["elements"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .enumerate()
        {
            if mask & (1 << i) != 0 {
                let j = element["node_j"].clone();
                element["node_j"] = element["node_k"].clone();
                element["node_k"] = j;
            }
        }
        let result = planned_solve(false, input).unwrap();
        for element in result["elements"].as_array().unwrap() {
            assert!((element["stress_x"].as_f64().unwrap() - 1.2).abs() < 1e-11);
        }
        assert!((result["total_strain_energy"].as_f64().unwrap() / 0.0006 - 1.0).abs() < 1e-11);
    }
}

#[test]
fn q4_translation_preserves_analytical_response_through_headless_plan() {
    for offset in [2f64.powi(40), -2f64.powi(40)] {
        let mut input = model(true, 1200.0, 1e-3);
        for node in input["nodes"].as_array_mut().unwrap() {
            node["x"] = json!(node["x"].as_f64().unwrap() + offset);
            node["y"] = json!(node["y"].as_f64().unwrap() - offset);
        }
        let result = planned_solve(true, input).unwrap();
        for (key, expected) in [
            ("max_displacement", 1e-3),
            ("max_stress", 1.2),
            ("total_strain_energy", 0.0006),
        ] {
            let actual = result[key]
                .as_f64()
                .expect("successful output must be numeric");
            assert!(
                (actual / expected - 1.0).abs() < 1e-11,
                "{key}: {actual} != {expected}"
            );
        }
    }
}

#[test]
fn q4_coefficient_overflow_is_identified_before_assembly_and_headless_replay_succeeds() {
    let mut invalid = model(true, 1200.0, 1e-3);
    invalid["elements"][0]["youngs_modulus"] = json!(f64::MAX);
    invalid["elements"][0]["poisson_ratio"] = json!(0.49);
    let error = planned_solve(true, invalid).unwrap_err();
    assert!(
        error.contains("bulk") && error.contains("coefficients are not representable"),
        "{error}"
    );
    let result = planned_solve(true, model(true, 1200.0, 1e-3)).unwrap();
    assert!((result["total_strain_energy"].as_f64().unwrap() / 0.0006 - 1.0).abs() < 1e-11);
}
