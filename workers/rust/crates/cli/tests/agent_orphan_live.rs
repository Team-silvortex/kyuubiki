use serde_json::{Value, json};
use std::error::Error;
use std::fs;
use std::io::Write;
use std::net::{Shutdown, TcpStream};
use std::thread;
use std::time::{Duration, Instant};

#[allow(dead_code)]
#[path = "support/agent_lifecycle.rs"]
mod support;
use support::*;

fn pending(
    agent: &LiveAgent,
    id: &str,
    method: &str,
    params: Value,
) -> Result<TcpStream, Box<dyn Error>> {
    let mut stream = TcpStream::connect(("127.0.0.1", agent.port))?;
    stream.set_read_timeout(Some(Duration::from_secs(3)))?;
    let bytes =
        serde_json::to_vec(&json!({"rpc_version":1,"id":id,"method":method,"params":params}))?;
    stream.write_all(&u32::try_from(bytes.len())?.to_be_bytes())?;
    stream.write_all(&bytes)?;
    Ok(stream)
}

fn final_frame(stream: &mut TcpStream) -> Result<Value, Box<dyn Error>> {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        let frame = read_json_frame(stream)?;
        if frame.get("ok").is_some() {
            return Ok(frame);
        }
    }
    Err("cancelled execution did not stop before computation".into())
}

#[test]
fn abandoned_structure_is_reaped_without_releasing_hold_or_poisoning_replay()
-> Result<(), Box<dyn Error>> {
    let agent = LiveAgent::start()?;
    let job = "orphan-same-job";
    fs::write(&agent.hold_path, job)?;
    let old = pending(&agent, "orphan-old", "solve_bar_1d", solver_params(job))?;
    wait_for_lifecycle(&agent, "accepting", 1)?;
    old.shutdown(Shutdown::Both)?;
    drop(old);
    wait_for_lifecycle(&agent, "accepting", 0)?;
    assert!(
        agent.hold_path.exists(),
        "cancellation must not depend on the test releasing the hold"
    );
    let descriptor = agent.request("orphan-observe", "describe_agent", json!({}))?;
    let watchdog = &descriptor["result"]["watchdog"];
    assert_eq!(watchdog["total_completed_execution_count"], 0);
    assert!(
        watchdog["recent_failures"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| {
                f["request_id"] == "orphan-old"
                    && f["job_id"] == job
                    && f["reason_code"] == "cancelled"
            })
    );
    fs::remove_file(&agent.hold_path)?;
    let next = agent.request("orphan-new", "solve_bar_1d", solver_params(job))?;
    assert_eq!(successful_result(&next, "orphan-new")["max_stress"], 10.0);
    Ok(())
}

#[test]
fn full_agent_rejects_before_execution_and_keeps_control_available() -> Result<(), Box<dyn Error>> {
    let agent = LiveAgent::start()?;
    fs::write(&agent.hold_path, "capacity-held")?;
    let mut held = pending(
        &agent,
        "capacity-held-request",
        "solve_bar_1d",
        solver_params("capacity-held"),
    )?;
    wait_for_lifecycle(&agent, "accepting", 1)?;
    let rejected = agent.request("capacity-excess", "solve_bar_1d", solver_params("excess"))?;
    assert_eq!(rejected["ok"], false);
    assert_eq!(rejected["error"]["code"], "agent_at_capacity");
    assert_eq!(
        agent.request("capacity-ping", "ping", json!({}))?["result"]["pong"],
        true
    );
    let descriptor = agent.request("capacity-observe", "describe_agent", json!({}))?;
    assert_eq!(
        descriptor["result"]["execution_admission"]["max_active_executions"],
        1
    );
    assert_eq!(
        descriptor["result"]["execution_admission"]["active_execution_count"],
        1
    );
    assert_eq!(
        descriptor["result"]["watchdog"]["total_started_execution_count"],
        1
    );
    fs::remove_file(&agent.hold_path)?;
    assert_eq!(
        successful_result(&final_frame(&mut held)?, "capacity-held-request")["max_stress"],
        10.0
    );
    wait_for_lifecycle(&agent, "accepting", 0)?;
    assert!(agent.request("capacity-next", "solve_bar_1d", solver_params("next"))?["ok"] == true);
    Ok(())
}

#[test]
fn explicit_cancellation_precedes_solver_and_task_ir_decoding() -> Result<(), Box<dyn Error>> {
    for (method, job) in [
        ("solve_bar_1d", "cancel-solver"),
        ("run_operator_task_ir", "cancel-ir"),
    ] {
        let agent = LiveAgent::start()?;
        fs::write(&agent.hold_path, job)?;
        // Invalid compute parameters prove cancellation happens before decoding or execution.
        let mut held = pending(&agent, job, method, json!({"job_id":job}))?;
        wait_for_lifecycle(&agent, "accepting", 1)?;
        let receipt = agent.request("cancel-control", "cancel_job", json!({"job_id":job}))?;
        assert!(
            receipt["result"]["cancelled"] == true
                || receipt["error"]["details"]["cancel_registered"] == true
        );
        let cancelled = final_frame(&mut held)?;
        assert_eq!(cancelled["error"]["code"], "cancelled");
        wait_for_lifecycle(&agent, "accepting", 0)?;
        fs::remove_file(&agent.hold_path)?;
        assert!(agent.request("cancel-next", "solve_bar_1d", solver_params(job))?["ok"] == true);
    }
    Ok(())
}

#[test]
fn simultaneous_clients_cannot_exceed_agent_execution_capacity() -> Result<(), Box<dyn Error>> {
    let agent = LiveAgent::start()?;
    fs::write(&agent.hold_path, "burst-held")?;
    let mut held = pending(
        &agent,
        "burst-held",
        "solve_bar_1d",
        solver_params("burst-held"),
    )?;
    wait_for_lifecycle(&agent, "accepting", 1)?;
    let requests: Vec<_> = (0..12)
        .map(|i| {
            let port = agent.port;
            thread::spawn(move || {
                rpc_request(
                    port,
                    &format!("burst-{i}"),
                    "solve_bar_1d",
                    solver_params("burst-excess"),
                )
                .map_err(|e| e.to_string())
            })
        })
        .collect();
    for request in requests {
        let response = request.join().map_err(|_| "client panicked")??;
        assert_eq!(response["error"]["code"], "agent_at_capacity");
    }
    let observation = agent.request("burst-observe", "describe_agent", json!({}))?;
    assert_eq!(
        observation["result"]["watchdog"]["total_started_execution_count"],
        1
    );
    assert_eq!(
        observation["result"]["lifecycle"]["active_execution_count"],
        1
    );
    fs::remove_file(&agent.hold_path)?;
    assert!(final_frame(&mut held)?["ok"] == true);
    Ok(())
}

#[test]
fn configured_capacity_preserves_the_other_generation_when_one_transport_dies()
-> Result<(), Box<dyn Error>> {
    let agent = LiveAgent::start_with_capacity("2")?;
    let job = "two-generation-job";
    fs::write(&agent.hold_path, job)?;
    let old = pending(
        &agent,
        "two-generation-old",
        "solve_bar_1d",
        solver_params(job),
    )?;
    let mut new = pending(
        &agent,
        "two-generation-new",
        "solve_bar_1d",
        solver_params(job),
    )?;
    wait_for_lifecycle(&agent, "accepting", 2)?;
    assert_eq!(
        agent.request(
            "two-generation-excess",
            "solve_bar_1d",
            solver_params("excess")
        )?["error"]["code"],
        "agent_at_capacity"
    );
    old.shutdown(Shutdown::Both)?;
    drop(old);
    wait_for_lifecycle(&agent, "accepting", 1)?;
    let observed = agent.request("two-generation-observe", "describe_agent", json!({}))?;
    assert_eq!(
        observed["result"]["execution_admission"]["max_active_executions"],
        2
    );
    let active = observed["result"]["watchdog"]["active_executions"]
        .as_array()
        .unwrap();
    assert_eq!(active.len(), 1);
    assert_eq!(active[0]["request_id"], "two-generation-new");
    fs::remove_file(&agent.hold_path)?;
    assert_eq!(
        successful_result(&final_frame(&mut new)?, "two-generation-new")["max_stress"],
        10.0
    );
    wait_for_lifecycle(&agent, "accepting", 0)?;
    Ok(())
}

#[test]
fn invalid_capacity_fails_before_listening() {
    for value in ["0", "unlimited", "1025"] {
        let result = LiveAgent::start_with_capacity(value);
        assert!(result.is_err(), "invalid capacity accepted: {value}");
        assert!(
            result
                .err()
                .unwrap()
                .to_string()
                .contains("KYUUBIKI_AGENT_MAX_ACTIVE_EXECUTIONS")
        );
    }
}
