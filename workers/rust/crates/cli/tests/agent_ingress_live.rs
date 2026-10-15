#[allow(dead_code)]
#[path = "support/agent_durable.rs"]
mod support;

use std::error::Error;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use support::{LiveAgent, read_json_frame, solver_params, successful_result, task};

fn connect(agent: &LiveAgent) -> Result<TcpStream, Box<dyn Error>> {
    let stream = TcpStream::connect(("127.0.0.1", agent.port()))?;
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;
    Ok(stream)
}

fn expect_closed(stream: &mut TcpStream) -> Result<(), Box<dyn Error>> {
    let mut byte = [0];
    match stream.read(&mut byte) {
        Ok(0) => Ok(()),
        Err(error)
            if matches!(
                error.kind(),
                std::io::ErrorKind::ConnectionReset | std::io::ErrorKind::ConnectionAborted
            ) =>
        {
            Ok(())
        }
        other => Err(format!("invalid or stalled ingress was not closed: {other:?}").into()),
    }
}

#[test]
fn oversized_request_header_is_rejected_without_body_and_agent_still_solves()
-> Result<(), Box<dyn Error>> {
    let mut agent = LiveAgent::start()?;
    agent.restart_with_ingress_limits(4096, 8192, 1000, 8)?;
    for length in [0_u32, 4097, u32::MAX] {
        let mut invalid = connect(&agent)?;
        invalid.write_all(&length.to_be_bytes())?;
        expect_closed(&mut invalid)?;
    }
    let response = agent.request("ingress-policy", "describe_agent", json!({}))?;
    let policy = &successful_result(&response, "ingress-policy")["request_ingress"];
    assert_eq!(policy["max_request_bytes"], 4096);
    assert_eq!(policy["max_in_flight_request_bytes"], 8192);
    assert_eq!(policy["request_timeout_ms"], 1000);
    assert_eq!(policy["max_connections"], 8);
    let response = agent.request("after-rejection", "solve_bar_1d", solver_params("bar"))?;
    assert_eq!(
        successful_result(&response, "after-rejection")["max_stress"],
        10.0
    );
    Ok(())
}

fn describe_on(stream: &mut TcpStream) -> Result<Value, Box<dyn Error>> {
    let payload = serde_json::to_vec(&json!({
        "rpc_version":1, "id":"ingress-monitor", "method":"describe_agent", "params":{}
    }))?;
    stream.write_all(&(payload.len() as u32).to_be_bytes())?;
    stream.write_all(&payload)?;
    let response = read_json_frame(stream)?;
    Ok(successful_result(&response, "ingress-monitor")["request_ingress"].clone())
}

fn wait_policy(
    control: &mut TcpStream,
    ready: impl Fn(&Value) -> bool,
) -> Result<Value, Box<dyn Error>> {
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        let policy = describe_on(control)?;
        if ready(&policy) {
            return Ok(policy);
        }
        if Instant::now() >= deadline {
            return Err(format!("ingress did not reach expected state: {policy}").into());
        }
        thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn full_connection_capacity_rejects_only_new_connections_and_recovers_slots()
-> Result<(), Box<dyn Error>> {
    let mut agent = LiveAgent::start()?;
    agent.restart_with_ingress_limits(4096, 8192, 5000, 3)?;
    let mut control = connect(&agent)?;
    wait_policy(&mut control, |policy| policy["active_connections"] == 1)?;
    let first = connect(&agent)?;
    let second = connect(&agent)?;
    wait_policy(&mut control, |policy| policy["active_connections"] == 3)?;
    let mut excess = connect(&agent)?;
    expect_closed(&mut excess)?;
    assert_eq!(describe_on(&mut control)?["active_connections"], 3);
    drop(first);
    drop(second);
    wait_policy(&mut control, |policy| policy["active_connections"] == 1)?;
    let response = agent.request("after-capacity", "solve_bar_1d", solver_params("bar"))?;
    assert_eq!(
        successful_result(&response, "after-capacity")["max_stress"],
        10.0
    );
    Ok(())
}

#[test]
fn partial_mesh_submission_times_out_without_admission_and_releases_byte_capacity()
-> Result<(), Box<dyn Error>> {
    let mut a = LiveAgent::start_with_durable_store()?;
    let mut b = LiveAgent::start_with_durable_store()?;
    b.restart_in_mesh(
        "bounded-ingress",
        vec![format!("127.0.0.1:{}", a.port())],
        1000,
    )?;
    a.restart_in_mesh(
        "bounded-ingress",
        vec![format!("127.0.0.1:{}", b.port())],
        1000,
    )?;
    a.restart_with_ingress_limits(32768, 49152, 750, 8)?;
    let mut control = connect(&a)?;
    wait_policy(&mut control, |policy| {
        policy["reserved_request_bytes"].as_u64().unwrap() < 32768
    })?;
    let mut stalled = connect(&a)?;
    let partial = serde_json::to_vec(&json!({
        "rpc_version":1, "id":"partial-submit", "method":"submit_durable_agent_task",
        "params":task('a', "must-not-run")
    }))?;
    let _: kyuubiki_protocol::RpcRequest = serde_json::from_slice(&partial)?;
    // Valid JSON is not a complete frame: dispatch must wait for all declared bytes.
    assert!(partial.len() < 32768);
    stalled.write_all(&32768_u32.to_be_bytes())?;
    stalled.write_all(&partial)?;
    wait_policy(&mut control, |policy| {
        policy["reserved_request_bytes"].as_u64().unwrap() >= 32768
    })?;
    let mut excess = connect(&a)?;
    excess.write_all(&32768_u32.to_be_bytes())?;
    expect_closed(&mut excess)?;
    assert!(
        describe_on(&mut control)?["reserved_request_bytes"]
            .as_u64()
            .unwrap()
            >= 32768
    );
    drop(control);
    expect_closed(&mut stalled)?;
    // The monitor also has an idle deadline; do not reuse it across this wait.
    let mut control = connect(&a)?;
    let after = wait_policy(&mut control, |policy| {
        policy["reserved_request_bytes"].as_u64().unwrap() < 32768
    })?;
    assert_eq!(after["automatic_retry"], false);
    assert_eq!(support::fetch(&a, 'a')?["status"], "not_found");
    // Positive control: the same body is executable when deliberately sent as a full frame.
    let mut complete = connect(&a)?;
    complete.write_all(&(partial.len() as u32).to_be_bytes())?;
    complete.write_all(&partial)?;
    let response = read_json_frame(&mut complete)?;
    assert_eq!(
        successful_result(&response, "partial-submit")["execution_id"],
        "a".repeat(32)
    );
    let completed = support::wait_status(&a, 'a', "completed")?;
    assert_eq!(completed["response"]["result"]["max_stress"], 10.0);
    for (agent, id) in [(&a, "recovered-ingress"), (&b, "unaffected-peer")] {
        let response = agent.request(id, "solve_bar_1d", solver_params("bar"))?;
        assert_eq!(successful_result(&response, id)["max_stress"], 10.0);
    }
    Ok(())
}

#[test]
fn partial_headers_close_but_complete_invalid_json_preserves_framing() -> Result<(), Box<dyn Error>>
{
    let mut agent = LiveAgent::start()?;
    agent.restart_with_ingress_limits(4096, 8192, 250, 8)?;
    for prefix in [vec![], vec![0], vec![0, 0, 0]] {
        let mut incomplete = connect(&agent)?;
        incomplete.write_all(&prefix)?;
        expect_closed(&mut incomplete)?;
    }
    let mut control = connect(&agent)?;
    for _ in 0..4 {
        control.write_all(&1_u32.to_be_bytes())?;
        control.write_all(b"{")?;
        let response = read_json_frame(&mut control)?;
        assert_eq!(response["ok"], false);
        assert_eq!(response["error"]["code"], "invalid_json");
        assert!(
            describe_on(&mut control)?["reserved_request_bytes"]
                .as_u64()
                .unwrap()
                < 4096
        );
    }
    Ok(())
}
