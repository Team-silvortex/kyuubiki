use kyuubiki_engine::run_solve_operator;
use kyuubiki_headless_sdk::{
    HeadlessExecutionBatch, build_execution_plan, engine_solver_headless_bridge_manifest,
};
use kyuubiki_solver::solver_control::{SolverControl, SolverStage, with_solver_observer};
use serde_json::{Value, json};

fn model() -> Value {
    json!({
        "buckling": {"frame": {
            "nodes": (0..=8).map(|index| json!({
                "id": format!("n{index}"), "x": 0.0, "y": 0.4 * index as f64,
                "fix_x": index == 0 || index == 8, "fix_y": index == 0,
                "fix_rz": false, "load_x": 0.0,
                "load_y": if index == 8 { -100_000.0 } else { 0.0 }, "moment_z": 0.0
            })).collect::<Vec<_>>(),
            "elements": (0..8).map(|index| json!({
                "id": format!("e{index}"), "node_i": index, "node_j": index + 1,
                "area": 0.01, "youngs_modulus": 205e9, "moment_of_inertia": 7.4e-6,
                "section_modulus": 1e-4
            })).collect::<Vec<_>>()
        }, "mode_count": 1},
        "imperfection_amplitude": 0.0032, "load_steps": 4
    })
}

fn planned_solve(model: Value) -> Result<Value, String> {
    let batch: HeadlessExecutionBatch = serde_json::from_value(json!({
        "schema_version": "kyuubiki.headless-execution-batch/v1",
        "exported_at": "2026-09-23T00:00:00Z", "language": "rust", "workflow_id": "p-delta-path",
        "steps": [{"index": 1, "action": "solve_frame_2d_p_delta", "risk": "normal", "payload": {"model": model}}]
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
fn headless_shape_rescaling_keeps_the_same_physical_path() {
    let baseline = planned_solve(model()).unwrap();
    for scale in [1e-200, 1e200] {
        let mut input = model();
        input["imperfection_shape"] = json!(
            baseline["initial_imperfection_shape"]
                .as_array()
                .unwrap()
                .iter()
                .map(|value| value.as_f64().unwrap() * scale)
                .collect::<Vec<_>>()
        );
        let result = planned_solve(input).unwrap();
        assert_eq!(result["converged"], true);
        let expected = baseline["max_imperfection_amplification"].as_f64().unwrap();
        assert!(
            (result["max_imperfection_amplification"].as_f64().unwrap() / expected - 1.0).abs()
                < 1e-9
        );
    }
}

#[test]
fn headless_extreme_amplitudes_do_not_serialize_successful_metrics_as_null() {
    for amplitude in [1e-200, 1e155] {
        let mut input = model();
        input["imperfection_amplitude"] = json!(amplitude);
        let result = planned_solve(input).unwrap();
        for step in result["steps"].as_array().unwrap() {
            assert_eq!(step["converged"], true);
            for key in [
                "residual_norm",
                "max_incremental_displacement",
                "imperfection_amplification",
            ] {
                assert!(
                    step[key].as_f64().is_some_and(f64::is_finite),
                    "{key}: {}",
                    step[key]
                );
            }
            let expected = 1.0 / (1.0 - step["critical_factor_ratio"].as_f64().unwrap());
            assert!(
                (step["imperfection_amplification"].as_f64().unwrap() / expected - 1.0).abs()
                    < 1e-7
            );
        }
    }
}

#[test]
fn headless_invalid_mode_index_returns_a_domain_error_and_replays_cleanly() {
    let mut input = model();
    input["imperfection_mode_index"] = json!(usize::MAX);
    let error = planned_solve(input).unwrap_err();
    assert!(
        error.contains("imperfection mode") && error.contains("unavailable"),
        "{error}"
    );
    assert_eq!(planned_solve(model()).unwrap()["converged"], true);
}

#[test]
fn headless_in_path_cancellation_is_not_a_successful_shortened_path() {
    let baseline = planned_solve(model()).unwrap();
    let control = SolverControl::default();
    let cancel = control.clone();
    let error = with_solver_observer(
        &control,
        move |point| {
            if point.stage == SolverStage::StabilityStep && point.completed_steps == 1 {
                cancel.request_cancel();
            }
        },
        || {
            let result = planned_solve(model());
            assert!(
                result.is_err(),
                "engine route must observe cancellation inside the path"
            );
            result
        },
    )
    .unwrap_err();
    assert!(error.contains("cancelled"), "{error}");
    assert_eq!(planned_solve(model()).unwrap(), baseline);
}
