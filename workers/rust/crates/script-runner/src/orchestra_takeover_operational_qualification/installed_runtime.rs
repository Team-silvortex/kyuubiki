use super::installed_report::{CleanupEvidence, InstallationEvidence, JourneyEvidence};
use super::report::LeasePhase;
use super::runtime::{
    HEARTBEAT_MS, LEASE_TTL_MS, LeaseObservation, RETRY_MS, distinct_port, ensure_child_alive,
    http_get_json, local_address, owner_role, parse_lease, random_token, stop_child,
};
use crate::operational_agent_support::{available_local_port, wait_endpoint_closed};
use crate::qualification_support::generated_at_unix_ms;
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::path::{Component, Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

type RunnerResult<T> = Result<T, String>;

#[derive(Clone, Copy)]
enum ProcessRole {
    Primary,
    Standby,
}

#[derive(Deserialize)]
struct ServiceManifest {
    schema_version: String,
    owner: String,
    policy: ServicePolicy,
    services: Vec<ServiceEntry>,
}

#[derive(Deserialize)]
struct ServicePolicy {
    source_fallback: bool,
    relative_paths_only: bool,
}

#[derive(Deserialize)]
struct ServiceEntry {
    id: String,
    command: String,
    #[serde(default)]
    args: Vec<String>,
    cwd: String,
}

#[derive(Deserialize)]
struct ActivationRecord {
    schema_version: String,
    generation: u64,
    version: String,
    relative_path: String,
    platform: String,
}

struct InstalledLaunch {
    command: PathBuf,
    args: Vec<String>,
    cwd: PathBuf,
    evidence: InstallationEvidence,
}

pub(crate) fn capture(
    managed_root: &Path,
    runtime_root: &Path,
    detached_source_root: &Path,
    package_version: &str,
    postgres_image: &str,
    timeout: Duration,
) -> RunnerResult<(JourneyEvidence, CleanupEvidence)> {
    let launch = inspect_installation(
        managed_root,
        runtime_root,
        detached_source_root,
        package_version,
    )?;
    let mut session = Session::new(managed_root, launch, postgres_image, timeout)?;
    let journey = session.run();
    let cleanup = session.cleanup();
    match (journey, cleanup) {
        (Ok(journey), Ok(cleanup)) => Ok((journey, cleanup)),
        (Err(error), Ok(_)) => Err(error),
        (Ok(_), Err(cleanup_error)) => Err(cleanup_error),
        (Err(error), Err(cleanup_error)) => Err(format!("{error}; cleanup: {cleanup_error}")),
    }
}

struct Session {
    managed_root: PathBuf,
    runtime_store: PathBuf,
    state_root: PathBuf,
    launch: InstalledLaunch,
    postgres_image: String,
    timeout: Duration,
    container_name: String,
    lease_name: String,
    primary_id: String,
    standby_id: String,
    token: String,
    release_cookie: String,
    run_id: String,
    database_port: Option<u16>,
    primary_port: u16,
    standby_port: u16,
    primary: Option<Child>,
    standby: Option<Child>,
    database_started: bool,
    cleaned: bool,
}

impl Session {
    fn new(
        managed_root: &Path,
        launch: InstalledLaunch,
        postgres_image: &str,
        timeout: Duration,
    ) -> RunnerResult<Self> {
        if std::env::consts::OS != "linux" {
            return Err("installed takeover host capture requires Linux".to_string());
        }
        let nonce = generated_at_unix_ms()?;
        let suffix = format!("{nonce}-{}", std::process::id());
        let state_root = managed_root.join("runtime-state");
        fs::create_dir_all(&state_root)
            .map_err(|error| format!("failed to create installed Runtime state: {error}"))?;
        let primary_port = available_local_port()?;
        let standby_port = distinct_port(&[primary_port])?;
        let runtime_store = launch
            .command
            .ancestors()
            .find(|path| path.file_name().and_then(|name| name.to_str()) == Some("versions"))
            .and_then(Path::parent)
            .ok_or_else(|| "installed Orchestra command is outside a Runtime store".to_string())?
            .to_path_buf();
        Ok(Self {
            managed_root: managed_root.to_path_buf(),
            runtime_store,
            state_root,
            launch,
            postgres_image: postgres_image.to_string(),
            timeout,
            container_name: format!("kyuubiki-installed-takeover-{suffix}"),
            lease_name: format!("installed-takeover-{suffix}"),
            primary_id: format!("installed-primary-{suffix}"),
            standby_id: format!("installed-standby-{suffix}"),
            token: random_token()?,
            release_cookie: random_token()?,
            run_id: std::env::var("KYUUBIKI_QUALIFICATION_RUN_ID")
                .unwrap_or_else(|_| suffix.clone()),
            database_port: None,
            primary_port,
            standby_port,
            primary: None,
            standby: None,
            database_started: false,
            cleaned: false,
        })
    }

    fn run(&mut self) -> RunnerResult<JourneyEvidence> {
        let primary_id = self.primary_id.clone();
        let standby_id = self.standby_id.clone();
        self.start_database()?;
        self.start_orchestra(ProcessRole::Primary, false)?;
        let initial_owner = self.wait_for_lease(ProcessRole::Primary, "owner", &primary_id)?;
        self.start_orchestra(ProcessRole::Standby, false)?;
        let initial_standby = self.wait_for_lease(ProcessRole::Standby, "standby", &primary_id)?;
        if initial_standby.fencing_token != initial_owner.fencing_token {
            return Err("installed standby did not observe the primary fencing token".to_string());
        }

        self.crash(ProcessRole::Primary)?;
        wait_endpoint_closed(local_address(self.primary_port), self.timeout)?;
        let started = Instant::now();
        let takeover = self.wait_for_lease(ProcessRole::Standby, "owner", &standby_id)?;
        let takeover_elapsed_ms = started.elapsed().as_millis();
        if takeover.fencing_token <= initial_owner.fencing_token {
            return Err("installed standby takeover did not increment fencing".to_string());
        }

        self.start_orchestra(ProcessRole::Primary, true)?;
        let former_owner_rejoin =
            self.wait_for_lease(ProcessRole::Primary, "standby", &standby_id)?;
        if former_owner_rejoin.fencing_token != takeover.fencing_token {
            return Err("installed former owner did not observe the new fencing token".to_string());
        }

        let evidence = std::mem::replace(
            &mut self.launch.evidence,
            InstallationEvidence {
                package_version: String::new(),
                architecture: String::new(),
                activation_generation: 0,
                payload_manifest_sha256: String::new(),
                service_manifest_sha256: String::new(),
                orchestra_executable_sha256: String::new(),
                source_tree_detached: false,
            },
        );
        Ok(JourneyEvidence {
            installation: evidence,
            initial_owner: self.phase("primary", initial_owner)?,
            initial_standby: self.phase("standby", initial_standby)?,
            takeover: self.phase("standby", takeover)?,
            former_owner_rejoin: self.phase("former-owner", former_owner_rejoin)?,
            takeover_elapsed_ms,
            primary_endpoint_closed: true,
        })
    }

    fn start_database(&mut self) -> RunnerResult<()> {
        let _ = Command::new("docker")
            .args(["rm", "-f", &self.container_name])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        let run_label = format!("io.kyuubiki.run={}", self.run_id);
        let status = Command::new("docker")
            .args([
                "run",
                "--detach",
                "--rm",
                "--name",
                &self.container_name,
                "--label",
                "io.kyuubiki.qualification=orchestra-installed-takeover",
                "--label",
                &run_label,
                "--tmpfs",
                "/var/lib/postgresql/data:rw,noexec,nosuid,nodev,size=256m",
                "-e",
                "POSTGRES_HOST_AUTH_METHOD=trust",
                "-p",
                "127.0.0.1::5432",
                &self.postgres_image,
            ])
            .stdout(Stdio::null())
            .status()
            .map_err(|error| format!("failed to start qualification PostgreSQL: {error}"))?;
        if !status.success() {
            return Err(format!(
                "qualification PostgreSQL failed to start: {status}"
            ));
        }
        self.database_started = true;
        let deadline = Instant::now() + Duration::from_secs(30);
        while Instant::now() < deadline {
            let ready = Command::new("docker")
                .args([
                    "exec",
                    &self.container_name,
                    "pg_isready",
                    "-U",
                    "postgres",
                    "-d",
                    "postgres",
                ])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .is_ok_and(|status| status.success());
            if ready {
                break;
            }
            thread::sleep(Duration::from_millis(200));
        }
        let output = Command::new("docker")
            .args(["port", &self.container_name, "5432/tcp"])
            .output()
            .map_err(|error| format!("failed to inspect qualification PostgreSQL: {error}"))?;
        if !output.status.success() {
            return Err("qualification PostgreSQL did not become ready".to_string());
        }
        let mapping = String::from_utf8_lossy(&output.stdout);
        self.database_port = Some(
            mapping
                .rsplit(':')
                .next()
                .and_then(|value| value.trim().parse().ok())
                .ok_or_else(|| "qualification PostgreSQL returned an invalid port".to_string())?,
        );
        Ok(())
    }

    fn start_orchestra(&mut self, role: ProcessRole, rejoin: bool) -> RunnerResult<()> {
        let (port, instance_id, slot, state_name, process_name) = match role {
            ProcessRole::Primary => (
                self.primary_port,
                &self.primary_id,
                &mut self.primary,
                "primary",
                if rejoin { "former-owner" } else { "primary" },
            ),
            ProcessRole::Standby => (
                self.standby_port,
                &self.standby_id,
                &mut self.standby,
                "standby",
                "standby",
            ),
        };
        if slot.is_some() {
            return Err(format!(
                "installed Orchestra {process_name} is already running"
            ));
        }
        let state = self.state_root.join(state_name);
        if rejoin && state.exists() {
            fs::remove_dir_all(&state)
                .map_err(|error| format!("failed to reset former-owner state: {error}"))?;
        }
        fs::create_dir_all(state.join("home"))
            .map_err(|error| format!("failed to create installed Orchestra state: {error}"))?;
        let log = File::create(self.state_root.join(format!("{process_name}.log")))
            .map_err(|error| format!("failed to create installed Orchestra log: {error}"))?;
        let stderr = log
            .try_clone()
            .map_err(|error| format!("failed to clone installed Orchestra log: {error}"))?;
        let database_port = self.database_port.ok_or("PostgreSQL port is unavailable")?;
        let release_node = format!("kyuubiki_{}_{}@127.0.0.1", state_name, std::process::id());
        let child = Command::new(&self.launch.command)
            .args(&self.launch.args)
            .current_dir(&self.launch.cwd)
            .env("HOME", state.join("home"))
            .env("RELEASE_TMP", state.join("release-tmp"))
            .env("RELEASE_NODE", release_node)
            .env("RELEASE_COOKIE", &self.release_cookie)
            .env("RELEASE_DISTRIBUTION", "none")
            .env("ERL_CRASH_DUMP", state.join("erl_crash.dump"))
            .env("PORT", port.to_string())
            .env("KYUUBIKI_HTTP_BIND_IP", "127.0.0.1")
            .env("KYUUBIKI_DEPLOYMENT_MODE", "distributed")
            .env("KYUUBIKI_STORAGE_BACKEND", "postgres")
            .env(
                "DATABASE_URL",
                format!("ecto://postgres@127.0.0.1:{database_port}/postgres"),
            )
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
            .map_err(|error| format!("failed to start installed Orchestra: {error}"))?;
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
            match http_get_json(port, "/api/health", &self.token).and_then(parse_lease) {
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
        Err(format!(
            "installed Orchestra lease phase timed out ({last})"
        ))
    }

    fn phase(&self, process_role: &str, observation: LeaseObservation) -> RunnerResult<LeasePhase> {
        let observed_owner_role = owner_role(
            &observation.owner_instance_id,
            &self.primary_id,
            &self.standby_id,
        );
        if observed_owner_role == "unknown" {
            return Err("installed lease exposed an unknown owner identity".to_string());
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
            ProcessRole::Primary => ensure_child_alive(&mut self.primary, "installed primary"),
            ProcessRole::Standby => ensure_child_alive(&mut self.standby, "installed standby"),
        }
    }

    fn port(&self, role: ProcessRole) -> u16 {
        match role {
            ProcessRole::Primary => self.primary_port,
            ProcessRole::Standby => self.standby_port,
        }
    }

    fn crash(&mut self, role: ProcessRole) -> RunnerResult<()> {
        match role {
            ProcessRole::Primary => stop_child(&mut self.primary, "installed primary crash target"),
            ProcessRole::Standby => stop_child(&mut self.standby, "installed standby crash target"),
        }
    }

    fn cleanup(&mut self) -> RunnerResult<CleanupEvidence> {
        if self.cleaned {
            return Err("installed takeover session was already cleaned".to_string());
        }
        let mut errors = Vec::new();
        let orchestra_processes_stopped = stop_child(&mut self.primary, "installed primary")
            .and_then(|_| stop_child(&mut self.standby, "installed standby"))
            .map(|_| true)
            .unwrap_or_else(|error| {
                errors.push(error);
                false
            });
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
        let remote_database_removed = self.remove_database().unwrap_or_else(|error| {
            errors.push(error);
            false
        });
        let remove_result = remove_managed_root(&self.managed_root);
        let runtime_store_removed = remove_result.is_ok() && !self.runtime_store.exists();
        let managed_run_root_removed = remove_result.is_ok() && !self.managed_root.exists();
        if let Err(error) = remove_result {
            errors.push(error);
        }
        if !errors.is_empty() {
            return Err(errors.join("; "));
        }
        self.cleaned = true;
        Ok(CleanupEvidence {
            orchestra_processes_stopped,
            orchestra_ports_closed,
            remote_database_removed,
            runtime_store_removed,
            managed_run_root_removed,
        })
    }

    fn remove_database(&mut self) -> RunnerResult<bool> {
        if !self.database_started {
            return Ok(true);
        }
        let status = Command::new("docker")
            .args(["rm", "-f", &self.container_name])
            .stdout(Stdio::null())
            .status()
            .map_err(|error| format!("failed to remove qualification PostgreSQL: {error}"))?;
        if !status.success() {
            return Err(format!(
                "failed to remove qualification PostgreSQL: {status}"
            ));
        }
        self.database_started = false;
        Ok(true)
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        if !self.cleaned {
            let _ = self.cleanup();
        }
    }
}

fn inspect_installation(
    managed_root: &Path,
    runtime_root: &Path,
    detached_source_root: &Path,
    package_version: &str,
) -> RunnerResult<InstalledLaunch> {
    if detached_source_root.exists() {
        return Err("installed takeover requires the Orchestra source tree to be detached".into());
    }
    let managed = canonical_directory(managed_root, "managed qualification root")?;
    let runtime = canonical_directory(runtime_root, "installed Runtime root")?;
    if !runtime.starts_with(&managed) {
        return Err("installed Runtime escapes the managed qualification root".into());
    }
    let payload_path = runtime.join("manifests/runtime-payload.json");
    let payload: Value = read_json_file(&payload_path)?;
    if payload.pointer("/schema_version").and_then(Value::as_str)
        != Some("kyuubiki.runtime-payload/v1")
        || payload.pointer("/version").and_then(Value::as_str) != Some(package_version)
        || payload.pointer("/platform").and_then(Value::as_str) != Some("linux")
    {
        return Err("installed Runtime payload identity is invalid".into());
    }
    let service_path = runtime.join("manifests/service-launch.json");
    let service: ServiceManifest = read_json_file(&service_path)?;
    if service.schema_version != "kyuubiki.service-launch/v1"
        || service.owner != "installer"
        || service.policy.source_fallback
        || !service.policy.relative_paths_only
    {
        return Err("installed Runtime service policy is invalid".into());
    }
    let entry = service
        .services
        .into_iter()
        .find(|entry| entry.id == "orchestrator")
        .ok_or("installed Runtime has no Orchestra service")?;
    if entry.args != ["start"] {
        return Err("installed Orchestra must use the production release start command".into());
    }
    let command = checked_installed_path(&runtime, &entry.command, "Orchestra command")?;
    let cwd = checked_installed_path(&runtime, &entry.cwd, "Orchestra cwd")?;
    if !command.is_file() || !cwd.is_dir() {
        return Err("installed Orchestra launch files are incomplete".into());
    }
    if !cwd.join("releases/start_erl.data").is_file()
        || !cwd
            .join(format!("releases/{package_version}/kyuubiki_web.rel"))
            .is_file()
    {
        return Err("installed Orchestra is not a production OTP release".into());
    }
    let activation = latest_activation(&runtime, package_version)?;
    let command_digest = sha256_file(&command)?;
    let recorded_digest = payload
        .pointer("/files")
        .and_then(Value::as_array)
        .and_then(|files| {
            files.iter().find_map(|file| {
                (file.get("path").and_then(Value::as_str) == Some(entry.command.as_str()))
                    .then(|| file.get("sha256").and_then(Value::as_str))
                    .flatten()
            })
        })
        .ok_or("Runtime payload does not digest the Orchestra executable")?;
    if recorded_digest != command_digest {
        return Err("installed Orchestra executable digest drifted from its payload".into());
    }
    Ok(InstalledLaunch {
        command,
        args: entry.args,
        cwd,
        evidence: InstallationEvidence {
            package_version: package_version.to_string(),
            architecture: std::env::consts::ARCH.to_string(),
            activation_generation: activation.generation,
            payload_manifest_sha256: sha256_file(&payload_path)?,
            service_manifest_sha256: sha256_file(&service_path)?,
            orchestra_executable_sha256: command_digest,
            source_tree_detached: true,
        },
    })
}

fn latest_activation(runtime_root: &Path, version: &str) -> RunnerResult<ActivationRecord> {
    let store = runtime_root
        .parent()
        .and_then(Path::parent)
        .ok_or("installed Runtime store layout is invalid")?;
    let mut records = fs::read_dir(store.join("activations"))
        .map_err(|error| format!("failed to read Runtime activations: {error}"))?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().extension().and_then(|value| value.to_str()) == Some("json"))
        .map(|entry| read_json_file::<ActivationRecord>(&entry.path()))
        .collect::<Result<Vec<_>, String>>()?;
    records.sort_by_key(|record| record.generation);
    let record = records
        .pop()
        .ok_or("Installer produced no Runtime activation")?;
    if record.schema_version != "kyuubiki.runtime-activation/v1"
        || record.version != version
        || record.relative_path != format!("versions/{version}")
        || record.platform != "linux"
    {
        return Err("installed Runtime activation record is invalid".into());
    }
    Ok(record)
}

fn checked_installed_path(root: &Path, relative: &str, label: &str) -> RunnerResult<PathBuf> {
    let path = Path::new(relative);
    if path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(format!("{label} escapes the installed Runtime: {relative}"));
    }
    let joined = root.join(path);
    if joined.exists() {
        let canonical = joined
            .canonicalize()
            .map_err(|error| format!("failed to resolve {label}: {error}"))?;
        if !canonical.starts_with(root) {
            return Err(format!("{label} resolves outside the installed Runtime"));
        }
        Ok(canonical)
    } else {
        Ok(joined)
    }
}

fn canonical_directory(path: &Path, label: &str) -> RunnerResult<PathBuf> {
    if path
        .symlink_metadata()
        .is_ok_and(|metadata| metadata.file_type().is_symlink())
        || !path.is_dir()
    {
        return Err(format!("{label} must be a real directory"));
    }
    path.canonicalize()
        .map_err(|error| format!("failed to resolve {label}: {error}"))
}

fn read_json_file<T: serde::de::DeserializeOwned>(path: &Path) -> RunnerResult<T> {
    let bytes =
        fs::read(path).map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))
}

fn sha256_file(path: &Path) -> RunnerResult<String> {
    let bytes =
        fs::read(path).map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn remove_managed_root(path: &Path) -> RunnerResult<()> {
    let rendered = path.to_string_lossy();
    if !rendered.contains("/.kyuubiki/lab-runs/orchestra-installed-takeover-") {
        return Err("refusing to remove an unmanaged installed takeover root".to_string());
    }
    if path.exists() {
        fs::remove_dir_all(path)
            .map_err(|error| format!("failed to remove managed takeover root: {error}"))?;
    }
    Ok(())
}
