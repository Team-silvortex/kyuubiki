use crate::linux_host_power_loss_validation::{
    HostPowerLossCleanup, HostPowerLossIntent, HostPowerLossPreparation,
    HostPowerLossQualificationReport, INTENT_SCHEMA, JOURNEY, QUALIFICATION_ID, REPORT_SCHEMA,
    RebootRecoveryObservation, SentinelObservation, digest_serializable, qualification_checks,
    validate_contract, validate_intent, validate_report,
};
use kyuubiki_installer::{
    Platform, active_agent_binary_in, install_agent_update_package_into,
    prepare_agent_update_package, run_agent_solver_operational_qualification,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

type RunnerResult<T> = Result<T, String>;

mod state;
use state::{ManagedStateRoot, ensure_state_root_scope, remove_state_root_durable};

const DEFAULT_STATE_ROOT: &str = "tmp/linux-host-power-loss-state";
const DEFAULT_REPORT: &str =
    "releases/usability-evidence/2.19.0/linux-host-power-loss-operational-qualification.json";
const INTENT_FILE: &str = "intent.json";

pub(crate) fn run_qualify(root: &Path, args: Vec<OsString>) -> RunnerResult<u8> {
    validate_contract(root)?;
    let mut args = args.into_iter();
    let action = args
        .next()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_else(|| "help".to_string());
    match action.as_str() {
        "prepare" => prepare(root, PrepareOptions::parse(root, args)?),
        "resume" => resume(root, ResumeOptions::parse(root, args)?),
        "cleanup" => cleanup(root, CleanupOptions::parse(root, args)?),
        "help" | "--help" | "-h" => {
            print_qualify_usage();
            Ok(0)
        }
        other => Err(format!("unknown Linux host power-loss action: {other}")),
    }
}

pub(crate) fn run_check(root: &Path, args: Vec<OsString>) -> RunnerResult<u8> {
    validate_contract(root)?;
    let mut self_test = false;
    let mut report = root.join(DEFAULT_REPORT);
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.to_string_lossy().as_ref() {
            "--self-test" => self_test = true,
            "--verify-report" | "--in" => {
                report = resolve(root, next_path(&mut args, "--verify-report")?);
            }
            "--help" | "-h" => {
                print_check_usage();
                return Ok(0);
            }
            other => return Err(format!("unknown host power-loss check option: {other}")),
        }
    }
    if self_test {
        validator_self_test(root)?;
        println!("Linux host power-loss qualification self-test passed");
        return Ok(0);
    }
    let report: HostPowerLossQualificationReport = read_json(&report)?;
    let summary = validate_report(&report)?;
    println!(
        "Linux host power-loss qualification passed: runtime {}, {}, {} checks",
        summary.runtime_version, summary.architecture, summary.check_count
    );
    Ok(0)
}

struct PrepareOptions {
    state_root: PathBuf,
    agent_binary: PathBuf,
    package_version: String,
}

impl PrepareOptions {
    fn parse(root: &Path, args: impl Iterator<Item = OsString>) -> RunnerResult<Self> {
        let mut options = Self {
            state_root: root.join(DEFAULT_STATE_ROOT),
            agent_binary: root.join("workers/rust/target/release/kyuubiki-cli"),
            package_version: development_version(root)?,
        };
        let mut args = args;
        while let Some(arg) = args.next() {
            match arg.to_string_lossy().as_ref() {
                "--state-root" => {
                    options.state_root = resolve(root, next_path(&mut args, "--state-root")?);
                }
                "--agent-binary" => {
                    options.agent_binary = resolve(root, next_path(&mut args, "--agent-binary")?);
                }
                "--package-version" => {
                    options.package_version = next_string(&mut args, "--package-version")?;
                }
                other => return Err(format!("unknown host power-loss prepare option: {other}")),
            }
        }
        Ok(options)
    }
}

struct ResumeOptions {
    state_root: PathBuf,
    output: PathBuf,
}

impl ResumeOptions {
    fn parse(root: &Path, args: impl Iterator<Item = OsString>) -> RunnerResult<Self> {
        let mut options = Self {
            state_root: root.join(DEFAULT_STATE_ROOT),
            output: root.join(DEFAULT_REPORT),
        };
        let mut args = args;
        while let Some(arg) = args.next() {
            match arg.to_string_lossy().as_ref() {
                "--state-root" => {
                    options.state_root = resolve(root, next_path(&mut args, "--state-root")?);
                }
                "--out" => options.output = resolve(root, next_path(&mut args, "--out")?),
                other => return Err(format!("unknown host power-loss resume option: {other}")),
            }
        }
        if options.output.starts_with(&options.state_root) {
            return Err("host power-loss report must be outside the removable state root".into());
        }
        Ok(options)
    }
}

struct CleanupOptions {
    state_root: PathBuf,
}

impl CleanupOptions {
    fn parse(root: &Path, args: impl Iterator<Item = OsString>) -> RunnerResult<Self> {
        let mut state_root = root.join(DEFAULT_STATE_ROOT);
        let mut args = args;
        while let Some(arg) = args.next() {
            match arg.to_string_lossy().as_ref() {
                "--state-root" => state_root = resolve(root, next_path(&mut args, "--state-root")?),
                other => return Err(format!("unknown host power-loss cleanup option: {other}")),
            }
        }
        Ok(Self { state_root })
    }
}

fn prepare(root: &Path, options: PrepareOptions) -> RunnerResult<u8> {
    ensure_remote_linux()?;
    let mut state_root = ManagedStateRoot::create(root, &options.state_root)?;
    let preflight = run_agent_solver_operational_qualification(
        &options.agent_binary,
        &options.state_root.join("preflight"),
        &options.package_version,
    )?;
    let package = prepare_agent_update_package(
        &options.agent_binary,
        &options.state_root.join("packages/agent"),
        &options.package_version,
        Platform::Linux,
    )?;
    let activation = install_agent_update_package_into(
        &options.state_root.join("packages/agent"),
        &options.state_root.join("managed-store"),
        Platform::Linux,
    )?;
    let active =
        active_agent_binary_in(&options.state_root.join("managed-store"), Platform::Linux)?;
    let active_digest = sha256_file(&active)?;
    let sentinel = SentinelProcess::start(&active, &options.state_root.join("logs/sentinel.log"))?;
    let payload = HostPowerLossPreparation {
        qualification_id: QUALIFICATION_ID.to_string(),
        execution_host_role: "remote-linux-qualification-host".to_string(),
        platform: "linux".to_string(),
        architecture: std::env::consts::ARCH.to_string(),
        runtime_version: options.package_version,
        prepared_at_unix_ms: unix_now_ms()?,
        machine_id_sha256: identity_digest(Path::new("/etc/machine-id"))?,
        pre_boot_id_sha256: identity_digest(Path::new("/proc/sys/kernel/random/boot_id"))?,
        pre_uptime_seconds: uptime_seconds()?,
        package,
        activation,
        active_entrypoint_sha256: active_digest.clone(),
        sentinel: SentinelObservation {
            process_id: sentinel.process_id(),
            port: sentinel.port,
            executable_sha256: active_digest,
            ready_before_reboot: true,
        },
        preflight,
    };
    let intent = HostPowerLossIntent {
        schema_version: INTENT_SCHEMA.to_string(),
        intent_sha256: digest_serializable(&payload)?,
        payload,
    };
    validate_intent(&intent)?;
    write_durable_json(&options.state_root.join(INTENT_FILE), &intent)?;
    state_root.retain();
    let (pid, port) = sentinel.detach();
    println!(
        "Linux host power-loss qualification prepared: runtime {}, sentinel pid {}, port {}; reboot the physical host before resume",
        intent.payload.runtime_version, pid, port
    );
    println!("state root: {}", display(root, &options.state_root));
    Ok(0)
}

fn resume(root: &Path, options: ResumeOptions) -> RunnerResult<u8> {
    ensure_remote_linux()?;
    ensure_state_root_scope(root, &options.state_root)?;
    if options.output.exists() {
        return Err(format!(
            "host power-loss report already exists: {}",
            options.output.display()
        ));
    }
    let intent: HostPowerLossIntent = read_json(&options.state_root.join(INTENT_FILE))?;
    validate_intent(&intent)?;
    let post_boot_id = identity_digest(Path::new("/proc/sys/kernel/random/boot_id"))?;
    let post_machine_id = identity_digest(Path::new("/etc/machine-id"))?;
    if post_boot_id == intent.payload.pre_boot_id_sha256 {
        return Err(
            "physical host boot identity did not change; reboot is required before resume".into(),
        );
    }
    if post_machine_id != intent.payload.machine_id_sha256 {
        return Err("host machine identity changed across the qualification boundary".into());
    }
    if port_listening(intent.payload.sentinel.port) {
        return Err("pre-reboot sentinel port is still occupied after reboot".into());
    }
    let active =
        active_agent_binary_in(&options.state_root.join("managed-store"), Platform::Linux)?;
    let active_digest = sha256_file(&active)?;
    if active_digest != intent.payload.active_entrypoint_sha256 {
        return Err("Installer-managed Agent payload changed across reboot".into());
    }
    let postflight = run_agent_solver_operational_qualification(
        &active,
        &options.state_root.join("postflight"),
        &intent.payload.runtime_version,
    )?;
    let mut report = HostPowerLossQualificationReport {
        schema_version: REPORT_SCHEMA.to_string(),
        qualification_id: QUALIFICATION_ID.to_string(),
        status: "pass".to_string(),
        journey: JOURNEY.to_string(),
        execution_host_role: intent.payload.execution_host_role.clone(),
        platform: intent.payload.platform.clone(),
        architecture: intent.payload.architecture.clone(),
        runtime_version: intent.payload.runtime_version.clone(),
        generated_at_unix_ms: unix_now_ms()?,
        preparation: intent.payload,
        intent_sha256: intent.intent_sha256,
        recovery: RebootRecoveryObservation {
            post_boot_id_sha256: post_boot_id,
            post_machine_id_sha256: post_machine_id,
            post_uptime_seconds: uptime_seconds()?,
            boot_identity_changed: true,
            same_machine: true,
            sentinel_port_free_before_resume: true,
            active_entrypoint_sha256: active_digest,
        },
        postflight,
        cleanup: HostPowerLossCleanup {
            scope: "qualification-state-root".to_string(),
            state_root_removed: true,
            sentinel_port_released: true,
            residue_count: 0,
        },
        checks: Vec::new(),
    };
    report.checks = qualification_checks(&report);
    validate_report(&report)?;
    let staged = stage_durable_json(&options.output, &report)?;
    remove_state_root_durable(&options.state_root).map_err(|error| {
        let _ = fs::remove_file(&staged);
        format!("failed to clean qualification state root: {error}")
    })?;
    if options.state_root.exists() || port_listening(report.preparation.sentinel.port) {
        let _ = fs::remove_file(&staged);
        return Err("host power-loss qualification cleanup left residue".into());
    }
    promote_staged_file(&staged, &options.output)?;
    println!(
        "Linux host power-loss qualification completed: {} checks, {}",
        report.checks.len(),
        display(root, &options.output)
    );
    Ok(0)
}

fn cleanup(root: &Path, options: CleanupOptions) -> RunnerResult<u8> {
    ensure_remote_linux()?;
    ensure_state_root_scope(root, &options.state_root)?;
    let intent: HostPowerLossIntent = read_json(&options.state_root.join(INTENT_FILE))?;
    validate_intent(&intent)?;
    let current_boot = identity_digest(Path::new("/proc/sys/kernel/random/boot_id"))?;
    if current_boot == intent.payload.pre_boot_id_sha256
        && port_listening(intent.payload.sentinel.port)
    {
        terminate_sentinel(&intent.payload.sentinel)?;
    }
    if port_listening(intent.payload.sentinel.port) {
        return Err("qualification sentinel port remains occupied; refusing state deletion".into());
    }
    remove_state_root_durable(&options.state_root)
        .map_err(|error| format!("failed to remove qualification state root: {error}"))?;
    println!(
        "Linux host power-loss qualification state removed: {}",
        display(root, &options.state_root)
    );
    Ok(0)
}

struct SentinelProcess {
    child: Option<Child>,
    port: u16,
}

impl SentinelProcess {
    fn start(binary: &Path, log_path: &Path) -> RunnerResult<Self> {
        let port = reserve_port()?;
        if let Some(parent) = log_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("failed to create sentinel log root: {error}"))?;
        }
        let log = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(log_path)
            .map_err(|error| format!("failed to create sentinel log: {error}"))?;
        let mut command = Command::new(binary);
        command
            .args(["agent", "--host", "127.0.0.1", "--port", &port.to_string()])
            .stdin(Stdio::null())
            .stdout(Stdio::from(
                log.try_clone().map_err(|error| error.to_string())?,
            ))
            .stderr(Stdio::from(log));
        configure_detached(&mut command);
        let child = command
            .spawn()
            .map_err(|error| format!("failed to launch reboot sentinel Agent: {error}"))?;
        let mut sentinel = Self {
            child: Some(child),
            port,
        };
        sentinel.wait_ready(Duration::from_secs(30))?;
        Ok(sentinel)
    }

    fn process_id(&self) -> u32 {
        self.child.as_ref().map(Child::id).unwrap_or_default()
    }

    fn wait_ready(&mut self, timeout: Duration) -> RunnerResult<()> {
        let started = Instant::now();
        while started.elapsed() < timeout {
            if port_listening(self.port) {
                return Ok(());
            }
            if let Some(status) = self
                .child
                .as_mut()
                .expect("sentinel child")
                .try_wait()
                .map_err(|error| error.to_string())?
            {
                return Err(format!("reboot sentinel Agent exited early: {status}"));
            }
            thread::sleep(Duration::from_millis(50));
        }
        Err("reboot sentinel Agent readiness timed out".into())
    }

    fn detach(mut self) -> (u32, u16) {
        let child = self.child.take().expect("sentinel child");
        let pid = child.id();
        drop(child);
        (pid, self.port)
    }
}

impl Drop for SentinelProcess {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

#[cfg(unix)]
fn configure_detached(command: &mut Command) {
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() == -1 {
                Err(std::io::Error::last_os_error())
            } else {
                Ok(())
            }
        });
    }
}

#[cfg(windows)]
fn configure_detached(_command: &mut Command) {}

fn terminate_sentinel(sentinel: &SentinelObservation) -> RunnerResult<()> {
    let executable = fs::read_link(format!("/proc/{}/exe", sentinel.process_id))
        .map_err(|error| format!("failed to inspect sentinel process: {error}"))?;
    if sha256_file(&executable)? != sentinel.executable_sha256 {
        return Err("sentinel process identity changed; refusing to signal it".into());
    }
    signal(sentinel.process_id, "-TERM")?;
    let started = Instant::now();
    while started.elapsed() < Duration::from_secs(5) && port_listening(sentinel.port) {
        thread::sleep(Duration::from_millis(100));
    }
    if port_listening(sentinel.port) {
        signal(sentinel.process_id, "-KILL")?;
    }
    wait_for_port_free(sentinel.port, Duration::from_secs(5))
}

fn signal(pid: u32, signal: &str) -> RunnerResult<()> {
    let status = Command::new("/bin/kill")
        .args([signal, &pid.to_string()])
        .status()
        .map_err(|error| format!("failed to signal qualification process: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("failed to signal qualification process {pid}"))
    }
}

fn wait_for_port_free(port: u16, timeout: Duration) -> RunnerResult<()> {
    let started = Instant::now();
    while started.elapsed() < timeout {
        if !port_listening(port) {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(100));
    }
    Err(format!("qualification port {port} did not close"))
}

fn reserve_port() -> RunnerResult<u16> {
    let listener = TcpListener::bind(("127.0.0.1", 0))
        .map_err(|error| format!("failed to reserve qualification port: {error}"))?;
    listener
        .local_addr()
        .map(|address| address.port())
        .map_err(|error| error.to_string())
}

fn port_listening(port: u16) -> bool {
    TcpStream::connect_timeout(
        &format!("127.0.0.1:{port}")
            .parse()
            .expect("loopback socket"),
        Duration::from_millis(200),
    )
    .is_ok()
}

fn ensure_remote_linux() -> RunnerResult<()> {
    if std::env::consts::OS != "linux" {
        return Err("host power-loss qualification must execute on Linux".into());
    }
    if std::env::var_os("SSH_CONNECTION").is_none() {
        return Err(
            "host power-loss release qualification requires a managed remote session".into(),
        );
    }
    Ok(())
}

fn identity_digest(path: &Path) -> RunnerResult<String> {
    let value = fs::read_to_string(path).map_err(|error| {
        format!(
            "failed to read Linux host identity {}: {error}",
            path.display()
        )
    })?;
    let value = value.trim();
    if value.is_empty() {
        return Err(format!("Linux host identity is empty: {}", path.display()));
    }
    Ok(format!("{:x}", Sha256::digest(value.as_bytes())))
}

fn uptime_seconds() -> RunnerResult<u64> {
    fs::read_to_string("/proc/uptime")
        .map_err(|error| format!("failed to read Linux uptime: {error}"))?
        .split_whitespace()
        .next()
        .ok_or_else(|| "Linux uptime is empty".to_string())?
        .split('.')
        .next()
        .ok_or_else(|| "Linux uptime is invalid".to_string())?
        .parse()
        .map_err(|error| format!("Linux uptime is invalid: {error}"))
}

fn sha256_file(path: &Path) -> RunnerResult<String> {
    let mut file =
        File::open(path).map_err(|error| format!("failed to open {}: {error}", path.display()))?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|error| format!("failed to hash {}: {error}", path.display()))?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn write_durable_json(path: &Path, value: &impl Serialize) -> RunnerResult<()> {
    let staged = stage_durable_json(path, value)?;
    promote_staged_file(&staged, path)
}

fn stage_durable_json(path: &Path, value: &impl Serialize) -> RunnerResult<PathBuf> {
    let parent = path.parent().ok_or("output path has no parent")?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or("invalid output name")?;
    let temporary = parent.join(format!(".{name}.{}.tmp", std::process::id()));
    let bytes = serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?;
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temporary)
        .map_err(|error| format!("failed to stage {}: {error}", temporary.display()))?;
    file.write_all(&bytes).map_err(|error| error.to_string())?;
    file.sync_all().map_err(|error| error.to_string())?;
    Ok(temporary)
}

fn promote_staged_file(staged: &Path, path: &Path) -> RunnerResult<()> {
    fs::rename(staged, path)
        .map_err(|error| format!("failed to promote {}: {error}", path.display()))?;
    File::open(path.parent().ok_or("output path has no parent")?)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| format!("failed to sync output directory: {error}"))
}

fn read_json<T: for<'de> serde::Deserialize<'de>>(path: &Path) -> RunnerResult<T> {
    let bytes =
        fs::read(path).map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid JSON {}: {error}", path.display()))
}

fn validator_self_test(root: &Path) -> RunnerResult<()> {
    state_root_self_test(root)?;
    let fixture: kyuubiki_installer::AgentSolverOperationalQualificationReport = read_json(
        &root
            .join("releases/usability-evidence/2.13.8/agent-solver-operational-qualification.json"),
    )?;
    let preparation = HostPowerLossPreparation {
        qualification_id: QUALIFICATION_ID.to_string(),
        execution_host_role: "remote-linux-qualification-host".to_string(),
        platform: "linux".to_string(),
        architecture: fixture.architecture.clone(),
        runtime_version: fixture.package.version.clone(),
        prepared_at_unix_ms: 1,
        machine_id_sha256: "c".repeat(64),
        pre_boot_id_sha256: "a".repeat(64),
        pre_uptime_seconds: 100,
        package: fixture.package.clone(),
        activation: fixture.activation.clone(),
        active_entrypoint_sha256: fixture.package.entrypoint_sha256.clone(),
        sentinel: SentinelObservation {
            process_id: 42,
            port: 5001,
            executable_sha256: fixture.package.entrypoint_sha256.clone(),
            ready_before_reboot: true,
        },
        preflight: fixture.clone(),
    };
    let mut report = HostPowerLossQualificationReport {
        schema_version: REPORT_SCHEMA.to_string(),
        qualification_id: QUALIFICATION_ID.to_string(),
        status: "pass".to_string(),
        journey: JOURNEY.to_string(),
        execution_host_role: preparation.execution_host_role.clone(),
        platform: preparation.platform.clone(),
        architecture: preparation.architecture.clone(),
        runtime_version: preparation.runtime_version.clone(),
        generated_at_unix_ms: 2,
        intent_sha256: digest_serializable(&preparation)?,
        preparation,
        recovery: RebootRecoveryObservation {
            post_boot_id_sha256: "b".repeat(64),
            post_machine_id_sha256: "c".repeat(64),
            post_uptime_seconds: 10,
            boot_identity_changed: true,
            same_machine: true,
            sentinel_port_free_before_resume: true,
            active_entrypoint_sha256: fixture.package.entrypoint_sha256.clone(),
        },
        postflight: fixture,
        cleanup: HostPowerLossCleanup {
            scope: "qualification-state-root".to_string(),
            state_root_removed: true,
            sentinel_port_released: true,
            residue_count: 0,
        },
        checks: Vec::new(),
    };
    report.checks = qualification_checks(&report);
    validate_report(&report)?;
    report.recovery.post_boot_id_sha256 = report.preparation.pre_boot_id_sha256.clone();
    if validate_report(&report).is_ok() {
        return Err("validator accepted an unchanged Linux boot identity".into());
    }
    report.recovery.post_boot_id_sha256 = "b".repeat(64);
    let postflight_platform = report.postflight.platform.clone();
    report.postflight.platform = "macos".to_string();
    if validate_report(&report).is_ok() {
        return Err("validator accepted mismatched post-reboot Agent identity".into());
    }
    report.postflight.platform = postflight_platform;
    report.intent_sha256 = "d".repeat(64);
    if validate_report(&report).is_ok() {
        return Err("validator accepted a forged reboot intent digest".into());
    }
    Ok(())
}

fn state_root_self_test(root: &Path) -> RunnerResult<()> {
    let outside = root.join("unsafe-host-power-loss-state");
    if ensure_state_root_scope(root, &outside).is_ok() {
        return Err("state-root guard accepted a path outside repository tmp".into());
    }
    let traversal = root.join("tmp/../unsafe-host-power-loss-state");
    if ensure_state_root_scope(root, &traversal).is_ok() {
        return Err("state-root guard accepted parent-directory traversal".into());
    }
    let path = root.join(format!(
        "tmp/linux-host-power-loss-self-test-{}",
        std::process::id()
    ));
    {
        let _guard = ManagedStateRoot::create(root, &path)?;
        fs::write(path.join("partial-state"), b"self-test")
            .map_err(|error| format!("failed to seed state-root self-test: {error}"))?;
    }
    if path.exists() {
        return Err("failed prepare state was not cleaned automatically".into());
    }
    Ok(())
}

fn development_version(root: &Path) -> RunnerResult<String> {
    let value: serde_json::Value = read_json(&root.join("docs/book-manifest.json"))?;
    value
        .get("current_development_version")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| "book manifest misses current_development_version".into())
}

fn unix_now_ms() -> RunnerResult<u128> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .map_err(|error| error.to_string())
}

fn next_path(args: &mut impl Iterator<Item = OsString>, flag: &str) -> RunnerResult<PathBuf> {
    args.next()
        .map(PathBuf::from)
        .ok_or_else(|| format!("{flag} requires a path"))
}

fn next_string(args: &mut impl Iterator<Item = OsString>, flag: &str) -> RunnerResult<String> {
    args.next()
        .map(|value| value.to_string_lossy().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{flag} requires a value"))
}

fn resolve(root: &Path, path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        path
    } else {
        root.join(path)
    }
}

fn display(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn print_qualify_usage() {
    println!(
        "usage: kyuubiki-script-runner qualify-linux-host-power-loss prepare [--state-root path] [--agent-binary path] [--package-version version]\n       kyuubiki-script-runner qualify-linux-host-power-loss resume [--state-root path] [--out report]\n       kyuubiki-script-runner qualify-linux-host-power-loss cleanup [--state-root path]"
    );
}

fn print_check_usage() {
    println!(
        "usage: kyuubiki-script-runner check-linux-host-power-loss-qualification [--self-test] [--verify-report report]"
    );
}
