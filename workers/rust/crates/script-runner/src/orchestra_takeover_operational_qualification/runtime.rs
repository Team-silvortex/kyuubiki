use super::partition_fixture::stale_owner_write_probe;
use super::partition_report::{PartitionJourneyEvidence, PartitionedOwnerPhase};
use super::report::{CleanupEvidence, JourneyEvidence, LeasePhase};
use super::runtime_http::{get_json as http_get_json, post_json as http_post_json};
use crate::operational_agent_support::{
    available_local_port, remove_local_work_root, wait_endpoint_closed,
};
use crate::qualification_support::generated_at_unix_ms;
use crate::remote_host::{shell_escape, ssh_output, ssh_status, ssh_success_quiet};
use getrandom::fill as fill_random;
use serde_json::Value;
use std::fs::{self, File};
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

type RunnerResult<T> = Result<T, String>;

pub(super) const LEASE_TTL_MS: u64 = 1_500;
pub(super) const HEARTBEAT_MS: u64 = 400;
pub(super) const RETRY_MS: u64 = 200;

pub(crate) fn capture(
    root: &Path,
    host: &str,
    postgres_image: &str,
    timeout: Duration,
) -> RunnerResult<(JourneyEvidence, CleanupEvidence)> {
    let mut session = Session::new(root, host, postgres_image, timeout)?;
    let journey = session.run();
    let cleanup = session.cleanup();
    match (journey, cleanup) {
        (Ok(journey), Ok(cleanup)) => Ok((journey, cleanup)),
        (Err(error), Ok(_)) => Err(error),
        (Ok(_), Err(cleanup_error)) => Err(cleanup_error),
        (Err(error), Err(cleanup_error)) => Err(format!("{error}; cleanup: {cleanup_error}")),
    }
}

pub(crate) fn capture_partition(
    root: &Path,
    host: &str,
    postgres_image: &str,
    timeout: Duration,
) -> RunnerResult<(PartitionJourneyEvidence, CleanupEvidence)> {
    let mut session = Session::new(root, host, postgres_image, timeout)?;
    let journey = session.run_partition();
    let cleanup = session.cleanup();
    match (journey, cleanup) {
        (Ok(journey), Ok(cleanup)) => Ok((journey, cleanup)),
        (Err(error), Ok(_)) => Err(error),
        (Ok(_), Err(cleanup_error)) => Err(cleanup_error),
        (Err(error), Err(cleanup_error)) => Err(format!("{error}; cleanup: {cleanup_error}")),
    }
}

#[derive(Clone, Copy)]
enum ProcessRole {
    Primary,
    Standby,
}

pub(super) struct LeaseObservation {
    pub(super) status: String,
    pub(super) owner_instance_id: String,
    pub(super) fencing_token: u64,
}

struct Session {
    root: PathBuf,
    host: String,
    postgres_image: String,
    timeout: Duration,
    work_root: PathBuf,
    container_name: String,
    lease_name: String,
    primary_id: String,
    standby_id: String,
    token: String,
    database_architecture: String,
    remote_database_port: Option<u16>,
    primary_tunnel_port: u16,
    standby_tunnel_port: u16,
    primary_port: u16,
    standby_port: u16,
    primary_tunnel: Option<Child>,
    standby_tunnel: Option<Child>,
    primary: Option<Child>,
    standby: Option<Child>,
    remote_database_started: bool,
    cleaned: bool,
}

impl Session {
    fn new(root: &Path, host: &str, postgres_image: &str, timeout: Duration) -> RunnerResult<Self> {
        if !cfg!(unix) {
            return Err(
                "SIGKILL takeover capture requires a Unix Orchestra qualification host".to_string(),
            );
        }
        let nonce = generated_at_unix_ms()?;
        let suffix = format!("{nonce}-{}", std::process::id());
        let work_root = root.join(format!("tmp/orchestra-takeover-{suffix}"));
        fs::create_dir_all(&work_root)
            .map_err(|error| format!("failed to create takeover work root: {error}"))?;
        let primary_tunnel_port = available_local_port()?;
        let standby_tunnel_port = distinct_port(&[primary_tunnel_port])?;
        let primary_port = distinct_port(&[primary_tunnel_port, standby_tunnel_port])?;
        let standby_port =
            distinct_port(&[primary_tunnel_port, standby_tunnel_port, primary_port])?;
        Ok(Self {
            root: root.to_path_buf(),
            host: host.to_string(),
            postgres_image: postgres_image.to_string(),
            timeout,
            work_root,
            container_name: format!("kyuubiki-orchestra-takeover-{suffix}"),
            lease_name: format!("workflow-recovery-takeover-{suffix}"),
            primary_id: format!("qualification-primary-{suffix}"),
            standby_id: format!("qualification-standby-{suffix}"),
            token: random_token()?,
            database_architecture: String::new(),
            remote_database_port: None,
            primary_tunnel_port,
            standby_tunnel_port,
            primary_port,
            standby_port,
            primary_tunnel: None,
            standby_tunnel: None,
            primary: None,
            standby: None,
            remote_database_started: false,
            cleaned: false,
        })
    }

    fn run(&mut self) -> RunnerResult<JourneyEvidence> {
        let standby_id = self.standby_id.clone();
        let (initial_owner, initial_standby) = self.initialize_cluster()?;

        self.crash(ProcessRole::Primary)?;
        wait_endpoint_closed(local_address(self.primary_port), self.timeout)?;
        let started = Instant::now();
        let takeover = self.wait_for_lease(ProcessRole::Standby, "owner", &standby_id)?;
        let takeover_elapsed_ms = started.elapsed().as_millis();
        if takeover.fencing_token <= initial_owner.fencing_token {
            return Err("standby takeover did not increment the fencing token".to_string());
        }

        self.start_orchestra(ProcessRole::Primary, true)?;
        let former_owner_rejoin =
            self.wait_for_lease(ProcessRole::Primary, "standby", &standby_id)?;
        if former_owner_rejoin.fencing_token != takeover.fencing_token {
            return Err("former owner identity did not observe the new fencing token".to_string());
        }

        Ok(JourneyEvidence {
            database_architecture: self.database_architecture.clone(),
            orchestra_platform: std::env::consts::OS.to_string(),
            initial_owner: self.phase("primary", initial_owner)?,
            initial_standby: self.phase("standby", initial_standby)?,
            takeover: self.phase("standby", takeover)?,
            former_owner_rejoin: self.phase("former-owner", former_owner_rejoin)?,
            takeover_elapsed_ms,
            primary_endpoint_closed: true,
        })
    }

    fn initialize_cluster(&mut self) -> RunnerResult<(LeaseObservation, LeaseObservation)> {
        let primary_id = self.primary_id.clone();
        self.compile_web()?;
        self.prepare_remote_database()?;
        self.start_tunnels()?;

        self.start_orchestra(ProcessRole::Primary, false)?;
        let initial_owner = self.wait_for_lease(ProcessRole::Primary, "owner", &primary_id)?;
        self.start_orchestra(ProcessRole::Standby, false)?;
        let initial_standby = self.wait_for_lease(ProcessRole::Standby, "standby", &primary_id)?;
        if initial_standby.fencing_token != initial_owner.fencing_token {
            return Err("standby did not observe the primary fencing token".to_string());
        }
        Ok((initial_owner, initial_standby))
    }

    fn run_partition(&mut self) -> RunnerResult<PartitionJourneyEvidence> {
        let standby_id = self.standby_id.clone();
        let (initial_owner, initial_standby) = self.initialize_cluster()?;
        let started = Instant::now();
        stop_child(
            &mut self.primary_tunnel,
            "partitioned primary PostgreSQL SSH tunnel",
        )?;
        wait_endpoint_closed(
            local_address(self.primary_tunnel_port),
            Duration::from_secs(5),
        )?;
        let partitioned_owner = self.wait_for_partitioned_owner()?;
        let partition_to_fail_closed_elapsed_ms = started.elapsed().as_millis();
        self.ensure_orchestra_alive(ProcessRole::Primary)?;
        let standby_tunnel_remained_open = TcpStream::connect_timeout(
            &local_address(self.standby_tunnel_port),
            Duration::from_millis(500),
        )
        .is_ok();
        if !standby_tunnel_remained_open {
            return Err("standby database tunnel failed during primary partition".to_string());
        }
        let takeover = self.wait_for_lease(ProcessRole::Standby, "owner", &standby_id)?;
        let takeover_elapsed_ms = started.elapsed().as_millis();
        if takeover.fencing_token <= initial_owner.fencing_token {
            return Err("partition takeover did not increment the fencing token".to_string());
        }

        self.start_tunnel(ProcessRole::Primary)?;
        let former_owner_rejoin =
            self.wait_for_lease(ProcessRole::Primary, "standby", &standby_id)?;
        if former_owner_rejoin.fencing_token != takeover.fencing_token {
            return Err("partitioned owner did not observe the new fencing token".to_string());
        }
        let stale_owner_submission_rejected = self.probe_stale_owner_submission()?;
        self.ensure_orchestra_alive(ProcessRole::Primary)?;

        Ok(PartitionJourneyEvidence {
            database_architecture: self.database_architecture.clone(),
            orchestra_platform: std::env::consts::OS.to_string(),
            initial_owner: self.phase("primary", initial_owner)?,
            initial_standby: self.phase("standby", initial_standby)?,
            partitioned_owner,
            takeover: self.phase("standby", takeover)?,
            former_owner_rejoin: self.phase("former-owner", former_owner_rejoin)?,
            partition_to_fail_closed_elapsed_ms,
            takeover_elapsed_ms,
            primary_process_survived: true,
            primary_endpoint_remained_open: true,
            isolated_tunnel_closed: true,
            standby_tunnel_remained_open,
            stale_owner_submission_rejected,
        })
    }

    fn compile_web(&self) -> RunnerResult<()> {
        let output = Command::new("mix")
            .arg("compile")
            .current_dir(self.root.join("apps/web"))
            .env("MIX_ENV", "dev")
            .output()
            .map_err(|error| format!("failed to compile Orchestra: {error}"))?;
        if output.status.success() {
            return Ok(());
        }
        let message = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        Err(format!(
            "Orchestra compilation failed: {}",
            tail(&message, 4_096)
        ))
    }

    fn prepare_remote_database(&mut self) -> RunnerResult<()> {
        self.database_architecture = ssh_output(&self.root, &self.host, "uname -m".to_string())?;
        let name = shell_escape(&self.container_name);
        let image = shell_escape(&self.postgres_image);
        let launch = format!(
            "set -eu; docker rm -f {name} >/dev/null 2>&1 || true; docker run --detach --rm --name {name} --label io.kyuubiki.qualification=orchestra-takeover --tmpfs /var/lib/postgresql/data:rw,noexec,nosuid,nodev,size=256m -e POSTGRES_HOST_AUTH_METHOD=trust -p 127.0.0.1::5432 {image} >/dev/null"
        );
        let status = ssh_status(&self.root, &self.host, launch)?;
        if status != 0 {
            return Err(format!(
                "failed to start remote PostgreSQL: status {status}"
            ));
        }
        self.remote_database_started = true;
        let ready = ssh_status(
            &self.root,
            &self.host,
            format!(
                "set -eu; count=0; until docker exec {name} pg_isready -U postgres -d postgres >/dev/null 2>&1; do count=$((count + 1)); test \"$count\" -lt 150; sleep 0.2; done"
            ),
        )?;
        if ready != 0 {
            return Err("remote PostgreSQL readiness timed out".to_string());
        }
        let mapping = ssh_output(
            &self.root,
            &self.host,
            format!("docker port {name} 5432/tcp"),
        )?;
        let port = mapping
            .rsplit(':')
            .next()
            .and_then(|value| value.trim().parse::<u16>().ok())
            .ok_or_else(|| "remote PostgreSQL returned an invalid port mapping".to_string())?;
        self.remote_database_port = Some(port);
        Ok(())
    }

    fn start_tunnels(&mut self) -> RunnerResult<()> {
        self.start_tunnel(ProcessRole::Primary)?;
        self.start_tunnel(ProcessRole::Standby)
    }

    fn start_tunnel(&mut self, role: ProcessRole) -> RunnerResult<()> {
        let remote_port = self
            .remote_database_port
            .ok_or("remote PostgreSQL port is unavailable")?;
        let (local_port, label) = match role {
            ProcessRole::Primary => (self.primary_tunnel_port, "primary"),
            ProcessRole::Standby => (self.standby_tunnel_port, "standby"),
        };
        if self.tunnel_mut(role).is_some() {
            return Err(format!("{label} PostgreSQL SSH tunnel is already running"));
        }
        let log = File::create(self.work_root.join(format!("ssh-tunnel-{label}.log")))
            .map_err(|error| format!("failed to create {label} tunnel log: {error}"))?;
        let stderr = log
            .try_clone()
            .map_err(|error| format!("failed to clone {label} tunnel log: {error}"))?;
        let forward = format!("127.0.0.1:{local_port}:127.0.0.1:{remote_port}");
        let child = Command::new("ssh")
            .args([
                "-N",
                "-T",
                "-o",
                "BatchMode=yes",
                "-o",
                "ExitOnForwardFailure=yes",
                "-o",
                "ConnectTimeout=10",
                "-o",
                "ServerAliveInterval=5",
                "-o",
                "ServerAliveCountMax=3",
                "-L",
                &forward,
                &self.host,
            ])
            .current_dir(&self.root)
            .stdout(Stdio::from(log))
            .stderr(Stdio::from(stderr))
            .spawn()
            .map_err(|error| format!("failed to start {label} PostgreSQL SSH tunnel: {error}"))?;
        *self.tunnel_mut(role) = Some(child);
        self.wait_tunnel(role)
    }

    fn wait_tunnel(&mut self, role: ProcessRole) -> RunnerResult<()> {
        let (port, label) = match role {
            ProcessRole::Primary => (self.primary_tunnel_port, "primary PostgreSQL SSH tunnel"),
            ProcessRole::Standby => (self.standby_tunnel_port, "standby PostgreSQL SSH tunnel"),
        };
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline {
            ensure_child_alive(self.tunnel_mut(role), label)?;
            if TcpStream::connect_timeout(&local_address(port), Duration::from_millis(250)).is_ok()
            {
                return Ok(());
            }
            thread::sleep(Duration::from_millis(100));
        }
        Err(format!("{label} readiness timed out"))
    }

    fn start_orchestra(&mut self, role: ProcessRole, rejoin: bool) -> RunnerResult<()> {
        let database_port = self.tunnel_port(role);
        let (port, instance_id, slot, label) = match role {
            ProcessRole::Primary => (
                self.primary_port,
                &self.primary_id,
                &mut self.primary,
                if rejoin { "former-owner" } else { "primary" },
            ),
            ProcessRole::Standby => (
                self.standby_port,
                &self.standby_id,
                &mut self.standby,
                "standby",
            ),
        };
        if slot.is_some() {
            return Err(format!("Orchestra {label} process is already running"));
        }
        let log = File::create(self.work_root.join(format!("orchestra-{label}.log")))
            .map_err(|error| format!("failed to create Orchestra {label} log: {error}"))?;
        let stderr = log
            .try_clone()
            .map_err(|error| format!("failed to clone Orchestra {label} log: {error}"))?;
        let database_url = format!("ecto://postgres@127.0.0.1:{database_port}/postgres");
        let child = Command::new("mix")
            .args(["run", "--no-halt", "--no-compile"])
            .current_dir(self.root.join("apps/web"))
            .env("MIX_ENV", "dev")
            .env("PORT", port.to_string())
            .env("KYUUBIKI_HTTP_BIND_IP", "127.0.0.1")
            .env("KYUUBIKI_DEPLOYMENT_MODE", "distributed")
            .env("KYUUBIKI_STORAGE_BACKEND", "postgres")
            .env("DATABASE_URL", database_url)
            .env("POOL_SIZE", "3")
            .env("KYUUBIKI_AGENT_DISCOVERY", "static")
            .env("KYUUBIKI_AGENT_ENDPOINTS", "")
            .env("KYUUBIKI_API_TOKEN", &self.token)
            .env("KYUUBIKI_CLUSTER_API_TOKEN", &self.token)
            .env("KYUUBIKI_PROTECT_READS", "true")
            .env("KYUUBIKI_ORCHESTRA_LEASE_NAME", &self.lease_name)
            .env("KYUUBIKI_ORCHESTRA_INSTANCE_ID", instance_id)
            .env("KYUUBIKI_ORCHESTRA_LEASE_TTL_MS", LEASE_TTL_MS.to_string())
            .env(
                "KYUUBIKI_ORCHESTRA_LEASE_HEARTBEAT_MS",
                HEARTBEAT_MS.to_string(),
            )
            .env("KYUUBIKI_ORCHESTRA_LEASE_RETRY_MS", RETRY_MS.to_string())
            .env("KYUUBIKI_ORCHESTRA_LEASE_QUERY_TIMEOUT_MS", "1000")
            .stdout(Stdio::from(log))
            .stderr(Stdio::from(stderr))
            .spawn()
            .map_err(|error| format!("failed to start Orchestra {label}: {error}"))?;
        *slot = Some(child);
        Ok(())
    }

    fn wait_for_lease(
        &mut self,
        role: ProcessRole,
        expected_status: &str,
        expected_owner: &str,
    ) -> RunnerResult<LeaseObservation> {
        let port = self.port(role);
        let deadline = Instant::now() + self.timeout;
        let mut last = "no health response".to_string();
        while Instant::now() < deadline {
            self.ensure_orchestra_alive(role)?;
            match http_get_json(port, "/api/v1/orchestra/lease", &self.token).and_then(parse_lease)
            {
                Ok(observation) => {
                    last = format!(
                        "status={} owner={} fencing={}",
                        observation.status,
                        owner_role(
                            &observation.owner_instance_id,
                            &self.primary_id,
                            &self.standby_id
                        ),
                        observation.fencing_token
                    );
                    if observation.status == expected_status
                        && observation.owner_instance_id == expected_owner
                    {
                        return Ok(observation);
                    }
                }
                Err(error) => last = error,
            }
            thread::sleep(Duration::from_millis(100));
        }
        Err(format!("Orchestra lease phase timed out ({last})"))
    }

    fn wait_for_partitioned_owner(&mut self) -> RunnerResult<PartitionedOwnerPhase> {
        let deadline = Instant::now() + self.timeout;
        let mut last = "no health response".to_string();
        while Instant::now() < deadline {
            self.ensure_orchestra_alive(ProcessRole::Primary)?;
            match http_get_json(self.primary_port, "/api/v1/orchestra/lease", &self.token) {
                Ok(value) => {
                    let lease = value
                        .pointer("/lease")
                        .ok_or("health response misses workflow recovery lease")?;
                    let status = lease
                        .get("status")
                        .and_then(Value::as_str)
                        .unwrap_or("unknown");
                    let owner = lease.get("owner_instance_id").and_then(Value::as_str);
                    let fencing_token = lease.get("fencing_token").and_then(Value::as_u64);
                    let last_error = lease.get("last_error").and_then(Value::as_str);
                    last = format!(
                        "status={status} owner={} fencing={fencing_token:?} error={last_error:?}",
                        owner.unwrap_or("none")
                    );
                    if status == "standby"
                        && owner.is_none()
                        && fencing_token.is_none()
                        && last_error == Some("orchestra_lease_store_unavailable")
                    {
                        return Ok(PartitionedOwnerPhase {
                            process_role: "partitioned-owner".to_string(),
                            lease_status: status.to_string(),
                            observed_owner_role: "none".to_string(),
                            visible_fencing_token: fencing_token,
                            last_error: last_error.unwrap_or_default().to_string(),
                        });
                    }
                }
                Err(error) => last = error,
            }
            thread::sleep(Duration::from_millis(100));
        }
        Err(format!(
            "partitioned Orchestra did not fail closed ({last})"
        ))
    }

    fn probe_stale_owner_submission(&self) -> RunnerResult<bool> {
        let request = stale_owner_write_probe();
        let response = http_post_json(
            self.primary_port,
            "/api/v1/workflows/graph/jobs",
            &self.token,
            &request,
        )?;
        if response.status != 422
            || response.body.get("error").and_then(Value::as_str) != Some(":orchestra_standby")
        {
            return Err(format!(
                "former owner accepted or misreported a fenced write: status={} body={}",
                response.status, response.body
            ));
        }
        Ok(true)
    }

    fn phase(&self, process_role: &str, observation: LeaseObservation) -> RunnerResult<LeasePhase> {
        let observed_owner_role = owner_role(
            &observation.owner_instance_id,
            &self.primary_id,
            &self.standby_id,
        );
        if observed_owner_role == "unknown" {
            return Err("lease exposed an unknown Orchestra owner identity".to_string());
        }
        Ok(LeasePhase {
            process_role: process_role.to_string(),
            lease_status: observation.status,
            observed_owner_role: observed_owner_role.to_string(),
            fencing_token: observation.fencing_token,
        })
    }

    fn ensure_orchestra_alive(&mut self, role: ProcessRole) -> RunnerResult<()> {
        match role {
            ProcessRole::Primary => ensure_child_alive(&mut self.primary, "primary Orchestra"),
            ProcessRole::Standby => ensure_child_alive(&mut self.standby, "standby Orchestra"),
        }
    }

    fn port(&self, role: ProcessRole) -> u16 {
        match role {
            ProcessRole::Primary => self.primary_port,
            ProcessRole::Standby => self.standby_port,
        }
    }

    fn tunnel_port(&self, role: ProcessRole) -> u16 {
        match role {
            ProcessRole::Primary => self.primary_tunnel_port,
            ProcessRole::Standby => self.standby_tunnel_port,
        }
    }

    fn tunnel_mut(&mut self, role: ProcessRole) -> &mut Option<Child> {
        match role {
            ProcessRole::Primary => &mut self.primary_tunnel,
            ProcessRole::Standby => &mut self.standby_tunnel,
        }
    }

    fn crash(&mut self, role: ProcessRole) -> RunnerResult<()> {
        let slot = match role {
            ProcessRole::Primary => &mut self.primary,
            ProcessRole::Standby => &mut self.standby,
        };
        stop_child(slot, "Orchestra crash target")
    }

    fn cleanup(&mut self) -> RunnerResult<CleanupEvidence> {
        if self.cleaned {
            return Err("takeover qualification session was already cleaned".to_string());
        }
        let mut errors = Vec::new();
        let orchestra_stop_results = [
            (&mut self.primary, "primary Orchestra"),
            (&mut self.standby, "standby Orchestra"),
        ]
        .map(|(process, label)| {
            stop_child(process, label)
                .map(|_| true)
                .unwrap_or_else(|error| {
                    errors.push(error);
                    false
                })
        });
        let orchestra_processes_stopped = orchestra_stop_results.into_iter().all(|stopped| stopped);
        let orchestra_ports_closed =
            [self.primary_port, self.standby_port]
                .into_iter()
                .all(|port| {
                    wait_endpoint_closed(local_address(port), Duration::from_secs(5))
                        .map(|_| true)
                        .unwrap_or_else(|error| {
                            errors.push(error);
                            false
                        })
                });
        let tunnel_stop_results = [
            (&mut self.primary_tunnel, "primary PostgreSQL SSH tunnel"),
            (&mut self.standby_tunnel, "standby PostgreSQL SSH tunnel"),
        ]
        .map(|(tunnel, label)| {
            stop_child(tunnel, label)
                .map(|_| true)
                .unwrap_or_else(|error| {
                    errors.push(error);
                    false
                })
        });
        let ssh_tunnel_stopped = tunnel_stop_results.into_iter().all(|stopped| stopped);
        let tunnel_port_results =
            [self.primary_tunnel_port, self.standby_tunnel_port].map(|port| {
                wait_endpoint_closed(local_address(port), Duration::from_secs(5))
                    .map(|_| true)
                    .unwrap_or_else(|error| {
                        errors.push(error);
                        false
                    })
            });
        let tunnel_port_closed = tunnel_port_results.into_iter().all(|closed| closed);
        let remote_database_removed = self.remove_remote_database().unwrap_or_else(|error| {
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
            orchestra_processes_stopped,
            orchestra_ports_closed,
            ssh_tunnel_stopped,
            tunnel_port_closed,
            remote_database_removed,
            local_work_root_removed,
        })
    }

    fn remove_remote_database(&mut self) -> RunnerResult<bool> {
        if !self.remote_database_started {
            return Ok(true);
        }
        let name = shell_escape(&self.container_name);
        let status = ssh_status(
            &self.root,
            &self.host,
            format!("set -eu; docker rm -f {name} >/dev/null"),
        )?;
        if status != 0 {
            return Err(format!(
                "failed to remove remote PostgreSQL container: status {status}"
            ));
        }
        let absent = ssh_success_quiet(
            &self.root,
            &self.host,
            format!("set -eu; ! docker inspect {name} >/dev/null 2>&1"),
        )?;
        if absent {
            self.remote_database_started = false;
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

pub(super) fn distinct_port(existing: &[u16]) -> RunnerResult<u16> {
    for _ in 0..16 {
        let port = available_local_port()?;
        if !existing.contains(&port) {
            return Ok(port);
        }
    }
    Err("failed to allocate distinct local qualification ports".to_string())
}

pub(super) fn random_token() -> RunnerResult<String> {
    let mut bytes = [0_u8; 32];
    fill_random(&mut bytes).map_err(|error| format!("failed to generate API token: {error}"))?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

pub(super) fn local_address(port: u16) -> SocketAddr {
    SocketAddr::from(([127, 0, 0, 1], port))
}

pub(super) fn ensure_child_alive(child: &mut Option<Child>, label: &str) -> RunnerResult<()> {
    let process = child
        .as_mut()
        .ok_or_else(|| format!("{label} process is unavailable"))?;
    if let Some(status) = process
        .try_wait()
        .map_err(|error| format!("failed to inspect {label}: {error}"))?
    {
        return Err(format!("{label} exited unexpectedly: {status}"));
    }
    Ok(())
}

pub(super) fn stop_child(child: &mut Option<Child>, label: &str) -> RunnerResult<()> {
    let Some(mut process) = child.take() else {
        return Ok(());
    };
    if process
        .try_wait()
        .map_err(|error| format!("failed to inspect {label}: {error}"))?
        .is_none()
    {
        process
            .kill()
            .map_err(|error| format!("failed to stop {label}: {error}"))?;
    }
    process
        .wait()
        .map_err(|error| format!("failed to reap {label}: {error}"))?;
    Ok(())
}

pub(super) fn parse_lease(value: Value) -> RunnerResult<LeaseObservation> {
    let lease = value
        .pointer("/workflow_recovery/lease")
        .or_else(|| value.pointer("/lease"))
        .ok_or("health response misses workflow recovery lease")?;
    let status = lease
        .get("status")
        .and_then(Value::as_str)
        .ok_or("health response misses lease status")?;
    let owner_instance_id = lease
        .get("owner_instance_id")
        .and_then(Value::as_str)
        .ok_or("health response misses lease owner")?;
    let fencing_token = lease
        .get("fencing_token")
        .and_then(Value::as_u64)
        .ok_or("health response misses fencing token")?;
    Ok(LeaseObservation {
        status: status.to_string(),
        owner_instance_id: owner_instance_id.to_string(),
        fencing_token,
    })
}

pub(super) fn owner_role(owner: &str, primary: &str, standby: &str) -> &'static str {
    if owner == primary {
        "primary"
    } else if owner == standby {
        "standby"
    } else {
        "unknown"
    }
}

fn tail(value: &str, max_bytes: usize) -> &str {
    if value.len() <= max_bytes {
        return value;
    }
    let mut start = value.len() - max_bytes;
    while !value.is_char_boundary(start) {
        start += 1;
    }
    &value[start..]
}
