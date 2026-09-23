use kyuubiki_engine::run_solve_operator;
use kyuubiki_headless_sdk::{
    HeadlessExecutionBatch, build_execution_plan, engine_solver_headless_bridge_manifest,
};
use serde_json::{Value, json};

fn model() -> Value {
    json!({
        "nodes":[
            {"id":"base", "x":0.0, "fix_x":true, "load_x":0.0, "mass":2.0,
                "initial_displacement":0.0, "initial_velocity":0.0},
            {"id":"tip", "x":1.0, "fix_x":false, "load_x":10.0, "mass":2.0,
                "initial_displacement":0.1, "initial_velocity":0.0}
        ],
        "elements":[{"id":"spring", "node_i":0, "node_j":1, "stiffness":100.0, "damping":0.5}],
        "time_step":1e-10, "steps":20, "history_stride":20
    })
}

fn planned_solve(model: Value) -> Result<Value, String> {
    let batch: HeadlessExecutionBatch = serde_json::from_value(json!({
        "schema_version":"kyuubiki.headless-execution-batch/v1",
        "exported_at":"2026-09-23T00:00:00Z", "language":"rust", "workflow_id":"transient-balance",
        "steps":[{"index":1, "action":"solve_transient_spring_1d", "risk":"normal", "payload":{"model":model}}]
    })).unwrap();
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
fn headless_transient_plan_preserves_static_equilibrium_at_small_time_steps() {
    let result = planned_solve(model()).unwrap();
    let tip = &result["nodes"][1];
    assert!((tip["ux"].as_f64().unwrap() - 0.1).abs() < 1e-14);
    assert!(tip["vx"].as_f64().unwrap().abs() < 1e-10);
    assert!(tip["ax"].as_f64().unwrap().abs() < 1e-8);
    assert_eq!(result["history"].as_array().unwrap().len(), 2);
}

#[test]
fn headless_sparse_history_reports_intermediate_energy_failure_and_allows_replay() {
    let mut input = model();
    input["nodes"][1]["mass"] = json!(1.0);
    input["nodes"][1]["initial_displacement"] = json!(0.0);
    input["nodes"][1]["load_x"] = json!(1.2e154);
    input["elements"][0]["stiffness"] = json!(1.0);
    input["elements"][0]["damping"] = json!(0.0);
    input["time_step"] = json!(0.1);
    input["steps"] = json!(63);
    input["history_stride"] = json!(63);
    let error = planned_solve(input).unwrap_err();
    assert!(
        error.contains("step") && error.contains("energy"),
        "{error}"
    );
    assert!(planned_solve(model()).is_ok());
}

#[test]
fn headless_result_state_can_seed_the_next_transient_batch() {
    let mut input = model();
    input["time_step"] = json!(0.01);
    input["nodes"][1]["load_x"] = json!(12.0);
    let full = planned_solve(input.clone()).unwrap();
    input["steps"] = json!(7);
    let first = planned_solve(input.clone()).unwrap();
    for index in 0..2 {
        input["nodes"][index]["initial_displacement"] = first["nodes"][index]["ux"].clone();
        input["nodes"][index]["initial_velocity"] = first["nodes"][index]["vx"].clone();
    }
    input["steps"] = json!(13);
    let resumed = planned_solve(input).unwrap();
    assert_eq!(resumed["nodes"], full["nodes"]);
    assert_eq!(resumed["elements"], full["elements"]);
}
