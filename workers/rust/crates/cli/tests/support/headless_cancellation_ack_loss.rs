use super::ack_loss_proxy::AckLossProxy;
use super::agent_support::{LiveAgent, wait_for_lifecycle};
use super::orchestra_completion::{assert_bar, bar_task, batch, http_json};
use super::{Scratch, start_live_server_with_state};
use kyuubiki_headless_sdk::{
    CancelExecutionRequest, HeadlessExecutor, HeadlessRunReport, ServiceHeadlessExecutor,
    execute_batch_with_executor,
};
use kyuubiki_protocol::compute_operator_task_digest;
use serde_json::{Value, json};
use std::error::Error;
use std::fs;
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

struct Hold<'a>(&'a Path);
impl Drop for Hold<'_> {
    fn drop(&mut self) {
        let _ = fs::remove_file(self.0);
    }
}

fn client(port: u16) -> ServiceHeadlessExecutor {
    ServiceHeadlessExecutor::new(&format!("http://127.0.0.1:{port}"))
}

fn inspect(port: u16, task: &Value) -> Result<Value, Box<dyn Error>> {
    client(port)
        .inspect_operator_task_dispatch(
            task["task_id"].as_str().unwrap(),
            task["integrity"]["task_digest"].as_str().unwrap(),
        )
        .map_err(|e| e.message.into())
}

fn fetch(port: u16, task: &Value, attempt: &str) -> Result<Value, Box<dyn Error>> {
    client(port)
        .fetch_operator_task_result(task, attempt)
        .map_err(|e| e.message.into())
}

fn cancel(
    port: u16,
    task: &Value,
    attempt: &str,
    target: &CancelExecutionRequest,
) -> Result<Value, kyuubiki_headless_sdk::HeadlessExecutorError> {
    client(port).cancel_operator_task_dispatch(
        task["task_id"].as_str().unwrap(),
        task["integrity"]["task_digest"].as_str().unwrap(),
        attempt,
        target,
    )
}

fn selected(
    port: u16,
    task: &Value,
    point: &Value,
) -> Result<(String, CancelExecutionRequest), Box<dyn Error>> {
    // A bounded read-only inspection can time out while other live suites boot.
    // Retry observation, never cancellation, and keep the owned identity fixed.
    let deadline = Instant::now() + Duration::from_secs(10);
    let report = loop {
        let report = inspect(port, task)?;
        if !report["attempts"][0]["observation"]["execution_target"].is_null() {
            break report;
        }
        if Instant::now() >= deadline {
            return Err(format!("owned target not observable: {report}").into());
        }
        thread::sleep(Duration::from_millis(20));
    };
    assert_eq!(report["retained_attempt_count"], 1);
    let attempt = &report["attempts"][0];
    let target: CancelExecutionRequest =
        serde_json::from_value(attempt["observation"]["execution_target"].clone())?;
    assert_eq!(
        json!(target),
        json!({"process_instance_id":point["process_instance_id"],
        "request_id":point["request_id"], "generation":point["generation"], "job_id":task["task_id"]})
    );
    Ok((attempt["attempt_id"].as_str().unwrap().into(), target))
}

fn execute(port: u16, task: Value) -> HeadlessRunReport {
    execute_batch_with_executor(
        &batch(task, Some("forbidden-after-ack-loss")),
        &mut client(port),
        false,
        false,
    )
}

fn placed_task(id: &str) -> Value {
    let mut task = bar_task(id, 0.01);
    // Use the real placement contract, not incidental pool order, to select
    // the held owner while retaining a known bystander in the control plane.
    task["runtime_hints"]["placement_tags"] = json!(["owned-live-owner"]);
    task["integrity"]["task_digest"] = json!(compute_operator_task_digest(&task).unwrap());
    task
}

fn wait_for_owned_bar(agent: &LiveAgent, task: &Value) -> Result<Value, Box<dyn Error>> {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let state = agent.request("ack-loss-hold", "describe_agent", json!({}))?;
        if let Some(point) = state["result"]["solver_control"]["active"]
            .as_array()
            .and_then(|entries| {
                entries
                    .iter()
                    .find(|entry| entry["job_id"] == task["task_id"])
            })
        {
            let mut point = point.clone();
            assert!(point["checkpoint"].is_null(), "bar started before its hold");
            point["process_instance_id"] =
                state["result"]["lifecycle"]["process_instance_id"].clone();
            return Ok(point);
        }
        if Instant::now() >= deadline {
            return Err("owned bar did not reach its precomputation hold".into());
        }
        thread::sleep(Duration::from_millis(20));
    }
}

fn no_downstream(report: &HeadlessRunReport) {
    assert_eq!(report.status, "failed");
    assert_eq!(report.steps.len(), 1, "downstream write attempted");
    assert_eq!(report.executed_step_count, 0);
}

fn assert_unknown(reply: &Value, target: &CancelExecutionRequest) {
    assert_eq!(reply["status"], "cancellation_outcome_unknown", "{reply}");
    assert!(reply["cancel_registered"].is_null());
    assert!(reply["agent_acknowledgement"].is_null());
    assert_eq!(reply["execution_target"], json!(target));
    assert_eq!(reply["delivery_attempted"], true);
    assert_eq!(reply["uncertainty_reason"], "original_endpoint_unreachable");
    for field in [
        "automatic_replay_authorized",
        "execution_terminal_confirmed",
        "journal_mutation_performed",
        "publication_performed",
        "job_wide_fallback_performed",
    ] {
        assert_eq!(reply[field], false, "{field}: {reply}");
    }
}

fn assert_native_drop(proxy: &AckLossProxy, target: &CancelExecutionRequest, registered: bool) {
    let capture = proxy.capture();
    assert!(
        capture.errors.is_empty(),
        "errors {:?}; replies {:?}",
        capture.errors,
        capture.replies
    );
    let cancels: Vec<_> = capture
        .requests
        .iter()
        .filter(|request| request["method"] == "cancel_execution")
        .collect();
    assert_eq!(
        cancels.len(),
        1,
        "cancel was automatically retried: {capture:?}"
    );
    assert_eq!(cancels[0]["params"], json!(target));
    assert!(!capture.requests.iter().any(|r| r["method"] == "cancel_job"));
    let ack = capture
        .discarded
        .iter()
        .find(|reply| reply["id"] == cancels[0]["id"])
        .unwrap();
    assert_eq!(ack["ok"], true, "{ack}");
    assert_eq!(ack["result"]["execution_target"], json!(target));
    assert_eq!(ack["result"]["cancel_registered"], registered);
    assert_eq!(
        ack["result"]["status"],
        if registered {
            "requested"
        } else {
            "target_not_observed"
        }
    );
    assert_eq!(ack["result"]["pending_cancellation_created"], false);
    assert_eq!(ack["result"]["execution_terminal_confirmed"], false);
    assert_eq!(ack["result"]["operator_package_cleanup_performed"], false);
    assert_eq!(ack["result"]["automatic_replay_authorized"], false);
}

fn assert_run_count(proxy: &AckLossProxy, count: usize) {
    assert_eq!(
        proxy
            .capture()
            .requests
            .iter()
            .filter(|r| r["method"] == "run_operator_task_ir")
            .count(),
        count,
        "execution RPC was retried"
    );
}

fn assert_original(
    port: u16,
    task: &Value,
    attempt: &str,
    target: &CancelExecutionRequest,
    cancelled: bool,
    relay: &AckLossProxy,
) -> Result<Value, Box<dyn Error>> {
    let result = fetch(port, task, attempt)?;
    assert_eq!(
        result["status"],
        "receipt_recovered",
        "{result}; errors {:?}; replies {:?}",
        relay.capture().errors,
        relay.capture().replies
    );
    assert_eq!(result["attempt_id"], attempt);
    assert_eq!(result["request_id"], target.request_id);
    assert_eq!(result["generation"], target.generation);
    assert_eq!(result["process_instance_id"], target.process_instance_id);
    assert_eq!(result["task_digest"], task["integrity"]["task_digest"]);
    assert_eq!(result["automatic_replay_authorized"], false);
    assert_eq!(result["publication_performed"], false);
    if cancelled {
        assert_eq!(result["outcome"], "failed");
        assert_eq!(
            result["completion"]["failure_receipt"]["reason_code"],
            "cancelled"
        );
        assert_eq!(
            result["completion"]["failure_receipt"]["recovery"]["retryable"],
            false
        );
    } else {
        assert_eq!(result["outcome"], "executed");
        assert_bar(&result["completion"]);
    }
    Ok(result)
}

fn started(agent: &LiveAgent) -> Result<u64, Box<dyn Error>> {
    Ok(agent.request("ack-loss-count", "describe_agent", json!({}))?
        ["result"]["watchdog"]["total_started_execution_count"].as_u64().unwrap())
}

fn check_native_loss(cancelled: bool) -> Result<(), Box<dyn Error>> {
    let data = Scratch::new()?;
    let owner = LiveAgent::start()?;
    let peer = LiveAgent::start()?;
    let relay = AckLossProxy::agent(owner.port(), true)?;
    let server = start_live_server_with_state(Some(relay.port), Some(peer.port()), Some(&data.0))?;
    let task = placed_task("native-cancellation-ack-loss");
    let (_, before) = http_json(server.port, "/api/v1/projects", None)?;
    let (attempt, target) = thread::scope(|scope| -> Result<_, Box<dyn Error>> {
        fs::write(&owner.hold_path, task["task_id"].as_str().unwrap())?;
        let hold = Hold(&owner.hold_path);
        let worker = scope.spawn(|| execute(server.port, task.clone()));
        let point = wait_for_owned_bar(&owner, &task)?;
        let (attempt, target) = selected(server.port, &task, &point)?;
        if !cancelled {
            // Finish first, but discard its terminal reply: the retained dispatch
            // is still unknown and a late cancel has a real negative acknowledgement.
            drop(hold);
            no_downstream(&worker.join().map_err(|_| "execution panicked")?);
            wait_for_lifecycle(&owner, "accepting", 0)?;
            let reply = cancel(server.port, &task, &attempt, &target).map_err(|e| e.message)?;
            assert_unknown(&reply, &target);
        } else {
            let reply = cancel(server.port, &task, &attempt, &target).map_err(|e| e.message)?;
            assert_unknown(&reply, &target);
            no_downstream(&worker.join().map_err(|_| "execution panicked")?);
            drop(hold);
            wait_for_lifecycle(&owner, "accepting", 0)?;
        }
        Ok((attempt, target))
    })?;
    assert_native_drop(&relay, &target, cancelled);
    let journal_path = data
        .0
        .join("operator-task-dispatches")
        .join(format!("{attempt}.json"));
    let journal = fs::read(&journal_path)?;
    let inspected = inspect(server.port, &task)?;
    assert_eq!(inspected["attempts"][0]["state"], "outcome_unknown");
    assert_eq!(
        inspected["attempts"][0]["observation"]["outcome"],
        "unknown"
    );
    let original = assert_original(server.port, &task, &attempt, &target, cancelled, &relay)?;
    assert_eq!(
        fetch(server.port, &task, &attempt)?,
        original,
        "read changed original receipt; relay errors: {:?}; replies: {:?}",
        relay.capture().errors,
        relay.capture().replies
    );
    assert_eq!(
        fs::read(&journal_path)?,
        journal,
        "read-only recovery mutated journal"
    );
    assert_eq!(inspect(server.port, &task)?["retained_attempt_count"], 1);
    assert_eq!(started(&owner)?, 1, "recovery automatically reexecuted");
    assert_eq!(started(&peer)?, 0, "peer got fallback execution");
    assert_eq!(http_json(server.port, "/api/v1/projects", None)?.1, before);
    assert_native_drop(&relay, &target, cancelled);
    assert_run_count(&relay, 1);

    // Restart only the owned Orchestra with the same journal and one registered
    // endpoint, so the explicitly requested rerun exercises the original Agent.
    drop(server);
    let server = start_live_server_with_state(Some(relay.port), None, Some(&data.0))?;
    assert_eq!(
        fetch(server.port, &task, &attempt)?,
        original,
        "restart recovery: errors {:?}, replies {:?}",
        relay.capture().errors,
        relay.capture().replies
    );
    // Explicitly authorize one new attempt after inspecting the original receipt.
    // Its reply is deliberately lost too; fetch proves no pending job cancel leaked.
    no_downstream(&execute(server.port, task.clone()));
    wait_for_lifecycle(&owner, "accepting", 0)?;
    let next = inspect(server.port, &task)?;
    assert_eq!(next["retained_attempt_count"], 2);
    let rerun = next["attempts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["attempt_id"] != attempt)
        .unwrap();
    let rerun = fetch(server.port, &task, rerun["attempt_id"].as_str().unwrap())?;
    assert_eq!(
        rerun["status"],
        "receipt_recovered",
        "rerun receipt: {rerun}; relay errors {:?}; replies {:?}",
        relay.capture().errors,
        relay.capture().replies
    );
    assert_bar(&rerun["completion"]);
    assert!(rerun["generation"].as_u64().unwrap() > target.generation);
    assert_eq!(started(&owner)?, 2);
    assert_eq!(started(&peer)?, 0);
    assert_eq!(
        fetch(server.port, &task, &attempt)?,
        original,
        "after rerun: errors {:?}, replies {:?}",
        relay.capture().errors,
        relay.capture().replies
    );
    assert_native_drop(&relay, &target, cancelled);
    assert_run_count(&relay, 2);
    Ok(())
}

#[test]
fn lost_native_cancel_ack_recovers_original_cancelled_receipt_without_replay()
-> Result<(), Box<dyn Error>> {
    check_native_loss(true)
}

#[test]
fn lost_negative_cancel_ack_does_not_relabel_completed_execution_as_cancelled()
-> Result<(), Box<dyn Error>> {
    check_native_loss(false)
}

#[test]
fn lost_http_cancel_ack_keeps_registration_unconfirmed_and_recovers_original_receipt()
-> Result<(), Box<dyn Error>> {
    let data = Scratch::new()?;
    let owner = LiveAgent::start()?;
    let peer = LiveAgent::start()?;
    let server =
        start_live_server_with_state(Some(owner.port()), Some(peer.port()), Some(&data.0))?;
    let relay = AckLossProxy::http(server.port)?;
    let task = placed_task("http-cancellation-ack-loss");
    let (_, before) = http_json(server.port, "/api/v1/projects", None)?;
    let (attempt, target) = thread::scope(|scope| -> Result<_, Box<dyn Error>> {
        fs::write(&owner.hold_path, task["task_id"].as_str().unwrap())?;
        let hold = Hold(&owner.hold_path);
        let worker = scope.spawn(|| execute(server.port, task.clone()));
        let point = wait_for_owned_bar(&owner, &task)?;
        let (attempt, target) = selected(server.port, &task, &point)?;
        let error = cancel(relay.port, &task, &attempt, &target).unwrap_err();
        assert!(
            error.message.contains("registration is unconfirmed"),
            "{error:?}"
        );
        assert!(
            error.message.contains("automatic retry is not authorized"),
            "{error:?}"
        );
        let report = worker.join().map_err(|_| "execution panicked")?;
        no_downstream(&report);
        assert_eq!(
            report.steps[0].result_preview["failure_receipt"]["reason_code"],
            "cancelled"
        );
        drop(hold);
        wait_for_lifecycle(&owner, "accepting", 0)?;
        Ok((attempt, target))
    })?;
    let capture = relay.capture();
    assert!(capture.errors.is_empty(), "{capture:?}");
    assert_eq!(capture.requests.len(), 1, "SDK retried cancellation");
    assert_eq!(capture.requests[0]["attempt_id"], attempt);
    assert_eq!(capture.requests[0]["execution_target"], json!(target));
    assert_eq!(capture.discarded.len(), 1);
    assert_eq!(capture.discarded[0]["status"], "requested");
    assert_eq!(capture.discarded[0]["cancel_registered"], true);
    assert_eq!(capture.discarded[0]["execution_target"], json!(target));
    for field in [
        "automatic_replay_authorized",
        "execution_terminal_confirmed",
        "journal_mutation_performed",
        "publication_performed",
        "job_wide_fallback_performed",
    ] {
        assert_eq!(capture.discarded[0][field], false);
    }
    assert_eq!(
        capture.discarded[0]["agent_acknowledgement"]["cancel_registered"],
        true
    );
    let original = assert_original(server.port, &task, &attempt, &target, true, &relay)?;
    assert_eq!(fetch(server.port, &task, &attempt)?, original);
    assert_eq!(inspect(server.port, &task)?["retained_attempt_count"], 1);
    assert_eq!(started(&owner)?, 1);
    assert_eq!(started(&peer)?, 0);
    assert_eq!(http_json(server.port, "/api/v1/projects", None)?.1, before);
    drop(server);
    let server = start_live_server_with_state(Some(owner.port()), None, Some(&data.0))?;
    assert_eq!(fetch(server.port, &task, &attempt)?, original);
    let replay = client(server.port)
        .execute_step("operator_task_execute", 1, &json!({"task":task}))
        .map_err(|e| e.message)?;
    assert_bar(&replay.result);
    assert_eq!(started(&owner)?, 2);
    assert_eq!(started(&peer)?, 0);
    assert_eq!(relay.capture().requests.len(), 1);
    Ok(())
}
