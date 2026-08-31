use super::model::{
    HostCapture, NumericalResult, PowerLossIntent, Preparation, ProcessIdentity, Recovery,
    RuntimeCleanup, seal_intent, validate_intent,
};
use super::{CAPTURE_SCHEMA, QUALIFICATION_ID, valid_version};
use crate::installed_runtime_operational_qualification::support::{
    Ports, RuntimeGuard, WORKFLOW_ID, canonical_dir, ensure_remote_linux, execution_result,
    headless_command, path_text, pid_residue, port_listening, read_json, reserve_ports,
    run_headless_workflow, run_success, runtime_env, runtime_pids, sha256_file, validate_fetch,
    validate_status, verify_installation, write_json,
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

type RunnerResult<T> = Result<T, String>;

const INTENT_FILE: &str = "power-loss-state/intent.json";
const CAPTURE_FILE: &str = "power-loss-state/resume-capture.json";

pub(super) fn run(args: Vec<OsString>) -> RunnerResult<u8> {
    ensure_remote_linux()?;
    let mut args = args.into_iter();
    let action = args
        .next()
        .and_then(|value| value.into_string().ok())
        .unwrap_or_else(|| "help".to_string());
    if matches!(action.as_str(), "help" | "--help" | "-h") {
        print_usage();
        return Ok(0);
    }
    let options = Options::parse(args)?;
    match action.as_str() {
        "prepare" => prepare(options),
        "resume" => resume(options),
        "cleanup" => cleanup(options),
        other => Err(format!(
            "unknown installed Runtime power-loss host action: {other}"
        )),
    }
}

struct Options {
    managed_root: PathBuf,
    runtime_root: PathBuf,
    detached_source_root: PathBuf,
    package_version: String,
}

impl Options {
    fn parse(args: impl Iterator<Item = OsString>) -> RunnerResult<Self> {
        let mut managed_root = None;
        let mut runtime_root = None;
        let mut detached_source_root = None;
        let mut package_version = None;
        let mut args = args;
        while let Some(arg) = args.next() {
            match arg.to_string_lossy().as_ref() {
                "--managed-root" => managed_root = Some(next_path(&mut args, "--managed-root")?),
                "--runtime-root" => runtime_root = Some(next_path(&mut args, "--runtime-root")?),
                "--detached-source-root" => {
                    detached_source_root = Some(next_path(&mut args, "--detached-source-root")?)
                }
                "--package-version" => {
                    package_version = Some(next_string(&mut args, "--package-version")?)
                }
                other => {
                    return Err(format!(
                        "unknown installed Runtime power-loss host option: {other}"
                    ));
                }
            }
        }
        let options = Self {
            managed_root: managed_root.ok_or("--managed-root is required")?,
            runtime_root: runtime_root.ok_or("--runtime-root is required")?,
            detached_source_root: detached_source_root
                .ok_or("--detached-source-root is required")?,
            package_version: package_version.ok_or("--package-version is required")?,
        };
        for (label, path) in [
            ("--managed-root", &options.managed_root),
            ("--runtime-root", &options.runtime_root),
            ("--detached-source-root", &options.detached_source_root),
        ] {
            if !path.is_absolute() {
                return Err(format!("{label} requires an absolute path"));
            }
        }
        if !valid_version(&options.package_version) {
            return Err("--package-version is invalid".into());
        }
        Ok(options)
    }

    fn scope(&self) -> RunnerResult<(PathBuf, PathBuf)> {
        if self.detached_source_root.exists() {
            return Err("source tree must remain detached across power-loss qualification".into());
        }
        let managed = canonical_dir(&self.managed_root, "managed power-loss root")?;
        let runtime = canonical_dir(&self.runtime_root, "installed Runtime root")?;
        if !runtime.starts_with(&managed) {
            return Err("installed Runtime escapes the managed power-loss root".into());
        }
        let source_parent = self
            .detached_source_root
            .parent()
            .ok_or("detached source path has no parent")?;
        if canonical_dir(source_parent, "detached source parent")? != managed {
            return Err("detached source path escapes the managed power-loss root".into());
        }
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .ok_or("HOME is unavailable")?;
        let lab_runs = home.join(".kyuubiki/lab-runs");
        let lab_runs = canonical_dir(&lab_runs, "managed lab-runs root")?;
        if !managed.starts_with(&lab_runs) || managed == lab_runs {
            return Err("managed power-loss root is outside the lab-runs sandbox".into());
        }
        Ok((managed, runtime))
    }
}

fn prepare(options: Options) -> RunnerResult<u8> {
    let (managed, runtime) = options.scope()?;
    let intent_path = managed.join(INTENT_FILE);
    if intent_path.exists() {
        return Err("installed Runtime power-loss intent already exists".into());
    }
    let payload_digests = verify_installation(&runtime, &options.package_version)?;
    let ports = reserve_ports()?;
    let state_root = managed.join("runtime-state");
    let work_root = managed.join("power-loss-work");
    let home = managed.join("home");
    fs::create_dir_all(&work_root)
        .and_then(|()| fs::create_dir_all(&home))
        .map_err(|error| format!("failed to create power-loss work roots: {error}"))?;
    let runtime_binary = runtime.join("bin/kyuubiki-runtime");
    let headless_binary = runtime.join("bin/kyuubiki-headless");
    let env = runtime_env(&runtime, &state_root, &home, ports);
    let mut guard = RuntimeGuard::new(runtime_binary, env.clone(), ports);
    guard.command("start-local")?;
    validate_status(&guard.command("status")?, ports)?;

    let workflow = work_root.join("workflow.json");
    run_success(
        "initialize pre-reboot installed Headless workflow",
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
    let solve_path = work_root.join("pre-reboot-solve.json");
    run_headless_workflow(
        &headless_binary,
        &env,
        &work_root,
        &workflow,
        &solve_path,
        ports.orchestrator,
    )?;
    let (job_id, tip, stress) = execution_result(&read_json(&solve_path)?)?;
    write_fetch_workflow(&work_root.join("fetch-workflow.json"), &job_id)?;
    sync_tree(&state_root.join("data"))?;
    sync_tree(&work_root)?;

    let pids = runtime_pids(&state_root, ports)?;
    let process_identities = process_identities(&pids)?;
    let preparation = Preparation {
        qualification_id: QUALIFICATION_ID.to_string(),
        execution_host_role: "remote-linux-qualification-host".to_string(),
        platform: "linux".to_string(),
        architecture: std::env::consts::ARCH.to_string(),
        runtime_version: options.package_version,
        prepared_at_unix_ms: unix_now_ms()?,
        machine_id_sha256: identity_digest(Path::new("/etc/machine-id"))?,
        pre_boot_id_sha256: identity_digest(Path::new("/proc/sys/kernel/random/boot_id"))?,
        pre_uptime_seconds: uptime_seconds()?,
        source_tree_detached: true,
        payload_digests,
        ports,
        process_identities,
        workflow_id: WORKFLOW_ID.to_string(),
        job_id,
        numerical_result: NumericalResult {
            tip_displacement: tip,
            max_stress: stress,
        },
        runtime_status_verified: true,
        agent_count: 2,
    };
    let intent = seal_intent(preparation)?;
    write_durable_json(&intent_path, &intent)?;
    guard.leave_running();
    println!(
        "installed Runtime power-loss qualification prepared: runtime {}, job {}, {} managed processes; reboot the physical host before resume",
        intent.payload.runtime_version,
        intent.payload.job_id,
        intent.payload.process_identities.len()
    );
    Ok(0)
}

fn resume(options: Options) -> RunnerResult<u8> {
    let (managed, runtime) = options.scope()?;
    let capture_path = managed.join(CAPTURE_FILE);
    if capture_path.exists() {
        return Err("installed Runtime power-loss resume capture already exists".into());
    }
    let intent: PowerLossIntent = read_typed_json(&managed.join(INTENT_FILE))?;
    validate_intent(&intent)?;
    if intent.payload.runtime_version != options.package_version {
        return Err("installed Runtime version changed after preparation".into());
    }
    let post_boot_id = identity_digest(Path::new("/proc/sys/kernel/random/boot_id"))?;
    let post_machine_id = identity_digest(Path::new("/etc/machine-id"))?;
    if post_boot_id == intent.payload.pre_boot_id_sha256 {
        return Err("physical host boot identity did not change; reboot is required".into());
    }
    if post_machine_id != intent.payload.machine_id_sha256 {
        return Err("host machine identity changed across the reboot boundary".into());
    }
    let interrupted = intent
        .payload
        .process_identities
        .iter()
        .filter(|identity| !exact_process_alive(identity))
        .count();
    if interrupted != intent.payload.process_identities.len() {
        return Err("a pre-reboot Runtime process identity remains alive".into());
    }
    if intent.payload.ports.any_listening() {
        return Err("a pre-reboot Runtime port remains occupied".into());
    }
    let payload_digests = verify_installation(&runtime, &options.package_version)?;
    if payload_digests != intent.payload.payload_digests {
        return Err("installed Runtime payload changed across reboot".into());
    }

    let state_root = managed.join("runtime-state");
    let work_root = managed.join("power-loss-work");
    let home = managed.join("home");
    let env = runtime_env(&runtime, &state_root, &home, intent.payload.ports);
    let runtime_binary = runtime.join("bin/kyuubiki-runtime");
    let headless_binary = runtime.join("bin/kyuubiki-headless");
    let mut guard = RuntimeGuard::new(runtime_binary, env.clone(), intent.payload.ports);
    guard.command("start-local")?;
    validate_status(&guard.command("status")?, intent.payload.ports)?;
    let fetch_path = work_root.join("post-reboot-fetch.json");
    run_headless_workflow(
        &headless_binary,
        &env,
        &work_root,
        &work_root.join("fetch-workflow.json"),
        &fetch_path,
        intent.payload.ports.orchestrator,
    )?;
    let fetch = read_json(&fetch_path)?;
    validate_fetch(
        &fetch,
        &intent.payload.job_id,
        intent.payload.numerical_result.tip_displacement,
        intent.payload.numerical_result.max_stress,
    )?;
    guard.stop()?;
    let residue = pid_residue(&state_root)? as u64;
    let ports_closed = !intent.payload.ports.any_listening();
    if residue != 0 || !ports_closed {
        return Err("installed Runtime cleanup left reboot qualification residue".into());
    }
    let capture = HostCapture {
        schema_version: CAPTURE_SCHEMA.to_string(),
        preparation: intent.payload,
        intent_sha256: intent.intent_sha256,
        recovery: Recovery {
            post_boot_id_sha256: post_boot_id,
            post_machine_id_sha256: post_machine_id,
            post_uptime_seconds: uptime_seconds()?,
            boot_identity_changed: true,
            same_machine: true,
            interrupted_process_count: interrupted as u64,
            pre_reboot_ports_released: true,
            payload_digests,
            source_tree_detached: true,
            runtime_policy: "installer-managed".to_string(),
            agent_count: 2,
            job_id: text_at(&fetch, "/steps/0/result_preview/job_id")?.to_string(),
            job_status: text_at(&fetch, "/steps/0/result_preview/status")?.to_string(),
            numerical_result: NumericalResult {
                tip_displacement: number_at(
                    &fetch,
                    "/steps/0/result_preview/result/tip_displacement",
                )?,
                max_stress: number_at(&fetch, "/steps/0/result_preview/result/max_stress")?,
            },
        },
        runtime_cleanup: RuntimeCleanup {
            runtime_stopped: true,
            ports_closed,
            pid_files_removed: residue == 0,
            residue_count: residue,
        },
    };
    write_durable_json(&capture_path, &capture)?;
    println!(
        "installed Runtime power-loss qualification resumed: job {} retained after physical reboot",
        capture.recovery.job_id
    );
    Ok(0)
}

fn cleanup(options: Options) -> RunnerResult<u8> {
    let (managed, runtime) = options.scope()?;
    let intent: PowerLossIntent = read_typed_json(&managed.join(INTENT_FILE))?;
    validate_intent(&intent)?;
    if intent.payload.runtime_version != options.package_version {
        return Err("cleanup Runtime version does not match the sealed intent".into());
    }
    if intent.payload.ports.any_listening() {
        let current_boot = identity_digest(Path::new("/proc/sys/kernel/random/boot_id"))?;
        let unverified_listener = intent.payload.process_identities.iter().any(|identity| {
            process_port(identity, intent.payload.ports)
                .is_some_and(|port| port_listening(port) && !exact_process_alive(identity))
        });
        if current_boot != intent.payload.pre_boot_id_sha256 || unverified_listener {
            return Err(
                "Runtime ports are occupied by an unverified process; refusing cleanup".into(),
            );
        }
        let env = runtime_env(
            &runtime,
            &managed.join("runtime-state"),
            &managed.join("home"),
            intent.payload.ports,
        );
        let mut guard = RuntimeGuard::new(
            runtime.join("bin/kyuubiki-runtime"),
            env,
            intent.payload.ports,
        );
        guard.command("stop")?;
    }
    if intent.payload.ports.any_listening() {
        return Err("Runtime ports remain occupied after cleanup".into());
    }
    println!("installed Runtime power-loss host state is safe to remove");
    Ok(0)
}

fn process_port(identity: &ProcessIdentity, ports: Ports) -> Option<u16> {
    match identity.role.as_str() {
        "orchestrator" => Some(ports.orchestrator),
        "agent_one" => Some(ports.agent_one),
        "agent_two" => Some(ports.agent_two),
        _ => None,
    }
}

fn process_identities(pids: &BTreeMap<String, u32>) -> RunnerResult<Vec<ProcessIdentity>> {
    pids.iter()
        .map(|(role, pid)| {
            let executable = fs::read_link(format!("/proc/{pid}/exe"))
                .map_err(|error| format!("failed to inspect Runtime process {pid}: {error}"))?;
            Ok(ProcessIdentity {
                role: role.clone(),
                process_id: *pid,
                executable_sha256: sha256_file(&executable)?,
            })
        })
        .collect()
}

fn exact_process_alive(identity: &ProcessIdentity) -> bool {
    fs::read_link(format!("/proc/{}/exe", identity.process_id))
        .ok()
        .and_then(|path| sha256_file(&path).ok())
        .is_some_and(|digest| digest == identity.executable_sha256)
}

fn write_fetch_workflow(path: &Path, job_id: &str) -> RunnerResult<()> {
    write_json(
        path,
        &json!({
            "schema_version": "kyuubiki.headless-workflow/v1",
            "exported_at": "1970-01-01T00:00:00.000Z",
            "language": "en-US",
            "workflow": {
                "id": format!("{WORKFLOW_ID}.power-loss-fetch"),
                "steps": [{"action": "result_fetch", "payload": {"job_id": job_id}}]
            }
        }),
    )
}

fn sync_tree(root: &Path) -> RunnerResult<()> {
    if !root.exists() {
        return Err(format!("durable state tree is missing: {}", root.display()));
    }
    let metadata = root
        .symlink_metadata()
        .map_err(|error| format!("failed to inspect {}: {error}", root.display()))?;
    if metadata.file_type().is_symlink() {
        return Err(format!(
            "durable state contains a symlink: {}",
            root.display()
        ));
    }
    if metadata.is_file() {
        return File::open(root)
            .and_then(|file| file.sync_all())
            .map_err(|error| format!("failed to sync {}: {error}", root.display()));
    }
    if !metadata.is_dir() {
        return Err(format!(
            "durable state contains a special file: {}",
            root.display()
        ));
    }
    for entry in
        fs::read_dir(root).map_err(|error| format!("failed to read {}: {error}", root.display()))?
    {
        sync_tree(&entry.map_err(|error| error.to_string())?.path())?;
    }
    File::open(root)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| format!("failed to sync {}: {error}", root.display()))
}

fn identity_digest(path: &Path) -> RunnerResult<String> {
    let value = fs::read_to_string(path)
        .map_err(|error| format!("failed to read Linux host identity: {error}"))?;
    let value = value.trim();
    if value.is_empty() {
        return Err("Linux host identity is empty".into());
    }
    Ok(format!("{:x}", Sha256::digest(value.as_bytes())))
}

fn uptime_seconds() -> RunnerResult<u64> {
    fs::read_to_string("/proc/uptime")
        .map_err(|error| format!("failed to read Linux uptime: {error}"))?
        .split_whitespace()
        .next()
        .and_then(|value| value.split('.').next())
        .ok_or_else(|| "Linux uptime is invalid".to_string())?
        .parse()
        .map_err(|error| format!("Linux uptime is invalid: {error}"))
}

fn unix_now_ms() -> RunnerResult<u128> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .map_err(|error| format!("system clock is before Unix epoch: {error}"))
}

fn write_durable_json(path: &Path, value: &impl Serialize) -> RunnerResult<()> {
    let parent = path.parent().ok_or("durable output has no parent")?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or("durable output has an invalid name")?;
    let staged = parent.join(format!(".{name}.{}.tmp", std::process::id()));
    let bytes = serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?;
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&staged)
        .map_err(|error| format!("failed to stage {}: {error}", staged.display()))?;
    file.write_all(&bytes).map_err(|error| error.to_string())?;
    file.sync_all().map_err(|error| error.to_string())?;
    fs::rename(&staged, path)
        .map_err(|error| format!("failed to promote {}: {error}", path.display()))?;
    File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| format!("failed to sync durable output directory: {error}"))
}

fn read_typed_json<T: DeserializeOwned>(path: &Path) -> RunnerResult<T> {
    let bytes =
        fs::read(path).map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid JSON {}: {error}", path.display()))
}

fn text_at<'a>(value: &'a serde_json::Value, pointer: &str) -> RunnerResult<&'a str> {
    value
        .pointer(pointer)
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| format!("missing string at {pointer}"))
}

fn number_at(value: &serde_json::Value, pointer: &str) -> RunnerResult<f64> {
    value
        .pointer(pointer)
        .and_then(serde_json::Value::as_f64)
        .ok_or_else(|| format!("missing number at {pointer}"))
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

fn print_usage() {
    println!(
        "usage: kyuubiki-script-runner installed-runtime-power-loss-host prepare|resume|cleanup --managed-root PATH --runtime-root PATH --detached-source-root PATH --package-version VERSION"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_options_require_absolute_paths() {
        let error = Options::parse(
            vec![
                "--managed-root".into(),
                "relative".into(),
                "--runtime-root".into(),
                "/tmp/runtime".into(),
                "--detached-source-root".into(),
                "/tmp/source".into(),
                "--package-version".into(),
                "2.19.0".into(),
            ]
            .into_iter(),
        )
        .err()
        .expect("relative root must fail");
        assert!(error.contains("--managed-root"));
    }
}
