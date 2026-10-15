use serde_json::{Value, json};
use std::{
    error::Error,
    fs,
    io::Write,
    net::{Shutdown, TcpListener, TcpStream},
    thread,
    time::{Duration, Instant},
};

#[allow(dead_code)]
#[path = "support/agent_durable.rs"]
mod support;
use support::*;

fn healthy_pair(agent: &LiveAgent) -> Result<(), Box<dyn Error>> {
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        let reply = agent.request("mesh", "describe_agent", json!({}))?;
        let descriptor = successful_result(&reply, "mesh");
        assert_eq!(descriptor["authority"]["authority_mode"], "offline_mesh");
        assert!(descriptor["authority"]["orchestrator_id"].is_null());
        assert_eq!(descriptor["peer_discovery"]["task_replication"], false);
        if descriptor["runtime"]["cluster_size"] == 2
            && descriptor["runtime"]["health_score"] == 100
        {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err(format!("mesh did not converge: {descriptor}").into());
        }
        thread::sleep(Duration::from_millis(30));
    }
}

fn pair() -> Result<(LiveAgent, LiveAgent), Box<dyn Error>> {
    let mut owner = LiveAgent::start_with_durable_store()?;
    let mut peer = LiveAgent::start_with_durable_store()?;
    owner.restart_in_mesh(
        "cancel-loss",
        vec![format!("127.0.0.1:{}", peer.port)],
        1000,
    )?;
    peer.restart_in_mesh(
        "cancel-loss",
        vec![format!("127.0.0.1:{}", owner.port)],
        1000,
    )?;
    healthy_pair(&owner)?;
    healthy_pair(&peer)?;
    Ok((owner, peer))
}

fn submit(agent: &LiveAgent, id: char) -> Result<Value, Box<dyn Error>> {
    let reply = agent.request(
        "same-request",
        "submit_durable_agent_task",
        task(id, "same-job"),
    )?;
    let admitted = successful_result(&reply, "same-request");
    assert_eq!(admitted["status"], "running");
    Ok(admitted.clone())
}

fn cancel_query(admitted: &Value) -> Value {
    json!({"execution_id":admitted["execution_id"], "expected_store_id":admitted["store_id"],
        "process_instance_id":admitted["process_instance_id"], "request_digest":admitted["request_digest"]})
}

fn write_frame(stream: &mut TcpStream, value: &Value) -> Result<(), Box<dyn Error>> {
    let payload = serde_json::to_vec(value)?;
    stream.write_all(&u32::try_from(payload.len())?.to_be_bytes())?;
    stream.write_all(&payload)?;
    Ok(())
}

fn cancel_with_lost_reply(agent: &LiveAgent, params: Value) -> Result<Value, Box<dyn Error>> {
    // A one-shot test relay observes acceptance but delivers no reply bytes to the caller.
    // The production Agent needs no extra fault hook or runtime dependency for this test.
    let listener = TcpListener::bind(("127.0.0.1", 0))?;
    let mut caller = TcpStream::connect(listener.local_addr()?)?;
    caller.set_read_timeout(Some(Duration::from_secs(2)))?;
    caller.set_write_timeout(Some(Duration::from_secs(2)))?;
    let (mut relay, _) = listener.accept()?;
    relay.set_read_timeout(Some(Duration::from_secs(2)))?;
    write_frame(
        &mut caller,
        &json!({"rpc_version":1, "id":"lost-cancel",
        "method":"cancel_durable_agent_execution", "params":params}),
    )?;
    let request = read_json_frame(&mut relay)?;
    let observed = agent.request(
        "lost-cancel",
        "cancel_durable_agent_execution",
        request["params"].clone(),
    )?;
    relay.shutdown(Shutdown::Both)?;
    assert!(
        read_json_frame(&mut caller).is_err(),
        "caller must not receive acceptance evidence"
    );
    Ok(observed)
}

fn assert_peer_untouched(peer: &LiveAgent, id: char) -> Result<(), Box<dyn Error>> {
    assert_eq!(fetch(peer, id)?["status"], "not_found");
    assert_eq!(
        lifecycle(peer, "no-migration")?["active_execution_count"],
        0
    );
    Ok(())
}

#[test]
fn mesh_cancel_reply_loss_recovers_committed_failure_after_forced_restart()
-> Result<(), Box<dyn Error>> {
    let (mut owner, peer) = pair()?;
    fs::write(&owner.hold_path, "same-job")?;
    let admitted = submit(&owner, 'a')?;
    let observed = cancel_with_lost_reply(&owner, cancel_query(&admitted))?;
    let acceptance = successful_result(&observed, "lost-cancel");
    assert_eq!(acceptance["status"], "requested");
    assert_eq!(acceptance["cancel_registered"], true);
    assert_eq!(acceptance["execution_terminal_confirmed"], false);
    let terminal = wait_status(&owner, 'a', "failed")?;
    assert_eq!(terminal["response"]["error"]["code"], "cancelled");
    assert_eq!(terminal["store_id"], admitted["store_id"]);
    assert_peer_untouched(&peer, 'a')?;

    owner.stop_process()?;
    owner.start_process()?;
    healthy_pair(&owner)?;
    let recovered = owner.request(
        "recover",
        "fetch_durable_agent_execution",
        json!({
            "execution_id":admitted["execution_id"], "expected_store_id":admitted["store_id"]
        }),
    )?;
    assert_eq!(successful_result(&recovered, "recover"), &terminal);
    assert_eq!(terminal["automatic_replay_authorized"], false);
    assert_eq!(lifecycle(&owner, "restarted")?["active_execution_count"], 0);

    // Reused transport/job names belong to a new explicit execution, not the old cancel.
    let fresh = submit(&owner, 'b')?;
    assert_ne!(
        fresh["process_instance_id"],
        admitted["process_instance_id"]
    );
    let old = owner.request(
        "late-cancel",
        "cancel_durable_agent_execution",
        cancel_query(&admitted),
    )?;
    assert_eq!(old["result"]["status"], "not_running");
    assert_eq!(old["result"]["cancel_registered"], false);
    assert_eq!(fetch(&owner, 'b')?["status"], "running");
    fs::remove_file(&owner.hold_path)?;
    assert_eq!(
        wait_status(&owner, 'b', "completed")?["response"]["result"]["max_stress"],
        10.0
    );
    assert_peer_untouched(&peer, 'a')?;
    assert_peer_untouched(&peer, 'b')?;
    Ok(())
}

#[test]
fn incomplete_mesh_cancel_then_forced_restart_keeps_outcome_unknown_without_replay()
-> Result<(), Box<dyn Error>> {
    let (mut owner, peer) = pair()?;
    fs::write(&owner.hold_path, "same-job")?;
    let admitted = submit(&owner, 'c')?;
    let request = serde_json::to_vec(&json!({"rpc_version":1, "id":"truncated-cancel",
        "method":"cancel_durable_agent_execution", "params":cancel_query(&admitted)}))?;
    let mut caller = TcpStream::connect(("127.0.0.1", owner.port))?;
    caller.set_read_timeout(Some(Duration::from_secs(2)))?;
    caller.set_write_timeout(Some(Duration::from_secs(2)))?;
    caller.write_all(&u32::try_from(request.len())?.to_be_bytes())?;
    caller.write_all(&request[..request.len() / 2])?;
    caller.shutdown(Shutdown::Write)?;
    if let Ok(reply) = read_json_frame(&mut caller) {
        assert_eq!(reply["ok"], false);
    }
    drop(caller);
    assert_eq!(fetch(&owner, 'c')?["status"], "running");
    owner.stop_process()?;
    fs::remove_file(&owner.hold_path)?;
    owner.start_process()?;
    healthy_pair(&owner)?;
    let unknown = fetch(&owner, 'c')?;
    assert_eq!(unknown["status"], "outcome_unknown");
    assert_eq!(unknown["store_id"], admitted["store_id"]);
    assert_eq!(
        unknown["process_instance_id"],
        admitted["process_instance_id"]
    );
    assert_eq!(unknown["automatic_replay_authorized"], false);
    assert!(unknown["response"].is_null());
    let old = owner.request(
        "explicit-inspected-cancel",
        "cancel_durable_agent_execution",
        cancel_query(&admitted),
    )?;
    assert_eq!(old["result"]["status"], "not_running");
    assert_eq!(old["result"]["cancel_registered"], false);
    let duplicate = owner.request(
        "explicit-duplicate",
        "submit_durable_agent_task",
        task('c', "same-job"),
    )?;
    assert_eq!(
        successful_result(&duplicate, "explicit-duplicate"),
        &unknown
    );
    assert_eq!(lifecycle(&owner, "no-replay")?["active_execution_count"], 0);
    assert_peer_untouched(&peer, 'c')?;
    submit(&peer, 'd')?;
    assert_eq!(
        wait_status(&peer, 'd', "completed")?["response"]["result"]["max_stress"],
        10.0
    );
    assert_eq!(fetch(&owner, 'd')?["status"], "not_found");
    assert_eq!(fetch(&owner, 'c')?, unknown);
    Ok(())
}
