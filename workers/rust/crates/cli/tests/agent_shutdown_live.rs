#![cfg(unix)]

use serde_json::json;
use std::error::Error;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::thread;
use std::time::Duration;

#[allow(dead_code)]
#[path = "support/agent_lifecycle.rs"]
mod support;
use support::*;

fn held_request(
    agent: &LiveAgent,
    id: &'static str,
) -> Result<thread::JoinHandle<Result<serde_json::Value, String>>, Box<dyn Error>> {
    fs::write(&agent.hold_path, id)?;
    let port = agent.port;
    let request = thread::spawn(move || {
        rpc_request(port, id, "solve_bar_1d", solver_params(id)).map_err(|error| error.to_string())
    });
    wait_for_lifecycle(agent, "accepting", 1)?;
    Ok(request)
}

#[test]
fn termination_drains_the_admitted_request_and_preserves_its_response() -> Result<(), Box<dyn Error>>
{
    let mut agent = LiveAgent::start_with_shutdown_timeout("5000")?;
    let request = held_request(&agent, "signal-held-job")?;
    agent.signal(libc::SIGTERM)?;
    let draining = wait_for_lifecycle(&agent, "draining", 1)?;
    assert_eq!(draining["safe_to_replace"], false);
    assert_eq!(
        agent.request("signal-reject", "solve_bar_1d", solver_params("new"))?["error"]["code"],
        "agent_draining"
    );
    assert_eq!(
        agent.request("signal-ping", "ping", json!({}))?["result"]["pong"],
        true
    );
    let policy = agent.request("signal-policy", "describe_agent", json!({}))?;
    assert_eq!(policy["result"]["shutdown_policy"]["timeout_ms"], 5000);
    agent.signal(libc::SIGINT)?;
    fs::remove_file(&agent.hold_path)?;
    let response = request.join().map_err(|_| "request thread panicked")??;
    let result = successful_result(&response, "signal-held-job");
    assert_eq!(result["max_stress"], 10.0);
    assert_eq!(result["tip_displacement"], 0.01);
    assert!(agent.wait_for_exit(Duration::from_secs(8))?.success());
    let events = agent.shutdown_events()?;
    assert_eq!(
        events
            .iter()
            .filter(|event| event["phase"] == "draining")
            .count(),
        1
    );
    assert!(
        events.iter().any(|event| event["phase"] == "completed"
            && event["lifecycle"]["active_execution_count"] == 0)
    );
    Ok(())
}

#[test]
fn termination_cannot_be_undone_by_the_existing_drain_owner() -> Result<(), Box<dyn Error>> {
    let mut agent = LiveAgent::start_with_shutdown_timeout("5000")?;
    let request = held_request(&agent, "signal-owned-drain-job")?;
    let drain = agent.request(
        "owned-drain",
        "begin_agent_drain",
        json!({
            "controller_id":"installer-existing-owner", "reason":"planned replacement"
        }),
    )?;
    agent.signal(libc::SIGTERM)?;
    // Wait for the irreversible latch, not merely the already-existing drain state.
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    while agent.shutdown_events()?.is_empty() && std::time::Instant::now() < deadline {
        thread::sleep(Duration::from_millis(10));
    }
    let resumed = agent.request(
        "signal-no-resume",
        "resume_agent_admission",
        json!({
            "controller_id":"installer-existing-owner",
            "drain_generation":drain["result"]["drain_generation"]
        }),
    )?;
    assert_eq!(resumed["error"]["code"], "agent_shutdown_in_progress");
    let current = lifecycle(&agent, "signal-owned-state")?;
    assert_eq!(current["drain_owner_id"], "installer-existing-owner");
    fs::remove_file(&agent.hold_path)?;
    assert_eq!(
        request.join().map_err(|_| "request thread panicked")??["ok"],
        true
    );
    assert!(agent.wait_for_exit(Duration::from_secs(8))?.success());
    Ok(())
}

#[test]
fn shutdown_deadline_preserves_unfinished_identity_and_allows_a_fresh_process()
-> Result<(), Box<dyn Error>> {
    let mut agent = LiveAgent::start_with_shutdown_timeout("500")?;
    let request = held_request(&agent, "signal-timeout-job")?;
    agent.signal(libc::SIGTERM)?;
    assert_eq!(agent.wait_for_exit(Duration::from_secs(5))?.code(), Some(1));
    assert!(
        request
            .join()
            .map_err(|_| "request thread panicked")?
            .is_err()
    );
    let events = agent.shutdown_events()?;
    let timeout = events
        .iter()
        .find(|event| event["reason_code"] == "agent_shutdown_timeout")
        .ok_or("shutdown timeout evidence missing")?;
    assert_eq!(timeout["phase"], "draining");
    assert_eq!(timeout["lifecycle"]["active_execution_count"], 1);
    assert_eq!(
        timeout["unfinished_executions"][0]["request_id"],
        "signal-timeout-job"
    );
    assert_eq!(
        timeout["unfinished_executions"][0]["job_id"],
        "signal-timeout-job"
    );
    assert_eq!(
        timeout["unfinished_executions"][0]["method"],
        "solve_bar_1d"
    );
    assert!(!events.iter().any(|event| event["phase"] == "completed"));
    agent.stop_process()?;
    fs::remove_file(&agent.hold_path)?;
    agent.start_process()?;
    let response = agent.request(
        "fresh-after-timeout",
        "solve_bar_1d",
        solver_params("fresh"),
    )?;
    assert_eq!(
        successful_result(&response, "fresh-after-timeout")["tip_displacement"],
        0.01
    );
    agent.signal(libc::SIGTERM)?;
    assert!(agent.wait_for_exit(Duration::from_secs(5))?.success());
    Ok(())
}

#[test]
fn idle_shutdown_does_not_wait_for_an_incomplete_request_header() -> Result<(), Box<dyn Error>> {
    let mut agent = LiveAgent::start_with_shutdown_timeout("1000")?;
    let mut incomplete = TcpStream::connect(("127.0.0.1", agent.port))?;
    incomplete.write_all(&[0, 0])?;
    agent.signal(libc::SIGTERM)?;
    assert!(agent.wait_for_exit(Duration::from_secs(5))?.success());
    Ok(())
}

#[test]
fn other_handled_unix_signals_also_exit_through_the_graceful_gate() -> Result<(), Box<dyn Error>> {
    for signal in [libc::SIGINT, libc::SIGHUP] {
        let mut agent = LiveAgent::start_with_shutdown_timeout("1000")?;
        agent.signal(signal)?;
        assert!(agent.wait_for_exit(Duration::from_secs(5))?.success());
        assert!(
            agent
                .shutdown_events()?
                .iter()
                .any(|event| event["phase"] == "completed")
        );
    }
    Ok(())
}

#[test]
fn shutdown_during_response_backpressure_never_claims_a_successful_drain()
-> Result<(), Box<dyn Error>> {
    let mut agent = LiveAgent::start_with_shutdown_timeout("1000")?;
    let mut slow = TcpStream::connect(("127.0.0.1", agent.port))?;
    slow.set_read_timeout(Some(Duration::from_secs(30)))?;
    let mut params = solver_params("signal-pending-response");
    params["elements"] = json!(100_000);
    let request = serde_json::to_vec(&json!({
        "rpc_version":RPC_VERSION, "id":"signal-pending-response",
        "method":"solve_bar_1d", "params":params
    }))?;
    slow.write_all(&u32::try_from(request.len())?.to_be_bytes())?;
    slow.write_all(&request)?;
    loop {
        let mut header = [0; 4];
        slow.read_exact(&mut header)?;
        let length = u32::from_be_bytes(header) as usize;
        if length > MAX_FRAME_BYTES {
            break;
        }
        let mut payload = vec![0; length];
        slow.read_exact(&mut payload)?;
        let progress: serde_json::Value = serde_json::from_slice(&payload)?;
        assert!(progress.get("ok").is_none(), "expected a large result body");
    }
    agent.signal(libc::SIGTERM)?;
    assert!(
        !wait_for_lifecycle(&agent, "draining", 1)?["safe_to_replace"]
            .as_bool()
            .unwrap()
    );
    assert_eq!(agent.wait_for_exit(Duration::from_secs(5))?.code(), Some(1));
    let events = agent.shutdown_events()?;
    let timeout = events
        .iter()
        .find(|event| event["reason_code"] == "agent_shutdown_timeout")
        .ok_or("pending response lost its shutdown timeout evidence")?;
    assert_eq!(
        timeout["unfinished_executions"][0]["job_id"],
        "signal-pending-response"
    );
    assert_eq!(timeout["lifecycle"]["active_execution_count"], 1);
    assert!(!events.iter().any(|event| event["phase"] == "completed"));
    Ok(())
}

#[test]
fn invalid_shutdown_budget_fails_before_agent_listens() {
    for value in ["0", "not-a-duration", "300001"] {
        assert!(
            LiveAgent::start_with_shutdown_timeout(value).is_err(),
            "{value}"
        );
    }
}
