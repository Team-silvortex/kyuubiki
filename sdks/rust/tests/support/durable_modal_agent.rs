use kyuubiki_headless_sdk::SolverRpcClient;
use serde_json::Value;
use std::{
    env, fs,
    net::TcpListener,
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::atomic::{AtomicU64, Ordering},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

static NEXT_AGENT: AtomicU64 = AtomicU64::new(0);

pub struct Agent {
    pub root: PathBuf,
    pub port: u16,
    child: Option<Child>,
    binary: PathBuf,
}

impl Agent {
    pub fn start() -> Result<Self> {
        let binary = fs::canonicalize(
            env::var_os("KYUUBIKI_TEST_AGENT_BINARY")
                .ok_or("set KYUUBIKI_TEST_AGENT_BINARY to the built native Agent binary")?,
        )?;
        let root = env::temp_dir().join(format!(
            "kyuubiki-sdk-modal-{}-{}-{}",
            std::process::id(),
            SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos(),
            NEXT_AGENT.fetch_add(1, Ordering::Relaxed)
        ));
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let port = listener.local_addr()?.port();
        fs::create_dir(&root)?;
        drop(listener);
        let mut agent = Self {
            root,
            port,
            child: None,
            binary,
        };
        agent.restart()?;
        Ok(agent)
    }

    pub fn client(&self) -> SolverRpcClient {
        SolverRpcClient::new("127.0.0.1", self.port)
    }

    pub fn store_id(&self) -> Result<String> {
        Ok(
            self.client().describe_agent()?.result["durable_execution"]["store_id"]
                .as_str()
                .ok_or("missing durable store identity")?
                .into(),
        )
    }

    pub fn restart(&mut self) -> Result<()> {
        assert!(self.child.is_none());
        let log = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.root.join("agent.log"))?;
        let mut command = Command::new(&self.binary);
        for (key, _) in env::vars_os() {
            if key.to_string_lossy().starts_with("KYUUBIKI_") {
                command.env_remove(key);
            }
        }
        self.child = Some(
            command
                .args([
                    "agent",
                    "--host",
                    "127.0.0.1",
                    "--port",
                    &self.port.to_string(),
                    "--agent-id",
                    "sdk-modal-research",
                ])
                .env(
                    "KYUUBIKI_AGENT_EXECUTION_STORE",
                    self.root.join("executions"),
                )
                .env(
                    "KYUUBIKI_AGENT_FAULT_INJECTION_HOLD_FILE",
                    self.root.join("solver.hold"),
                )
                .env(
                    "KYUUBIKI_AGENT_FAULT_INJECTION_HOLD_METHOD",
                    "solve_modal_frame_2d",
                )
                .env(
                    "KYUUBIKI_AGENT_FAULT_INJECTION_SOLVER_STAGE",
                    "modal_shape_norm",
                )
                .env(
                    "KYUUBIKI_AGENT_FAULT_INJECTION_STORE_READ_HOLD_FILE",
                    self.root.join("store-read.hold"),
                )
                .stdout(Stdio::from(log.try_clone()?))
                .stderr(Stdio::from(log))
                .spawn()?,
        );
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if self.client().ping().is_ok() {
                return Ok(());
            }
            if self.child.as_mut().unwrap().try_wait()?.is_some() || Instant::now() > deadline {
                return Err(format!(
                    "Agent failed startup: {}",
                    fs::read_to_string(self.root.join("agent.log"))?
                )
                .into());
            }
            thread::sleep(Duration::from_millis(20));
        }
    }

    /// Child::kill is SIGKILL on macOS/Linux, not a graceful shutdown or host power cut.
    pub fn kill(&mut self) -> Result<u32> {
        let mut child = self.child.take().ok_or("no child")?;
        let pid = child.id();
        child.kill()?;
        let status = child.wait()?;
        assert!(!status.success());
        Ok(pid)
    }

    pub fn wait(
        &self,
        handle: &kyuubiki_headless_sdk::DurableExecutionHandle,
        status: &str,
    ) -> Result<Value> {
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            let report = self.client().fetch_durable_agent_execution(handle)?;
            if report["status"] == status {
                return Ok(report);
            }
            if Instant::now() > deadline {
                return Err(format!("expected {status}: {report}").into());
            }
            thread::sleep(Duration::from_millis(10));
        }
    }

    #[allow(dead_code)] // Publication-only suites do not use the solver checkpoint hook.
    pub fn held(&self) -> Result<()> {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let state = self.client().describe_agent()?.result;
            if state["solver_control"]["active"]
                .as_array()
                .is_some_and(|entries| {
                    entries
                        .iter()
                        .any(|entry| entry["checkpoint"]["stage"] == "modal_shape_norm")
                })
            {
                return Ok(());
            }
            if Instant::now() > deadline {
                return Err(format!("solver never reached hold: {state}").into());
            }
            thread::sleep(Duration::from_millis(10));
        }
    }
}

impl Drop for Agent {
    fn drop(&mut self) {
        if self.child.is_some() {
            let _ = self.kill();
        }
        if !thread::panicking() {
            let _ = fs::remove_dir_all(&self.root);
        } else {
            eprintln!("failed research evidence: {}", self.root.display());
        }
    }
}
