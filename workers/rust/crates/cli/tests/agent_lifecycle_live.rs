use kyuubiki_installer::{AgentLifecycleClient, replace_agent_with_drain};
use serde_json::{Value, json};
use std::cell::RefCell;
use std::error::Error;
use std::fs;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::thread;
use std::time::Duration;

#[allow(dead_code)]
#[path = "support/agent_lifecycle.rs"]
mod support;
use support::*;

#[test]
fn live_agent_drains_without_losing_inflight_work_and_fences_resume() -> Result<(), Box<dyn Error>>
{
    let agent = LiveAgent::start()?;
    let initial = lifecycle(&agent, "lifecycle-initial")?;
    assert_eq!(initial["state"], "accepting");
    assert_eq!(initial["accepting_new_work"], true);

    let held_job = "lifecycle-held-job";
    fs::write(&agent.hold_path, format!("{held_job}\n"))?;
    let port = agent.port;
    let held_request = thread::spawn(move || {
        rpc_request(
            port,
            "lifecycle-held-request",
            "solve_bar_1d",
            solver_params(held_job),
        )
        .map_err(|error| error.to_string())
    });
    wait_for_lifecycle(&agent, "accepting", 1)?;

    let drain = agent.request(
        "lifecycle-drain",
        "begin_agent_drain",
        json!({
            "controller_id": "installer-rolling-1",
            "reason": "rolling replacement qualification"
        }),
    )?;
    let drain = successful_result(&drain, "lifecycle-drain");
    let generation = drain["drain_generation"]
        .as_u64()
        .expect("drain generation");
    assert_eq!(drain["state"], "draining");
    assert_eq!(drain["active_execution_count"], 1);
    assert_eq!(drain["safe_to_replace"], false);

    let rejected = agent.request(
        "lifecycle-rejected-new-work",
        "solve_bar_1d",
        solver_params("lifecycle-new-job"),
    )?;
    assert_eq!(rejected["ok"], false);
    assert_eq!(rejected["error"]["code"], "agent_draining");
    let ping = agent.request("lifecycle-control-ping", "ping", json!({}))?;
    assert_eq!(
        successful_result(&ping, "lifecycle-control-ping")["pong"],
        true
    );

    let retry = agent.request(
        "lifecycle-drain-retry",
        "begin_agent_drain",
        json!({
            "controller_id": "installer-rolling-1",
            "reason": "retry must retain the active lease"
        }),
    )?;
    assert_eq!(
        successful_result(&retry, "lifecycle-drain-retry")["drain_generation"],
        generation
    );
    let competing = agent.request(
        "lifecycle-competing-controller",
        "begin_agent_drain",
        json!({ "controller_id": "installer-rolling-2", "reason": "competing" }),
    )?;
    assert_eq!(competing["error"]["code"], "agent_drain_owned");

    fs::remove_file(&agent.hold_path)?;
    let completed = held_request
        .join()
        .map_err(|_| "held request thread panicked")??;
    let result = successful_result(&completed, "lifecycle-held-request");
    assert_eq!(result["max_stress"], 10.0);
    let quiescent = wait_for_lifecycle(&agent, "quiescent", 0)?;
    assert_eq!(quiescent["safe_to_replace"], true);

    let stale = agent.request(
        "lifecycle-stale-resume",
        "resume_agent_admission",
        json!({
            "controller_id": "installer-rolling-2",
            "drain_generation": generation
        }),
    )?;
    assert_eq!(stale["error"]["code"], "stale_agent_drain_generation");
    let resumed = agent.request(
        "lifecycle-resume",
        "resume_agent_admission",
        json!({
            "controller_id": "installer-rolling-1",
            "drain_generation": generation
        }),
    )?;
    let resumed = successful_result(&resumed, "lifecycle-resume");
    assert_eq!(resumed["state"], "accepting");
    assert_eq!(resumed["accepting_new_work"], true);

    let followup = agent.request(
        "lifecycle-followup",
        "solve_bar_1d",
        solver_params("lifecycle-followup-job"),
    )?;
    assert_eq!(
        successful_result(&followup, "lifecycle-followup")["max_stress"],
        10.0
    );
    Ok(())
}

#[test]
fn installer_replaces_two_agents_while_a_peer_keeps_serving() -> Result<(), Box<dyn Error>> {
    let first = RefCell::new(LiveAgent::start()?);
    let second = RefCell::new(LiveAgent::start()?);
    let first_port = first.borrow().port;
    let second_port = second.borrow().port;
    let first_control = AgentLifecycleClient::new(
        SocketAddr::from(([127, 0, 0, 1], first_port)),
        Duration::from_secs(10),
    )?;
    let second_control = AgentLifecycleClient::new(
        SocketAddr::from(([127, 0, 0, 1], second_port)),
        Duration::from_secs(10),
    )?;

    let first_receipt = replace_agent_with_drain(
        &first_control,
        "agent-01",
        "installer-rolling-live",
        "rolling replacement integration qualification",
        || {
            let mut target = first.borrow_mut();
            target.stop_process().map_err(|error| error.to_string())?;
            let peer = second
                .borrow()
                .request(
                    "rolling-peer-second",
                    "solve_bar_1d",
                    solver_params("rolling-peer-second-job"),
                )
                .map_err(|error| error.to_string())?;
            if successful_result(&peer, "rolling-peer-second")["max_stress"] != 10.0 {
                return Err("second Agent returned an invalid continuity result".to_string());
            }
            target.start_process().map_err(|error| error.to_string())
        },
        || {
            first
                .borrow_mut()
                .start_process()
                .map_err(|error| error.to_string())
        },
    )?;
    assert_ne!(
        first_receipt.previous_process_instance_id,
        first_receipt.active_process_instance_id
    );
    assert!(first_receipt.quiescent_observed);

    let second_receipt = replace_agent_with_drain(
        &second_control,
        "agent-02",
        "installer-rolling-live",
        "rolling replacement integration qualification",
        || {
            let mut target = second.borrow_mut();
            target.stop_process().map_err(|error| error.to_string())?;
            let peer = first
                .borrow()
                .request(
                    "rolling-peer-first",
                    "solve_bar_1d",
                    solver_params("rolling-peer-first-job"),
                )
                .map_err(|error| error.to_string())?;
            if successful_result(&peer, "rolling-peer-first")["max_stress"] != 10.0 {
                return Err("first Agent returned an invalid continuity result".to_string());
            }
            target.start_process().map_err(|error| error.to_string())
        },
        || {
            second
                .borrow_mut()
                .start_process()
                .map_err(|error| error.to_string())
        },
    )?;
    assert_ne!(
        second_receipt.previous_process_instance_id,
        second_receipt.active_process_instance_id
    );

    for (agent, id) in [
        (&first, "rolling-final-first"),
        (&second, "rolling-final-second"),
    ] {
        let response = agent
            .borrow()
            .request(id, "solve_bar_1d", solver_params(id))?;
        assert_eq!(successful_result(&response, id)["max_stress"], 10.0);
    }
    Ok(())
}

#[test]
fn slow_result_consumer_cannot_make_drain_quiescent_or_hold_it_forever()
-> Result<(), Box<dyn Error>> {
    let agent = LiveAgent::start_with_reply_timeout("5000")?;
    let mut slow = TcpStream::connect(("127.0.0.1", agent.port))?;
    slow.set_read_timeout(Some(Duration::from_secs(30)))?;
    let mut params = solver_params("reply-backpressure-job");
    params["elements"] = json!(100_000);
    let payload = serde_json::to_vec(&json!({
        "rpc_version":RPC_VERSION, "id":"reply-backpressure-request",
        "method":"solve_bar_1d", "params":params
    }))?;
    slow.write_all(&(u32::try_from(payload.len())?).to_be_bytes())?;
    slow.write_all(&payload)?;
    loop {
        let mut header = [0; 4];
        slow.read_exact(&mut header)?;
        let length = u32::from_be_bytes(header) as usize;
        if length > MAX_FRAME_BYTES {
            // The numerical solve and serialization finished; do not consume its large body.
            break;
        }
        let mut progress = vec![0; length];
        slow.read_exact(&mut progress)?;
        let value: Value = serde_json::from_slice(&progress)?;
        assert!(
            value.get("ok").is_none(),
            "expected a large real solver result: {value}"
        );
    }

    let drain = agent.request("reply-drain", "begin_agent_drain", json!({
        "controller_id":"reply-backpressure-controller", "reason":"check pending result delivery"
    }))?;
    let draining = successful_result(&drain, "reply-drain");
    assert_eq!(draining["safe_to_replace"], false);
    assert_eq!(draining["active_execution_count"], 1);
    assert_eq!(draining["state"], "draining");
    let rejected = agent.request("reply-rejected", "solve_bar_1d", solver_params("reply-new"))?;
    assert_eq!(rejected["error"]["code"], "agent_draining");
    let ping = agent.request("reply-ping", "ping", json!({}))?;
    assert_eq!(successful_result(&ping, "reply-ping")["pong"], true);

    let quiescent = wait_for_lifecycle(&agent, "quiescent", 0)?;
    assert_eq!(quiescent["safe_to_replace"], true);
    let descriptor = agent.request("reply-descriptor", "describe_agent", json!({}))?;
    let watchdog = &successful_result(&descriptor, "reply-descriptor")["watchdog"];
    let policy = &successful_result(&descriptor, "reply-descriptor")["reply_delivery"];
    assert_eq!(policy["timeout_ms"], 5000);
    assert_eq!(policy["durable_receiver_acknowledgement"], false);
    assert_eq!(watchdog["total_completed_execution_count"], 0);
    let failures = watchdog["recent_failures"]
        .as_array()
        .ok_or("missing watchdog failures")?;
    let failure = failures
        .iter()
        .find(|row| row["request_id"] == "reply-backpressure-request")
        .ok_or("delivery failure did not retain the request")?;
    assert_eq!(failure["reason_code"], "result_delivery_failed");
    assert_eq!(failure["job_id"], "reply-backpressure-job");
    assert_eq!(failure["method"], "solve_bar_1d");
    drop(slow);

    let resumed = agent.request("reply-resume", "resume_agent_admission", json!({
        "controller_id":"reply-backpressure-controller", "drain_generation":draining["drain_generation"]
    }))?;
    assert_eq!(
        successful_result(&resumed, "reply-resume")["accepting_new_work"],
        true
    );
    let completed = agent.request(
        "reply-followup",
        "solve_bar_1d",
        solver_params("reply-followup-job"),
    )?;
    let result = successful_result(&completed, "reply-followup");
    assert_eq!(result["max_stress"], 10.0);
    assert_eq!(result["tip_displacement"], 0.01);
    Ok(())
}

#[test]
fn invalid_reply_deadline_is_rejected_before_agent_listens() {
    assert!(LiveAgent::start_with_reply_timeout("0").is_err());
    assert!(LiveAgent::start_with_reply_timeout("not-a-number").is_err());
}
