use super::distribution;
use super::http;
use super::installed;
use super::remote::{self, AGENT_ID, CLUSTER_ID};
use super::report::{
    AgentEvidence, CentralRequestEvidence, CleanupEvidence, ExecutionEvidence, JourneyEvidence,
};
use super::task;
use crate::operational_agent_support::{
    available_local_port, connection_profile, query_agent_descriptor_value, remove_local_work_root,
    wait_endpoint_closed,
};
use crate::qualification_support::{combined_output, generated_at_unix_ms};
use getrandom::fill as fill_random;
use serde_json::{Value, json};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

type RunnerResult<T> = Result<T, String>;

const EXECUTE_PATH: &str = "/api/v1/operator-tasks/execute";

pub(crate) fn capture(
    root: &Path,
    host: &str,
    package_version: &str,
    timeout: Duration,
) -> RunnerResult<(JourneyEvidence, CleanupEvidence)> {
    let mut session = Session::new(root, host, package_version, timeout)?;
    let journey = session.run();
    let diagnostic = journey
        .as_ref()
        .err()
        .and_then(|_| session.safe_orchestra_diagnostic());
    let cleanup = session.cleanup();
    match (journey, cleanup) {
        (Ok(journey), Ok(cleanup)) => Ok((journey, cleanup)),
        (Err(error), Ok(_)) => Err(append_diagnostic(error, diagnostic)),
        (Ok(_), Err(cleanup_error)) => Err(cleanup_error),
        (Err(error), Err(cleanup_error)) => Err(format!(
            "{}; cleanup: {cleanup_error}",
            append_diagnostic(error, diagnostic)
        )),
    }
}

struct Session {
    root: PathBuf,
    host: String,
    package_version: String,
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
    orchestra_log: PathBuf,
    remote_prepared: bool,
    remote_started: bool,
    cleaned: bool,
}

impl Session {
    fn new(
        root: &Path,
        host: &str,
        package_version: &str,
        timeout: Duration,
    ) -> RunnerResult<Self> {
        if std::env::consts::OS != "macos" {
            return Err(
                "two-host package acquisition capture currently requires a macOS Orchestra host"
                    .to_string(),
            );
        }
        let connection = connection_profile(root, host)?;
        let nonce = generated_at_unix_ms()?;
        let work_root = root.join(format!(
            "tmp/operator-package-acquisition-{nonce}-{}",
            std::process::id()
        ));
        fs::create_dir_all(&work_root)
            .map_err(|error| format!("failed to create qualification work root: {error}"))?;
        Ok(Self {
            root: root.to_path_buf(),
            host: host.to_string(),
            package_version: package_version.to_string(),
            timeout,
            run_root: format!(
                "~/.kyuubiki/lab-runs/operator-package-acquisition-{nonce}-{}",
                std::process::id()
            ),
            orchestra_log: work_root.join("orchestra.log"),
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

    fn run(&mut self) -> RunnerResult<JourneyEvidence> {
        self.compile_orchestra()?;
        self.remote_prepared = true;
        remote::prepare_source(&self.root, &self.host, &self.run_root)?;
        remote::build_remote(&self.root, &self.host, &self.run_root)?;

        let distribution_root = self.work_root.join("operator-distributions");
        let entrypoint = distribution::entrypoint_path(&distribution_root);
        fs::create_dir_all(
            entrypoint
                .parent()
                .ok_or("operator distribution entrypoint has no parent")?,
        )
        .map_err(|error| format!("failed to create local distribution root: {error}"))?;
        remote::retrieve_operator(&self.root, &self.host, &self.run_root, &entrypoint)?;
        let remote_operator_artifact_absent =
            remote::remove_remote_operator_build(&self.root, &self.host, &self.run_root)?;
        if !remote_operator_artifact_absent {
            return Err("remote host retained the operator build artifact".to_string());
        }
        let distribution = distribution::seal(&distribution_root)?;

        remote::prepare_installed_agent(
            &self.root,
            &self.host,
            &self.run_root,
            &self.package_version,
        )?;
        let installation_path = self.work_root.join("installation.json");
        remote::retrieve_installation(&self.root, &self.host, &self.run_root, &installation_path)?;
        let installation = installed::read(&installation_path)?;

        self.start_orchestra(&distribution_root)?;
        self.wait_orchestra_ready()?;
        self.start_remote_agent()?;
        let initial_agent = self.wait_agent_ready(0)?;
        self.wait_registry_ready()?;

        let first = self.execute_task("operator-package-acquisition-first", &distribution)?;
        self.wait_agent_ready(0)?;
        let second = self.execute_task("operator-package-acquisition-second", &distribution)?;
        let final_agent = self.wait_agent_ready(0)?;

        self.stop_orchestra()?;
        let central_requests = self.central_request_evidence()?;
        Ok(JourneyEvidence {
            remote_architecture: self.remote_architecture.clone(),
            installation,
            distribution,
            remote_operator_artifact_absent,
            initial_agent,
            final_agent,
            executions: vec![first, second],
            central_requests,
        })
    }

    fn compile_orchestra(&self) -> RunnerResult<()> {
        let output = Command::new("mix")
            .arg("compile")
            .current_dir(self.root.join("apps/web"))
            .env("MIX_ENV", "dev")
            .output()
            .map_err(|error| format!("failed to invoke Orchestra compilation: {error}"))?;
        if output.status.success() {
            Ok(())
        } else {
            Err(format!(
                "Orchestra compilation failed: {}",
                combined_output(&output)
            ))
        }
    }

    fn start_orchestra(&mut self, distribution_root: &Path) -> RunnerResult<()> {
        let stdout = File::create(&self.orchestra_log)
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
            .env("KYUUBIKI_ENABLE_TRANSITIONAL_WORKER_ADAPTERS", "false")
            .env("KYUUBIKI_OPERATOR_PACKAGE_DISTRIBUTIONS", distribution_root)
            .env(
                "KYUUBIKI_ORCHESTRA_INSTANCE_ID",
                format!("operator-package-acquisition-{}", std::process::id()),
            )
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr))
            .spawn()
            .map_err(|error| format!("failed to start real Elixir Orchestra: {error}"))?;
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
            if let Ok(response) = http::get_json(self.orchestra_port, "/api/health", &self.token)
                && response.status == 200
            {
                return Ok(());
            }
            thread::sleep(Duration::from_millis(100));
        }
        Err("Orchestra readiness timed out".to_string())
    }

    fn start_remote_agent(&mut self) -> RunnerResult<()> {
        let seed = u16::from_str_radix(&self.token[..4], 16)
            .map_err(|error| format!("failed to derive qualification port seed: {error}"))?;
        let first_port = 46_000 + (seed % 12_000);
        for offset in 0..8_u16 {
            let port = first_port + offset;
            self.transfer_secret()?;
            let started = remote::start_agent(remote::AgentStart {
                root: &self.root,
                host: &self.host,
                run_root: &self.run_root,
                package_version: &self.package_version,
                local_ip: self.local_ip,
                remote_ip: self.remote_ip,
                orchestra_port: self.orchestra_port,
                agent_port: port,
            })?;
            let _ = fs::remove_file(self.local_secret_path());
            if started {
                self.agent_port = Some(port);
                self.remote_started = true;
                if remote::agent_alive(
                    &self.root,
                    &self.host,
                    &self.run_root,
                    &self.package_version,
                )? {
                    return Ok(());
                }
            }
            let _ = remote::stop_agent(
                &self.root,
                &self.host,
                &self.run_root,
                &self.package_version,
            );
            self.remote_started = false;
            self.agent_port = None;
        }
        Err("could not allocate an isolated remote Agent port".to_string())
    }

    fn transfer_secret(&self) -> RunnerResult<()> {
        let path = self.local_secret_path();
        write_secret_file(&path, &self.token)?;
        let result = remote::transfer_secret(&self.root, &self.host, &self.run_root, &path);
        let _ = fs::remove_file(&path);
        result
    }

    fn local_secret_path(&self) -> PathBuf {
        self.work_root.join("control.env")
    }

    fn agent_address(&self) -> RunnerResult<SocketAddr> {
        Ok(SocketAddr::new(
            IpAddr::V4(self.remote_ip),
            self.agent_port.ok_or("remote Agent port is unavailable")?,
        ))
    }

    fn wait_agent_ready(&self, expected_count: u64) -> RunnerResult<AgentEvidence> {
        let address = self.agent_address()?;
        let deadline = Instant::now() + self.timeout;
        let mut last = "no descriptor received".to_string();
        while Instant::now() < deadline {
            if let Ok(descriptor) = query_agent_descriptor_value(address) {
                match agent_evidence(&descriptor) {
                    Ok(evidence)
                        if evidence.package_runtime_ready
                            && evidence.activated_package_count == expected_count
                            && evidence.control_link_state == "registered"
                            && evidence.successful_registration_count > 0 =>
                    {
                        return Ok(evidence);
                    }
                    Ok(evidence) => {
                        last = format!(
                            "ready={} active={} link={} registrations={}",
                            evidence.package_runtime_ready,
                            evidence.activated_package_count,
                            evidence.control_link_state,
                            evidence.successful_registration_count
                        );
                    }
                    Err(error) => last = error,
                }
            }
            thread::sleep(Duration::from_millis(100));
        }
        Err(format!("remote Agent readiness timed out ({last})"))
    }

    fn wait_registry_ready(&self) -> RunnerResult<()> {
        let deadline = Instant::now() + self.timeout;
        while Instant::now() < deadline {
            if let Ok(response) = http::get_json(self.orchestra_port, "/api/v1/agents", &self.token)
                && response.status == 200
                && registry_has_ready_agent(&response.body)
            {
                return Ok(());
            }
            thread::sleep(Duration::from_millis(100));
        }
        Err("Orchestra registry did not expose a package-ready Agent".to_string())
    }

    fn execute_task(
        &self,
        task_id: &str,
        distribution: &distribution::DistributionEvidence,
    ) -> RunnerResult<ExecutionEvidence> {
        let task = task::build(task_id, &distribution.entrypoint_sha256)?;
        let response = http::post_json(
            self.orchestra_port,
            EXECUTE_PATH,
            &self.token,
            &json!({"task": task}),
        )?;
        execution_evidence(response, task_id, &distribution.entrypoint_sha256)
    }

    fn central_request_evidence(&self) -> RunnerResult<CentralRequestEvidence> {
        let log = fs::read_to_string(&self.orchestra_log)
            .map_err(|error| format!("failed to read Orchestra request log: {error}"))?;
        let [resolve, manifest, entrypoint] = distribution::expected_paths();
        let resolution_count = count_occurrences(&log, &format!("GET {resolve}"));
        let manifest_count = count_occurrences(&log, &format!("GET {manifest}"));
        let entrypoint_count = count_occurrences(&log, &format!("GET {entrypoint}"));
        Ok(CentralRequestEvidence {
            resolution_count,
            manifest_count,
            entrypoint_count,
            successful_sequence_count: resolution_count.min(manifest_count).min(entrypoint_count),
            protected_reads: true,
        })
    }

    fn safe_orchestra_diagnostic(&self) -> Option<String> {
        let log = fs::read_to_string(&self.orchestra_log).ok()?;
        let root = self.root.to_string_lossy();
        let work_root = self.work_root.to_string_lossy();
        let local_ip = self.local_ip.to_string();
        let remote_ip = self.remote_ip.to_string();
        let lines = log
            .lines()
            .filter(|line| {
                line.contains("[error]")
                    || line.contains("** (")
                    || line.contains("POST /api/v1/operator-tasks/execute")
                    || line.contains("GET /api/v1/central/operator-packages/")
            })
            .map(|line| {
                line.replace(work_root.as_ref(), "<work-root>")
                    .replace(root.as_ref(), "<repo-root>")
                    .replace(&self.run_root, "<remote-root>")
                    .replace(&self.host, "<remote-host>")
                    .replace(&local_ip, "<local-address>")
                    .replace(&remote_ip, "<remote-address>")
                    .replace(&self.token, "<redacted>")
            })
            .rev()
            .take(12)
            .collect::<Vec<_>>();
        (!lines.is_empty()).then(|| lines.into_iter().rev().collect::<Vec<_>>().join(" | "))
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
        let remote_agent_stopped = self.stop_remote_agent().unwrap_or_else(|error| {
            errors.push(error);
            false
        });
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
        let secret_files_removed = self.secrets_removed().unwrap_or_else(|error| {
            errors.push(error);
            false
        });
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
            local_work_root_removed,
            secret_files_removed,
        })
    }

    fn stop_remote_agent(&mut self) -> RunnerResult<bool> {
        if !self.remote_prepared {
            return Ok(true);
        }
        remote::stop_agent(
            &self.root,
            &self.host,
            &self.run_root,
            &self.package_version,
        )?;
        self.remote_started = false;
        Ok(true)
    }

    fn secrets_removed(&self) -> RunnerResult<bool> {
        let local = !self.local_secret_path().exists();
        let remote = if self.remote_prepared {
            remote::secret_removed(&self.root, &self.host, &self.run_root)?
        } else {
            true
        };
        Ok(local && remote)
    }

    fn remove_remote_root(&mut self) -> RunnerResult<bool> {
        if !self.remote_prepared {
            return Ok(true);
        }
        let removed = remote::remove_run_root(&self.root, &self.host, &self.run_root)?;
        if removed {
            self.remote_prepared = false;
        }
        Ok(removed)
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        if !self.cleaned {
            let _ = self.cleanup();
        }
    }
}

fn agent_evidence(descriptor: &Value) -> RunnerResult<AgentEvidence> {
    Ok(AgentEvidence {
        program: required_string(descriptor, "/program")?,
        package_runtime_ready: required_bool(descriptor, "/operator_package_runtime/ready")?,
        activated_package_count: required_u64(
            descriptor,
            "/operator_package_runtime/attachment/activated_package_count",
        )?,
        control_link_state: required_string(descriptor, "/control_plane_link/state")?,
        successful_registration_count: required_u64(
            descriptor,
            "/control_plane_link/successful_registration_count",
        )?,
    })
}

fn registry_has_ready_agent(value: &Value) -> bool {
    value
        .get("agents")
        .and_then(Value::as_array)
        .and_then(|agents| {
            agents
                .iter()
                .find(|agent| agent.get("id").and_then(Value::as_str) == Some(AGENT_ID))
        })
        .is_some_and(|agent| {
            agent.pointer("/operator_package_runtime/ready") == Some(&Value::Bool(true))
                && agent
                    .pointer("/control_plane_link/state")
                    .and_then(Value::as_str)
                    == Some("registered")
        })
}

fn execution_evidence(
    response: http::HttpResponse,
    expected_task_id: &str,
    expected_entrypoint_sha256: &str,
) -> RunnerResult<ExecutionEvidence> {
    if response.status != 200 {
        return Err(format!(
            "Orchestra task dispatch returned HTTP {}: {}",
            response.status, response.body
        ));
    }
    let body = response.body;
    if body
        .pointer("/result/execution_runtime_status")
        .and_then(Value::as_str)
        != Some("external_operator_package_executed")
    {
        return Err(format!(
            "Orchestra did not execute the external package: {body}"
        ));
    }
    let task_id = required_string(&body, "/task_id")?;
    if task_id != expected_task_id {
        return Err("Orchestra returned the wrong task identity".to_string());
    }
    let entrypoint_sha256 = required_string(
        &body,
        "/result/operator_package_execution/entrypoint_sha256",
    )?;
    if entrypoint_sha256 != expected_entrypoint_sha256 {
        return Err("Agent executed an unexpected operator artifact".to_string());
    }
    Ok(ExecutionEvidence {
        task_id,
        status: required_string(&body, "/status")?,
        origin: required_string(&body, "/result/operator_package_execution/origin")?,
        cache_status: required_string(&body, "/result/operator_package_execution/cache_status")?,
        package_id: required_string(&body, "/result/operator_package_execution/package_id")?,
        package_version: required_string(
            &body,
            "/result/operator_package_execution/package_version",
        )?,
        entrypoint_sha256,
        integrity_verified: required_bool(
            &body,
            "/result/operator_package_execution/integrity_verified",
        )?,
        result_sum: required_f64(&body, "/result/result/summary/sum")?,
        eviction_disposition: required_string(
            &body,
            "/result/operator_package_execution/cache_eviction/disposition",
        )?,
        remaining_activated_package_count: required_u64(
            &body,
            "/result/operator_package_execution/cache_eviction/remaining_activated_package_count",
        )?,
    })
}

fn required_string(value: &Value, pointer: &str) -> RunnerResult<String> {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or_else(|| format!("qualification payload misses string {pointer}"))
}

fn required_bool(value: &Value, pointer: &str) -> RunnerResult<bool> {
    value
        .pointer(pointer)
        .and_then(Value::as_bool)
        .ok_or_else(|| format!("qualification payload misses bool {pointer}"))
}

fn required_u64(value: &Value, pointer: &str) -> RunnerResult<u64> {
    value
        .pointer(pointer)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("qualification payload misses integer {pointer}"))
}

fn required_f64(value: &Value, pointer: &str) -> RunnerResult<f64> {
    value
        .pointer(pointer)
        .and_then(Value::as_f64)
        .ok_or_else(|| format!("qualification payload misses number {pointer}"))
}

fn count_occurrences(haystack: &str, needle: &str) -> u64 {
    haystack.match_indices(needle).count() as u64
}

fn append_diagnostic(error: String, diagnostic: Option<String>) -> String {
    match diagnostic {
        Some(diagnostic) => format!("{error}; Orchestra diagnostic: {diagnostic}"),
        None => error,
    }
}

fn random_token() -> RunnerResult<String> {
    let mut bytes = [0_u8; 32];
    fill_random(&mut bytes)
        .map_err(|error| format!("failed to generate qualification token: {error}"))?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

fn write_secret_file(path: &Path, token: &str) -> RunnerResult<()> {
    #[cfg(unix)]
    let file = {
        use std::os::unix::fs::OpenOptionsExt;
        OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .mode(0o600)
            .open(path)
    };
    #[cfg(not(unix))]
    let file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(path);
    file.map_err(|error| format!("failed to create ephemeral token file: {error}"))?
        .write_all(format!("KYUUBIKI_CLUSTER_API_TOKEN={token}\n").as_bytes())
        .map_err(|error| format!("failed to write ephemeral token file: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_sanitized_agent_evidence() {
        let value = json!({
            "program": "kyuubiki-rust-agent",
            "operator_package_runtime": {
                "ready": true,
                "attachment": {"activated_package_count": 0}
            },
            "control_plane_link": {
                "state": "registered",
                "successful_registration_count": 1
            }
        });
        let evidence = agent_evidence(&value).expect("agent evidence");
        assert!(evidence.package_runtime_ready);
        assert_eq!(evidence.activated_package_count, 0);
    }

    #[test]
    fn counts_only_complete_request_lines() {
        let path = distribution::expected_paths()[0].clone();
        let log = format!("GET {path}\nGET /api/health\nGET {path}\n");
        assert_eq!(count_occurrences(&log, &format!("GET {path}")), 2);
    }
}
