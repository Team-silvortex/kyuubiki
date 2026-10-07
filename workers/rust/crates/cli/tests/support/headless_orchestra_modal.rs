use super::agent_support::{LiveAgent, wait_for_lifecycle};
use super::{modal_support, start_live_server_with_agent};
use kyuubiki_headless_sdk::{HeadlessExecutor, OperatorTaskRequestBudget, ServiceHeadlessExecutor};
use serde_json::json;
use std::error::Error;

#[test]
fn real_orchestra_modal_taskir_retains_full_modes_and_independent_residuals()
-> Result<(), Box<dyn Error>> {
    let agent = LiveAgent::start()?;
    let server = start_live_server_with_agent(Some(agent.port()))?;
    let mut executor =
        ServiceHeadlessExecutor::try_new(&format!("http://127.0.0.1:{}", server.port))
            .map_err(|error| error.message)?;

    for (segments, space) in [(96, false), (100, false), (100, true)] {
        let task = modal_support::task(
            &format!("orchestra-modal-{segments}-{space}"),
            space,
            modal_support::model(segments, space),
        );
        let outcome = executor
            .execute_step("operator_task_execute", 1, &json!({"task":task}))
            .map_err(|error| format!("{}\n{}", error.message, server.logs()))?;
        assert_eq!(
            outcome.status,
            "executed",
            "{}\n{}",
            outcome.result,
            server.logs()
        );
        let receipt = &outcome.result["result"];
        assert_eq!(
            outcome.result["execution_budget"],
            OperatorTaskRequestBudget::default().to_value()
        );
        assert_eq!(receipt["operator_task_ir_status"], "executed");
        assert_eq!(
            receipt["execution_runtime_status"],
            "agent_engine_solver_executed"
        );
        assert_eq!(receipt["task_id"], task["task_id"]);
        assert_eq!(receipt["task_digest"], task["integrity"]["task_digest"]);
        assert_eq!(receipt["operator_id"], task["operator"]["id"]);
        // Check the full result before report compaction, including every mode.
        modal_support::check_result(&receipt["result"], segments, space);
        wait_for_lifecycle(&agent, "accepting", 0)?;
    }
    Ok(())
}

#[test]
fn real_orchestra_modal_solver_failure_can_be_repaired_on_the_same_agent()
-> Result<(), Box<dyn Error>> {
    let agent = LiveAgent::start()?;
    let server = start_live_server_with_agent(Some(agent.port()))?;
    let mut executor =
        ServiceHeadlessExecutor::try_new(&format!("http://127.0.0.1:{}", server.port))
            .map_err(|error| error.message)?;
    let mut model = modal_support::model(100, true);
    model["elements"][0]["density"] = json!(0.0);
    let bad = modal_support::task("orchestra-modal-repair", true, model);
    let failure = executor
        .execute_step("operator_task_execute", 1, &json!({"task":bad}))
        .map_err(|error| format!("{}\n{}", error.message, server.logs()))?;
    assert_eq!(failure.status, "failed", "{}", failure.result);
    let receipt = &failure.result["failure_receipt"];
    assert_eq!(receipt["task_digest"], bad["integrity"]["task_digest"]);
    assert_eq!(
        receipt["reason_code"],
        "operator_task_solver_execution_failed"
    );
    assert_eq!(receipt["failure_stage"], "dispatch_engine_solver");
    assert_eq!(receipt["recovery"]["retryable"], false);
    assert_eq!(
        receipt["recovery"]["required_action"],
        "inspect_engine_solver_failure"
    );
    wait_for_lifecycle(&agent, "accepting", 0)?;

    let repaired = modal_support::task(
        "orchestra-modal-repair",
        true,
        modal_support::model(100, true),
    );
    assert_ne!(
        bad["integrity"]["task_digest"],
        repaired["integrity"]["task_digest"]
    );
    let completed = executor
        .execute_step("operator_task_execute", 1, &json!({"task":repaired}))
        .map_err(|error| format!("{}\n{}", error.message, server.logs()))?;
    assert_eq!(completed.status, "executed", "{}", completed.result);
    assert_eq!(completed.result["task_id"], "orchestra-modal-repair");
    assert_eq!(
        completed.result["task_digest"],
        repaired["integrity"]["task_digest"]
    );
    modal_support::check_result(&completed.result["result"]["result"], 100, true);
    wait_for_lifecycle(&agent, "accepting", 0)?;
    Ok(())
}
