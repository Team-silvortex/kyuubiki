use super::agent_support::{LiveAgent, wait_for_lifecycle};
use super::orchestra_cancellation::{assert_cancelled, cancel_execution};
use super::orchestra_completion::http_json;
use super::start_live_server_with_agent;
use super::write_ack_loss::count;
use kyuubiki_headless_sdk::{HeadlessExecutor, ServiceHeadlessExecutor};
use kyuubiki_protocol::compute_operator_task_digest;
use serde_json::{Value, json};
use std::error::Error;

fn diagnostic_task() -> Value {
    let result = kyuubiki_engine::run_solve_operator("solve.advection_diffusion_bar_1d", json!({
        "scheme":"upwind", "nodes":(0..=256).map(|i| json!({"x":i as f64/256.0,
            "fix_concentration":i==0 || i==256,"concentration":if i==256 {1.0} else {0.0}})).collect::<Vec<_>>(),
        "elements":(0..256).map(|i| json!({"node_i":i,"node_j":i+1,"area":1.0,"diffusivity":1.0,"velocity":20.0})).collect::<Vec<_>>()})).unwrap();
    let mut task: Value = serde_json::from_str(include_str!(
        "../../../../../../schemas/examples.operator-task-ir.json"
    ))
    .unwrap();
    let id = "extract.transport_result_diagnostics";
    task["task_id"] = json!("cancel-native-transport-diagnostics");
    task["operator"] = json!({"id":id,"family":"transport","kind":"extract"});
    task["input_artifact"] = result;
    task["config"] = json!({});
    task["execution_program"]["program_id"] = json!(id);
    task["execution_program"]["program_family"] = json!("transport");
    task["execution_program"]["program_kind"] = json!("extract");
    task["execution_program"]["package_ref"] = Value::Null;
    task["execution_program"]["package_version"] = json!("built-in");
    task["execution_program"]["entrypoint"] =
        json!({"kind":"operator_id","name":id,"operator_kind":"extract"});
    task["runtime_hints"] = json!({"authority_mode":"agent_local","execution_mode":"agent_native",
        "source_ref":format!("builtin://engine/{id}"),"package_ref":null,"package_version":"built-in",
        "agent_fetchable":false,"cache_scope":"none","operator_kind":"extract"});
    task["integrity"]["task_digest"] = json!(compute_operator_task_digest(&task).unwrap());
    task
}

#[test]
fn real_native_diagnostic_scan_cancel_retains_failure_and_explicit_same_task_rerun_recovers()
-> Result<(), Box<dyn Error>> {
    let agent =
        LiveAgent::start_with_solver_hold("result_diagnostics", "run_operator_task_ir", "1")?;
    let server = start_live_server_with_agent(Some(agent.port()))?;
    let task = diagnostic_task();
    let (_, before) = http_json(server.port, "/api/v1/projects", None)?;
    let report = cancel_execution(&agent, server.port, &task, Some("result_diagnostics"))?;
    assert_cancelled(&report, &task, "execute_diagnostics");
    assert!(
        report.steps[0].result_preview["failure_receipt"]["message"]
            .as_str()
            .unwrap()
            .contains("result_diagnostics after 64 steps")
    );
    wait_for_lifecycle(&agent, "accepting", 0)?;
    assert_eq!(
        http_json(server.port, "/api/v1/projects", None)?.1,
        before,
        "cancelled diagnostics reached a downstream write"
    );
    assert_eq!(count(&agent)?, 1, "cancellation must not cause replay");

    let mut executor =
        ServiceHeadlessExecutor::try_new(&format!("http://127.0.0.1:{}", server.port))
            .map_err(|error| error.message)?;
    let task_id = task["task_id"].as_str().unwrap();
    let digest = task["integrity"]["task_digest"].as_str().unwrap();
    let cancelled = executor
        .inspect_operator_task_dispatch(task_id, digest)
        .map_err(|error| error.message)?;
    assert_eq!(cancelled["attempts"].as_array().unwrap().len(), 1);
    assert_eq!(cancelled["attempts"][0]["state"], "observed_failed");
    assert_eq!(cancelled["terminal_result_available"], false);
    let old_attempt = cancelled["attempts"][0]["attempt_id"].clone();

    let recovered = executor
        .execute_step("operator_task_execute", 1, &json!({"task":task.clone()}))
        .map_err(|error| format!("{}\n{}", error.message, server.logs()))?;
    assert_eq!(recovered.status, "executed");
    assert_eq!(recovered.result["task_digest"], digest);
    assert_eq!(
        recovered.result["result"]["operator_task_ir_status"],
        "executed"
    );
    let summary = &recovered.result["result"]["result"];
    assert_eq!(summary["diagnostic_node_count"], 257);
    assert_eq!(summary["diagnostic_element_count"], 256);
    assert_eq!(summary["transport_artificial_diffusivity_peak"], 0.0390625);
    assert_eq!(summary["transport_source_sum"], 0.0);
    assert_eq!(summary["diagnostic_scheme"], "upwind");
    wait_for_lifecycle(&agent, "accepting", 0)?;
    let retained = executor
        .inspect_operator_task_dispatch(task_id, digest)
        .map_err(|error| error.message)?;
    let attempts = retained["attempts"].as_array().unwrap();
    assert_eq!(attempts.len(), 2);
    assert!(
        attempts
            .iter()
            .any(|entry| entry["attempt_id"] == old_attempt && entry["state"] == "observed_failed")
    );
    assert!(
        attempts.iter().any(
            |entry| entry["attempt_id"] != old_attempt && entry["state"] == "observed_executed"
        )
    );
    assert_eq!(
        count(&agent)?,
        2,
        "one cancellation and one explicit rerun only"
    );
    Ok(())
}
