use kyuubiki_engine::run_solve_operator;
use kyuubiki_headless_sdk::{
    HeadlessExecutionBatch, build_execution_plan, engine_solver_headless_bridge_manifest,
};
use serde_json::{Value, json};

fn model(thermal: bool) -> Value {
    let mut element = json!({
        "id": "beam-0", "node_i": 0, "node_j": 1,
        "youngs_modulus": 1000.0, "moment_of_inertia": 1.0,
        "section_modulus": 1.0, "distributed_load_y": 10.0
    });
    if thermal {
        element["thermal_expansion"] = json!(0.01);
        element["section_depth"] = json!(1.0);
        element["temperature_gradient_y"] = json!(0.0);
    }
    json!({
        "nodes": [
            {"id": "left", "x": 0.0, "fix_y": true, "fix_rz": false, "load_y": 0.0, "moment_z": 0.0},
            {"id": "right", "x": 2.0, "fix_y": true, "fix_rz": false, "load_y": 0.0, "moment_z": 0.0}
        ],
        "elements": [element]
    })
}

fn planned_solve(thermal: bool, input: Value) -> Result<Value, String> {
    let action = if thermal {
        "solve_thermal_beam_1d"
    } else {
        "solve_beam_1d"
    };
    let batch: HeadlessExecutionBatch = serde_json::from_value(json!({
        "schema_version": "kyuubiki.headless-execution-batch/v1", "language": "rust",
        "exported_at": "2026-09-27T00:00:00Z", "workflow_id": "beam-output",
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

fn close(value: &Value, expected: f64) {
    let actual = value.as_f64().expect("physical result must be numeric");
    assert!(actual.is_finite());
    assert!((actual - expected).abs() < 1e-10, "{actual} != {expected}");
}

#[test]
fn headless_uniform_beam_recovers_interior_moment_and_energy() {
    for thermal in [false, true] {
        let result = planned_solve(thermal, model(thermal)).unwrap();
        close(&result["max_moment"], 5.0);
        close(&result["max_stress"], 5.0);
        close(&result["elements"][0]["max_bending_stress"], 5.0);
        close(
            &result["total_strain_energy"],
            100.0 * 32.0 / (240.0 * 1000.0),
        );
    }
}

#[test]
fn headless_restrained_thermal_beam_has_energy_without_displacement() {
    let mut input = model(true);
    input["nodes"][0]["fix_rz"] = json!(true);
    input["nodes"][1]["fix_rz"] = json!(true);
    input["elements"][0]["distributed_load_y"] = json!(0.0);
    input["elements"][0]["temperature_gradient_y"] = json!(2.0);
    let result = planned_solve(true, input).unwrap();
    close(&result["max_displacement"], 0.0);
    close(&result["max_moment"], 20.0);
    close(&result["total_strain_energy"], 0.4);
}

#[test]
fn headless_beam_reversed_connectivity_keeps_global_rotations() {
    for thermal in [false, true] {
        let mut input = model(thermal);
        input["nodes"][0]["fix_rz"] = json!(true);
        input["nodes"][1]["fix_y"] = json!(false);
        input["elements"][0]["node_i"] = json!(1);
        input["elements"][0]["node_j"] = json!(0);
        let result = planned_solve(thermal, input).unwrap();
        close(&result["nodes"][1]["uy"], 0.02);
        close(&result["nodes"][1]["rz"], 10.0 * 8.0 / 6000.0);
        close(&result["elements"][0]["moment_j"], -20.0);
        close(&result["total_strain_energy"], 0.08);
    }
}

#[test]
fn headless_beam_numerical_failure_is_not_success_and_normal_input_replays() {
    for thermal in [false, true] {
        let mut input = model(thermal);
        input["nodes"][0]["fix_rz"] = json!(true);
        input["nodes"][1]["fix_rz"] = json!(true);
        input["elements"][0]["youngs_modulus"] = json!(1e308);
        input["elements"][0]["moment_of_inertia"] = json!(1e308);
        let error = planned_solve(thermal, input).unwrap_err();
        assert!(
            error.contains("beam-0") && error.contains("representable"),
            "{error}"
        );
        close(
            &planned_solve(thermal, model(thermal)).unwrap()["max_moment"],
            5.0,
        );
    }
}
