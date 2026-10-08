use super::agent_support::{LiveAgent, wait_for_lifecycle};
use super::orchestra_completion::{assert_bar, bar_task, batch, http_json};
use super::{modal_support, start_live_server_with_agent};
use kyuubiki_headless_sdk::{
    CancelExecutionRequest, HeadlessExecutor, HeadlessRunReport, OperatorTaskRequestBudget,
    ServiceHeadlessExecutor, execute_batch_with_executor,
};
use serde_json::{Value, json};
use std::error::Error;
use std::fs;
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

struct HoldMarker<'a>(&'a Path);
impl Drop for HoldMarker<'_> {
    fn drop(&mut self) {
        let _ = fs::remove_file(self.0);
    }
}

fn executor(port: u16) -> Result<ServiceHeadlessExecutor, String> {
    ServiceHeadlessExecutor::try_new(&format!("http://127.0.0.1:{port}"))
        .map_err(|error| error.message)
}

pub(super) fn wait_for_task(
    agent: &LiveAgent,
    task_id: &str,
    stage: Option<&str>,
) -> Result<Value, Box<dyn Error>> {
    let deadline = Instant::now() + OperatorTaskRequestBudget::default().request_timeout();
    loop {
        let state = agent.request("cancel-progress", "describe_agent", json!({}))?;
        if let Some(entry) = state["result"]["solver_control"]["active"]
            .as_array()
            .and_then(|entries| {
                entries.iter().find(|entry| {
                    entry["job_id"] == task_id
                        && stage.is_none_or(|stage| {
                            entry["checkpoint"]["stage"] == stage
                                && entry["checkpoint"]["completed_steps"]
                                    .as_u64()
                                    .is_some_and(|n| n >= 3)
                        })
                })
            })
        {
            let mut point = entry.clone();
            point["process_instance_id"] =
                state["result"]["lifecycle"]["process_instance_id"].clone();
            return Ok(point);
        }
        if Instant::now() >= deadline {
            return Err(format!(
                "task {task_id} did not reach owned hold stage {stage:?}: {state}"
            )
            .into());
        }
        thread::sleep(Duration::from_millis(20));
    }
}

pub(super) fn cancel_execution(
    agent: &LiveAgent,
    port: u16,
    task: &Value,
    stage: Option<&str>,
) -> Result<HeadlessRunReport, Box<dyn Error>> {
    let job = task["task_id"].as_str().ok_or("missing task id")?;
    thread::scope(|scope| {
        fs::write(&agent.hold_path, job)?;
        // Release the owned hold on every exit, before joining the HTTP request.
        let marker = HoldMarker(&agent.hold_path);
        let workflow = batch(task.clone(), Some("forbidden-after-cancellation"));
        let request = scope.spawn(move || {
            let mut executor = executor(port)?;
            Ok::<_, String>(execute_batch_with_executor(
                &workflow,
                &mut executor,
                false,
                false,
            ))
        });
        let cancelled = (|| -> Result<(), Box<dyn Error>> {
            let point = wait_for_task(agent, job, stage)?;
            if stage.is_none() {
                assert!(
                    point["checkpoint"].is_null(),
                    "calculation started before cancellation: {point}"
                );
            } else {
                assert_eq!(point["checkpoint"]["resumable"], false);
            }
            let client = executor(port)?;
            let task_id = task["task_id"].as_str().ok_or("missing task id")?;
            let digest = task["integrity"]["task_digest"]
                .as_str()
                .ok_or("missing task digest")?;
            let inspection = client
                .inspect_operator_task_dispatch(task_id, digest)
                .map_err(|error| error.message)?;
            let attempt = inspection["attempts"]
                .as_array()
                .ok_or("missing attempts")?
                .iter()
                .find(|attempt| attempt["request_id"] == point["request_id"])
                .ok_or("owned execution not retained")?;
            let target = attempt["observation"]["execution_target"].clone();
            assert_eq!(
                target,
                json!({"process_instance_id":point["process_instance_id"],
                "request_id":point["request_id"], "generation":point["generation"], "job_id":job})
            );
            let typed_target: CancelExecutionRequest = serde_json::from_value(target.clone())?;
            let reply = client
                .cancel_operator_task_dispatch(
                    task_id,
                    digest,
                    attempt["attempt_id"].as_str().ok_or("missing attempt id")?,
                    &typed_target,
                )
                .map_err(|error| error.message)?;
            assert_eq!(
                reply["schema_version"],
                "kyuubiki.operator-task-dispatch-cancellation/v1"
            );
            assert_eq!(reply["execution_target"], target);
            assert_eq!(reply["status"], "requested");
            assert_eq!(reply["cancel_registered"], true);
            assert_eq!(reply["job_wide_fallback_performed"], false);
            assert_eq!(reply["journal_mutation_performed"], false);
            assert_eq!(
                reply["agent_acknowledgement"]["pending_cancellation_created"],
                false
            );
            assert_eq!(
                reply["agent_acknowledgement"]["operator_package_cleanup_performed"],
                false
            );
            assert_eq!(reply["execution_terminal_confirmed"], false);
            Ok(())
        })();
        // Leave the hold in place until the cancellation response, not until normal success.
        if let Err(error) = cancelled {
            drop(marker);
            let _ = request.join();
            return Err(error);
        }
        let report = request
            .join()
            .map_err(|_| "cancellation HTTP worker panicked")??;
        drop(marker);
        Ok(report)
    })
}

pub(super) fn assert_cancelled(report: &HeadlessRunReport, task: &Value, stage: &str) {
    assert_eq!(report.status, "failed", "{report:?}");
    assert_eq!(report.executed_step_count, 0);
    assert_eq!(report.steps.len(), 1, "downstream step was attempted");
    let receipt = &report.steps[0].result_preview;
    assert_eq!(receipt["status"], "failed");
    let failure = &receipt["failure_receipt"];
    assert_eq!(
        failure["schema_version"],
        "kyuubiki.agent-operator-task-failure/v1"
    );
    assert_eq!(failure["failure_owner"], "agent_runtime");
    assert_eq!(failure["reason_code"], "cancelled");
    assert_eq!(failure["failure_stage"], stage);
    assert_eq!(failure["task_id"], task["task_id"]);
    assert_eq!(failure["operator_id"], task["operator"]["id"]);
    assert_eq!(failure["task_digest"], task["integrity"]["task_digest"]);
    assert_eq!(failure["recovery"]["retryable"], false);
    assert_eq!(failure["recovery"]["safe_to_continue_other_tasks"], true);
    assert_eq!(
        failure["recovery"]["required_action"],
        "inspect_cancellation_before_explicit_rerun"
    );
    assert!(
        receipt["result"]["result"].is_null(),
        "cancelled result escaped publication"
    );
    assert_eq!(receipt["execution_readiness"]["ready_to_dispatch"], false);
    let recovery = report
        .execution_summary
        .failure
        .as_ref()
        .expect("missing cancellation summary");
    assert_eq!(
        recovery.recommended_action,
        "inspect_cancellation_before_explicit_rerun"
    );
    assert!(!recovery.retryable);
}

#[test]
fn real_orchestra_precomputation_cancel_stops_downstream_and_allows_same_task_rerun()
-> Result<(), Box<dyn Error>> {
    let agent = LiveAgent::start()?;
    let server = start_live_server_with_agent(Some(agent.port()))?;
    let task = bar_task("cancel-before-bar", 0.01);
    let (_, before) = http_json(server.port, "/api/v1/projects", None)?;
    let report = cancel_execution(&agent, server.port, &task, None)?;
    assert_cancelled(&report, &task, "before_execution");
    let observed = executor(server.port)?
        .inspect_operator_task_dispatch(
            task["task_id"].as_str().ok_or("missing task id")?,
            task["integrity"]["task_digest"]
                .as_str()
                .ok_or("missing digest")?,
        )
        .map_err(|error| error.message)?;
    assert_eq!(observed["attempts"][0]["state"], "observed_failed");
    assert_eq!(observed["terminal_result_available"], false);
    wait_for_lifecycle(&agent, "accepting", 0)?;
    let (_, after) = http_json(server.port, "/api/v1/projects", None)?;
    assert_eq!(before, after, "cancellation created a downstream project");
    let mut executor = executor(server.port)?;
    let replay = executor
        .execute_step("operator_task_execute", 1, &json!({"task":task}))
        .map_err(|error| format!("{}\n{}", error.message, server.logs()))?;
    assert_eq!(replay.status, "executed");
    assert_bar(&replay.result);
    wait_for_lifecycle(&agent, "accepting", 0)?;
    Ok(())
}

fn check_modal_cancellation(stage: &str, space: bool) -> Result<(), Box<dyn Error>> {
    let agent = LiveAgent::start_with_solver_hold(stage, "run_operator_task_ir", "1")?;
    let server = start_live_server_with_agent(Some(agent.port()))?;
    let task = modal_support::task(
        "cancel-numerical-modal",
        space,
        modal_support::model(100, space),
    );
    let (_, before) = http_json(server.port, "/api/v1/projects", None)?;
    let report = cancel_execution(&agent, server.port, &task, Some(stage))?;
    assert_cancelled(&report, &task, "execute_solver");
    wait_for_lifecycle(&agent, "accepting", 0)?;
    let (_, after) = http_json(server.port, "/api/v1/projects", None)?;
    assert_eq!(before, after, "cancellation created a downstream project");
    let mut executor = executor(server.port)?;
    let replay = executor
        .execute_step("operator_task_execute", 1, &json!({"task":task}))
        .map_err(|error| format!("{}\n{}", error.message, server.logs()))?;
    assert_eq!(replay.status, "executed", "{}", replay.result);
    assert_eq!(
        replay.result["task_digest"],
        task["integrity"]["task_digest"]
    );
    modal_support::check_result(&replay.result["result"]["result"], 100, space);
    wait_for_lifecycle(&agent, "accepting", 0)?;
    Ok(())
}

#[test]
fn real_orchestra_modal_matvec_cancel_recovers_without_publishing_partial_results()
-> Result<(), Box<dyn Error>> {
    check_modal_cancellation("sparse_matvec", false)
}

#[test]
fn real_orchestra_spatial_modal_final_validation_cancel_recovers() -> Result<(), Box<dyn Error>> {
    check_modal_cancellation("modal_shape_norm", true)
}
