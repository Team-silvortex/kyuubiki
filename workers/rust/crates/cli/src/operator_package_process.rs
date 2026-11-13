//! Per-call process fault containment, not a security sandbox for malicious code.
mod fetch;
mod worker;
use self::worker as wire;
pub(crate) use fetch::{fetch_deadline, fetch_package};
use kyuubiki_protocol::{OperatorRunRequest, OperatorRunResult};
use std::{
    io::{self, Read, Write},
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
    sync::OnceLock,
    time::{Duration, Instant},
};
use worker::{FailureKind, Reply, Request};
pub(crate) use worker::{LoadedOperatorPackage, maybe_run};

#[derive(Debug)]
pub(crate) struct ProcessError {
    pub code: &'static str,
    pub message: String,
}
impl ProcessError {
    fn protocol(message: impl ToString) -> Self {
        Self {
            code: "operator_process_protocol_failed",
            message: message.to_string(),
        }
    }
}

#[derive(Clone, Copy)]
struct Policy {
    load_ms: u64,
    run_ms: u64,
    fetch_ms: u64,
}
impl Default for Policy {
    fn default() -> Self {
        Self {
            load_ms: 10_000,
            run_ms: 300_000,
            fetch_ms: 120_000,
        }
    }
}
static POLICY: OnceLock<Policy> = OnceLock::new();

fn timeout(name: &str, value: Option<&str>, default: u64) -> Result<u64, String> {
    let Some(value) = value else {
        return Ok(default);
    };
    value
        .parse::<u64>()
        .ok()
        .filter(|n| (100..=3_600_000).contains(n))
        .ok_or_else(|| format!("{name} must be an integer in 100..=3600000 milliseconds"))
}

pub(crate) fn configure() -> Result<(), String> {
    let read = |name, default| {
        let value = std::env::var(name).map(Some).or_else(|e| match e {
            std::env::VarError::NotPresent => Ok(None),
            _ => Err(format!("invalid {name}")),
        })?;
        timeout(name, value.as_deref(), default)
    };
    POLICY
        .set(Policy {
            load_ms: read("KYUUBIKI_OPERATOR_PROCESS_LOAD_TIMEOUT_MS", 10_000)?,
            run_ms: read("KYUUBIKI_OPERATOR_PROCESS_RUN_TIMEOUT_MS", 300_000)?,
            fetch_ms: read("KYUUBIKI_OPERATOR_PACKAGE_FETCH_TIMEOUT_MS", 120_000)?,
        })
        .map_err(|_| "native operator process policy already configured".into())
}

pub(crate) fn snapshot() -> serde_json::Value {
    let p = POLICY.get().copied().unwrap_or_default();
    serde_json::json!({"mode":"per_call_process", "load_timeout_ms":p.load_ms,
        "run_timeout_ms":p.run_ms,"max_frame_bytes":wire::MAX_FRAME_BYTES,
        "automatic_retry":false,"environment":"minimal_allowlist", "security_sandbox":false,
        "parent_loss":"stdin_eof_exit",
        "package_fetch": {"mode":"supervised_installer_process", "timeout_ms":p.fetch_ms,
            "budget_scope":"lifecycle_wait_and_download", "automatic_retry":false,
            "temporary_storage":"unpublished_generation", "token_delivery":"private_ipc",
            "loads_operator_code":false,
            "session_recovery":crate::operator_package_generation_session::recovery_policy()},
        "parent_loss_cleanup":if cfg!(unix) {"owned_process_group"} else {"direct_child_only"},
        "descendant_cleanup":if cfg!(unix) {"process_group"} else {"direct_child_only"}})
}

pub(crate) struct IsolatedOperatorHost {
    root: PathBuf,
    packages: Vec<LoadedOperatorPackage>,
}
impl IsolatedOperatorHost {
    pub(crate) fn load(root: &Path) -> Result<Self, ProcessError> {
        let policy = POLICY.get().copied().unwrap_or_default();
        match invoke(Request::Probe { root: root.into() }, policy.load_ms)? {
            Reply::Probed(packages) => Ok(Self {
                root: root.into(),
                packages,
            }),
            _ => Err(ProcessError::protocol("unexpected native probe reply")),
        }
    }
    pub(crate) fn packages(&self) -> &[LoadedOperatorPackage] {
        &self.packages
    }
    pub(crate) fn run_operator(
        &self,
        request: OperatorRunRequest,
    ) -> Result<OperatorRunResult, ProcessError> {
        let policy = POLICY.get().copied().unwrap_or_default();
        match invoke(
            Request::Run {
                root: self.root.clone(),
                expected: self.packages.clone(),
                request,
            },
            policy.run_ms,
        )? {
            Reply::Executed(result) => Ok(result),
            _ => Err(ProcessError::protocol("unexpected native execution reply")),
        }
    }
}

struct ChildGuard(Option<Child>);
impl ChildGuard {
    fn stop(&mut self) -> io::Result<Option<ExitStatus>> {
        let Some(mut child) = self.0.take() else {
            return Ok(None);
        };
        // Kill the owned process group before reaping its leader, so PID reuse is impossible.
        #[cfg(unix)]
        unsafe {
            libc::kill(-(child.id() as i32), libc::SIGKILL);
        }
        let _ = child.kill();
        child.wait().map(Some)
    }
}
impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

fn invoke(request: Request, timeout_ms: u64) -> Result<Reply, ProcessError> {
    let deadline = Instant::now() + Duration::from_millis(timeout_ms);
    invoke_until(request, deadline, None)
}

fn invoke_until(
    request: Request,
    deadline: Instant,
    temporary: Option<&Path>,
) -> Result<Reply, ProcessError> {
    check(deadline)?;
    let bytes = zeroize::Zeroizing::new(wire::encode(&request).map_err(ProcessError::protocol)?);
    drop(request);
    check(deadline)?;
    let listener = TcpListener::bind(("127.0.0.1", 0)).map_err(ProcessError::protocol)?;
    listener
        .set_nonblocking(true)
        .map_err(ProcessError::protocol)?;
    let mut nonce = [0; 32];
    getrandom::fill(&mut nonce).map_err(ProcessError::protocol)?;
    let mut command = Command::new(std::env::current_exe().map_err(ProcessError::protocol)?);
    command
        .arg(wire::CHILD_MODE)
        .arg(
            listener
                .local_addr()
                .map_err(ProcessError::protocol)?
                .to_string(),
        )
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    for key in ["SystemRoot", "WINDIR", "TMPDIR", "TMP", "TEMP"] {
        if let Some(value) = std::env::var_os(key) {
            command.env(key, value);
        }
    }
    if let Some(root) = temporary {
        fetch::configure_download_environment(&mut command, root);
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    let mut child = ChildGuard(Some(command.spawn().map_err(ProcessError::protocol)?));
    // Only 32 bytes; kept open for the child's parent-liveness monitor.
    child
        .0
        .as_mut()
        .expect("owned child")
        .stdin
        .as_mut()
        .ok_or_else(|| ProcessError::protocol("missing child stdin"))?
        .write_all(&nonce)
        .map_err(ProcessError::protocol)?;
    let mut result = exchange(listener, nonce, bytes, deadline);
    let status = child.stop().map_err(ProcessError::protocol)?;
    if let Err(error) = &mut result {
        if error.code == "operator_process_crashed" {
            if let Some(status) = status {
                error.message = format!("{} ({status})", error.message);
            }
        }
    }
    result
}

fn exchange(
    listener: TcpListener,
    nonce: [u8; 32],
    bytes: zeroize::Zeroizing<Vec<u8>>,
    deadline: Instant,
) -> Result<Reply, ProcessError> {
    let waiting_since = Instant::now();
    let mut stream = loop {
        check(deadline)?;
        match listener.accept() {
            Ok((stream, _)) => break stream,
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => pause(waiting_since, deadline),
            Err(e) => return Err(ProcessError::protocol(e)),
        }
    };
    stream
        .set_nonblocking(true)
        .map_err(ProcessError::protocol)?;
    stream.set_nodelay(true).map_err(ProcessError::protocol)?;
    let mut received = [0; 32];
    read(&mut stream, &mut received, deadline)?;
    if received != nonce {
        return Err(ProcessError::protocol("native IPC authentication failed"));
    }
    write(&mut stream, &(bytes.len() as u32).to_be_bytes(), deadline)?;
    write(&mut stream, &bytes, deadline)?;
    drop(bytes);
    let mut header = [0; 4];
    read(&mut stream, &mut header, deadline)?;
    let mut bytes = vec![0; wire::frame_length(header).map_err(ProcessError::protocol)?];
    read(&mut stream, &mut bytes, deadline)?;
    check(deadline)?;
    let reply = serde_json::from_slice(&bytes).map_err(ProcessError::protocol)?;
    // Parent cleanup kills/reaps even after success: no persistent host or loaded library.
    match reply {
        Reply::Failed { kind, message } => Err(ProcessError {
            code: match kind {
                FailureKind::Activation => "operator_package_activation_failed",
                FailureKind::Identity => "operator_package_identity_mismatch",
                FailureKind::Execution => "operator_package_dispatch_failed",
                FailureKind::Fetch => "operator_package_fetch_failed",
            },
            message,
        }),
        reply => Ok(reply),
    }
}

fn check(deadline: Instant) -> Result<(), ProcessError> {
    kyuubiki_solver::solver_control::check_cancellation().map_err(|message| ProcessError {
        code: "operator_process_cancelled",
        message,
    })?;
    if Instant::now() >= deadline {
        return Err(ProcessError {
            code: "operator_process_timeout",
            message:
                "Native operator process exceeded its wall-clock budget; terminated without retry"
                    .into(),
        });
    }
    Ok(())
}

fn poll_delay(idle: Duration, remaining: Duration) -> Duration {
    // Low latency for a newly spawned worker, without rapid polling on a hung plugin.
    Duration::from_millis(if idle < Duration::from_millis(20) {
        1
    } else {
        5
    })
    .min(remaining)
}

fn pause(waiting_since: Instant, deadline: Instant) {
    let now = Instant::now();
    std::thread::sleep(poll_delay(
        now.saturating_duration_since(waiting_since),
        deadline.saturating_duration_since(now),
    ));
}

fn transfer(
    mut bytes: &mut [u8],
    deadline: Instant,
    mut operation: impl FnMut(&mut [u8]) -> io::Result<usize>,
) -> Result<(), ProcessError> {
    let mut waiting_since = Instant::now();
    while !bytes.is_empty() {
        check(deadline)?;
        match operation(bytes) {
            Ok(0) => {
                return Err(ProcessError {
                    code: "operator_process_crashed",
                    message: "Native operator process closed before completing its response".into(),
                });
            }
            Ok(n) => {
                bytes = &mut bytes[n..];
                waiting_since = Instant::now();
            }
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => pause(waiting_since, deadline),
            Err(_) => {
                return Err(ProcessError {
                    code: "operator_process_crashed",
                    message: "Native operator process connection failed".into(),
                });
            }
        }
    }
    check(deadline)
}
fn read(stream: &mut TcpStream, bytes: &mut [u8], deadline: Instant) -> Result<(), ProcessError> {
    transfer(bytes, deadline, |bytes| stream.read(bytes))
}
fn write(stream: &mut TcpStream, mut bytes: &[u8], deadline: Instant) -> Result<(), ProcessError> {
    let mut waiting_since = Instant::now();
    while !bytes.is_empty() {
        check(deadline)?;
        match stream.write(bytes) {
            Ok(0) => return Err(ProcessError::protocol("native IPC write closed")),
            Ok(n) => {
                bytes = &bytes[n..];
                waiting_since = Instant::now();
            }
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => pause(waiting_since, deadline),
            Err(e) => return Err(ProcessError::protocol(e)),
        }
    }
    check(deadline)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn short_ipc_waits_are_responsive_without_spinning_on_long_stalls() {
        for idle_ms in [0, 1, 19] {
            assert_eq!(
                poll_delay(Duration::from_millis(idle_ms), Duration::from_secs(1)),
                Duration::from_millis(1)
            );
        }
        for idle_ms in [20, 100, 30_000] {
            assert_eq!(
                poll_delay(Duration::from_millis(idle_ms), Duration::from_secs(1)),
                Duration::from_millis(5)
            );
        }
        assert_eq!(
            poll_delay(Duration::ZERO, Duration::from_micros(50)),
            Duration::from_micros(50)
        );
        assert_eq!(
            poll_delay(Duration::from_secs(1), Duration::ZERO),
            Duration::ZERO
        );
    }
    #[test]
    fn process_bounds_never_silently_disable_deadlines() {
        for value in ["", "0", "99", "3600001", "-1", "invalid"] {
            assert!(timeout("test", Some(value), 1000).is_err());
        }
        assert_eq!(timeout("test", None, 1000).unwrap(), 1000);
        assert_eq!(timeout("test", Some("3600000"), 1000).unwrap(), 3600000);
        assert!(wire::frame_length([0; 4]).is_err());
        assert!(wire::frame_length(u32::MAX.to_be_bytes()).is_err());
        assert_eq!(wire::frame_length(8u32.to_be_bytes()).unwrap(), 8);
    }
    #[test]
    fn process_wait_observes_current_execution_cancellation() {
        use kyuubiki_solver::solver_control::{SolverControl, with_solver_control};
        let control = SolverControl::default();
        let result: Result<(), String> = with_solver_control(&control, || {
            control.request_cancel();
            let error = check(Instant::now() + Duration::from_secs(5)).unwrap_err();
            assert_eq!(error.code, "operator_process_cancelled");
            Err(error.message)
        });
        assert!(result.is_err());
        assert!(control.was_interrupted());
    }
    #[test]
    fn partial_frames_and_stalled_io_cannot_extend_the_deadline() {
        for partial in [false, true] {
            let mut bytes = [0; 32];
            let deadline = Instant::now() + Duration::from_millis(20);
            let error = transfer(&mut bytes, deadline, |_| {
                std::thread::sleep(Duration::from_millis(5));
                if partial {
                    Ok(1)
                } else {
                    Err(io::ErrorKind::WouldBlock.into())
                }
            })
            .unwrap_err();
            assert_eq!(error.code, "operator_process_timeout");
        }
    }
}
