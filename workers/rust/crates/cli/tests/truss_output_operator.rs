use kyuubiki_engine::run_solve_operator;
use kyuubiki_headless_sdk::{
    HeadlessExecutionBatch, build_execution_plan, engine_solver_headless_bridge_manifest,
};
use serde_json::{Value, json};

fn model(spatial: bool, modulus: f64, area: f64, length: f64, load: f64) -> Value {
    let nodes: Vec<_> = (0..3)
        .map(|index| {
            let mut node = json!({
                "id": format!("n{index}"), "x": index as f64 * length, "y": 0.0,
                "fix_x": index == 0, "fix_y": true,
                "load_x": if index == 2 { load } else { 0.0 }, "load_y": 0.0
            });
            if spatial {
                node["z"] = json!(0.0);
                node["fix_z"] = json!(true);
                node["load_z"] = json!(0.0);
            }
            node
        })
        .collect();
    let elements: Vec<_> = (0..2)
        .map(|index| {
            json!({
                "id": format!("member-{index}"), "node_i": index, "node_j": index + 1,
                "area": area, "youngs_modulus": modulus
            })
        })
        .collect();
    json!({"nodes": nodes, "elements": elements})
}

fn planned_solve(spatial: bool, input: Value) -> Result<Value, String> {
    let action = if spatial {
        "solve_truss_3d"
    } else {
        "solve_truss_2d"
    };
    let batch: HeadlessExecutionBatch = serde_json::from_value(json!({
        "schema_version": "kyuubiki.headless-execution-batch/v1", "language": "rust",
        "exported_at": "2026-09-27T00:00:00Z", "workflow_id": "mechanical-truss-output",
        "steps": [{"index": 1, "action": action, "risk": "normal", "payload": {"model": input}}]
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
fn headless_mechanical_truss_energy_failure_is_not_success_and_valid_input_replays() {
    for spatial in [false, true] {
        let error = planned_solve(spatial, model(spatial, 1e308, 0.5, 1e4, 1e306)).unwrap_err();
        assert!(
            error.contains("total strain energy") && error.contains("representable"),
            "{error}"
        );
        let result = planned_solve(spatial, model(spatial, 1200.0, 1.0, 1.0, 1.2)).unwrap();
        assert!((result["total_strain_energy"].as_f64().unwrap() / 0.0012 - 1.0).abs() < 1e-12);
        assert!((result["max_displacement"].as_f64().unwrap() / 0.002 - 1.0).abs() < 1e-12);
    }
}

#[test]
fn headless_mechanical_truss_preserves_small_displacement_and_energy_scales() {
    for spatial in [false, true] {
        for (modulus, area, length, load, displacement, energy) in [
            (1e200, 1.0, 1.0, 1.0, 2e-200, 1e-200),
            (1e-100, 1e-100, 1e100, 1e-300, 2.0, 1e-300),
        ] {
            let result =
                planned_solve(spatial, model(spatial, modulus, area, length, load)).unwrap();
            assert!(
                (result["max_displacement"].as_f64().unwrap() / displacement - 1.0).abs() < 1e-12
            );
            assert!((result["total_strain_energy"].as_f64().unwrap() / energy - 1.0).abs() < 1e-12);
        }
    }
}

#[test]
fn headless_mechanical_truss_translation_cannot_bypass_displacement_guard() {
    for spatial in [false, true] {
        for offset in [0.0, 1e6, -1e6] {
            let mut input = model(spatial, 1.0, 1.0, 1.0, 1.0);
            for node in input["nodes"].as_array_mut().unwrap() {
                node["x"] = json!(offset + node["x"].as_f64().unwrap());
            }
            let error = planned_solve(spatial, input).unwrap_err();
            assert!(error.contains("small-deformation limit"), "{error}");
        }
    }
}
