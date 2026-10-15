#[allow(dead_code)]
#[path = "support/agent_durable.rs"]
mod support;

use serde_json::{Value, json};
use std::{
    error::Error,
    fs,
    io::Write,
    net::TcpStream,
    path::PathBuf,
    thread,
    time::{Duration, Instant},
};
use support::{LiveAgent, read_json_frame, successful_result, task};

fn rpc(port: u16, method: &str, params: Value) -> Result<Value, String> {
    let call = || -> Result<Value, Box<dyn Error>> {
        let mut socket = TcpStream::connect(("127.0.0.1", port))?;
        socket.set_read_timeout(Some(Duration::from_secs(2)))?;
        socket.set_write_timeout(Some(Duration::from_secs(2)))?;
        let bytes = serde_json::to_vec(
            &json!({"rpc_version":1, "id":"contention", "method":method, "params":params}),
        )?;
        socket.write_all(&(bytes.len() as u32).to_be_bytes())?;
        socket.write_all(&bytes)?;
        read_json_frame(&mut socket)
    };
    call().map_err(|e| e.to_string())
}

struct HeldRead {
    path: PathBuf,
    thread: Option<thread::JoinHandle<Result<Value, String>>>,
}

impl HeldRead {
    fn start(agent: &LiveAgent, identity: Value) -> Result<Self, Box<dyn Error>> {
        let path = agent.root.join("store-read.hold");
        let id = identity["execution_id"]
            .as_str()
            .ok_or("missing execution ID")?;
        fs::write(&path, id)?;
        let port = agent.port();
        let marker = format!("agent store-read fault hold acquired: {id}");
        let held = Self {
            path,
            thread: Some(thread::spawn(move || {
                rpc(port, "fetch_durable_agent_execution", identity)
            })),
        };
        let deadline = Instant::now() + Duration::from_secs(2);
        while !fs::read_to_string(&agent.log_path)?.contains(&marker) {
            if Instant::now() >= deadline {
                return Err("store-read hold was not reached".into());
            }
            thread::sleep(Duration::from_millis(10));
        }
        Ok(held)
    }

    fn finish(mut self) -> Result<Value, Box<dyn Error>> {
        fs::remove_file(&self.path)?;
        Ok(self
            .thread
            .take()
            .unwrap()
            .join()
            .map_err(|_| "held reader panicked")??)
    }
}

impl Drop for HeldRead {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

#[test]
fn slow_store_read_does_not_block_mesh_description_or_fabricate_recovery_state()
-> Result<(), Box<dyn Error>> {
    let mut agent = LiveAgent::start_with_durable_store()?;
    let mut peer = LiveAgent::start_with_durable_store()?;
    agent.restart_in_mesh(
        "store-contention",
        vec![format!("127.0.0.1:{}", peer.port())],
        1000,
    )?;
    peer.restart_in_mesh(
        "store-contention",
        vec![format!("127.0.0.1:{}", agent.port())],
        1000,
    )?;
    for member in [&agent, &peer] {
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            let descriptor = rpc(member.port(), "describe_agent", json!({}))?;
            let descriptor = successful_result(&descriptor, "contention");
            assert_eq!(descriptor["authority"]["authority_mode"], "offline_mesh");
            if descriptor["runtime"]["cluster_size"] == 2
                && descriptor["runtime"]["health_score"] == 100
            {
                break;
            }
            if Instant::now() >= deadline {
                return Err("mesh did not converge".into());
            }
            thread::sleep(Duration::from_millis(30));
        }
    }
    fs::write(&agent.hold_path, "held-work")?;
    let admission = rpc(
        agent.port(),
        "submit_durable_agent_task",
        task('a', "held-work"),
    )?;
    let original = successful_result(&admission, "contention");
    let identity = json!({"execution_id":"a".repeat(32), "expected_store_id":original["store_id"]});
    let held = HeldRead::start(&agent, identity.clone())?;
    let started = Instant::now();
    let descriptor = rpc(agent.port(), "describe_agent", json!({}))?;
    assert!(started.elapsed() < Duration::from_secs(1));
    let store = &successful_result(&descriptor, "contention")["durable_execution"];
    assert_eq!(store["enabled"], true);
    assert_eq!(store["available"], false);
    assert_eq!(store["unavailable_reason"], "store_busy");
    assert!(store.get("stored_entries").is_none());
    assert!(store.get("store_id").is_none());
    let mut cancel = identity.clone();
    cancel["process_instance_id"] = original["process_instance_id"].clone();
    cancel["request_digest"] = original["request_digest"].clone();
    for (method, params) in [
        ("fetch_durable_agent_execution", identity.clone()),
        (
            "list_durable_agent_executions",
            json!({"limit":24, "expected_store_id":original["store_id"]}),
        ),
        ("cancel_durable_agent_execution", cancel.clone()),
    ] {
        let started = Instant::now();
        let response = rpc(agent.port(), method, params)?;
        assert!(started.elapsed() < Duration::from_secs(1));
        assert_eq!(response["id"], "contention");
        assert_eq!(response["ok"], false);
        assert_eq!(response["error"]["code"], "durable_execution_busy");
        assert_eq!(
            response["error"]["details"]["automatic_replay_authorized"],
            false
        );
        assert_eq!(response["error"]["details"]["operation_started"], false);
        assert_eq!(response["error"]["details"]["execution_state_known"], false);
        assert_eq!(
            response["error"]["details"]["identity_check_performed"],
            false
        );
        assert!(response.get("result").is_none_or(Value::is_null));
    }
    assert_eq!(
        successful_result(&held.finish()?, "contention")["status"],
        "running"
    );
    let still_running = rpc(
        agent.port(),
        "fetch_durable_agent_execution",
        identity.clone(),
    )?;
    assert_eq!(
        successful_result(&still_running, "contention")["status"],
        "running"
    );
    let cancelled = rpc(agent.port(), "cancel_durable_agent_execution", cancel)?;
    assert_eq!(
        successful_result(&cancelled, "contention")["status"],
        "requested"
    );
    let terminal = support::wait_status(&agent, 'a', "failed")?;
    assert_eq!(terminal["response"]["error"]["code"], "cancelled");
    fs::remove_file(&agent.hold_path)?;
    agent.stop_process()?;
    agent.start_process()?;
    assert_eq!(support::fetch(&agent, 'a')?, terminal);
    let listed = rpc(
        agent.port(),
        "list_durable_agent_executions",
        json!({"limit":24}),
    )?;
    assert_eq!(
        listed["result"]["executions"][0]["execution_id"],
        "a".repeat(32)
    );
    assert_eq!(support::fetch(&peer, 'a')?["status"], "not_found");
    assert_eq!(
        support::lifecycle(&peer, "no-migration")?["active_execution_count"],
        0
    );
    Ok(())
}
