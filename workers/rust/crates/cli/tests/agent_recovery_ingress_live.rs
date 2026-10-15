#[allow(dead_code)]
#[path = "support/agent_durable.rs"]
mod support;

use std::error::Error;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use support::{LiveAgent, read_json_frame, successful_result, task};

fn connect(agent: &LiveAgent) -> Result<TcpStream, Box<dyn Error>> {
    let stream = TcpStream::connect(("127.0.0.1", agent.port()))?;
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;
    Ok(stream)
}

fn exchange(stream: &mut TcpStream, method: &str, params: Value) -> Result<Value, Box<dyn Error>> {
    let bytes = serde_json::to_vec(&json!({
        "rpc_version":1, "id":"recovery", "method":method, "params":params
    }))?;
    stream.write_all(&(bytes.len() as u32).to_be_bytes())?;
    stream.write_all(&bytes)?;
    for _ in 0..128 {
        let response = read_json_frame(stream)?;
        if response.get("ok").is_some() {
            assert_eq!(response["id"], "recovery");
            return Ok(response);
        }
    }
    Err("too many progress frames before terminal reply".into())
}

fn rpc(agent: &LiveAgent, method: &str, params: Value) -> Result<Value, Box<dyn Error>> {
    exchange(&mut connect(agent)?, method, params)
}

fn general_connection(agent: &LiveAgent) -> Result<TcpStream, Box<dyn Error>> {
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        let mut stream = connect(agent)?;
        let response = exchange(&mut stream, "describe_agent", json!({}))?;
        if response["result"]["request_ingress"]["recovery_reserve"]["active_connections"] == 0 {
            return Ok(stream);
        }
        if Instant::now() >= deadline {
            return Err("normal ingress did not become available".into());
        }
        drop(stream);
        thread::sleep(Duration::from_millis(10));
    }
}

fn policy(agent: &LiveAgent, ready: impl Fn(&Value) -> bool) -> Result<Value, Box<dyn Error>> {
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        let response = rpc(agent, "describe_agent", json!({}))?;
        let policy = &successful_result(&response, "recovery")["request_ingress"];
        if ready(policy) {
            return Ok(policy.clone());
        }
        if Instant::now() >= deadline {
            return Err(format!("recovery ingress state not reached: {policy}").into());
        }
        thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn recovery_controls_survive_general_byte_exhaustion_without_admitting_work()
-> Result<(), Box<dyn Error>> {
    let mut agent = LiveAgent::start_with_durable_store()?;
    agent.restart_with_ingress_limits(4096, 4096, 5000, 8)?;
    fs::write(&agent.hold_path, "held-work")?;
    let admission = rpc(&agent, "submit_durable_agent_task", task('a', "held-work"))?;
    let original = successful_result(&admission, "recovery");
    assert_eq!(original["status"], "running");
    let mut stalled = connect(&agent)?;
    stalled.write_all(&4096_u32.to_be_bytes())?;
    let snapshot = policy(&agent, |v| {
        v["reserved_request_bytes"].as_u64().unwrap() >= 4096
    })?;
    assert_eq!(snapshot["recovery_reserve"]["authentication"], false);

    let identity = json!({"execution_id":"a".repeat(32), "expected_store_id":original["store_id"]});
    let inspected = rpc(&agent, "fetch_durable_agent_execution", identity.clone())?;
    assert_eq!(
        successful_result(&inspected, "recovery")["status"],
        "running"
    );
    let listed = rpc(
        &agent,
        "list_durable_agent_executions",
        json!({
            "limit":24, "expected_store_id":original["store_id"]
        }),
    )?;
    assert_eq!(
        successful_result(&listed, "recovery")["executions"][0]["execution_id"],
        "a".repeat(32)
    );
    let mut foreign = identity.clone();
    foreign["expected_store_id"] = json!("f".repeat(32));
    assert_eq!(
        rpc(&agent, "fetch_durable_agent_execution", foreign)?["ok"],
        false
    );

    let rejected = rpc(
        &agent,
        "submit_durable_agent_task",
        task('b', "must-not-run"),
    )?;
    assert_eq!(rejected["ok"], false);
    assert_eq!(rejected["error"]["code"], "agent_recovery_ingress_only");
    assert_eq!(support::fetch(&agent, 'b')?["status"], "not_found");
    let mut cancellation = identity;
    cancellation["process_instance_id"] = original["process_instance_id"].clone();
    cancellation["request_digest"] = original["request_digest"].clone();
    let cancelled = rpc(&agent, "cancel_durable_agent_execution", cancellation)?;
    assert_eq!(
        successful_result(&cancelled, "recovery")["status"],
        "requested"
    );
    assert_eq!(cancelled["result"]["execution_terminal_confirmed"], false);
    let terminal = support::wait_status(&agent, 'a', "failed")?;
    assert_eq!(terminal["response"]["error"]["code"], "cancelled");
    drop(stalled);
    policy(&agent, |v| {
        v["reserved_request_bytes"].as_u64().unwrap() < 4096
    })?;
    fs::remove_file(&agent.hold_path)?;
    rpc(
        &agent,
        "submit_durable_agent_task",
        task('b', "explicit-new-work"),
    )?;
    assert_eq!(
        support::wait_status(&agent, 'b', "completed")?["response"]["result"]["max_stress"],
        10.0
    );
    agent.stop_process()?;
    agent.start_process()?;
    assert_eq!(support::fetch(&agent, 'a')?, terminal);
    Ok(())
}

#[test]
fn mesh_connection_overflow_keeps_recovery_but_never_promotes_its_socket_to_work()
-> Result<(), Box<dyn Error>> {
    let mut agent = LiveAgent::start_with_durable_store()?;
    let mut peer = LiveAgent::start()?;
    peer.restart_in_mesh(
        "recovery-ingress",
        vec![format!("127.0.0.1:{}", agent.port())],
        1000,
    )?;
    agent.restart_in_mesh(
        "recovery-ingress",
        vec![format!("127.0.0.1:{}", peer.port())],
        1000,
    )?;
    fs::write(&agent.hold_path, "interrupted")?;
    let original = rpc(
        &agent,
        "submit_durable_agent_task",
        task('c', "interrupted"),
    )?;
    assert_eq!(
        successful_result(&original, "recovery")["status"],
        "running"
    );
    // Restart uses a forced process termination, preserving the durable store.
    agent.restart_with_ingress_limits(4096, 8192, 5000, 1)?;
    fs::remove_file(&agent.hold_path)?;
    let mut general = general_connection(&agent)?;
    let response = exchange(&mut general, "describe_agent", json!({}))?;
    assert_eq!(
        successful_result(&response, "recovery")["runtime"]["runtime_mode"],
        "peer_mesh"
    );
    let mut recovery = connect(&agent)?;
    let response = exchange(&mut recovery, "describe_agent", json!({}))?;
    let snapshot = &successful_result(&response, "recovery")["request_ingress"];
    assert!(
        snapshot["recovery_reserve"]["active_connections"]
            .as_u64()
            .unwrap()
            >= 1
    );
    let unknown = exchange(
        &mut recovery,
        "fetch_durable_agent_execution",
        json!({"execution_id":"c".repeat(32), "expected_store_id":original["result"]["store_id"]}),
    )?;
    assert_eq!(
        successful_result(&unknown, "recovery")["status"],
        "outcome_unknown"
    );
    assert_eq!(unknown["result"]["automatic_replay_authorized"], false);
    let drain = exchange(
        &mut recovery,
        "begin_agent_drain",
        json!({"controller_id":"recovery-test", "reason":"overload recovery"}),
    )?;
    let drain = successful_result(&drain, "recovery");
    assert_eq!(drain["state"], "quiescent");
    let stale = exchange(
        &mut recovery,
        "resume_agent_admission",
        json!({"controller_id":"foreign", "drain_generation":drain["drain_generation"]}),
    )?;
    assert_eq!(stale["error"]["code"], "stale_agent_drain_generation");
    let resumed = exchange(
        &mut recovery,
        "resume_agent_admission",
        json!({"controller_id":"recovery-test", "drain_generation":drain["drain_generation"]}),
    )?;
    assert_eq!(
        successful_result(&resumed, "recovery")["state"],
        "accepting"
    );
    drop(general);
    // A reserved socket stays control-only even after a normal slot returns.
    let rejected = exchange(
        &mut recovery,
        "submit_durable_agent_task",
        task('a', "must-not-run"),
    )?;
    assert_eq!(rejected["error"]["code"], "agent_recovery_ingress_only");
    drop(recovery);
    assert_eq!(support::fetch(&agent, 'a')?["status"], "not_found");
    for (node, job) in [(&agent, "recovered"), (&peer, "unaffected")] {
        let response = exchange(
            &mut general_connection(node)?,
            "solve_bar_1d",
            support::solver_params(job),
        )?;
        assert_eq!(successful_result(&response, "recovery")["max_stress"], 10.0);
    }
    assert_eq!(support::fetch(&agent, 'c')?["status"], "outcome_unknown");
    Ok(())
}

#[test]
fn exhausted_recovery_connection_reserve_is_bounded_and_idle_slots_return()
-> Result<(), Box<dyn Error>> {
    let mut agent = LiveAgent::start()?;
    agent.restart_with_ingress_limits(4096, 8192, 5000, 1)?;
    let mut general = general_connection(&agent)?;
    let mut reserves = Vec::new();
    for _ in 0..4 {
        let mut stream = connect(&agent)?;
        assert_eq!(
            exchange(&mut stream, "ping", json!({}))?["result"]["pong"],
            true
        );
        reserves.push(stream);
    }
    let response = exchange(&mut general, "describe_agent", json!({}))?;
    let snapshot = &response["result"]["request_ingress"];
    assert_eq!(snapshot["active_connections"], 5);
    assert_eq!(snapshot["max_total_connections"], 5);
    assert_eq!(snapshot["recovery_reserve"]["active_connections"], 4);
    let mut excess = connect(&agent)?;
    match excess.read(&mut [0]) {
        Ok(0) => {}
        Err(error)
            if matches!(
                error.kind(),
                std::io::ErrorKind::ConnectionReset | std::io::ErrorKind::ConnectionAborted
            ) => {}
        other => return Err(format!("reserve overflow was not closed: {other:?}").into()),
    }
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        let response = exchange(&mut general, "describe_agent", json!({}))?;
        if response["result"]["request_ingress"]["recovery_reserve"]["active_connections"] == 0 {
            break;
        }
        if Instant::now() >= deadline {
            return Err("idle recovery connections did not expire".into());
        }
        thread::sleep(Duration::from_millis(10));
    }
    let response = rpc(&agent, "describe_agent_lifecycle", json!({}))?;
    assert_eq!(
        successful_result(&response, "recovery")["state"],
        "accepting"
    );
    drop(reserves);
    Ok(())
}
