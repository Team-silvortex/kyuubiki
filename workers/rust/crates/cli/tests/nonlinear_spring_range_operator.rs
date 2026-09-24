use kyuubiki_engine::run_solve_operator;
use kyuubiki_headless_sdk::{
    HeadlessExecutionBatch, build_execution_plan, engine_solver_headless_bridge_manifest,
};
use serde_json::{Value, json};

fn model(contact: bool, stiffness: f64, cubic: f64, load: f64) -> Value {
    let mut model = json!({
        "nodes":[
            {"id":"base", "x":0.0, "fix_x":true, "load_x":0.0},
            {"id":"tip", "x":1.0, "fix_x":false, "load_x":load}
        ],
        "elements":[{"id":"spring", "node_i":0, "node_j":1,
            "stiffness":stiffness, "cubic_stiffness":cubic}],
        "load_steps":1, "max_iterations":64, "tolerance":1e-12 * load.abs().max(1.0)
    });
    if contact {
        model["contacts"] = json!([{"id":"stop", "node":1, "gap":0.0, "normal_stiffness":1.0}]);
    }
    model
}

fn planned_solve(contact: bool, model: Value) -> Result<Value, String> {
    let action = if contact {
        "solve_contact_gap_1d"
    } else {
        "solve_nonlinear_spring_1d"
    };
    let batch: HeadlessExecutionBatch = serde_json::from_value(json!({
        "schema_version":"kyuubiki.headless-execution-batch/v1",
        "exported_at":"2026-09-24T00:00:00Z", "language":"rust", "workflow_id":"spring-range",
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

fn close(value: &Value, expected: f64) {
    let actual = value
        .as_f64()
        .expect("a successful numerical field must not serialize as null");
    assert!(actual.is_finite());
    assert!(
        (actual / expected - 1.0).abs() < 1e-11,
        "{actual} != {expected}"
    );
}

#[test]
fn headless_linear_limit_keeps_finite_spring_and_contact_outputs() {
    for contact in [false, true] {
        for load in [-2e200, 2e200] {
            let result = planned_solve(contact, model(contact, 1.0, 0.0, load)).unwrap();
            let expected = if contact && load > 0.0 {
                load / 2.0
            } else {
                load
            };
            assert_eq!(result["converged"], true);
            close(&result["nodes"][1]["ux"], expected);
            close(&result["elements"][0]["force"], expected);
            close(&result["elements"][0]["tangent_stiffness"], 1.0);
            close(&result["max_force"], expected.abs());
            if contact {
                assert_eq!(result["contacts"][0]["active"], load > 0.0);
                assert!(result["contacts"][0]["force"].as_f64().unwrap().is_finite());
            }
        }
    }
}

#[test]
fn headless_cubic_model_uses_weighted_powers_in_real_engine_execution() {
    for contact in [false, true] {
        for sign in [-1.0, 1.0] {
            let mut input = model(contact, 1.0, 1e-300, sign * 2e150);
            if contact {
                input["contacts"][0]["gap"] = json!(1e151);
            }
            let result = planned_solve(contact, input).unwrap();
            assert_eq!(result["converged"], true);
            close(&result["nodes"][1]["ux"], sign * 1e150);
            close(&result["elements"][0]["force"], sign * 2e150);
            close(&result["elements"][0]["tangent_stiffness"], 4.0);
        }
        let result = planned_solve(contact, model(contact, 1.0, 1e308, 0.0)).unwrap();
        assert_eq!(result["converged"], true);
        close(&result["elements"][0]["tangent_stiffness"], 1.0);
    }
}

#[test]
fn headless_rejects_final_trial_overflow_instead_of_publishing_json_null() {
    for contact in [false, true] {
        let mut input = model(contact, 1.0, 1e200, 1e100);
        input["max_iterations"] = json!(1);
        let error = planned_solve(contact, input).unwrap_err();
        assert!(
            error.contains("spring") && error.contains("non-finite"),
            "{error}"
        );
        let result = planned_solve(contact, model(contact, 1.0, 0.0, 2.0)).unwrap();
        assert_eq!(result["converged"], true);
        close(&result["nodes"][1]["ux"], if contact { 1.0 } else { 2.0 });
    }
}

#[test]
fn headless_rejects_assembly_overflow_even_without_a_linear_solve() {
    for contact in [false, true] {
        let mut input = model(contact, 1e308, 0.0, 0.0);
        let element = input["elements"][0].clone();
        input["elements"].as_array_mut().unwrap().push(element);
        let error = planned_solve(contact, input).unwrap_err();
        assert!(
            error.contains("tangent") && error.contains("non-finite"),
            "{error}"
        );
    }
}

#[test]
fn headless_contact_penalty_overflow_remains_an_error_across_the_bridge() {
    let mut input = model(true, 1.0, 0.0, 1e100);
    input["max_iterations"] = json!(1);
    input["contacts"][0]["normal_stiffness"] = json!(1e300);
    let error = planned_solve(true, input).unwrap_err();
    assert!(
        error.contains("stop") && error.contains("non-finite"),
        "{error}"
    );
}

#[test]
fn headless_last_correction_can_finish_without_an_extra_iteration_budget() {
    for contact in [false, true] {
        let mut input = model(contact, 1.0, 0.0, -2.0);
        input["max_iterations"] = json!(1);
        let result = planned_solve(contact, input).unwrap();
        assert_eq!(result["converged"], true);
        assert_eq!(result["achieved_load_factor"], 1.0);
        close(&result["nodes"][1]["ux"], -2.0);
        assert_eq!(result["steps"][0]["iterations"], 1);
        assert_eq!(result["residual_norm"], 0.0);
    }
}

#[test]
fn headless_failed_trial_does_not_replace_the_committed_physical_state() {
    for contact in [false, true] {
        let mut input = model(contact, 1.0, 1.0, -2.0);
        input["max_iterations"] = json!(1);
        let result = planned_solve(contact, input).unwrap();
        assert_eq!(result["converged"], false);
        assert_eq!(result["achieved_load_factor"], 0.0);
        assert_eq!(result["nodes"][1]["ux"], 0.0);
        assert_eq!(result["elements"][0]["force"], 0.0);
        assert_eq!(result["max_displacement"], 0.0);
        assert_eq!(result["residual_norm"], 0.0);
        close(&result["steps"][0]["residual_norm"], 8.0);
    }
}

#[test]
fn headless_partial_contact_load_is_explicit_and_replay_can_reach_the_target() {
    let mut input = model(true, 1.0, 0.0, 2.0);
    input["contacts"][0]["gap"] = json!(1.0);
    input["contacts"][0]["normal_stiffness"] = json!(10.0);
    input["load_steps"] = json!(4);
    input["max_iterations"] = json!(1);
    let result = planned_solve(true, input.clone()).unwrap();
    assert_eq!(result["converged"], false);
    assert_eq!(result["achieved_load_factor"], 0.5);
    close(&result["nodes"][1]["ux"], 1.0);
    close(&result["elements"][0]["force"], 1.0);
    assert_eq!(result["contacts"][0]["active"], false);
    assert_eq!(result["active_contact_count"], 0);
    assert_eq!(result["steps"].as_array().unwrap().len(), 3);
    close(&result["steps"][2]["residual_norm"], 5.0);
    input["max_iterations"] = json!(2);
    let replay = planned_solve(true, input.clone()).unwrap();
    assert_eq!(replay["converged"], true);
    assert_eq!(replay["achieved_load_factor"], 1.0);
    close(&replay["nodes"][1]["ux"], 12.0 / 11.0);
    assert_eq!(replay, planned_solve(true, input).unwrap());
}
