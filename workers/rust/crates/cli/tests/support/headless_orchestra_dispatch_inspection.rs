use super::agent_support::{LiveAgent, wait_for_lifecycle};
use super::orchestra_cancellation::wait_for_task;
use super::orchestra_completion::{assert_bar, bar_task, http_json};
use super::{Scratch, start_live_server_with_state};
use kyuubiki_headless_sdk::{HeadlessExecutor, OperatorTaskRequestBudget, ServiceHeadlessExecutor};
use serde_json::{Value, json};
use std::error::Error;
use std::fs;
use std::path::Path;
use std::thread;

struct Hold<'a>(&'a Path);
impl Drop for Hold<'_> {
    fn drop(&mut self) {
        let _ = fs::remove_file(self.0);
    }
}

fn executor(port: u16) -> ServiceHeadlessExecutor {
    ServiceHeadlessExecutor::new(&format!("http://127.0.0.1:{port}"))
}

fn inspect(port: u16, task: &Value) -> Result<Value, Box<dyn Error>> {
    executor(port)
        .inspect_operator_task_dispatch(
            task["task_id"].as_str().ok_or("missing id")?,
            task["integrity"]["task_digest"]
                .as_str()
                .ok_or("missing digest")?,
        )
        .map_err(|error| error.message.into())
}

fn fetch(port: u16, task: &Value, attempt: &str) -> Result<Value, Box<dyn Error>> {
    executor(port)
        .fetch_operator_task_result(task, attempt)
        .map_err(|error| error.message.into())
}

#[test]
fn real_orchestra_restart_retains_unknown_dispatch_without_peer_probe_or_automatic_replay()
-> Result<(), Box<dyn Error>> {
    let data = Scratch::new()?;
    let agent = LiveAgent::start()?;
    let peer = LiveAgent::start()?;
    let server =
        start_live_server_with_state(Some(agent.port()), Some(peer.port()), Some(&data.0))?;
    let task = bar_task("durable-dispatch-bar", 0.01);
    let (_, before) = http_json(server.port, "/api/v1/projects", None)?;
    fs::write(&agent.hold_path, "durable-dispatch-bar")?;
    let hold = Hold(&agent.hold_path);
    let point = thread::scope(|scope| -> Result<Value, Box<dyn Error>> {
        let request_task = task.clone();
        let port = server.port;
        let worker = scope.spawn(move || {
            executor(port)
                .with_operator_task_budget(OperatorTaskRequestBudget::new(500, 2_000).unwrap())
                .execute_step("operator_task_execute", 1, &json!({"task":request_task}))
        });
        let point = wait_for_task(&agent, "durable-dispatch-bar", None)?;
        let inspected = inspect(server.port, &task)?;
        let observation = &inspected["attempts"][0]["observation"];
        assert_eq!(
            observation["status"], "original_endpoint_reports_active_request",
            "{inspected}"
        );
        assert_eq!(inspected["attempts"][0]["request_id"], point["request_id"]);
        assert_eq!(observation["generation"], point["generation"]);
        assert_eq!(observation["outcome"], "unknown");
        assert_eq!(inspected["automatic_replay_authorized"], false);
        let error = worker
            .join()
            .map_err(|_| "execution worker panicked")?
            .unwrap_err();
        assert!(
            error.message.contains("agent_transport_timeout"),
            "{error:?}"
        );
        Ok(point)
    })?;

    let first = inspect(server.port, &task)?;
    assert_eq!(first["retained_attempt_count"], 1);
    assert_eq!(first["attempts"][0]["state"], "outcome_unknown");
    assert_eq!(first["attempts"][0]["request_id"], point["request_id"]);
    let attempt = first["attempts"][0]["attempt_id"].clone();
    let (_, after) = http_json(server.port, "/api/v1/projects", None)?;
    assert_eq!(before, after);

    // Restart only the owned temporary Orchestra; its dispatch directory stays stable.
    drop(server);
    let restarted =
        start_live_server_with_state(Some(agent.port()), Some(peer.port()), Some(&data.0))?;
    let recovered = inspect(restarted.port, &task)?;
    assert_eq!(recovered["retained_attempt_count"], 1);
    assert_eq!(recovered["attempts"][0]["attempt_id"], attempt);
    assert_eq!(recovered["attempts"][0]["state"], "outcome_unknown");
    assert_eq!(
        recovered["attempts"][0]["observation"]["outcome"],
        "unknown"
    );
    assert_eq!(recovered["terminal_result_available"], false);
    // Keep the computation held until the closed transport cancels its owner.
    // A job-wide cancel after that owner exits would instead cancel the next admission.
    wait_for_lifecycle(&agent, "accepting", 0)?;
    drop(hold);
    let idle = inspect(restarted.port, &task)?;
    assert_eq!(
        idle["attempts"][0]["observation"]["status"],
        "request_not_observed_active"
    );
    assert_eq!(idle["attempts"][0]["observation"]["outcome"], "unknown");
    let terminal = fetch(restarted.port, &task, attempt.as_str().unwrap())?;
    assert_eq!(terminal["status"], "receipt_recovered", "{terminal}");
    assert_eq!(terminal["outcome"], "failed", "{terminal}");
    assert_eq!(terminal["generation"], point["generation"], "{terminal}");
    assert_eq!(
        terminal["completion"]["failure_receipt"]["reason_code"],
        "cancelled"
    );
    assert_eq!(terminal["publication_performed"], false);
    assert_eq!(terminal["automatic_replay_authorized"], false);

    let peer_state = peer.request("untouched-peer", "describe_agent", json!({}))?;
    assert_eq!(
        peer_state["result"]["watchdog"]["total_started_execution_count"], 0,
        "peer got execution before an explicit rerun: {peer_state}"
    );

    // This is an explicit rerun by the test, not a recovery action.
    let replay = executor(restarted.port)
        .execute_step("operator_task_execute", 1, &json!({"task":task.clone()}))
        .map_err(|error| error.message)?;
    assert_bar(&replay.result);
    let final_report = inspect(restarted.port, &task)?;
    assert_eq!(final_report["retained_attempt_count"], 2);
    assert!(
        final_report["attempts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry["attempt_id"] == attempt && entry["state"] == "outcome_unknown")
    );
    assert!(
        final_report["attempts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry["state"] == "observed_executed")
    );
    Ok(())
}

#[test]
fn completed_bar_after_transport_timeout_is_retrievable_without_reexecution()
-> Result<(), Box<dyn Error>> {
    let data = Scratch::new()?;
    let mut agent = LiveAgent::start()?;
    let peer = LiveAgent::start()?;
    let server =
        start_live_server_with_state(Some(agent.port()), Some(peer.port()), Some(&data.0))?;
    let task = bar_task("recover-computed-bar", 0.01);
    let (_, before) = http_json(server.port, "/api/v1/projects", None)?;
    fs::write(&agent.hold_path, "recover-computed-bar")?;
    let hold = Hold(&agent.hold_path);
    thread::scope(|scope| -> Result<(), Box<dyn Error>> {
        let task = task.clone();
        let port = server.port;
        let worker = scope.spawn(move || {
            executor(port)
                .with_operator_task_budget(OperatorTaskRequestBudget::new(500, 250).unwrap())
                .execute_step("operator_task_execute", 1, &json!({"task":task}))
        });
        wait_for_task(&agent, "recover-computed-bar", None)?;
        let error = worker.join().map_err(|_| "request panicked")?.unwrap_err();
        assert!(
            error.message.contains("agent_transport_timeout"),
            "{error:?}"
        );
        Ok(())
    })?;
    // Release before the first heartbeat. The computation may finish even though
    // Orchestra already closed its socket; delivery success is not assumed.
    drop(hold);
    wait_for_lifecycle(&agent, "accepting", 0)?;
    let observed = inspect(server.port, &task)?;
    let attempt = observed["attempts"][0]["attempt_id"]
        .as_str()
        .ok_or("no attempt")?;
    let result = fetch(server.port, &task, attempt)?;
    assert_eq!(result["status"], "receipt_recovered", "{result}");
    assert_eq!(result["outcome"], "executed", "{result}");
    assert_bar(&result["completion"]);
    assert_eq!(result["automatic_replay_authorized"], false);
    assert_eq!(result["publication_performed"], false);
    let (_, after) = http_json(server.port, "/api/v1/projects", None)?;
    assert_eq!(before, after);
    drop(server);
    let restarted =
        start_live_server_with_state(Some(agent.port()), Some(peer.port()), Some(&data.0))?;
    let again = fetch(restarted.port, &task, attempt)?;
    assert_eq!(again, result);
    let owner = agent.request("owner-count", "describe_agent", json!({}))?;
    assert_eq!(
        owner["result"]["watchdog"]["total_started_execution_count"],
        1
    );
    let other = peer.request("peer-count", "describe_agent", json!({}))?;
    assert_eq!(
        other["result"]["watchdog"]["total_started_execution_count"],
        0
    );
    let mut child = agent.child.take().ok_or("Agent already stopped")?;
    child.kill()?;
    child.wait()?;
    agent.start_process()?;
    let missing = fetch(restarted.port, &task, attempt)?;
    assert_eq!(missing["status"], "not_retained", "{missing}");
    assert_eq!(missing["outcome"], "unknown");
    assert_eq!(missing["automatic_replay_authorized"], false);
    let owner = agent.request("after-restart-count", "describe_agent", json!({}))?;
    assert_eq!(
        owner["result"]["watchdog"]["total_started_execution_count"],
        0
    );
    Ok(())
}
