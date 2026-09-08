use serde_json::{Value, json};
use std::error::Error;
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub(crate) const RPC_VERSION: u8 = 1;
pub(crate) const MAX_FRAME_BYTES: usize = 16 * 1024 * 1024;

// Serialize port selection through readiness, not the live tests themselves.
static STARTUP: Mutex<()> = Mutex::new(());

pub(crate) struct LiveAgent {
    pub(crate) child: Option<Child>,
    pub(crate) root: PathBuf,
    pub(crate) log_path: PathBuf,
    pub(crate) hold_path: PathBuf,
    pub(crate) port: u16,
    agent_id: String,
    reply_timeout_ms: String,
    shutdown_timeout_ms: String,
    capacity: String,
    solver_hold: Option<(String, String)>,
    retain_evidence: bool,
}

impl LiveAgent {
    pub(crate) fn start() -> Result<Self, Box<dyn Error>> {
        Self::start_with_reply_timeout("10000")
    }

    pub(crate) fn start_with_reply_timeout(reply_timeout_ms: &str) -> Result<Self, Box<dyn Error>> {
        Self::start_with_timeouts(reply_timeout_ms, "30000")
    }

    pub(crate) fn start_with_shutdown_timeout(timeout_ms: &str) -> Result<Self, Box<dyn Error>> {
        Self::start_with_timeouts("10000", timeout_ms)
    }

    fn start_with_timeouts(
        reply_timeout_ms: &str,
        shutdown_timeout_ms: &str,
    ) -> Result<Self, Box<dyn Error>> {
        Self::start_with_limits(reply_timeout_ms, shutdown_timeout_ms, "1")
    }

    pub(crate) fn start_with_capacity(capacity: &str) -> Result<Self, Box<dyn Error>> {
        Self::start_with_limits("10000", "30000", capacity)
    }

    fn start_with_limits(
        reply_timeout_ms: &str,
        shutdown_timeout_ms: &str,
        capacity: &str,
    ) -> Result<Self, Box<dyn Error>> {
        Self::start_configured(reply_timeout_ms, shutdown_timeout_ms, capacity, None)
    }

    pub(crate) fn start_with_solver_hold(
        stage: &str,
        method: &str,
        capacity: &str,
    ) -> Result<Self, Box<dyn Error>> {
        Self::start_configured(
            "10000",
            "30000",
            capacity,
            Some((stage.into(), method.into())),
        )
    }

    fn start_configured(
        reply_timeout_ms: &str,
        shutdown_timeout_ms: &str,
        capacity: &str,
        solver_hold: Option<(String, String)>,
    ) -> Result<Self, Box<dyn Error>> {
        let unique = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let evidence_root = std::env::var_os("KYUUBIKI_TEST_AGENT_EVIDENCE_DIR").map(PathBuf::from);
        let root = evidence_root
            .clone()
            .unwrap_or_else(std::env::temp_dir)
            .join(format!(
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
            port: 0,
            agent_id: format!("lifecycle-live-{}-{unique}", std::process::id()),
            reply_timeout_ms: reply_timeout_ms.into(),
            shutdown_timeout_ms: shutdown_timeout_ms.into(),
            capacity: capacity.into(),
            solver_hold,
            retain_evidence: evidence_root.is_some(),
        };
        agent.start_process()?;
        Ok(agent)
    }

    pub(crate) fn start_process(&mut self) -> Result<(), Box<dyn Error>> {
        let _startup = STARTUP.lock().map_err(|_| "Agent startup lock poisoned")?;
        if self.child.is_some() {
            return Ok(());
        }
        if self.port == 0 {
            self.port = reserve_port()?;
        }
        let log = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)?;
        let binary = std::env::var_os("KYUUBIKI_TEST_AGENT_BINARY")
            .unwrap_or_else(|| env!("CARGO_BIN_EXE_kyuubiki-cli").into());
        let mut command = Command::new(binary);
        command
            .args([
                "agent",
                "--host",
                "127.0.0.1",
                "--port",
                &self.port.to_string(),
                "--agent-id",
                &self.agent_id,
                "--watchdog-scan-interval-ms",
                "50",
                "--watchdog-stale-execution-ms",
                "10000",
            ])
            .env("KYUUBIKI_AGENT_FAULT_INJECTION_HOLD_FILE", &self.hold_path)
            .env_remove("KYUUBIKI_AGENT_FAULT_INJECTION_HOLD_METHOD")
            .env_remove("KYUUBIKI_AGENT_FAULT_INJECTION_SOLVER_STAGE")
            .env("KYUUBIKI_AGENT_MAX_ACTIVE_EXECUTIONS", &self.capacity)
            .env("KYUUBIKI_AGENT_REPLY_TIMEOUT_MS", &self.reply_timeout_ms)
            .env(
                "KYUUBIKI_AGENT_SHUTDOWN_TIMEOUT_MS",
                &self.shutdown_timeout_ms,
            )
            .stdout(Stdio::from(log.try_clone()?))
            .stderr(Stdio::from(log));
        if let Some((stage, method)) = &self.solver_hold {
            command
                .env("KYUUBIKI_AGENT_FAULT_INJECTION_SOLVER_STAGE", stage)
                .env("KYUUBIKI_AGENT_FAULT_INJECTION_HOLD_METHOD", method);
        }
        let child = command.spawn()?;
        self.child = Some(child);
        self.wait_until_ready(Duration::from_secs(30))
    }

    pub(crate) fn stop_process(&mut self) -> Result<(), Box<dyn Error>> {
        if let Some(mut child) = self.child.take() {
            if child.try_wait()?.is_none() {
                child.kill()?;
            }
            child.wait()?;
        }
        Ok(())
    }

    #[cfg(unix)]
    pub(crate) fn signal(&mut self, signal: libc::c_int) -> Result<(), Box<dyn Error>> {
        let child = self.child.as_mut().ok_or("Agent process is not started")?;
        if child.try_wait()?.is_some() {
            return Err("cannot signal an exited Agent".into());
        }
        let pid = libc::pid_t::try_from(child.id())?;
        // The un-reaped child owns this PID; never signal a discovered external process.
        if unsafe { libc::kill(pid, signal) } != 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        Ok(())
    }

    pub(crate) fn wait_for_exit(
        &mut self,
        timeout: Duration,
    ) -> Result<ExitStatus, Box<dyn Error>> {
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            if let Some(status) = self
                .child
                .as_mut()
                .ok_or("Agent is not started")?
                .try_wait()?
            {
                return Ok(status);
            }
            thread::sleep(Duration::from_millis(10));
        }
        Err(format!(
            "Agent did not exit; log: {}",
            fs::read_to_string(&self.log_path)?
        )
        .into())
    }

    pub(crate) fn shutdown_events(&self) -> Result<Vec<Value>, Box<dyn Error>> {
        Ok(fs::read_to_string(&self.log_path)?
            .lines()
            .filter_map(|line| serde_json::from_str::<Value>(line).ok())
            .filter(|row| row["schema_version"] == "kyuubiki.agent-shutdown/v1")
            .collect())
    }

    fn wait_until_ready(&mut self, timeout: Duration) -> Result<(), Box<dyn Error>> {
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
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
            if TcpStream::connect(("127.0.0.1", self.port)).is_ok() {
                if let Ok(reply) = self.request("owned-readiness", "describe_agent", json!({})) {
                    if reply["ok"] == true
                        && reply["result"]["deployment_readiness"]["agent_id"] == self.agent_id
                    {
                        return Ok(());
                    }
                }
            }
            thread::sleep(Duration::from_millis(25));
        }
        Err(format!("agent did not listen on port {}", self.port).into())
    }

    pub(crate) fn request(
        &self,
        id: &str,
        method: &str,
        params: Value,
    ) -> Result<Value, Box<dyn Error>> {
        rpc_request(self.port, id, method, params)
    }
}

#[cfg(test)]
mod readiness_tests {
    use super::*;

    #[test]
    fn another_agents_listener_cannot_mask_a_rejected_child() -> Result<(), Box<dyn Error>> {
        let owner = LiveAgent::start()?;
        let mut rejected = LiveAgent::start()?;
        rejected.stop_process()?;
        rejected.port = owner.port;
        rejected.solver_hold = Some(("linear_prepare".into(), "solve_heat_plane_quad_2d".into()));
        let error = rejected
            .start_process()
            .expect_err("foreign listener accepted as child readiness");
        assert!(
            error
                .to_string()
                .contains("KYUUBIKI_AGENT_FAULT_INJECTION_SOLVER_STAGE")
        );
        let response = owner.request("still-owned", "describe_agent", json!({}))?;
        assert_eq!(
            response["result"]["deployment_readiness"]["agent_id"],
            owner.agent_id
        );
        Ok(())
    }
}

impl Drop for LiveAgent {
    fn drop(&mut self) {
        let _ = self.stop_process();
        if self.retain_evidence {
            eprintln!("retained Agent lifecycle evidence: {}", self.root.display());
        } else {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
}

fn reserve_port() -> Result<u16, Box<dyn Error>> {
    let listener = TcpListener::bind(("127.0.0.1", 0))?;
    Ok(listener.local_addr()?.port())
}

pub(crate) fn rpc_request(
    port: u16,
    id: &str,
    method: &str,
    params: Value,
) -> Result<Value, Box<dyn Error>> {
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

pub(crate) fn read_json_frame(stream: &mut TcpStream) -> Result<Value, Box<dyn Error>> {
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

pub(crate) fn solver_params(job_id: &str) -> Value {
    json!({
        "job_id": job_id,
        "length": 1.0,
        "area": 2.0,
        "youngs_modulus": 1000.0,
        "elements": 2,
        "tip_force": 20.0
    })
}

pub(crate) fn successful_result<'a>(response: &'a Value, id: &str) -> &'a Value {
    assert_eq!(response["id"], id);
    assert_eq!(response["ok"], true, "response: {response}");
    response.get("result").expect("successful RPC result")
}

pub(crate) fn lifecycle(agent: &LiveAgent, id: &str) -> Result<Value, Box<dyn Error>> {
    let response = agent.request(id, "describe_agent_lifecycle", json!({}))?;
    Ok(successful_result(&response, id).clone())
}

pub(crate) fn wait_for_lifecycle(
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
