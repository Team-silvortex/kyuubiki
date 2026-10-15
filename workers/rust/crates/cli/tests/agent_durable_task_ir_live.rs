use serde_json::{Value, json};
use std::error::Error;
use std::fs;
use std::thread;
use std::time::{Duration, Instant};

#[allow(dead_code)]
#[path = "support/agent_durable.rs"]
mod support;
use support::*;

fn execution() -> Result<Value, Box<dyn Error>> {
    Ok(
        json!({"execution_id":"a".repeat(32), "method":"run_operator_task_ir",
        "params":{"mode":"execute", "job_id":"durable-task-ir", "dispatch_attempt_id":"c".repeat(32),
        "task_ir":native_task_ir("durable-task-ir")?}}),
    )
}

fn cached(agent: &LiveAgent, task: &Value) -> Result<Value, Box<dyn Error>> {
    let ir = &task["params"]["task_ir"];
    let response = agent.request(
        "cache-query",
        "fetch_operator_task_result",
        json!({
            "attempt_id":task["params"]["dispatch_attempt_id"], "request_id":"original",
            "task_id":ir["task_id"], "task_digest":ir["integrity"]["task_digest"],
            "operator_id":ir["operator"]["id"], "program_id":ir["execution_program"]["program_id"]
        }),
    )?;
    Ok(successful_result(&response, "cache-query").clone())
}

fn wait_cache(agent: &LiveAgent, task: &Value, status: &str) -> Result<Value, Box<dyn Error>> {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let report = cached(agent, task)?;
        if report["status"] == status {
            return Ok(report);
        }
        if Instant::now() >= deadline {
            return Err(format!("cache expected {status}, got {report}").into());
        }
        thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn durable_task_ir_panic_never_leaks_precommit_success_to_transient_cache()
-> Result<(), Box<dyn Error>> {
    let mut agent = LiveAgent::start_with_durable_panic(&"a".repeat(32))?;
    let task = execution()?;
    let response = agent.request("original", "submit_durable_agent_task", task.clone())?;
    assert_eq!(
        successful_result(&response, "original")["status"],
        "running"
    );
    let failed = wait_status(&agent, 'a', "failed")?;
    wait_for_lifecycle(&agent, "accepting", 0)?;
    assert_eq!(
        failed["response"]["error"]["code"],
        "durable_execution_panicked"
    );
    let cache = cached(&agent, &task)?;
    assert_eq!(cache["status"], "receipt_retained");
    assert_eq!(cache["response"], failed["response"]);
    assert_eq!(cache["automatic_replay_authorized"], false);
    agent.stop_process()?;
    agent.start_process()?;
    assert_eq!(fetch(&agent, 'a')?, failed);
    assert_eq!(cached(&agent, &task)?["status"], "not_retained");
    for method in ["submit_durable_agent_task", "run_durable_agent_task"] {
        let duplicate = agent.request("duplicate", method, task.clone())?;
        assert_eq!(*successful_result(&duplicate, "duplicate"), failed);
        assert_eq!(cached(&agent, &task)?["status"], "not_retained");
    }
    assert_eq!(lifecycle(&agent, "no-replay")?["active_execution_count"], 0);
    Ok(())
}

#[test]
fn durable_task_ir_failed_commit_never_exposes_a_successful_cached_response()
-> Result<(), Box<dyn Error>> {
    for (method, panic) in [
        ("submit_durable_agent_task", false),
        ("run_durable_agent_task", false),
        ("submit_durable_agent_task", true),
    ] {
        let mut agent = if panic {
            LiveAgent::start_with_durable_panic(&"a".repeat(32))?
        } else {
            LiveAgent::start_with_durable_store()?
        };
        let task = execution()?;
        let obstruction = agent
            .root
            .join("executions")
            .join(format!("{}.receipt.next", "a".repeat(32)));
        fs::create_dir(&obstruction)?;
        let response = agent.request("original", method, task.clone())?;
        if method == "run_durable_agent_task" {
            assert_eq!(response["ok"], false);
        } else {
            assert_eq!(
                successful_result(&response, "original")["status"],
                "running"
            );
        }
        let unknown = wait_status(&agent, 'a', "outcome_unknown")?;
        let cache = wait_cache(&agent, &task, "result_not_retained")?;
        assert!(cache["response"].is_null());
        assert_eq!(cache["automatic_replay_authorized"], false);
        wait_for_lifecycle(&agent, "accepting", 0)?;
        let descriptor = agent.request("descriptor", "describe_agent", json!({}))?;
        let watchdog = &successful_result(&descriptor, "descriptor")["watchdog"];
        assert_eq!(
            watchdog["total_completed_execution_count"], 0,
            "{method}: failed receipt commit must not count as completed execution"
        );
        assert_eq!(
            watchdog["total_failed_execution_count"], 1,
            "{method}: failed receipt commit must leave a failure diagnostic"
        );
        fs::remove_dir(obstruction)?;
        agent.stop_process()?;
        agent.start_process()?;
        assert_eq!(fetch(&agent, 'a')?, unknown);
        assert_eq!(cached(&agent, &task)?["status"], "not_retained");
        let duplicate = agent.request("duplicate", method, task)?;
        assert_eq!(*successful_result(&duplicate, "duplicate"), unknown);
        assert_eq!(lifecycle(&agent, "no-replay")?["active_execution_count"], 0);
    }
    Ok(())
}

#[test]
fn durable_task_ir_cache_tracks_only_the_committed_receipt_in_sync_and_async_modes()
-> Result<(), Box<dyn Error>> {
    for method in ["submit_durable_agent_task", "run_durable_agent_task"] {
        let mut agent = LiveAgent::start_with_durable_store()?;
        let task = execution()?;
        let descriptor = agent.request("descriptor", "describe_agent", json!({}))?;
        assert_eq!(
            successful_result(&descriptor, "descriptor")["durable_execution"]["task_result_cache"],
            json!({
                "publication_boundary":"durable_receipt_commit", "response_source":"committed_receipt",
                "commit_failure":"result_not_retained", "survives_agent_restart":false, "duplicate_rebuilds_cache":false
            })
        );
        fs::write(&agent.hold_path, "durable-task-ir")?;
        let receipt_hold = agent.root.join("receipt.hold");
        fs::write(&receipt_hold, "a".repeat(32))?;
        let (port, request_task) = (agent.port, task.clone());
        let request = thread::spawn(move || {
            rpc_request(port, "original", method, request_task).map_err(|e| e.to_string())
        });
        wait_status(&agent, 'a', "running")?;
        let pending = wait_cache(&agent, &task, "pending")?;
        assert!(pending["response"].is_null());
        fs::remove_file(&agent.hold_path)?;
        let complete = wait_status(&agent, 'a', "completed")?;
        let cache = wait_cache(&agent, &task, "receipt_retained")?;
        assert_eq!(cache["response"], complete["response"]);
        assert_eq!(cache["response"]["result"]["result"]["max_stress"], 10.0);
        if method == "run_durable_agent_task" {
            assert!(
                !request.is_finished(),
                "cache publication must not wait for transport delivery"
            );
        }
        fs::remove_file(receipt_hold)?;
        let response = request.join().map_err(|_| "request thread panicked")??;
        assert_eq!(response["ok"], true);
        agent.stop_process()?;
        agent.start_process()?;
        assert_eq!(fetch(&agent, 'a')?, complete);
        assert_eq!(cached(&agent, &task)?["status"], "not_retained");
    }
    Ok(())
}

#[test]
fn cancelled_durable_task_ir_publishes_the_same_failure_to_both_recovery_paths()
-> Result<(), Box<dyn Error>> {
    let agent = LiveAgent::start_with_durable_store()?;
    let task = execution()?;
    fs::write(&agent.hold_path, "durable-task-ir")?;
    let admitted = agent.request("original", "submit_durable_agent_task", task.clone())?;
    let admitted = successful_result(&admitted, "original");
    wait_cache(&agent, &task, "pending")?;
    let cancel = agent.request("cancel", "cancel_durable_agent_execution", json!({
        "execution_id":admitted["execution_id"], "process_instance_id":admitted["process_instance_id"],
        "request_digest":admitted["request_digest"]
    }))?;
    assert_eq!(successful_result(&cancel, "cancel")["status"], "requested");
    let failed = wait_status(&agent, 'a', "failed")?;
    assert_eq!(failed["response"]["error"]["code"], "cancelled");
    let cache = wait_cache(&agent, &task, "receipt_retained")?;
    assert_eq!(cache["response"], failed["response"]);
    assert!(cache["response"]["result"].is_null());
    wait_for_lifecycle(&agent, "accepting", 0)?;
    Ok(())
}

#[test]
fn ordinary_task_ir_retention_remains_available_without_a_durable_ledger()
-> Result<(), Box<dyn Error>> {
    let agent = LiveAgent::start()?;
    let task = execution()?;
    let response = agent.request("original", "run_operator_task_ir", task["params"].clone())?;
    assert_eq!(response["ok"], true);
    let cache = cached(&agent, &task)?;
    assert_eq!(cache["status"], "receipt_retained");
    assert_eq!(cache["response"], response);
    Ok(())
}
