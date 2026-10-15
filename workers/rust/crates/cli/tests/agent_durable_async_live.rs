use serde_json::{Value, json};
use std::error::Error;
use std::fs;
use std::io::Write;
use std::net::TcpStream;
use std::thread;
use std::time::{Duration, Instant};

#[allow(dead_code)]
#[path = "support/agent_durable.rs"]
mod support;
use support::*;

#[test]
fn async_admission_returns_while_held_and_duplicates_do_not_consume_capacity()
-> Result<(), Box<dyn Error>> {
    let mut agent = LiveAgent::start_with_durable_store()?;
    fs::write(&agent.hold_path, "async-held")?;
    let response = agent.request(
        "submit",
        "submit_durable_agent_task",
        task('a', "async-held"),
    )?;
    assert_eq!(successful_result(&response, "submit")["status"], "running");
    assert_eq!(lifecycle(&agent, "active")?["active_execution_count"], 1);
    for method in ["submit_durable_agent_task", "run_durable_agent_task"] {
        let duplicate = agent.request("duplicate", method, task('a', "async-held"))?;
        assert_eq!(
            successful_result(&duplicate, "duplicate")["status"],
            "running"
        );
    }
    let conflict = agent.request(
        "conflict",
        "submit_durable_agent_task",
        task('a', "other-input"),
    )?;
    assert_eq!(conflict["ok"], false);
    let full = agent.request("full", "submit_durable_agent_task", task('b', "capacity"))?;
    let rejected = successful_result(&full, "full");
    assert_eq!(
        rejected["schema_version"],
        kyuubiki_protocol::DURABLE_AGENT_EXECUTION_SCHEMA
    );
    assert_eq!(rejected["status"], "failed");
    assert_eq!(rejected["execution_id"], "b".repeat(32));
    assert_eq!(rejected["request_id"], "full");
    assert_eq!(
        rejected["response"]["error"]["code"],
        "durable_admission_rejected"
    );
    assert_eq!(
        rejected["response"]["error"]["details"]["execution_started"],
        false
    );
    assert_eq!(rejected["automatic_replay_authorized"], false);
    assert!(
        rejected["response"]["error"]["message"]
            .as_str()
            .unwrap()
            .contains("agent_at_capacity")
    );
    assert_eq!(fetch(&agent, 'b')?, *rejected);
    assert_eq!(
        lifecycle(&agent, "after-rejection")?["active_execution_count"],
        1
    );
    fs::remove_file(&agent.hold_path)?;
    let report = wait_status(&agent, 'a', "completed")?;
    assert_eq!(report["response"]["id"], "submit");
    assert_eq!(report["response"]["result"]["max_stress"], 10.0);
    wait_for_lifecycle(&agent, "accepting", 0)?;
    agent.stop_process()?;
    agent.start_process()?;
    assert_eq!(fetch(&agent, 'b')?, *rejected);
    let duplicate = agent.request(
        "must-not-run",
        "submit_durable_agent_task",
        task('b', "capacity"),
    )?;
    assert_eq!(*successful_result(&duplicate, "must-not-run"), *rejected);
    assert_eq!(
        lifecycle(&agent, "rejected-retry")?["active_execution_count"],
        0
    );
    let manual = agent.request(
        "manual-new-run",
        "submit_durable_agent_task",
        task('c', "capacity"),
    )?;
    assert_eq!(
        successful_result(&manual, "manual-new-run")["status"],
        "running"
    );
    assert_eq!(
        wait_status(&agent, 'c', "completed")?["response"]["id"],
        "manual-new-run"
    );
    Ok(())
}

#[test]
fn lost_admission_connection_does_not_cancel_worker_and_original_receipt_survives_kill()
-> Result<(), Box<dyn Error>> {
    let mut agent = LiveAgent::start_with_durable_store()?;
    fs::write(&agent.hold_path, "lost-admission")?;
    let bytes = serde_json::to_vec(&json!({"rpc_version":RPC_VERSION, "id":"lost-ack",
        "method":"submit_durable_agent_task", "params":task('c', "lost-admission")}))?;
    let mut socket = TcpStream::connect(("127.0.0.1", agent.port))?;
    socket.write_all(&(bytes.len() as u32).to_be_bytes())?;
    socket.write_all(&bytes)?;
    drop(socket); // No response read, and no socket is retained by the worker.
    wait_status(&agent, 'c', "running")?;
    fs::remove_file(&agent.hold_path)?;
    let original = wait_status(&agent, 'c', "completed")?;
    assert_eq!(original["response"]["result"]["max_stress"], 10.0);
    fs::write(&agent.hold_path, "lost-admission")?;
    for _ in 0..2 {
        agent.stop_process()?;
        agent.start_process()?;
        assert_eq!(fetch(&agent, 'c')?, original);
        let duplicate = agent.request(
            "reconnect",
            "submit_durable_agent_task",
            task('c', "lost-admission"),
        )?;
        assert_eq!(*successful_result(&duplicate, "reconnect"), original);
        assert_eq!(lifecycle(&agent, "no-replay")?["active_execution_count"], 0);
    }
    Ok(())
}

#[test]
fn lost_rejection_reply_survives_force_kill_and_never_reexecutes_the_rejected_identity()
-> Result<(), Box<dyn Error>> {
    let mut agent = LiveAgent::start_with_durable_store()?;
    fs::write(&agent.hold_path, "keep-busy")?;
    let first = agent.request("busy", "submit_durable_agent_task", task('a', "keep-busy"))?;
    assert_eq!(successful_result(&first, "busy")["status"], "running");
    let receipt_hold = agent.root.join("receipt.hold");
    fs::write(&receipt_hold, "b".repeat(32))?;
    let payload = serde_json::to_vec(&json!({"rpc_version":RPC_VERSION,
        "id":"rejection-not-received", "method":"submit_durable_agent_task", "params":task('b', "rejected")}))?;
    let mut socket = TcpStream::connect(("127.0.0.1", agent.port))?;
    socket.write_all(&(payload.len() as u32).to_be_bytes())?;
    socket.write_all(&payload)?;
    // The exact-ID hook holds delivery after receipt sync. The caller never reads a reply.
    let rejected = wait_status(&agent, 'b', "failed")?;
    assert_eq!(rejected["response"]["id"], "rejection-not-received");
    assert_eq!(
        rejected["response"]["error"]["code"],
        "durable_admission_rejected"
    );
    assert_eq!(
        rejected["response"]["error"]["details"]["execution_started"],
        false
    );
    assert_eq!(
        lifecycle(&agent, "one-real-task")?["active_execution_count"],
        1
    );
    agent.stop_process()?;
    drop(socket);
    fs::remove_file(receipt_hold)?;
    for _ in 0..2 {
        agent.start_process()?;
        assert_eq!(fetch(&agent, 'b')?, rejected);
        assert_eq!(fetch(&agent, 'a')?["status"], "outcome_unknown");
        for method in ["submit_durable_agent_task", "run_durable_agent_task"] {
            let duplicate = agent.request("not-a-new-attempt", method, task('b', "rejected"))?;
            assert_eq!(
                *successful_result(&duplicate, "not-a-new-attempt"),
                rejected
            );
        }
        let index = agent.request(
            "index",
            "list_durable_agent_executions",
            json!({"limit":24}),
        )?;
        let records = successful_result(&index, "index")["executions"]
            .as_array()
            .unwrap();
        assert!(records.iter().any(|r| r["execution_id"] == "b".repeat(32)));
        assert_eq!(lifecycle(&agent, "no-replay")?["active_execution_count"], 0);
        agent.stop_process()?;
    }
    Ok(())
}

#[test]
fn killing_async_worker_before_receipt_never_authorizes_replay() -> Result<(), Box<dyn Error>> {
    let mut agent = LiveAgent::start_with_durable_store()?;
    fs::write(&agent.hold_path, "kill-async")?;
    let response = agent.request("kill", "submit_durable_agent_task", task('d', "kill-async"))?;
    assert_eq!(successful_result(&response, "kill")["status"], "running");
    agent.stop_process()?;
    agent.start_process()?;
    let report = fetch(&agent, 'd')?;
    assert_eq!(report["status"], "outcome_unknown");
    assert_eq!(report["automatic_replay_authorized"], false);
    let response = agent.request(
        "retry",
        "submit_durable_agent_task",
        task('d', "kill-async"),
    )?;
    assert_eq!(*successful_result(&response, "retry"), report);
    assert_eq!(lifecycle(&agent, "no-replay")?["active_execution_count"], 0);
    Ok(())
}

#[test]
fn drain_waits_for_durable_owner_and_persists_rejection_without_executing_it()
-> Result<(), Box<dyn Error>> {
    let agent = LiveAgent::start_with_durable_store()?;
    fs::write(agent.root.join("receipt.hold"), "e".repeat(32))?;
    let response = agent.request(
        "held-receipt",
        "submit_durable_agent_task",
        task('e', "receipt-held"),
    )?;
    assert_eq!(
        successful_result(&response, "held-receipt")["status"],
        "running"
    );
    wait_status(&agent, 'e', "completed")?;
    let drain = agent.request(
        "drain",
        "begin_agent_drain",
        json!({"controller_id":"test", "reason":"test"}),
    )?;
    assert!(drain["ok"].as_bool().unwrap(), "{drain}");
    assert_eq!(
        lifecycle(&agent, "still-active")?["active_execution_count"],
        1
    );
    let blocked = agent.request(
        "blocked",
        "submit_durable_agent_task",
        task('f', "draining"),
    )?;
    assert_eq!(successful_result(&blocked, "blocked")["status"], "failed");
    assert_eq!(
        blocked["result"]["response"]["error"]["code"],
        "durable_admission_rejected"
    );
    assert!(
        blocked["result"]["response"]["error"]["message"]
            .as_str()
            .unwrap()
            .contains("agent_draining")
    );
    assert_eq!(fetch(&agent, 'f')?["status"], "failed");
    fs::remove_file(agent.root.join("receipt.hold"))?;
    wait_for_lifecycle(&agent, "quiescent", 0)?;
    Ok(())
}

#[test]
fn async_cancellation_has_a_durable_failed_receipt() -> Result<(), Box<dyn Error>> {
    let mut agent = LiveAgent::start_with_durable_store()?;
    fs::write(&agent.hold_path, "cancel-async")?;
    let response = agent.request(
        "cancel-target",
        "submit_durable_agent_task",
        task('1', "cancel-async"),
    )?;
    assert_eq!(
        successful_result(&response, "cancel-target")["status"],
        "running"
    );
    let descriptor = agent.request("descriptor", "describe_agent", json!({}))?;
    let active = descriptor["result"]["solver_control"]["active"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["request_id"] == "cancel-target")
        .unwrap();
    let cancel = agent.request("cancel", "cancel_execution", json!({
        "process_instance_id":descriptor["result"]["lifecycle"]["process_instance_id"],
        "request_id":active["request_id"], "generation":active["generation"], "job_id":active["job_id"]}))?;
    assert_eq!(
        successful_result(&cancel, "cancel")["cancel_registered"],
        true
    );
    let failed = wait_status(&agent, '1', "failed")?;
    assert_eq!(failed["response"]["error"]["code"], "cancelled");
    wait_for_lifecycle(&agent, "accepting", 0)?;
    agent.stop_process()?;
    agent.start_process()?;
    assert_eq!(fetch(&agent, '1')?, failed);
    Ok(())
}

#[test]
fn async_receipt_write_failure_releases_slot_but_does_not_publish_success()
-> Result<(), Box<dyn Error>> {
    let agent = LiveAgent::start_with_durable_store()?;
    fs::write(&agent.hold_path, "write-failure")?;
    let response = agent.request(
        "write-failure",
        "submit_durable_agent_task",
        task('2', "write-failure"),
    )?;
    assert_eq!(
        successful_result(&response, "write-failure")["status"],
        "running"
    );
    let receipt = agent
        .root
        .join("executions")
        .join(format!("{}.receipt.next", "2".repeat(32)));
    fs::create_dir(&receipt)?;
    fs::remove_file(&agent.hold_path)?;
    wait_status(&agent, '2', "outcome_unknown")?;
    wait_for_lifecycle(&agent, "accepting", 0)?;
    let blocked = agent.request(
        "blocked",
        "submit_durable_agent_task",
        task('3', "write-blocked"),
    )?;
    assert_eq!(blocked["ok"], false);
    assert_eq!(fetch(&agent, '3')?["status"], "not_found");
    Ok(())
}

#[test]
fn asynchronous_native_task_ir_uses_the_reserved_engine_guard() -> Result<(), Box<dyn Error>> {
    let agent = LiveAgent::start_with_durable_store()?;
    let mut ir: Value = serde_json::from_str(include_str!(
        "../../../../../schemas/examples.operator-task-ir.json"
    ))?;
    ir["operator"] = json!({"id":"solve.bar_1d", "family":"mechanical", "kind":"solver"});
    ir["input_artifact"] = solver_params("async-task-ir");
    let program = &mut ir["execution_program"];
    program["program_id"] = json!("solve.bar_1d");
    program["program_family"] = json!("mechanical");
    program["program_kind"] = json!("solver");
    program["package_ref"] = Value::Null;
    program["runtime_protocol"] = json!("kyuubiki.solver-rpc/v1");
    program["abi"]["kind"] = json!("solver_rpc");
    program["entrypoint"] =
        json!({"kind":"solver_method", "name":"solve_bar_1d", "operator_kind":"solver"});
    ir["runtime_hints"] = json!({"authority_mode":"agent_local", "execution_mode":"agent_native",
        "cache_scope":"none", "agent_fetchable":false, "operator_kind":"solver"});
    ir["integrity"] = json!({"task_digest":kyuubiki_protocol::compute_operator_task_digest(&ir)?});
    let response = agent.request("async-task-ir", "submit_durable_agent_task", json!({
        "execution_id":"4".repeat(32), "method":"run_operator_task_ir", "params":{"mode":"execute", "task_ir":ir}
    }))?;
    assert_eq!(
        successful_result(&response, "async-task-ir")["status"],
        "running"
    );
    let report = wait_status(&agent, '4', "completed")?;
    let result = &report["response"]["result"]["result"];
    assert_eq!(result["max_stress"], 10.0);
    assert!((result["tip_displacement"].as_f64().unwrap() - 0.01).abs() < 1e-12);
    wait_for_lifecycle(&agent, "accepting", 0)?;
    Ok(())
}

#[test]
fn worker_start_failure_retains_terminal_identity_and_releases_the_exact_owner()
-> Result<(), Box<dyn Error>> {
    let agent = LiveAgent::start_with_durable_start_failure(&"7".repeat(32))?;
    // If this job reached the engine, the request would remain held rather than fail to start.
    fs::write(&agent.hold_path, "start-failure")?;
    let response = agent.request(
        "start",
        "submit_durable_agent_task",
        task('7', "start-failure"),
    )?;
    let failed = successful_result(&response, "start");
    assert_eq!(failed["status"], "failed");
    assert_eq!(failed["response"]["id"], "start");
    assert_eq!(
        failed["response"]["error"]["code"],
        "durable_execution_start_failed"
    );
    assert_eq!(
        failed["response"]["error"]["details"],
        json!({
            "phase":"before_engine_dispatch", "execution_started":false,
            "automatic_replay_authorized":false
        })
    );
    assert_eq!(failed["automatic_replay_authorized"], false);
    assert_eq!(fetch(&agent, '7')?, *failed);
    wait_for_lifecycle(&agent, "accepting", 0)?;
    let descriptor = agent.request("descriptor", "describe_agent", json!({}))?;
    let watchdog = &successful_result(&descriptor, "descriptor")["watchdog"];
    assert_eq!(watchdog["total_completed_execution_count"], 0);
    assert!(
        watchdog["recent_failures"]
            .as_array()
            .unwrap()
            .iter()
            .any(|failure| failure["request_id"] == "start"
                && failure["reason_code"] == "durable_execution_start_failed")
    );
    for method in ["submit_durable_agent_task", "run_durable_agent_task"] {
        let duplicate = agent.request("duplicate", method, task('7', "start-failure"))?;
        assert_eq!(*successful_result(&duplicate, "duplicate"), *failed);
    }
    let conflict = agent.request(
        "conflict",
        "submit_durable_agent_task",
        task('7', "different"),
    )?;
    assert_eq!(conflict["ok"], false);
    // Reuse the transport/job IDs, but not the durable execution identity.
    let next = agent.request(
        "start",
        "submit_durable_agent_task",
        task('8', "start-failure"),
    )?;
    assert_eq!(successful_result(&next, "start")["status"], "running");
    let cancel = agent.request("old-cancel", "cancel_durable_agent_execution", json!({
        "execution_id":failed["execution_id"], "process_instance_id":failed["process_instance_id"],
        "request_digest":failed["request_digest"]
    }))?;
    assert_eq!(
        successful_result(&cancel, "old-cancel")["status"],
        "not_running"
    );
    assert_eq!(lifecycle(&agent, "new-owner")?["active_execution_count"], 1);
    fs::remove_file(&agent.hold_path)?;
    assert_eq!(
        wait_status(&agent, '8', "completed")?["response"]["result"]["max_stress"],
        10.0
    );
    wait_for_lifecycle(&agent, "accepting", 0)?;
    Ok(())
}

#[test]
fn worker_start_failure_survives_kill_before_reply_without_replay() -> Result<(), Box<dyn Error>> {
    let mut agent = LiveAgent::start_with_durable_start_failure(&"7".repeat(32))?;
    let hold = agent.root.join("receipt.hold");
    fs::write(&hold, "7".repeat(32))?;
    let payload = serde_json::to_vec(
        &json!({"rpc_version":RPC_VERSION, "id":"unreceived-start-failure",
        "method":"submit_durable_agent_task", "params":task('7', "never-dispatched")}),
    )?;
    let mut socket = TcpStream::connect(("127.0.0.1", agent.port))?;
    socket.write_all(&(payload.len() as u32).to_be_bytes())?;
    socket.write_all(&payload)?;
    let failed = wait_status(&agent, '7', "failed")?;
    assert_eq!(failed["response"]["id"], "unreceived-start-failure");
    assert_eq!(
        failed["response"]["error"]["code"],
        "durable_execution_start_failed"
    );
    assert_eq!(
        lifecycle(&agent, "retained-owner")?["active_execution_count"],
        1
    );
    let drain = agent.request(
        "drain",
        "begin_agent_drain",
        json!({"controller_id":"test", "reason":"test"}),
    )?;
    assert_eq!(successful_result(&drain, "drain")["safe_to_replace"], false);
    agent.stop_process()?;
    drop(socket);
    fs::remove_file(hold)?;
    for _ in 0..2 {
        agent.start_process()?;
        assert_eq!(fetch(&agent, '7')?, failed);
        for method in ["submit_durable_agent_task", "run_durable_agent_task"] {
            let duplicate = agent.request("retry", method, task('7', "never-dispatched"))?;
            assert_eq!(*successful_result(&duplicate, "retry"), failed);
        }
        assert_eq!(lifecycle(&agent, "no-replay")?["active_execution_count"], 0);
        agent.stop_process()?;
    }
    Ok(())
}

#[test]
fn worker_start_failure_with_failed_receipt_commit_stays_unknown_and_fences_admission()
-> Result<(), Box<dyn Error>> {
    let mut agent = LiveAgent::start_with_durable_start_failure(&"7".repeat(32))?;
    let obstruction = agent
        .root
        .join("executions")
        .join(format!("{}.receipt.next", "7".repeat(32)));
    fs::create_dir(&obstruction)?;
    let response = agent.request(
        "commit-failed",
        "submit_durable_agent_task",
        task('7', "no-start"),
    )?;
    assert_eq!(response["ok"], false);
    assert_eq!(response["error"]["code"], "durable_execution_unavailable");
    assert_eq!(
        response["error"]["details"]["automatic_replay_authorized"],
        false
    );
    assert_eq!(fetch(&agent, '7')?["status"], "outcome_unknown");
    wait_for_lifecycle(&agent, "accepting", 0)?;
    // Removing the obstruction is not permission to forget an uncertain commit in this process.
    fs::remove_dir(&obstruction)?;
    let blocked = agent.request("blocked", "submit_durable_agent_task", task('8', "new"))?;
    assert_eq!(blocked["ok"], false);
    assert_eq!(fetch(&agent, '8')?["status"], "not_found");
    agent.stop_process()?;
    agent.start_process()?;
    for method in ["submit_durable_agent_task", "run_durable_agent_task"] {
        let duplicate = agent.request("inspect-only", method, task('7', "no-start"))?;
        let report = successful_result(&duplicate, "inspect-only");
        assert_eq!(report["status"], "outcome_unknown");
        assert_eq!(report["automatic_replay_authorized"], false);
        assert!(report["response"].is_null());
    }
    assert_eq!(lifecycle(&agent, "no-replay")?["active_execution_count"], 0);
    let manual = agent.request("manual", "submit_durable_agent_task", task('8', "new"))?;
    assert_eq!(successful_result(&manual, "manual")["status"], "running");
    assert_eq!(
        wait_status(&agent, '8', "completed")?["response"]["result"]["max_stress"],
        10.0
    );
    Ok(())
}

#[test]
fn async_disabled_store_and_control_methods_fail_before_admission() -> Result<(), Box<dyn Error>> {
    let disabled = LiveAgent::start()?;
    let response = disabled.request(
        "disabled",
        "submit_durable_agent_task",
        task('5', "disabled"),
    )?;
    assert_eq!(response["ok"], false);
    assert_eq!(lifecycle(&disabled, "idle")?["active_execution_count"], 0);
    let enabled = LiveAgent::start_with_durable_store()?;
    for method in [
        "submit_durable_agent_task",
        "run_durable_agent_task",
        "begin_agent_drain",
        "ping",
    ] {
        let response = enabled.request(
            "nested",
            "submit_durable_agent_task",
            json!({
                "execution_id":"5".repeat(32), "method":method, "params":{}
            }),
        )?;
        assert_eq!(response["ok"], false);
        assert_eq!(fetch(&enabled, '5')?["status"], "not_found");
    }
    Ok(())
}

#[cfg(unix)]
#[test]
fn termination_signal_waits_for_detached_durable_owner_before_exit() -> Result<(), Box<dyn Error>> {
    let mut agent = LiveAgent::start_with_durable_store()?;
    fs::write(agent.root.join("receipt.hold"), "6".repeat(32))?;
    let response = agent.request(
        "termination",
        "submit_durable_agent_task",
        task('6', "termination"),
    )?;
    assert_eq!(
        successful_result(&response, "termination")["status"],
        "running"
    );
    let report = wait_status(&agent, '6', "completed")?;
    agent.signal(libc::SIGTERM)?;
    let deadline = Instant::now() + Duration::from_secs(5);
    while !agent
        .shutdown_events()?
        .iter()
        .any(|event| event["phase"] == "draining")
    {
        if Instant::now() >= deadline {
            return Err("Agent did not start shutdown".into());
        }
        thread::sleep(Duration::from_millis(10));
    }
    assert!(agent.child.as_mut().unwrap().try_wait()?.is_none());
    fs::remove_file(agent.root.join("receipt.hold"))?;
    assert!(agent.wait_for_exit(Duration::from_secs(5))?.success());
    assert!(
        agent
            .shutdown_events()?
            .iter()
            .any(|event| event["reason_code"] == "agent_shutdown_complete")
    );
    agent.stop_process()?;
    agent.start_process()?;
    assert_eq!(fetch(&agent, '6')?, report);
    Ok(())
}
