use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs::{self, File};
use std::io::Read;
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::Duration;

type RunnerResult<T> = Result<T, String>;

const WORKFLOW_ID: &str = "qualification.installed-runtime.bar";

pub(super) fn run(args: Vec<OsString>) -> RunnerResult<u8> {
    let options = Options::parse(args)?;
    ensure_remote_linux()?;
    capture(&options)?;
    println!(
        "installed Runtime host capture passed: {}",
        options.output.display()
    );
    Ok(0)
}

struct Options {
    managed_root: PathBuf,
    runtime_root: PathBuf,
    detached_source_root: PathBuf,
    output: PathBuf,
    package_version: String,
}

impl Options {
    fn parse(args: Vec<OsString>) -> RunnerResult<Self> {
        let mut managed_root = None;
        let mut runtime_root = None;
        let mut detached_source_root = None;
        let mut output = None;
        let mut package_version = None;
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.to_string_lossy().as_ref() {
                "--managed-root" => managed_root = Some(next_path(&mut args, "--managed-root")?),
                "--runtime-root" => runtime_root = Some(next_path(&mut args, "--runtime-root")?),
                "--detached-source-root" => {
                    detached_source_root = Some(next_path(&mut args, "--detached-source-root")?)
                }
                "--out" => output = Some(next_path(&mut args, "--out")?),
                "--package-version" => {
                    package_version = Some(next_string(&mut args, "--package-version")?)
                }
                other => return Err(format!("unknown installed Runtime host option: {other}")),
            }
        }
        let options = Self {
            managed_root: managed_root.ok_or("--managed-root is required")?,
            runtime_root: runtime_root.ok_or("--runtime-root is required")?,
            detached_source_root: detached_source_root
                .ok_or("--detached-source-root is required")?,
            output: output.ok_or("--out is required")?,
            package_version: package_version.ok_or("--package-version is required")?,
        };
        for (label, path) in [
            ("--managed-root", &options.managed_root),
            ("--runtime-root", &options.runtime_root),
            ("--detached-source-root", &options.detached_source_root),
            ("--out", &options.output),
        ] {
            if !path.is_absolute() {
                return Err(format!("{label} requires an absolute path"));
            }
        }
        if !super::valid_version(&options.package_version) {
            return Err("--package-version is invalid".to_string());
        }
        Ok(options)
    }
}

fn capture(options: &Options) -> RunnerResult<()> {
    if options.detached_source_root.exists() {
        return Err("source tree must be removed before installed capture".to_string());
    }
    let managed = canonical_dir(&options.managed_root, "managed root")?;
    let runtime = canonical_dir(&options.runtime_root, "installed Runtime")?;
    if !runtime.starts_with(&managed) {
        return Err("installed Runtime escapes the managed root".to_string());
    }
    let detached_parent = options
        .detached_source_root
        .parent()
        .ok_or("detached source path has no parent")?;
    if canonical_dir(detached_parent, "detached source parent")? != managed {
        return Err("detached source path escapes the managed root".to_string());
    }
    let output_parent = options
        .output
        .parent()
        .ok_or("host capture output has no parent")?;
    let output_parent = canonical_dir(output_parent, "host capture output parent")?;
    if !output_parent.starts_with(&managed) {
        return Err("host capture output must stay inside the managed root".to_string());
    }
    let installation = verify_installation(&runtime, &options.package_version)?;
    let ports = reserve_ports()?;
    let state_root = managed.join("runtime-state");
    let work_root = managed.join("isolated-work");
    let home = managed.join("home");
    fs::create_dir_all(&options.output)
        .and_then(|()| fs::create_dir_all(&work_root))
        .and_then(|()| fs::create_dir_all(&home))
        .map_err(|error| format!("failed to create capture roots: {error}"))?;

    let runtime_binary = runtime.join("bin/kyuubiki-runtime");
    let headless_binary = runtime.join("bin/kyuubiki-headless");
    let env = runtime_env(&runtime, &state_root, &home, ports);
    let mut guard = RuntimeGuard::new(runtime_binary, env.clone(), ports);
    guard.command("start-local")?;
    let status = guard.command("status")?;
    validate_status(&status, ports)?;

    let workflow = work_root.join("workflow.json");
    run_success(
        "initialize installed Headless workflow",
        headless_command(
            &headless_binary,
            &env,
            &work_root,
            [
                "init",
                "--template",
                "direct_bar_1d",
                "--workflow-id",
                WORKFLOW_ID,
                "--out",
                path_text(&workflow)?,
                "--json",
            ],
        ),
    )?;
    let solve_path = options.output.join("solve.json");
    run_headless_workflow(
        &headless_binary,
        &env,
        &work_root,
        &workflow,
        &solve_path,
        ports.orchestrator,
    )?;
    let solve = read_json(&solve_path)?;
    let (job_id, tip, stress) = execution_result(&solve)?;

    let fetch_workflow = work_root.join("fetch-workflow.json");
    write_json(
        &fetch_workflow,
        &json!({
            "schema_version": "kyuubiki.headless-workflow/v1",
            "exported_at": "1970-01-01T00:00:00.000Z",
            "language": "en-US",
            "workflow": {
                "id": format!("{WORKFLOW_ID}.fetch"),
                "steps": [{"action": "result_fetch", "payload": {"job_id": job_id}}]
            }
        }),
    )?;
    guard.command("restart-local")?;
    let restart_path = options.output.join("fetch-report.json");
    run_headless_workflow(
        &headless_binary,
        &env,
        &work_root,
        &fetch_workflow,
        &restart_path,
        ports.orchestrator,
    )?;
    validate_fetch(&read_json(&restart_path)?, &job_id, tip, stress)?;

    guard.command("restart-local")?;
    if options.detached_source_root.exists() {
        return Err("source tree reappeared during installed restart".to_string());
    }
    let detached_path = options.output.join("detached-fetch-report.json");
    run_headless_workflow(
        &headless_binary,
        &env,
        &work_root,
        &fetch_workflow,
        &detached_path,
        ports.orchestrator,
    )?;
    validate_fetch(&read_json(&detached_path)?, &job_id, tip, stress)?;
    let detached_status = guard.command("status")?;
    validate_status(&detached_status, ports)?;
    fs::write(
        options.output.join("detached-status.txt"),
        detached_status.as_bytes(),
    )
    .map_err(|error| format!("failed to write detached status: {error}"))?;
    fs::write(
        options.output.join("installed-digests.txt"),
        render_digests(&installation),
    )
    .map_err(|error| format!("failed to write installed digests: {error}"))?;

    guard.stop()?;
    if ports.any_listening() || pid_residue(&state_root)? != 0 {
        return Err("installed Runtime cleanup left a process or PID residue".to_string());
    }
    Ok(())
}

#[derive(Clone, Copy)]
struct Ports {
    orchestrator: u16,
    agent_one: u16,
    agent_two: u16,
}

impl Ports {
    fn any_listening(self) -> bool {
        [self.orchestrator, self.agent_one, self.agent_two]
            .into_iter()
            .any(port_listening)
    }
}

struct RuntimeGuard {
    binary: PathBuf,
    env: BTreeMap<String, String>,
    ports: Ports,
    running: bool,
}

impl RuntimeGuard {
    fn new(binary: PathBuf, env: BTreeMap<String, String>, ports: Ports) -> Self {
        Self {
            binary,
            env,
            ports,
            running: false,
        }
    }

    fn command(&mut self, action: &str) -> RunnerResult<String> {
        let output = run_success(
            &format!("installed Runtime {action}"),
            configured_command(&self.binary, &self.env, [action]),
        )?;
        self.running = action != "stop";
        String::from_utf8(output.stdout)
            .map_err(|error| format!("Runtime {action} output is not UTF-8: {error}"))
    }

    fn stop(&mut self) -> RunnerResult<()> {
        if self.running {
            self.command("stop")?;
        }
        self.running = false;
        if self.ports.any_listening() {
            return Err("installed Runtime ports remain open after stop".to_string());
        }
        Ok(())
    }
}

impl Drop for RuntimeGuard {
    fn drop(&mut self) {
        if self.running {
            let _ = configured_command(&self.binary, &self.env, ["stop"]).output();
        }
    }
}

fn run_headless_workflow(
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

fn headless_command<'a>(
    binary: &Path,
    env: &BTreeMap<String, String>,
    cwd: &Path,
    args: impl IntoIterator<Item = &'a str>,
) -> Command {
    let mut command = configured_command(binary, env, args);
    command.current_dir(cwd);
    command
}

fn configured_command<'a>(
    binary: &Path,
    env: &BTreeMap<String, String>,
    args: impl IntoIterator<Item = &'a str>,
) -> Command {
    let mut command = Command::new(binary);
    command.env_clear().envs(env).args(args);
    command
}

fn run_success(label: &str, mut command: Command) -> RunnerResult<Output> {
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

fn runtime_env(
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

fn validate_status(status: &str, ports: Ports) -> RunnerResult<()> {
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

fn execution_result(report: &Value) -> RunnerResult<(String, f64, f64)> {
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

fn validate_fetch(report: &Value, job_id: &str, tip: f64, stress: f64) -> RunnerResult<()> {
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

fn verify_installation(runtime: &Path, version: &str) -> RunnerResult<BTreeMap<String, String>> {
    let manifest_path = runtime.join("manifests/runtime-payload.json");
    let manifest = read_json(&manifest_path)?;
    require_text(&manifest, "/schema_version", "kyuubiki.runtime-payload/v1")?;
    require_text(&manifest, "/version", version)?;
    require_text(&manifest, "/platform", "linux")?;
    let files = [
        ("runtime-payload.json", None, manifest_path),
        (
            "service-launch.json",
            Some("manifests/service-launch.json"),
            runtime.join("manifests/service-launch.json"),
        ),
        (
            "kyuubiki-runtime",
            Some("bin/kyuubiki-runtime"),
            runtime.join("bin/kyuubiki-runtime"),
        ),
        (
            "kyuubiki-headless",
            Some("bin/kyuubiki-headless"),
            runtime.join("bin/kyuubiki-headless"),
        ),
        (
            "kyuubiki-cli",
            Some("bin/kyuubiki-cli"),
            runtime.join("bin/kyuubiki-cli"),
        ),
    ];
    let mut digests = BTreeMap::new();
    for (name, relative, path) in files {
        ensure_regular(&path)?;
        let digest = sha256_file(&path)?;
        if let Some(relative) = relative {
            require_manifest_digest(&manifest, relative, &digest)?;
        }
        digests.insert(name.to_string(), digest);
    }
    Ok(digests)
}

fn require_manifest_digest(manifest: &Value, path: &str, digest: &str) -> RunnerResult<()> {
    let entry = manifest
        .get("files")
        .and_then(Value::as_array)
        .and_then(|files| {
            files
                .iter()
                .find(|entry| entry.get("path").and_then(Value::as_str) == Some(path))
        })
        .ok_or_else(|| format!("Runtime payload manifest misses {path}"))?;
    if entry.get("sha256").and_then(Value::as_str) != Some(digest) {
        return Err(format!(
            "installed Runtime digest drifted from payload: {path}"
        ));
    }
    Ok(())
}

fn render_digests(digests: &BTreeMap<String, String>) -> String {
    digests
        .iter()
        .map(|(name, digest)| format!("{digest}  {name}\n"))
        .collect()
}

fn reserve_ports() -> RunnerResult<Ports> {
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

fn port_listening(port: u16) -> bool {
    TcpStream::connect_timeout(
        &format!("127.0.0.1:{port}")
            .parse()
            .expect("loopback address"),
        Duration::from_millis(180),
    )
    .is_ok()
}

fn pid_residue(state: &Path) -> RunnerResult<usize> {
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

fn ensure_remote_linux() -> RunnerResult<()> {
    if std::env::consts::OS != "linux" || std::env::var_os("SSH_CONNECTION").is_none() {
        return Err(
            "installed Runtime host capture requires a managed remote Linux session".into(),
        );
    }
    Ok(())
}

fn canonical_dir(path: &Path, label: &str) -> RunnerResult<PathBuf> {
    fs::canonicalize(path).map_err(|error| format!("failed to resolve {label}: {error}"))
}

fn ensure_regular(path: &Path) -> RunnerResult<()> {
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

fn read_json(path: &Path) -> RunnerResult<Value> {
    let bytes =
        fs::read(path).map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid JSON {}: {error}", path.display()))
}

fn write_json(path: &Path, value: &Value) -> RunnerResult<()> {
    let bytes = serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?;
    fs::write(path, bytes).map_err(|error| format!("failed to write {}: {error}", path.display()))
}

fn text_at<'a>(value: &'a Value, pointer: &str) -> RunnerResult<&'a str> {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("missing string at {pointer}"))
}

fn require_text(value: &Value, pointer: &str, expected: &str) -> RunnerResult<()> {
    if text_at(value, pointer)? == expected {
        Ok(())
    } else {
        Err(format!("unexpected value at {pointer}"))
    }
}

fn number_at(value: &Value, pointer: &str) -> RunnerResult<f64> {
    value
        .pointer(pointer)
        .and_then(Value::as_f64)
        .ok_or_else(|| format!("missing number at {pointer}"))
}

fn path_text(path: &Path) -> RunnerResult<&str> {
    path.to_str()
        .ok_or_else(|| format!("path is not UTF-8: {}", path.display()))
}

fn next_string(args: &mut impl Iterator<Item = OsString>, option: &str) -> RunnerResult<String> {
    args.next()
        .and_then(|value| value.into_string().ok())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{option} requires a UTF-8 value"))
}

fn next_path(args: &mut impl Iterator<Item = OsString>, option: &str) -> RunnerResult<PathBuf> {
    Ok(PathBuf::from(next_string(args, option)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_options_require_absolute_managed_paths() {
        let error = Options::parse(vec![
            "--managed-root".into(),
            "relative".into(),
            "--runtime-root".into(),
            "/tmp/runtime".into(),
            "--detached-source-root".into(),
            "/tmp/source".into(),
            "--out".into(),
            "/tmp/output".into(),
            "--package-version".into(),
            "2.19.0".into(),
        ])
        .err()
        .expect("relative path must fail");
        assert!(error.contains("--managed-root"));
    }

    #[test]
    fn payload_digest_binding_rejects_drift() {
        let manifest = json!({
            "files": [{"path": "bin/kyuubiki-headless", "sha256": "a".repeat(64)}]
        });
        assert!(
            require_manifest_digest(&manifest, "bin/kyuubiki-headless", &"a".repeat(64)).is_ok()
        );
        assert!(
            require_manifest_digest(&manifest, "bin/kyuubiki-headless", &"b".repeat(64)).is_err()
        );
    }
}
