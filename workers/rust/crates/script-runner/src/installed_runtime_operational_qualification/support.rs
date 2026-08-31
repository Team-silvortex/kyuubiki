use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::Read;
use std::net::{TcpListener, TcpStream};
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Output};
use std::time::Duration;

type RunnerResult<T> = Result<T, String>;

pub(crate) const WORKFLOW_ID: &str = "qualification.installed-runtime.bar";

#[derive(Clone, Copy, Debug, serde::Deserialize, serde::Serialize)]
pub(crate) struct Ports {
    pub(crate) orchestrator: u16,
    pub(crate) agent_one: u16,
    pub(crate) agent_two: u16,
}

impl Ports {
    pub(crate) fn all(self) -> [u16; 3] {
        [self.orchestrator, self.agent_one, self.agent_two]
    }

    pub(crate) fn any_listening(self) -> bool {
        self.all().into_iter().any(port_listening)
    }
}

pub(crate) struct RuntimeGuard {
    binary: PathBuf,
    env: BTreeMap<String, String>,
    ports: Ports,
    running: bool,
}

impl RuntimeGuard {
    pub(crate) fn new(binary: PathBuf, env: BTreeMap<String, String>, ports: Ports) -> Self {
        Self {
            binary,
            env,
            ports,
            running: false,
        }
    }

    pub(crate) fn command(&mut self, action: &str) -> RunnerResult<String> {
        let output = run_success(
            &format!("installed Runtime {action}"),
            configured_command(&self.binary, &self.env, [action]),
        )?;
        self.running = action != "stop";
        String::from_utf8(output.stdout)
            .map_err(|error| format!("Runtime {action} output is not UTF-8: {error}"))
    }

    pub(crate) fn stop(&mut self) -> RunnerResult<()> {
        if self.running {
            self.command("stop")?;
        }
        self.running = false;
        if self.ports.any_listening() {
            return Err("installed Runtime ports remain open after stop".to_string());
        }
        Ok(())
    }

    pub(crate) fn leave_running(mut self) {
        self.running = false;
    }
}

impl Drop for RuntimeGuard {
    fn drop(&mut self) {
        if self.running {
            let _ = configured_command(&self.binary, &self.env, ["stop"]).output();
        }
    }
}

pub(crate) fn run_headless_workflow(
    binary: &Path,
    env: &BTreeMap<String, String>,
    cwd: &Path,
    workflow: &Path,
    report: &Path,
    port: u16,
) -> RunnerResult<()> {
    let api = format!("http://127.0.0.1:{port}");
    run_success(
        "execute installed Headless service workflow",
        headless_command(
            binary,
            env,
            cwd,
            [
                "run",
                path_text(workflow)?,
                "--execute",
                "--executor",
                "service",
                "--api-base-url",
                api.as_str(),
                "--json",
                "--report-out",
                path_text(report)?,
            ],
        ),
    )?;
    Ok(())
}

pub(crate) fn headless_command<'a>(
    binary: &Path,
    env: &BTreeMap<String, String>,
    cwd: &Path,
    args: impl IntoIterator<Item = &'a str>,
) -> Command {
    let mut command = configured_command(binary, env, args);
    command.current_dir(cwd);
    command
}

pub(crate) fn configured_command<'a>(
    binary: &Path,
    env: &BTreeMap<String, String>,
    args: impl IntoIterator<Item = &'a str>,
) -> Command {
    let mut command = Command::new(binary);
    command.env_clear().envs(env).args(args);
    command
}

pub(crate) fn run_success(label: &str, mut command: Command) -> RunnerResult<Output> {
    let output = command
        .output()
        .map_err(|error| format!("failed to {label}: {error}"))?;
    if output.status.success() {
        return Ok(output);
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    Err(format!(
        "{label} failed with {}: {}",
        output.status,
        stderr.lines().rev().take(8).collect::<Vec<_>>().join(" | ")
    ))
}

pub(crate) fn runtime_env(
    runtime: &Path,
    state: &Path,
    home: &Path,
    ports: Ports,
) -> BTreeMap<String, String> {
    BTreeMap::from([
        ("HOME".into(), home.display().to_string()),
        ("LANG".into(), "C.UTF-8".into()),
        ("PATH".into(), "/usr/bin:/bin".into()),
        (
            "KYUUBIKI_RUNTIME_ROOT".into(),
            runtime.display().to_string(),
        ),
        (
            "KYUUBIKI_RUNTIME_STATE_ROOT".into(),
            state.display().to_string(),
        ),
        (
            "KYUUBIKI_AGENT_ENDPOINTS".into(),
            format!(
                "127.0.0.1:{},127.0.0.1:{}",
                ports.agent_one, ports.agent_two
            ),
        ),
        (
            "KYUUBIKI_ORCHESTRATOR_PORT".into(),
            ports.orchestrator.to_string(),
        ),
        ("KYUUBIKI_RUNTIME_FRONTEND_DISABLED".into(), "true".into()),
    ])
}

pub(crate) fn validate_status(status: &str, ports: Ports) -> RunnerResult<()> {
    for required in [
        "runtime-policy: installer-managed".to_string(),
        format!(
            "orchestrator: running on http://127.0.0.1:{}",
            ports.orchestrator
        ),
        format!(
            "agent[{}]: running on tcp://127.0.0.1:{}",
            ports.agent_one, ports.agent_one
        ),
        format!(
            "agent[{}]: running on tcp://127.0.0.1:{}",
            ports.agent_two, ports.agent_two
        ),
        "frontend: disabled by runtime configuration".to_string(),
    ] {
        if !status.contains(&required) {
            return Err(format!("installed Runtime status misses {required}"));
        }
    }
    if status.contains("development-source") {
        return Err("installed Runtime fell back to the source tree".to_string());
    }
    Ok(())
}

pub(crate) fn execution_result(report: &Value) -> RunnerResult<(String, f64, f64)> {
    require_text(
        report,
        "/schema_version",
        "kyuubiki.headless-execution-run/v1",
    )?;
    require_text(report, "/workflow_id", WORKFLOW_ID)?;
    require_text(report, "/mode", "execute:service")?;
    require_text(report, "/status", "ok")?;
    let job_id = text_at(report, "/execution_summary/job_ids/0")?.to_string();
    let tip = number_at(report, "/steps/2/result_preview/result/tip_displacement")?;
    let stress = number_at(report, "/steps/2/result_preview/result/max_stress")?;
    if !job_id.is_empty() && tip.is_finite() && tip > 0.0 && stress.is_finite() && stress > 0.0 {
        Ok((job_id, tip, stress))
    } else {
        Err("installed Headless numerical result is invalid".to_string())
    }
}

pub(crate) fn validate_fetch(
    report: &Value,
    job_id: &str,
    tip: f64,
    stress: f64,
) -> RunnerResult<()> {
    require_text(
        report,
        "/schema_version",
        "kyuubiki.headless-execution-run/v1",
    )?;
    require_text(report, "/mode", "execute:service")?;
    require_text(report, "/status", "ok")?;
    require_text(report, "/steps/0/result_preview/job_id", job_id)?;
    require_text(report, "/steps/0/result_preview/status", "completed")?;
    if number_at(report, "/steps/0/result_preview/result/tip_displacement")? != tip
        || number_at(report, "/steps/0/result_preview/result/max_stress")? != stress
    {
        return Err("installed result changed after Runtime restart".to_string());
    }
    Ok(())
}

pub(crate) fn verify_installation(
    runtime: &Path,
    version: &str,
) -> RunnerResult<BTreeMap<String, String>> {
    let manifest_path = runtime.join("manifests/runtime-payload.json");
    ensure_regular(&manifest_path)?;
    let manifest = read_json(&manifest_path)?;
    require_text(&manifest, "/schema_version", "kyuubiki.runtime-payload/v1")?;
    require_text(&manifest, "/version", version)?;
    require_text(&manifest, "/platform", "linux")?;
    let files = manifest
        .get("files")
        .and_then(Value::as_array)
        .ok_or("Runtime payload manifest misses files")?;
    if files.is_empty() {
        return Err("Runtime payload manifest contains no files".to_string());
    }
    let canonical_runtime = canonical_dir(runtime, "installed Runtime payload root")?;
    let mut verified = BTreeMap::new();
    let mut expected_files = BTreeSet::from(["manifests/runtime-payload.json".to_string()]);
    for entry in files {
        let relative = entry
            .get("path")
            .and_then(Value::as_str)
            .ok_or("Runtime payload file misses path")?;
        let expected = entry
            .get("sha256")
            .and_then(Value::as_str)
            .filter(|value| valid_digest(value))
            .ok_or("Runtime payload file misses a valid digest")?;
        let path = runtime.join(safe_relative_path(relative)?);
        ensure_regular(&path)?;
        let canonical_path = fs::canonicalize(&path)
            .map_err(|error| format!("failed to resolve installed payload file: {error}"))?;
        if !canonical_path.starts_with(&canonical_runtime) {
            return Err(format!(
                "installed Runtime payload file escapes its root: {relative}"
            ));
        }
        let observed = sha256_file(&path)?;
        if observed != expected {
            return Err(format!(
                "installed Runtime digest drifted from payload: {relative}"
            ));
        }
        if !expected_files.insert(relative.to_string()) {
            return Err(format!("Runtime payload manifest repeats file {relative}"));
        }
        verified.insert(relative.to_string(), observed);
    }
    let mut actual_files = BTreeSet::new();
    collect_installation_files(runtime, runtime, &mut actual_files)?;
    if actual_files != expected_files {
        let unmanaged = actual_files.difference(&expected_files).next();
        let missing = expected_files.difference(&actual_files).next();
        return Err(format!(
            "installed Runtime file set drifted (unmanaged={}, missing={})",
            unmanaged.map(String::as_str).unwrap_or("none"),
            missing.map(String::as_str).unwrap_or("none")
        ));
    }
    let selected = [
        ("runtime-payload.json", manifest_path),
        (
            "service-launch.json",
            runtime.join("manifests/service-launch.json"),
        ),
        ("kyuubiki-runtime", runtime.join("bin/kyuubiki-runtime")),
        ("kyuubiki-headless", runtime.join("bin/kyuubiki-headless")),
        ("kyuubiki-cli", runtime.join("bin/kyuubiki-cli")),
    ];
    let mut digests = BTreeMap::new();
    for (name, path) in selected {
        ensure_regular(&path)?;
        let digest = sha256_file(&path)?;
        if name != "runtime-payload.json" {
            let relative = path
                .strip_prefix(runtime)
                .map_err(|_| "installed Runtime file escaped its root")?
                .to_string_lossy()
                .replace('\\', "/");
            if verified.get(&relative) != Some(&digest) {
                return Err(format!(
                    "Runtime payload manifest misses verified file {relative}"
                ));
            }
        }
        digests.insert(name.to_string(), digest);
    }
    Ok(digests)
}

fn collect_installation_files(
    root: &Path,
    directory: &Path,
    files: &mut BTreeSet<String>,
) -> RunnerResult<()> {
    for entry in fs::read_dir(directory)
        .map_err(|error| format!("failed to inspect {}: {error}", directory.display()))?
    {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        let metadata = path
            .symlink_metadata()
            .map_err(|error| format!("failed to inspect {}: {error}", path.display()))?;
        if metadata.file_type().is_symlink() {
            return Err(format!(
                "installed Runtime contains a symlink: {}",
                path.display()
            ));
        }
        if metadata.is_dir() {
            collect_installation_files(root, &path, files)?;
        } else if metadata.is_file() {
            let relative = path
                .strip_prefix(root)
                .map_err(|_| "installed Runtime file escaped its root")?
                .to_string_lossy()
                .replace('\\', "/");
            files.insert(relative);
        } else {
            return Err(format!(
                "installed Runtime contains a special file: {}",
                path.display()
            ));
        }
    }
    Ok(())
}

fn safe_relative_path(value: &str) -> RunnerResult<&Path> {
    let path = Path::new(value);
    if value.is_empty()
        || path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(format!(
            "Runtime payload file path escapes its root: {value}"
        ));
    }
    Ok(path)
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

pub(crate) fn render_digests(digests: &BTreeMap<String, String>) -> String {
    digests
        .iter()
        .map(|(name, digest)| format!("{digest}  {name}\n"))
        .collect()
}

pub(crate) fn reserve_ports() -> RunnerResult<Ports> {
    let listeners = (0..3)
        .map(|_| TcpListener::bind(("127.0.0.1", 0)))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("failed to reserve Runtime ports: {error}"))?;
    let ports = listeners
        .iter()
        .map(|listener| listener.local_addr().map(|address| address.port()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("failed to inspect Runtime ports: {error}"))?;
    Ok(Ports {
        orchestrator: ports[0],
        agent_one: ports[1],
        agent_two: ports[2],
    })
}

pub(crate) fn port_listening(port: u16) -> bool {
    TcpStream::connect_timeout(
        &format!("127.0.0.1:{port}")
            .parse()
            .expect("loopback address"),
        Duration::from_millis(180),
    )
    .is_ok()
}

pub(crate) fn pid_residue(state: &Path) -> RunnerResult<usize> {
    let run = state.join("run");
    if !run.is_dir() {
        return Ok(0);
    }
    Ok(fs::read_dir(&run)
        .map_err(|error| format!("failed to inspect Runtime state: {error}"))?
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().and_then(|value| value.to_str()) == Some("pid"))
        .count())
}

pub(crate) fn runtime_pids(state: &Path, ports: Ports) -> RunnerResult<BTreeMap<String, u32>> {
    let run = state.join("run");
    let files = [
        (
            "orchestrator",
            run.join(format!("orchestrator-{}.pid", ports.orchestrator)),
        ),
        (
            "agent_one",
            run.join(format!("agent-{}.pid", ports.agent_one)),
        ),
        (
            "agent_two",
            run.join(format!("agent-{}.pid", ports.agent_two)),
        ),
    ];
    let mut pids = BTreeMap::new();
    for (role, path) in files {
        let pid = fs::read_to_string(&path)
            .map_err(|error| format!("failed to read Runtime PID {}: {error}", path.display()))?
            .trim()
            .parse::<u32>()
            .ok()
            .filter(|pid| *pid > 0)
            .ok_or_else(|| format!("Runtime PID is invalid: {}", path.display()))?;
        pids.insert(role.to_string(), pid);
    }
    Ok(pids)
}

pub(crate) fn ensure_remote_linux() -> RunnerResult<()> {
    if std::env::consts::OS != "linux" || std::env::var_os("SSH_CONNECTION").is_none() {
        return Err(
            "installed Runtime host capture requires a managed remote Linux session".into(),
        );
    }
    Ok(())
}

pub(crate) fn canonical_dir(path: &Path, label: &str) -> RunnerResult<PathBuf> {
    fs::canonicalize(path).map_err(|error| format!("failed to resolve {label}: {error}"))
}

pub(crate) fn ensure_regular(path: &Path) -> RunnerResult<()> {
    let metadata = path
        .symlink_metadata()
        .map_err(|error| format!("failed to inspect {}: {error}", path.display()))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(format!(
            "installed artifact is not a regular file: {}",
            path.display()
        ));
    }
    Ok(())
}

pub(crate) fn sha256_file(path: &Path) -> RunnerResult<String> {
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

pub(crate) fn read_json(path: &Path) -> RunnerResult<Value> {
    let bytes =
        fs::read(path).map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid JSON {}: {error}", path.display()))
}

pub(crate) fn write_json(path: &Path, value: &Value) -> RunnerResult<()> {
    let bytes = serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?;
    fs::write(path, bytes).map_err(|error| format!("failed to write {}: {error}", path.display()))
}

pub(crate) fn text_at<'a>(value: &'a Value, pointer: &str) -> RunnerResult<&'a str> {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("missing string at {pointer}"))
}

pub(crate) fn require_text(value: &Value, pointer: &str, expected: &str) -> RunnerResult<()> {
    if text_at(value, pointer)? == expected {
        Ok(())
    } else {
        Err(format!("unexpected value at {pointer}"))
    }
}

pub(crate) fn number_at(value: &Value, pointer: &str) -> RunnerResult<f64> {
    value
        .pointer(pointer)
        .and_then(Value::as_f64)
        .ok_or_else(|| format!("missing number at {pointer}"))
}

pub(crate) fn path_text(path: &Path) -> RunnerResult<&str> {
    path.to_str()
        .ok_or_else(|| format!("path is not UTF-8: {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payload_paths_cannot_escape_the_installation() {
        assert!(safe_relative_path("bin/kyuubiki-runtime").is_ok());
        assert!(safe_relative_path("../outside").is_err());
        assert!(safe_relative_path("/outside").is_err());
    }

    #[test]
    fn installation_scan_rejects_symlinks() {
        let root = std::env::temp_dir().join(format!(
            "kyuubiki-installed-runtime-scan-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("bin")).expect("root");
        #[cfg(unix)]
        std::os::unix::fs::symlink("/tmp", root.join("bin/escape")).expect("symlink");
        let mut files = BTreeSet::new();
        #[cfg(unix)]
        assert!(collect_installation_files(&root, &root, &mut files).is_err());
        let _ = fs::remove_dir_all(root);
    }
}
