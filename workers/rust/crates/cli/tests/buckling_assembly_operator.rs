use kyuubiki_engine::run_solve_operator;
use kyuubiki_headless_sdk::{
    HeadlessExecutionBatch, build_execution_plan, engine_solver_headless_bridge_manifest,
};
use kyuubiki_solver::solver_control::{SolverControl, SolverStage, with_solver_observer};
use serde_json::{Value, json};

fn beam_model() -> Value {
    json!({
        "nodes":(0..=8).map(|index| json!({
            "id":format!("n{index}"), "x":0.4 * index as f64,
            "fix_y":index == 0 || index == 8, "fix_rz":false
        })).collect::<Vec<_>>(),
        "elements":(0..8).map(|index| json!({
            "id":format!("e{index}"), "node_i":index, "node_j":index + 1,
            "youngs_modulus":205e9, "moment_of_inertia":7.4e-6,
            "reference_compressive_force":100_000.0
        })).collect::<Vec<_>>(), "mode_count":3
    })
}

fn frame_model() -> Value {
    let beam = beam_model();
    json!({"frame":{
        "nodes":beam["nodes"].as_array().unwrap().iter().enumerate().map(|(index, node)| json!({
            "id":node["id"], "x":node["x"], "y":0.0,
            "fix_x":index == 0, "fix_y":node["fix_y"], "fix_rz":false,
            "load_x":if index == 8 { -100_000.0 } else { 0.0 }, "load_y":0.0, "moment_z":0.0
        })).collect::<Vec<_>>(),
        "elements":beam["elements"].as_array().unwrap().iter().map(|element| json!({
            "id":element["id"], "node_i":element["node_i"], "node_j":element["node_j"],
            "youngs_modulus":element["youngs_modulus"], "moment_of_inertia":element["moment_of_inertia"],
            "area":0.01, "section_modulus":1e-4
        })).collect::<Vec<_>>()
    }, "mode_count":3})
}

fn planned_solve(action: &str, model: Value) -> Result<Value, String> {
    let batch: HeadlessExecutionBatch = serde_json::from_value(json!({
        "schema_version":"kyuubiki.headless-execution-batch/v1",
        "exported_at":"2026-09-23T00:00:00Z", "language":"rust", "workflow_id":"buckling-assembly",
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
fn headless_reversed_beam_matches_forward_modes_and_the_euler_reference() {
    let mut input = beam_model();
    let baseline = planned_solve("solve_buckling_beam_1d", input.clone()).unwrap();
    for element in input["elements"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .step_by(2)
    {
        let first = element["node_i"].clone();
        element["node_i"] = element["node_j"].clone();
        element["node_j"] = first;
    }
    let result = planned_solve("solve_buckling_beam_1d", input).unwrap();
    assert_eq!(result["modes"], baseline["modes"]);
    let expected = std::f64::consts::PI.powi(2) * 205e9 * 7.4e-6 / 3.2_f64.powi(2);
    assert!(
        (result["minimum_load_factor"].as_f64().unwrap() * 100_000.0 / expected - 1.0).abs() < 1e-4
    );
}

#[test]
fn headless_small_scaled_frame_keeps_compressive_preloads_and_load_factors() {
    let mut input = frame_model();
    let baseline = planned_solve("solve_buckling_frame_2d", input.clone()).unwrap();
    for node in input["frame"]["nodes"].as_array_mut().unwrap() {
        node["load_x"] = json!(node["load_x"].as_f64().unwrap() * 1e-20);
    }
    for element in input["frame"]["elements"].as_array_mut().unwrap() {
        element["youngs_modulus"] = json!(element["youngs_modulus"].as_f64().unwrap() * 1e-20);
    }
    let result = planned_solve("solve_buckling_frame_2d", input).unwrap();
    assert!(
        (result["minimum_load_factor"].as_f64().unwrap()
            / baseline["minimum_load_factor"].as_f64().unwrap()
            - 1.0)
            .abs()
            < 1e-8
    );
    for element in result["element_preloads"].as_array().unwrap() {
        assert_eq!(element["active_in_geometric_stiffness"], true);
        assert!(element["reference_compressive_force"].as_f64().unwrap() > 0.0);
    }
}

#[test]
fn headless_invalid_assembly_reports_the_element_and_replays_cleanly() {
    let mut input = beam_model();
    input["elements"][2]["youngs_modulus"] = json!(1e308);
    input["elements"][2]["moment_of_inertia"] = json!(2.0);
    let error = planned_solve("solve_buckling_beam_1d", input).unwrap_err();
    assert!(
        error.contains("element 2") && error.contains("finite"),
        "{error}"
    );
    assert!(planned_solve("solve_buckling_beam_1d", beam_model()).is_ok());
}

#[test]
fn headless_mode_recovery_cancellation_is_not_partial_success() {
    for (action, model) in [
        ("solve_buckling_beam_1d", beam_model()),
        ("solve_buckling_frame_2d", frame_model()),
    ] {
        let baseline = planned_solve(action, model.clone()).unwrap();
        let control = SolverControl::default();
        let cancel = control.clone();
        let error = with_solver_observer(
            &control,
            move |point| {
                if point.stage == SolverStage::ResultFreeDofs && point.completed_steps > 0 {
                    cancel.request_cancel();
                }
            },
            || {
                let result = planned_solve(action, model.clone());
                assert!(
                    result.is_err(),
                    "cancel must be observed inside the engine route"
                );
                result
            },
        )
        .unwrap_err();
        assert!(error.contains("cancelled"), "{error}");
        assert_eq!(planned_solve(action, model).unwrap(), baseline);
    }
}
