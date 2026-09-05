use kyuubiki_installer::{AgentLifecycleClient, replace_agent_with_drain};
use serde_json::{Value, json};
use std::cell::RefCell;
use std::error::Error;
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const RPC_VERSION: u8 = 1;
const MAX_FRAME_BYTES: usize = 16 * 1024 * 1024;

struct LiveAgent {
    child: Option<Child>,
    root: PathBuf,
    log_path: PathBuf,
    hold_path: PathBuf,
    port: u16,
}

impl LiveAgent {
    fn start() -> Result<Self, Box<dyn Error>> {
        let port = reserve_port()?;
        let unique = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let root = std::env::temp_dir().join(format!(
            "kyuubiki-agent-lifecycle-live-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(&root)?;
        let log_path = root.join("agent.log");
        let hold_path = root.join("execution.hold");
        let mut agent = Self {
            child: None,
            root,
            log_path,
            hold_path,
            port,
        };
        agent.start_process()?;
        Ok(agent)
    }

    fn start_process(&mut self) -> Result<(), Box<dyn Error>> {
        if self.child.is_some() {
            return Ok(());
        }
        let log = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)?;
        let child = Command::new(env!("CARGO_BIN_EXE_kyuubiki-cli"))
            .args([
                "agent",
                "--host",
                "127.0.0.1",
                "--port",
                &self.port.to_string(),
                "--agent-id",
                "lifecycle-live-agent",
                "--watchdog-scan-interval-ms",
                "50",
                "--watchdog-stale-execution-ms",
                "10000",
            ])
            .env("KYUUBIKI_AGENT_FAULT_INJECTION_HOLD_FILE", &self.hold_path)
            .stdout(Stdio::from(log.try_clone()?))
            .stderr(Stdio::from(log))
            .spawn()?;
        self.child = Some(child);
        self.wait_until_ready(Duration::from_secs(30))
    }

    fn stop_process(&mut self) -> Result<(), Box<dyn Error>> {
        if let Some(mut child) = self.child.take() {
            if child.try_wait()?.is_none() {
                child.kill()?;
            }
            child.wait()?;
        }
        Ok(())
    }

    fn wait_until_ready(&mut self, timeout: Duration) -> Result<(), Box<dyn Error>> {
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            if TcpStream::connect(("127.0.0.1", self.port)).is_ok() {
                return Ok(());
            }
            if let Some(status) = self
                .child
                .as_mut()
                .ok_or("Agent process is not started")?
                .try_wait()?
            {
                return Err(format!(
                    "agent exited with {status}; log:\n{}",
                    fs::read_to_string(&self.log_path).unwrap_or_default()
                )
                .into());
            }
            thread::sleep(Duration::from_millis(25));
        }
        Err(format!("agent did not listen on port {}", self.port).into())
    }

    fn request(&self, id: &str, method: &str, params: Value) -> Result<Value, Box<dyn Error>> {
        rpc_request(self.port, id, method, params)
    }
}

impl Drop for LiveAgent {
    fn drop(&mut self) {
        let _ = self.stop_process();
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn reserve_port() -> Result<u16, Box<dyn Error>> {
    let listener = TcpListener::bind(("127.0.0.1", 0))?;
    Ok(listener.local_addr()?.port())
}

fn rpc_request(port: u16, id: &str, method: &str, params: Value) -> Result<Value, Box<dyn Error>> {
    let mut stream = TcpStream::connect(("127.0.0.1", port))?;
    stream.set_read_timeout(Some(Duration::from_secs(30)))?;
    stream.set_write_timeout(Some(Duration::from_secs(30)))?;
    let payload = serde_json::to_vec(&json!({
        "rpc_version": RPC_VERSION,
        "id": id,
        "method": method,
        "params": params
    }))?;
    let frame_length = u32::try_from(payload.len())?;
    stream.write_all(&frame_length.to_be_bytes())?;
    stream.write_all(&payload)?;

    loop {
        let response = read_json_frame(&mut stream)?;
        if response.get("ok").is_some() {
            return Ok(response);
        }
    }
}

fn read_json_frame(stream: &mut TcpStream) -> Result<Value, Box<dyn Error>> {
    let mut header = [0_u8; 4];
    stream.read_exact(&mut header)?;
    let length = u32::from_be_bytes(header) as usize;
    if length == 0 || length > MAX_FRAME_BYTES {
        return Err(format!("agent frame length must be within 1..={MAX_FRAME_BYTES}").into());
    }
    let mut payload = vec![0_u8; length];
    stream.read_exact(&mut payload)?;
    Ok(serde_json::from_slice(&payload)?)
}

fn solver_params(job_id: &str) -> Value {
    json!({
        "job_id": job_id,
        "length": 1.0,
        "area": 2.0,
        "youngs_modulus": 1000.0,
        "elements": 2,
        "tip_force": 20.0
    })
}

fn successful_result<'a>(response: &'a Value, id: &str) -> &'a Value {
    assert_eq!(response["id"], id);
    assert_eq!(response["ok"], true, "response: {response}");
    response.get("result").expect("successful RPC result")
}

fn lifecycle(agent: &LiveAgent, id: &str) -> Result<Value, Box<dyn Error>> {
    let response = agent.request(id, "describe_agent_lifecycle", json!({}))?;
    Ok(successful_result(&response, id).clone())
}

fn wait_for_lifecycle(
    agent: &LiveAgent,
    expected_state: &str,
    expected_active: u64,
) -> Result<Value, Box<dyn Error>> {
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        let snapshot = lifecycle(agent, "lifecycle-poll")?;
        if snapshot["state"] == expected_state
            && snapshot["active_execution_count"].as_u64() == Some(expected_active)
        {
            return Ok(snapshot);
        }
        thread::sleep(Duration::from_millis(20));
    }
    Err(format!(
        "agent never reached lifecycle state {expected_state} with {expected_active} active executions"
    )
    .into())
}

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
