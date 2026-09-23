use kyuubiki_engine::run_solve_operator;
use kyuubiki_headless_sdk::{
    HeadlessExecutionBatch, build_execution_plan, engine_solver_headless_bridge_manifest,
};
use kyuubiki_solver::solver_control::{SolverControl, SolverStage, with_solver_observer};
use serde_json::{Value, json};
use std::f64::consts::TAU;

fn model() -> Value {
    json!({
        "nodes":[
            {"id":"base", "x":0.0, "fix_x":true, "load_x":0.0, "mass":1.0},
            {"id":"tip", "x":1.0, "fix_x":false, "load_x":1.0, "mass":1.0}
        ],
        "elements":[{"id":"spring", "node_i":0, "node_j":1, "stiffness":1.0, "damping":0.01}],
        "frequencies_hz":[0.0, 1.0/TAU, 2.0/TAU]
    })
}

fn planned_solve(model: Value) -> Result<Value, String> {
    let batch: HeadlessExecutionBatch = serde_json::from_value(json!({
        "schema_version":"kyuubiki.headless-execution-batch/v1",
        "exported_at":"2026-09-23T00:00:00Z", "language":"rust", "workflow_id":"harmonic-balance",
        "steps":[{"index":1, "action":"solve_harmonic_spring_1d", "risk":"normal", "payload":{"model":model}}]
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
fn headless_harmonic_sweep_preserves_resonance_and_finite_serialized_fields() {
    let result = planned_solve(model()).unwrap();
    let frame = &result["frequencies"][1];
    assert!(
        (frame["nodes"][1]["displacement_phase_deg"]
            .as_f64()
            .unwrap()
            + 90.0)
            .abs()
            < 1e-12
    );
    assert!((result["max_displacement"].as_f64().unwrap() - 100.0).abs() < 1e-10);
    assert_eq!(result["peak_frequency_hz"], json!(1.0 / TAU));
    for frame in result["frequencies"].as_array().unwrap() {
        for field in [
            "max_displacement",
            "max_velocity",
            "max_acceleration",
            "max_force",
        ] {
            assert!(frame[field].as_f64().unwrap().is_finite());
        }
    }
}

#[test]
fn headless_unrepresentable_acceleration_is_an_error_not_a_null_success() {
    let mut input = model();
    input["nodes"][1]["mass"] = json!(1e-200);
    input["nodes"][1]["load_x"] = json!(1e150);
    input["elements"][0]["stiffness"] = json!(2.0);
    input["elements"][0]["damping"] = json!(0.0);
    input["frequencies_hz"] = json!([0.0, 1e100 / TAU]);
    let error = planned_solve(input).unwrap_err();
    assert!(
        error.contains("frequency 1") && error.contains("acceleration"),
        "{error}"
    );
    assert!(planned_solve(model()).is_ok());
}

#[test]
fn headless_singular_frequency_does_not_prevent_a_corrected_batch_from_running() {
    let mut input = model();
    input["elements"][0]["damping"] = json!(0.0);
    let error = planned_solve(input.clone()).unwrap_err();
    assert!(
        error.contains("frequency 1") && error.contains("singular"),
        "{error}"
    );
    input["elements"][0]["damping"] = json!(0.2);
    let result = planned_solve(input).unwrap();
    assert_eq!(result["frequencies"].as_array().unwrap().len(), 3);
    assert!((result["max_displacement"].as_f64().unwrap() - 5.0).abs() < 1e-12);
}

#[test]
fn headless_cancellation_discards_partial_sweeps_and_replays_cleanly() {
    let baseline = planned_solve(model()).unwrap();
    let control = SolverControl::default();
    let cancel = control.clone();
    let error = with_solver_observer(
        &control,
        move |point| {
            if point.stage == SolverStage::HarmonicSweep && point.completed_steps == 1 {
                cancel.request_cancel();
            }
        },
        || {
            let result = planned_solve(model());
            assert!(
                result.is_err(),
                "engine route must not publish earlier frames as success"
            );
            result
        },
    )
    .unwrap_err();
    assert!(error.contains("cancelled"), "{error}");
    assert_eq!(planned_solve(model()).unwrap(), baseline);
}
