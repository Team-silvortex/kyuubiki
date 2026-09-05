use kyuubiki_protocol::{RPC_VERSION, RpcMethod, RpcRequest, RpcResponse};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const MAX_FRAME_BYTES: usize = 4 * 1024 * 1024;

#[derive(Clone, Debug)]
pub(crate) struct SolverProbeResult {
    pub(crate) max_stress: f64,
    pub(crate) tip_displacement: f64,
}

pub(crate) struct ManagedQualificationAgent {
    node_id: String,
    port: u16,
    binary: PathBuf,
    log_path: PathBuf,
    child: Option<Child>,
}

impl ManagedQualificationAgent {
    pub(crate) fn new(root: &Path, node_id: &str, binary: PathBuf) -> Result<Self, String> {
        Ok(Self {
            node_id: node_id.to_string(),
            port: reserve_port()?,
            binary,
            log_path: root.join(format!("{node_id}.log")),
            child: None,
        })
    }

    pub(crate) fn address(&self) -> SocketAddr {
        SocketAddr::from(([127, 0, 0, 1], self.port))
    }

    pub(crate) fn replace_binary(&mut self, binary: PathBuf) -> Result<(), String> {
        self.stop()?;
        self.binary = binary;
        self.start()
    }

    pub(crate) fn restore_binary(&mut self, binary: PathBuf) -> Result<(), String> {
        self.stop()?;
        self.binary = binary;
        self.start()
    }

    pub(crate) fn start(&mut self) -> Result<(), String> {
        if self.child.is_some() {
            return Ok(());
        }
        let log = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)
            .map_err(|error| format!("failed to open Agent qualification log: {error}"))?;
        let child = Command::new(&self.binary)
            .args([
                "agent",
                "--host",
                "127.0.0.1",
                "--port",
                &self.port.to_string(),
                "--agent-id",
                &self.node_id,
                "--watchdog-scan-interval-ms",
                "50",
                "--watchdog-stale-execution-ms",
                "10000",
            ])
            .stdout(Stdio::from(log.try_clone().map_err(|error| {
                format!("failed to clone Agent log: {error}")
            })?))
            .stderr(Stdio::from(log))
            .spawn()
            .map_err(|error| format!("failed to start {}: {error}", self.node_id))?;
        self.child = Some(child);
        self.wait_ready(Duration::from_secs(20))
    }

    pub(crate) fn stop(&mut self) -> Result<(), String> {
        if let Some(mut child) = self.child.take() {
            if child
                .try_wait()
                .map_err(|error| format!("failed to inspect {}: {error}", self.node_id))?
                .is_none()
            {
                child
                    .kill()
                    .map_err(|error| format!("failed to stop {}: {error}", self.node_id))?;
            }
            child
                .wait()
                .map_err(|error| format!("failed to reap {}: {error}", self.node_id))?;
        }
        Ok(())
    }

    pub(crate) fn solve_bar(&self, request_id: &str) -> Result<SolverProbeResult, String> {
        let response = rpc_request(
            self.address(),
            RpcRequest {
                rpc_version: RPC_VERSION,
                id: request_id.to_string(),
                method: RpcMethod::SolveBar1d,
                params: json!({
                    "job_id": format!("{request_id}-job"),
                    "length": 1.0,
                    "area": 2.0,
                    "youngs_modulus": 1000.0,
                    "elements": 2,
                    "tip_force": 20.0
                }),
            },
        )?;
        if !response.ok {
            return Err(response
                .error
                .map(|error| format!("{}: {}", error.code, error.message))
                .unwrap_or_else(|| "Agent rejected solver probe".to_string()));
        }
        let result = response
            .result
            .ok_or("Agent solver probe omitted its result")?;
        Ok(SolverProbeResult {
            max_stress: required_f64(&result, "max_stress")?,
            tip_displacement: required_f64(&result, "tip_displacement")?,
        })
    }

    fn wait_ready(&mut self, timeout: Duration) -> Result<(), String> {
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            if TcpStream::connect_timeout(&self.address(), Duration::from_millis(100)).is_ok() {
                return Ok(());
            }
            if let Some(status) = self
                .child
                .as_mut()
                .ok_or("Agent qualification process is absent")?
                .try_wait()
                .map_err(|error| format!("failed to inspect {}: {error}", self.node_id))?
            {
                return Err(format!(
                    "{} exited with {status}: {}",
                    self.node_id,
                    fs::read_to_string(&self.log_path).unwrap_or_default()
                ));
            }
            thread::sleep(Duration::from_millis(25));
        }
        Err(format!("{} did not become ready", self.node_id))
    }
}

impl Drop for ManagedQualificationAgent {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

pub(crate) fn prepare_binary_copy(source: &Path, destination: &Path) -> Result<String, String> {
    let metadata = fs::symlink_metadata(source)
        .map_err(|error| format!("Agent qualification binary is unavailable: {error}"))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err("Agent qualification binary must be a regular file".to_string());
    }
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create qualification bin directory: {error}"))?;
    }
    fs::copy(source, destination)
        .map_err(|error| format!("failed to copy Agent qualification binary: {error}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(destination, fs::Permissions::from_mode(0o700))
            .map_err(|error| format!("failed to secure Agent qualification binary: {error}"))?;
    }
    sha256_file(destination)
}

fn rpc_request(address: SocketAddr, request: RpcRequest) -> Result<RpcResponse, String> {
    let timeout = Duration::from_secs(10);
    let mut stream = TcpStream::connect_timeout(&address, Duration::from_secs(2))
        .map_err(|error| format!("Agent solver probe is unavailable: {error}"))?;
    stream
        .set_read_timeout(Some(timeout))
        .and_then(|_| stream.set_write_timeout(Some(timeout)))
        .map_err(|error| format!("failed to configure Agent solver probe: {error}"))?;
    let payload = serde_json::to_vec(&request)
        .map_err(|error| format!("failed to encode Agent solver probe: {error}"))?;
    let length = u32::try_from(payload.len())
        .map_err(|_| "Agent solver probe request is too large".to_string())?;
    stream
        .write_all(&length.to_be_bytes())
        .and_then(|_| stream.write_all(&payload))
        .map_err(|error| format!("failed to write Agent solver probe: {error}"))?;
    loop {
        let value = read_json_frame(&mut stream)?;
        if value.get("event").and_then(Value::as_str) == Some("progress") {
            continue;
        }
        return serde_json::from_value(value)
            .map_err(|error| format!("invalid Agent solver response: {error}"));
    }
}

fn read_json_frame(stream: &mut TcpStream) -> Result<Value, String> {
    let mut header = [0_u8; 4];
    stream
        .read_exact(&mut header)
        .map_err(|error| format!("failed to read Agent solver frame header: {error}"))?;
    let length = u32::from_be_bytes(header) as usize;
    if length == 0 || length > MAX_FRAME_BYTES {
        return Err("Agent solver response frame length is invalid".to_string());
    }
    let mut payload = vec![0_u8; length];
    stream
        .read_exact(&mut payload)
        .map_err(|error| format!("failed to read Agent solver frame payload: {error}"))?;
    serde_json::from_slice(&payload)
        .map_err(|error| format!("Agent solver response is not JSON: {error}"))
}

fn reserve_port() -> Result<u16, String> {
    TcpListener::bind(("127.0.0.1", 0))
        .and_then(|listener| listener.local_addr())
        .map(|address| address.port())
        .map_err(|error| format!("failed to reserve Agent qualification port: {error}"))
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file = File::open(path)
        .map_err(|error| format!("failed to open Agent qualification binary: {error}"))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| format!("failed to hash Agent qualification binary: {error}"))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn required_f64(value: &Value, key: &str) -> Result<f64, String> {
    value
        .get(key)
        .and_then(Value::as_f64)
        .ok_or_else(|| format!("Agent solver result omitted {key}"))
}
