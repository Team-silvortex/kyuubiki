use kyuubiki_engine::run_solve_operator;
use kyuubiki_headless_sdk::{
    HeadlessExecutionBatch, build_execution_plan, engine_solver_headless_bridge_manifest,
};
use serde_json::{Value, json};

fn model(thermal: bool) -> Value {
    let mut input = json!({
        "nodes": [
            {"id": "root", "x": 0.0, "y": 0.0, "fix_x": true, "fix_y": true,
             "fix_rz": true, "load_x": 0.0, "load_y": 0.0, "moment_z": 0.0},
            {"id": "tip", "x": 2.0, "y": 0.0, "fix_x": false, "fix_y": false,
             "fix_rz": false, "load_x": 10.0, "load_y": 20.0, "moment_z": 5.0}
        ],
        "elements": [{"id": "member-0", "node_i": 0, "node_j": 1,
            "youngs_modulus": 1000.0, "area": 2.0, "moment_of_inertia": 1.0, "section_modulus": 1.0}]
    });
    if thermal {
        for node in input["nodes"].as_array_mut().unwrap() {
            node["temperature_delta"] = json!(0.0);
        }
        input["elements"][0]["thermal_expansion"] = json!(1e-5);
        input["elements"][0]["section_depth"] = json!(1.0);
        input["elements"][0]["temperature_gradient_y"] = json!(0.0);
    }
    input
}

fn planned_solve(thermal: bool, input: Value) -> Result<Value, String> {
    let action = if thermal {
        "solve_thermal_frame_2d"
    } else {
        "solve_frame_2d"
    };
    let batch: HeadlessExecutionBatch = serde_json::from_value(json!({
        "schema_version": "kyuubiki.headless-execution-batch/v1", "language": "rust",
        "exported_at": "2026-09-27T00:00:00Z", "workflow_id": "frame-2d-output",
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
    let actual = value.as_f64().expect("physical field must remain numeric");
    assert!(actual.is_finite());
    assert!(
        (actual / expected - 1.0).abs() < 1e-10,
        "{actual:e} != {expected:e}"
    );
}

#[test]
fn headless_thermal_frame_zero_temperature_matches_mechanical_work() {
    for thermal in [false, true] {
        let result = planned_solve(thermal, model(thermal)).unwrap();
        let tip = &result["nodes"][1];
        let work = 0.5
            * (10.0 * tip["ux"].as_f64().unwrap()
                + 20.0 * tip["uy"].as_f64().unwrap()
                + 5.0 * tip["rz"].as_f64().unwrap());
        close(&result["total_strain_energy"], 0.8083333333333333);
        close(&result["total_strain_energy"], work);
    }
}

#[test]
fn headless_free_thermal_expansion_preserves_mechanical_strain_energy() {
    let mut input = model(true);
    for node in input["nodes"].as_array_mut().unwrap() {
        node["temperature_delta"] = json!(20.0);
    }
    input["elements"][0]["temperature_gradient_y"] = json!(10.0);
    let result = planned_solve(true, input).unwrap();
    close(&result["total_strain_energy"], 0.8083333333333333);
    close(&result["nodes"][1]["ux"], 0.0104);
    close(
        &result["nodes"][1]["uy"],
        20.0 * 8.0 / 3000.0 + 5.0 * 4.0 / 2000.0 + 0.0002,
    );
}

#[test]
fn headless_frame_numerical_failure_is_not_success_and_valid_input_replays() {
    for thermal in [false, true] {
        let mut input = model(thermal);
        for node in input["nodes"].as_array_mut().unwrap() {
            for field in ["fix_x", "fix_y", "fix_rz"] {
                node[field] = json!(true);
            }
        }
        input["elements"][0]["youngs_modulus"] = json!(1e200);
        input["elements"][0]["area"] = json!(1e200);
        let error = planned_solve(thermal, input).unwrap_err();
        assert!(
            error.contains("member-0") && error.contains("representable"),
            "{error}"
        );
        close(
            &planned_solve(thermal, model(thermal)).unwrap()["total_strain_energy"],
            0.8083333333333333,
        );
    }
}

#[test]
fn headless_frame_tiny_response_remains_numeric_and_nonzero() {
    for thermal in [false, true] {
        let mut input = model(thermal);
        input["nodes"][1]["load_x"] = json!(1.0);
        input["nodes"][1]["load_y"] = json!(0.0);
        input["nodes"][1]["moment_z"] = json!(0.0);
        input["elements"][0]["youngs_modulus"] = json!(1e200);
        input["elements"][0]["area"] = json!(1.0);
        let result = planned_solve(thermal, input).unwrap();
        close(&result["max_displacement"], 2e-200);
        close(&result["total_strain_energy"], 1e-200);
    }
}
