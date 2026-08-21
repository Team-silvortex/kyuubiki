use super::report::{CleanupEvidence, JourneyPhases, PhaseEvidence};
use crate::operational_agent_support::{
    available_local_port, connection_profile, query_agent_descriptor, remove_local_work_root,
    wait_endpoint_closed,
};
use crate::qualification_support::generated_at_unix_ms;
use crate::remote_host::{
    remote_shell_path, rsync_to, shell_escape, ssh_status, ssh_success_quiet,
};
use getrandom::fill as fill_random;
use kyuubiki_protocol::AgentControlLinkDescriptor;
use serde_json::Value;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

type RunnerResult<T> = Result<T, String>;

const AGENT_ID: &str = "control-link-qualification-agent";
const CLUSTER_ID: &str = "control-link-qualification-cluster";
const MAX_HTTP_BYTES: u64 = 4 * 1024 * 1024;

pub(crate) fn capture(
    root: &Path,
    host: &str,
    timeout: Duration,
) -> RunnerResult<(JourneyPhases, CleanupEvidence)> {
    let mut session = Session::new(root, host, timeout)?;
    let journey = session.run();
    let cleanup = session.cleanup();
    match (journey, cleanup) {
        (Ok(phases), Ok(cleanup)) => Ok((phases, cleanup)),
        (Err(error), Ok(_)) => Err(error),
        (Ok(_), Err(cleanup_error)) => Err(cleanup_error),
        (Err(error), Err(cleanup_error)) => Err(format!("{error}; cleanup: {cleanup_error}")),
    }
}

struct Session {
    root: PathBuf,
    host: String,
    timeout: Duration,
    run_root: String,
    work_root: PathBuf,
    token: String,
    local_ip: Ipv4Addr,
    remote_ip: Ipv4Addr,
    remote_architecture: String,
    orchestra_port: u16,
    agent_port: Option<u16>,
    orchestra: Option<Child>,
    remote_prepared: bool,
    remote_started: bool,
    cleaned: bool,
}

impl Session {
    fn new(root: &Path, host: &str, timeout: Duration) -> RunnerResult<Self> {
        if std::env::consts::OS != "macos" {
            return Err(
                "two-host control-link capture currently requires a macOS Orchestra host"
                    .to_string(),
            );
        }
        let connection = connection_profile(root, host)?;
        let nonce = generated_at_unix_ms()?;
        let run_root = format!(
            "~/.kyuubiki/lab-runs/agent-control-link-{nonce}-{}",
            std::process::id()
        );
        let work_root = root.join(format!(
            "tmp/agent-control-link-operational-{nonce}-{}",
            std::process::id()
        ));
        fs::create_dir_all(&work_root)
            .map_err(|error| format!("failed to create qualification work root: {error}"))?;
        Ok(Self {
            root: root.to_path_buf(),
            host: host.to_string(),
            timeout,
            run_root,
            work_root,
            token: random_token()?,
            local_ip: connection.local_ip,
            remote_ip: connection.remote_ip,
            remote_architecture: connection.remote_architecture,
            orchestra_port: available_local_port()?,
            agent_port: None,
            orchestra: None,
            remote_prepared: false,
            remote_started: false,
            cleaned: false,
        })
    }

    fn run(&mut self) -> RunnerResult<JourneyPhases> {
        self.prepare_remote()?;
        self.start_orchestra(1)?;
        self.wait_orchestra_ready()?;
        self.start_remote_agent()?;

        let initial_link = self.wait_for_link(|link| {
            link.state == "registered"
                && link.successful_registration_count >= 1
                && link.successful_heartbeat_count >= 1
                && link.consecutive_failure_count == 0
        })?;
        self.wait_for_registry(initial_link.successful_registration_count)?;
        let initial = self.phase(true, true, initial_link)?;

        self.stop_orchestra()?;
        wait_endpoint_closed(
            SocketAddr::from(([127, 0, 0, 1], self.orchestra_port)),
            self.timeout,
        )?;
        let outage_link = self.wait_for_link(|link| {
            link.state == "degraded"
                && link.consecutive_failure_count >= 1
                && matches!(
                    link.last_failure_code.as_deref(),
                    Some("endpoint_unreachable" | "transport_failed")
                )
        })?;
        let outage = self.phase(false, false, outage_link)?;

        self.start_orchestra(2)?;
        self.wait_orchestra_ready()?;
        let initial_registrations = initial.control_link.successful_registration_count;
        let initial_heartbeats = initial.control_link.successful_heartbeat_count;
        let recovered_link = self.wait_for_link(|link| {
            link.state == "registered"
                && link.successful_registration_count > initial_registrations
                && link.successful_heartbeat_count > initial_heartbeats
                && link.consecutive_failure_count == 0
        })?;
        self.wait_for_registry(recovered_link.successful_registration_count)?;
        let recovered = self.phase(true, true, recovered_link)?;

        Ok(JourneyPhases {
            remote_architecture: self.remote_architecture.clone(),
            initial,
            outage,
            recovered,
        })
    }

    fn phase(
        &self,
        orchestrator_available: bool,
        registry_visible: bool,
        control_link: AgentControlLinkDescriptor,
    ) -> RunnerResult<PhaseEvidence> {
        let agent_process_alive = self.remote_agent_alive()?;
        if !agent_process_alive {
            return Err("remote Agent process did not survive the qualification phase".to_string());
        }
        Ok(PhaseEvidence {
            orchestrator_available,
            registry_visible,
            agent_process_alive,
            control_link,
        })
    }

    fn prepare_remote(&mut self) -> RunnerResult<()> {
        let run_root = remote_shell_path(&self.run_root);
        let status = ssh_status(
            &self.root,
            &self.host,
            format!("set -eu; umask 077; mkdir -p {run_root}/workers/rust"),
        )?;
        if status != 0 {
            return Err(format!(
                "failed to prepare remote run root: status {status}"
            ));
        }
        self.remote_prepared = true;
        let sync = rsync_to(
            &self.root,
            &["target/", ".DS_Store"],
            &[self.root.join("workers/rust/")],
            &format!("{}:{}/workers/rust/", self.host, self.run_root),
        )?;
        if sync != 0 {
            return Err(format!("failed to synchronize Agent source: status {sync}"));
        }
        let build = ssh_status(&self.root, &self.host, remote_build_command(&self.run_root))?;
        if build != 0 {
            return Err(format!(
                "failed to build remote Release Agent: status {build}"
            ));
        }
        Ok(())
    }

    fn start_remote_agent(&mut self) -> RunnerResult<()> {
        let seed = u16::from_str_radix(&self.token[..4], 16)
            .map_err(|error| format!("failed to derive qualification port seed: {error}"))?;
        let first_port = 46_000 + (seed % 12_000);
        for offset in 0..8_u16 {
            let port = first_port + offset;
            self.transfer_secret()?;
            let status = ssh_status(
                &self.root,
                &self.host,
                remote_start_command(
                    &self.run_root,
                    self.local_ip,
                    self.remote_ip,
                    self.orchestra_port,
                    port,
                ),
            )?;
            let _ = fs::remove_file(self.local_secret_path());
            if status == 0 {
                self.agent_port = Some(port);
                self.remote_started = true;
                if self.remote_agent_alive()? {
                    return Ok(());
                }
            }
            let _ = ssh_status(
                &self.root,
                &self.host,
                remote_reset_agent_command(&self.run_root),
            );
        }
        Err("could not allocate an isolated remote Agent port".to_string())
    }

    fn transfer_secret(&self) -> RunnerResult<()> {
        let path = self.local_secret_path();
        write_secret_file(&path, &self.token)?;
        let status = rsync_to(
            &self.root,
            &[],
            &[path.clone()],
            &format!("{}:{}/control.env", self.host, self.run_root),
        );
        let _ = fs::remove_file(&path);
        match status? {
            0 => Ok(()),
            code => Err(format!(
                "failed to transfer ephemeral control token: status {code}"
            )),
        }
    }

    fn local_secret_path(&self) -> PathBuf {
        self.work_root.join("control.env")
    }

    fn start_orchestra(&mut self, generation: u8) -> RunnerResult<()> {
        if self.orchestra.is_some() {
            return Err("Orchestra qualification process is already running".to_string());
        }
        let log_path = self
            .work_root
            .join(format!("orchestra-generation-{generation}.log"));
        let stdout = File::create(&log_path)
            .map_err(|error| format!("failed to create Orchestra log: {error}"))?;
        let stderr = stdout
            .try_clone()
            .map_err(|error| format!("failed to clone Orchestra log: {error}"))?;
        let child = Command::new("mix")
            .args(["run", "--no-halt"])
            .current_dir(self.root.join("apps/web"))
            .env("MIX_ENV", "dev")
            .env("PORT", self.orchestra_port.to_string())
            .env("KYUUBIKI_HTTP_BIND_IP", "0.0.0.0")
            .env("KYUUBIKI_DEPLOYMENT_MODE", "distributed")
            .env("KYUUBIKI_STORAGE_BACKEND", "memory")
            .env("KYUUBIKI_AGENT_DISCOVERY", "registry")
            .env("KYUUBIKI_AGENT_ENDPOINTS", "")
            .env("KYUUBIKI_API_TOKEN", &self.token)
            .env("KYUUBIKI_CLUSTER_API_TOKEN", &self.token)
            .env("KYUUBIKI_CLUSTER_ALLOWED_AGENT_IDS", AGENT_ID)
            .env("KYUUBIKI_CLUSTER_ALLOWED_CLUSTER_IDS", CLUSTER_ID)
            .env("KYUUBIKI_CLUSTER_REQUIRE_FINGERPRINT", "false")
            .env("KYUUBIKI_PROTECT_READS", "true")
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr))
            .spawn()
            .map_err(|error| {
                format!("failed to start Orchestra generation {generation}: {error}")
            })?;
        self.orchestra = Some(child);
        Ok(())
    }

    fn wait_orchestra_ready(&mut self) -> RunnerResult<()> {
        let deadline = Instant::now() + self.timeout;
        while Instant::now() < deadline {
            if let Some(status) = self
                .orchestra
                .as_mut()
                .ok_or("Orchestra process is unavailable")?
                .try_wait()
                .map_err(|error| format!("failed to inspect Orchestra process: {error}"))?
            {
                return Err(format!("Orchestra exited before readiness: {status}"));
            }
            if http_get_json(self.orchestra_port, "/api/health", &self.token).is_ok() {
                return Ok(());
            }
            thread::sleep(Duration::from_millis(100));
        }
        Err("Orchestra readiness timed out".to_string())
    }

    fn wait_for_link(
        &self,
        predicate: impl Fn(&AgentControlLinkDescriptor) -> bool,
    ) -> RunnerResult<AgentControlLinkDescriptor> {
        let port = self.agent_port.ok_or("remote Agent port is unavailable")?;
        let address = SocketAddr::new(IpAddr::V4(self.remote_ip), port);
        let deadline = Instant::now() + self.timeout;
        let mut last = None;
        while Instant::now() < deadline {
            if let Ok(descriptor) = query_agent_descriptor(address) {
                let link = descriptor.control_plane_link;
                if predicate(&link) {
                    return Ok(link);
                }
                last = Some(format!(
                    "state={} registrations={} heartbeats={} failures={}",
                    link.state,
                    link.successful_registration_count,
                    link.successful_heartbeat_count,
                    link.consecutive_failure_count
                ));
            }
            thread::sleep(Duration::from_millis(100));
        }
        Err(format!(
            "Agent control-link phase timed out ({})",
            last.unwrap_or_else(|| "no descriptor received".to_string())
        ))
    }

    fn wait_for_registry(&self, minimum_registrations: u64) -> RunnerResult<()> {
        let deadline = Instant::now() + self.timeout;
        while Instant::now() < deadline {
            if let Ok(value) = http_get_json(self.orchestra_port, "/api/v1/agents", &self.token)
                && registry_contains_agent(&value, minimum_registrations)
            {
                return Ok(());
            }
            thread::sleep(Duration::from_millis(100));
        }
        Err("Orchestra registry did not expose the recovered Agent".to_string())
    }

    fn remote_agent_alive(&self) -> RunnerResult<bool> {
        if !self.remote_started {
            return Ok(false);
        }
        ssh_success_quiet(&self.root, &self.host, remote_alive_command(&self.run_root))
    }

    fn stop_orchestra(&mut self) -> RunnerResult<()> {
        if let Some(mut child) = self.orchestra.take() {
            child
                .kill()
                .map_err(|error| format!("failed to stop Orchestra: {error}"))?;
            child
                .wait()
                .map_err(|error| format!("failed to reap Orchestra: {error}"))?;
        }
        Ok(())
    }

    fn cleanup(&mut self) -> RunnerResult<CleanupEvidence> {
        if self.cleaned {
            return Err("qualification session was already cleaned".to_string());
        }
        let mut errors = Vec::new();
        let remote_agent_stopped = match self.stop_remote_agent() {
            Ok(()) => true,
            Err(error) => {
                errors.push(error);
                false
            }
        };
        let remote_port_closed = match self.agent_port {
            Some(port) => wait_endpoint_closed(
                SocketAddr::new(IpAddr::V4(self.remote_ip), port),
                Duration::from_secs(5),
            )
            .map(|_| true)
            .unwrap_or_else(|error| {
                errors.push(error);
                false
            }),
            None => true,
        };
        let secret_files_removed = self.remote_secret_removed().unwrap_or_else(|error| {
            errors.push(error);
            false
        }) && !self.local_secret_path().exists();
        let local_orchestra_stopped = self.stop_orchestra().map(|_| true).unwrap_or_else(|error| {
            errors.push(error);
            false
        });
        let local_port_closed = wait_endpoint_closed(
            SocketAddr::from(([127, 0, 0, 1], self.orchestra_port)),
            Duration::from_secs(5),
        )
        .map(|_| true)
        .unwrap_or_else(|error| {
            errors.push(error);
            false
        });
        let managed_remote_root_removed = self.remove_remote_root().unwrap_or_else(|error| {
            errors.push(error);
            false
        });
        let local_work_root_removed =
            remove_local_work_root(&self.work_root).unwrap_or_else(|error| {
                errors.push(error);
                false
            });
        if !errors.is_empty() {
            return Err(errors.join("; "));
        }
        self.cleaned = true;
        Ok(CleanupEvidence {
            local_orchestra_stopped,
            remote_agent_stopped,
            local_port_closed,
            remote_port_closed,
            managed_remote_root_removed,
            secret_files_removed,
            local_work_root_removed,
        })
    }

    fn stop_remote_agent(&mut self) -> RunnerResult<()> {
        if !self.remote_prepared {
            return Ok(());
        }
        let status = ssh_status(
            &self.root,
            &self.host,
            remote_reset_agent_command(&self.run_root),
        )?;
        if status != 0 {
            return Err(format!(
                "failed to stop managed remote Agent: status {status}"
            ));
        }
        self.remote_started = false;
        Ok(())
    }

    fn remote_secret_removed(&self) -> RunnerResult<bool> {
        if !self.remote_prepared {
            return Ok(true);
        }
        ssh_success_quiet(
            &self.root,
            &self.host,
            format!(
                "set -eu; run_root={}; test ! -e \"$run_root/control.env\"",
                remote_shell_path(&self.run_root)
            ),
        )
    }

    fn remove_remote_root(&mut self) -> RunnerResult<bool> {
        if !self.remote_prepared {
            return Ok(true);
        }
        let run_root = remote_shell_path(&self.run_root);
        let status = ssh_status(
            &self.root,
            &self.host,
            format!(
                "set -eu; run_root={run_root}; case \"$run_root\" in \"$HOME/.kyuubiki/lab-runs/\"*) rm -rf \"$run_root\" ;; *) exit 2 ;; esac"
            ),
        )?;
        if status != 0 {
            return Err(format!(
                "failed to remove managed remote root: status {status}"
            ));
        }
        let absent = ssh_success_quiet(
            &self.root,
            &self.host,
            format!("set -eu; run_root={run_root}; test ! -e \"$run_root\""),
        )?;
        if absent {
            self.remote_prepared = false;
        }
        Ok(absent)
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        if !self.cleaned {
            let _ = self.cleanup();
        }
    }
}

fn random_token() -> RunnerResult<String> {
    let mut bytes = [0_u8; 32];
    fill_random(&mut bytes)
        .map_err(|error| format!("failed to generate control token: {error}"))?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

fn write_secret_file(path: &Path, token: &str) -> RunnerResult<()> {
    #[cfg(unix)]
    let mut file = {
        use std::os::unix::fs::OpenOptionsExt;
        OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .mode(0o600)
            .open(path)
    };
    #[cfg(not(unix))]
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(path);
    file.as_mut()
        .map_err(|error| format!("failed to create ephemeral control token file: {error}"))?
        .write_all(format!("KYUUBIKI_CLUSTER_API_TOKEN={token}\n").as_bytes())
        .map_err(|error| format!("failed to write ephemeral control token file: {error}"))
}

fn remote_build_command(run_root: &str) -> String {
    let run_root = remote_shell_path(run_root);
    format!(
        "set -eu; umask 077; run_root={run_root}; source_root=\"$run_root/workers/rust\"; target_root=\"$HOME/.kyuubiki/cache/cargo-target/agent-control-link-operational\"; mkdir -p \"$target_root\"; cd \"$source_root\"; CARGO_TARGET_DIR=\"$target_root\" cargo build --release -p kyuubiki-cli; install -m 700 \"$target_root/release/kyuubiki-cli\" \"$run_root/kyuubiki-agent\""
    )
}

fn remote_start_command(
    run_root: &str,
    local_ip: Ipv4Addr,
    remote_ip: Ipv4Addr,
    orchestra_port: u16,
    agent_port: u16,
) -> String {
    let run_root = remote_shell_path(run_root);
    let orchestra_url = shell_escape(&format!("http://{local_ip}:{orchestra_port}"));
    let advertise_host = shell_escape(&remote_ip.to_string());
    format!(
        "set -eu; umask 077; run_root={run_root}; test -x \"$run_root/kyuubiki-agent\"; test -f \"$run_root/control.env\"; set -a; . \"$run_root/control.env\"; set +a; rm -f \"$run_root/agent.pid\"; nohup \"$run_root/kyuubiki-agent\" agent --host 0.0.0.0 --port {agent_port} --agent-id {AGENT_ID} --advertise-host {advertise_host} --orchestrator-url {orchestra_url} --cluster-id {CLUSTER_ID} --register-interval-ms 250 >\"$run_root/agent.log\" 2>&1 </dev/null & pid=$!; printf '%s\\n' \"$pid\" >\"$run_root/agent.pid\"; rm -f \"$run_root/control.env\"; sleep 1; kill -0 \"$pid\"; test \"$(readlink -f \"/proc/$pid/exe\")\" = \"$run_root/kyuubiki-agent\""
    )
}

fn remote_alive_command(run_root: &str) -> String {
    let run_root = remote_shell_path(run_root);
    format!(
        "set -eu; run_root={run_root}; pid=$(cat \"$run_root/agent.pid\"); case \"$pid\" in ''|*[!0-9]*) exit 2 ;; esac; kill -0 \"$pid\"; test \"$(readlink -f \"/proc/$pid/exe\")\" = \"$run_root/kyuubiki-agent\""
    )
}

fn remote_reset_agent_command(run_root: &str) -> String {
    let run_root = remote_shell_path(run_root);
    format!(
        "set -eu; run_root={run_root}; if test -f \"$run_root/agent.pid\"; then pid=$(cat \"$run_root/agent.pid\"); case \"$pid\" in ''|*[!0-9]*) exit 2 ;; esac; if kill -0 \"$pid\" 2>/dev/null; then actual=$(readlink -f \"/proc/$pid/exe\" || true); test \"$actual\" = \"$run_root/kyuubiki-agent\"; kill \"$pid\"; count=0; while kill -0 \"$pid\" 2>/dev/null && test \"$count\" -lt 50; do sleep 0.1; count=$((count + 1)); done; if kill -0 \"$pid\" 2>/dev/null; then kill -9 \"$pid\"; fi; fi; fi; rm -f \"$run_root/agent.pid\" \"$run_root/control.env\""
    )
}

fn http_get_json(port: u16, path: &str, token: &str) -> RunnerResult<Value> {
    let address = SocketAddr::from(([127, 0, 0, 1], port));
    let mut stream = TcpStream::connect_timeout(&address, Duration::from_millis(500))
        .map_err(|error| format!("Orchestra HTTP unavailable: {error}"))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(|error| format!("failed to configure Orchestra HTTP timeout: {error}"))?;
    let request = format!(
        "GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nAuthorization: Bearer {token}\r\nConnection: close\r\n\r\n"
    );
    stream
        .write_all(request.as_bytes())
        .map_err(|error| format!("failed to write Orchestra HTTP request: {error}"))?;
    let mut response = Vec::new();
    stream
        .take(MAX_HTTP_BYTES + 1)
        .read_to_end(&mut response)
        .map_err(|error| format!("failed to read Orchestra HTTP response: {error}"))?;
    if response.len() as u64 > MAX_HTTP_BYTES {
        return Err("Orchestra HTTP response exceeded 4 MiB".to_string());
    }
    let separator = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or("Orchestra returned invalid HTTP")?;
    let headers = String::from_utf8_lossy(&response[..separator]);
    if !(headers.starts_with("HTTP/1.1 2") || headers.starts_with("HTTP/1.0 2")) {
        return Err("Orchestra returned a non-success HTTP status".to_string());
    }
    let body = if headers.lines().any(|line| {
        line.to_ascii_lowercase()
            .starts_with("transfer-encoding: chunked")
    }) {
        decode_chunked(&response[(separator + 4)..])?
    } else {
        response[(separator + 4)..].to_vec()
    };
    serde_json::from_slice(&body).map_err(|error| format!("invalid Orchestra JSON: {error}"))
}

fn decode_chunked(mut bytes: &[u8]) -> RunnerResult<Vec<u8>> {
    let mut decoded = Vec::new();
    loop {
        let line_end = bytes
            .windows(2)
            .position(|window| window == b"\r\n")
            .ok_or("invalid chunked HTTP length")?;
        let length_text = std::str::from_utf8(&bytes[..line_end])
            .map_err(|_| "invalid chunked HTTP length encoding")?;
        let length = usize::from_str_radix(length_text.split(';').next().unwrap_or(""), 16)
            .map_err(|_| "invalid chunked HTTP length value")?;
        bytes = &bytes[(line_end + 2)..];
        if length == 0 {
            return Ok(decoded);
        }
        if bytes.len() < length + 2 || &bytes[length..(length + 2)] != b"\r\n" {
            return Err("truncated chunked HTTP body".to_string());
        }
        decoded.extend_from_slice(&bytes[..length]);
        bytes = &bytes[(length + 2)..];
    }
}

fn registry_contains_agent(value: &Value, minimum_registrations: u64) -> bool {
    value
        .get("agents")
        .and_then(Value::as_array)
        .and_then(|agents| {
            agents
                .iter()
                .find(|agent| agent.get("id").and_then(Value::as_str) == Some(AGENT_ID))
        })
        .and_then(|agent| agent.get("control_plane_link"))
        .is_some_and(|link| {
            link.get("state").and_then(Value::as_str) == Some("registered")
                && link
                    .get("successful_registration_count")
                    .and_then(Value::as_u64)
                    .is_some_and(|count| count >= minimum_registrations)
        })
}

#[cfg(test)]
#[path = "runtime_tests.rs"]
mod tests;
