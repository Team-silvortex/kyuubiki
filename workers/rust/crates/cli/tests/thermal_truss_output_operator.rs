use kyuubiki_engine::run_solve_operator;
use kyuubiki_headless_sdk::{
    HeadlessExecutionBatch, build_execution_plan, engine_solver_headless_bridge_manifest,
};
use serde_json::{Value, json};

fn model(spatial: bool, temperature: f64, expansion: f64) -> Value {
    let nodes: Vec<_> = (0..3)
        .map(|index| {
            let mut node = json!({
                "id": format!("n{index}"), "x": index as f64, "y": 0.0,
                "fix_x": true, "fix_y": true, "load_x": 0.0, "load_y": 0.0,
                "temperature_delta": temperature
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
                "area": 1.0, "youngs_modulus": 1200.0, "thermal_expansion": expansion
            })
        })
        .collect();
    json!({"nodes": nodes, "elements": elements})
}

fn planned_solve(spatial: bool, input: Value) -> Result<Value, String> {
    let action = if spatial {
        "solve_thermal_truss_3d"
    } else {
        "solve_thermal_truss_2d"
    };
    let batch: HeadlessExecutionBatch = serde_json::from_value(json!({
        "schema_version": "kyuubiki.headless-execution-batch/v1", "language": "rust",
        "exported_at": "2026-09-27T00:00:00Z", "workflow_id": "thermal-truss-output",
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
fn headless_thermal_truss_nonfinite_energy_is_an_error_and_normal_input_replays() {
    for spatial in [false, true] {
        let error = planned_solve(spatial, model(spatial, 1.0, 1e155)).unwrap_err();
        assert!(
            error.contains("member-0") && error.contains("representable"),
            "{error}"
        );
        let result = planned_solve(spatial, model(spatial, 50.0, 1e-5)).unwrap();
        let stress = result["elements"][0]["stress"].as_f64().unwrap();
        let energy = result["total_strain_energy"].as_f64().unwrap();
        assert!((stress + 0.6).abs() < 1e-12);
        assert!((energy / 0.0003 - 1.0).abs() < 1e-12);
    }
}

#[test]
fn headless_thermal_truss_temperature_mean_stays_numeric() {
    for spatial in [false, true] {
        for temperature in [f64::MAX, -f64::MAX, f64::from_bits(1)] {
            let result = planned_solve(spatial, model(spatial, temperature, 0.0)).unwrap();
            assert_eq!(
                result["elements"][0]["average_temperature_delta"].as_f64(),
                Some(temperature)
            );
            assert_eq!(result["total_strain_energy"].as_f64(), Some(0.0));
        }
    }
}

#[test]
fn headless_thermal_truss_translated_excessive_deformation_is_rejected() {
    for spatial in [false, true] {
        for offset in [0.0, 1e6, -1e6] {
            let mut input = model(spatial, 0.0, 0.0);
            for (index, node) in input["nodes"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .enumerate()
            {
                node["x"] = json!(offset + index as f64);
                node["fix_x"] = json!(index == 0);
            }
            input["nodes"][2]["load_x"] = json!(1200.0);
            let error = planned_solve(spatial, input).unwrap_err();
            assert!(error.contains("small-deformation limit"), "{error}");
        }
    }
}
