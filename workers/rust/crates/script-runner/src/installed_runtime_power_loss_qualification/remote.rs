use super::model::{HostCapture, build_report, validate_capture};
use super::{Contract, load_contract, valid_version, validate_contract};
use crate::installed_runtime_operational_qualification::remote::{
    RuntimeProvision, prepare_run_root, provision_runtime, remove_run_root,
};
use crate::native_time::utc_timestamp_slug;
use crate::qualification_support::generated_at_unix_ms;
use crate::remote_host::{
    remote_shell_path, scp_from, shell_escape, ssh_status, ssh_success_quiet, valid_ssh_alias,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Component, Path, PathBuf};

type RunnerResult<T> = Result<T, String>;

const SESSION_SCHEMA: &str = "kyuubiki.installed-runtime-power-loss-remote-session/v1";
const DEFAULT_SESSION: &str = "tmp/installed-runtime-power-loss-session.json";

pub(super) fn run(root: &Path, args: Vec<OsString>) -> RunnerResult<u8> {
    let contract = load_contract(root)?;
    validate_contract(root, &contract)?;
    let mut args = args.into_iter();
    let action = args
        .next()
        .and_then(|value| value.into_string().ok())
        .unwrap_or_else(|| "help".to_string());
    if matches!(action.as_str(), "help" | "--help" | "-h") {
        print_usage();
        return Ok(0);
    }
    let options = Options::parse(root, &contract, args)?;
    match action.as_str() {
        "prepare" => prepare(root, &contract, options),
        "reboot" => reboot(root, options),
        "resume" => resume(root, &contract, options),
        "cleanup" => cleanup(root, options),
        other => Err(format!(
            "unknown installed Runtime power-loss remote action: {other}"
        )),
    }
}

struct Options {
    host: Option<String>,
    session_path: PathBuf,
    output: PathBuf,
    output_explicit: bool,
    confirm_reboot: bool,
}

impl Options {
    fn parse(
        root: &Path,
        contract: &Contract,
        args: impl Iterator<Item = OsString>,
    ) -> RunnerResult<Self> {
        let mut host = None;
        let mut session_path = repo_path(root, DEFAULT_SESSION)?;
        let mut output = repo_path(root, &contract.retention.report_path)?;
        let mut output_explicit = false;
        let mut confirm_reboot = false;
        let mut args = args;
        while let Some(arg) = args.next() {
            match arg.to_string_lossy().as_ref() {
                "--host" => host = Some(next_string(&mut args, "--host")?),
                "--session" => {
                    session_path = repo_path(root, &next_string(&mut args, "--session")?)?
                }
                "--out" => {
                    output = repo_path(root, &next_string(&mut args, "--out")?)?;
                    output_explicit = true;
                }
                "--confirm-physical-reboot" => confirm_reboot = true,
                other => {
                    return Err(format!(
                        "unknown installed Runtime power-loss remote option: {other}"
                    ));
                }
            }
        }
        if !session_path.starts_with(root.join("tmp")) {
            return Err("power-loss remote session must stay inside repository tmp".into());
        }
        ensure_tmp_scope(root, &session_path)?;
        if host.as_deref().is_some_and(|host| !valid_ssh_alias(host)) {
            return Err("remote qualification host must be a plain SSH alias".into());
        }
        Ok(Self {
            host,
            session_path,
            output,
            output_explicit,
            confirm_reboot,
        })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Session {
    schema_version: String,
    host: String,
    remote_run_root: String,
    package_version: String,
    report_path: String,
    session_sha256: String,
}

fn prepare(root: &Path, contract: &Contract, options: Options) -> RunnerResult<u8> {
    if options.session_path.exists() {
        return Err(format!(
            "power-loss remote session already exists: {}",
            display(root, &options.session_path)
        ));
    }
    let host = options
        .host
        .or_else(|| std::env::var("KYUUBIKI_LAB_HOST").ok())
        .unwrap_or_else(|| "kyuubiki-lab".to_string());
    if !valid_ssh_alias(&host) {
        return Err("remote qualification host must be a plain SSH alias".into());
    }
    let slug = format!("installed-runtime-power-loss-{}", utc_timestamp_slug());
    let remote_run_root = format!("~/.kyuubiki/lab-runs/{slug}");
    let provision = RuntimeProvision::from_workspace(root, remote_run_root.clone())?;
    if provision.package_version != contract.execution.package_version {
        return Err(format!(
            "workspace version {} does not match power-loss contract {}",
            provision.package_version, contract.execution.package_version
        ));
    }
    prepare_run_root(root, &host, &remote_run_root)?;
    let action = host_action("prepare");
    if let Err(error) = provision_runtime(root, &host, &provision, &action) {
        let _ = remove_run_root(root, &host, &remote_run_root);
        return Err(error);
    }
    let mut session = Session {
        schema_version: SESSION_SCHEMA.to_string(),
        host,
        remote_run_root,
        package_version: provision.package_version,
        report_path: display(root, &options.output),
        session_sha256: String::new(),
    };
    session.session_sha256 = session_digest(&session)?;
    if let Err(error) = validate_session(&session)
        .and_then(|()| write_durable_json(&options.session_path, &session))
    {
        let _ = invoke_host_action(root, &session, "cleanup");
        let _ = remove_run_root(root, &session.host, &session.remote_run_root);
        return Err(error);
    }
    println!(
        "installed Runtime power-loss remote preparation passed; session: {}",
        display(root, &options.session_path)
    );
    println!(
        "next: qualify-installed-runtime-power-loss-remote reboot --session {} --confirm-physical-reboot",
        display(root, &options.session_path)
    );
    Ok(0)
}

fn reboot(root: &Path, options: Options) -> RunnerResult<u8> {
    if !options.confirm_reboot {
        return Err("physical reboot requires --confirm-physical-reboot".into());
    }
    let session = read_session(&options.session_path)?;
    require_requested_host(&session, options.host.as_deref())?;
    if !ssh_success_quiet(root, &session.host, remote_intent_probe(&session))? {
        return Err("remote power-loss intent is not ready; refusing reboot".into());
    }
    let status = ssh_status(root, &session.host, "sudo -n systemctl reboot".to_string())?;
    if status != 0 && status != 255 {
        return Err(format!(
            "physical reboot command failed with status {status}"
        ));
    }
    println!(
        "physical reboot requested; after the host returns run resume with session {}",
        display(root, &options.session_path)
    );
    Ok(0)
}

fn resume(root: &Path, contract: &Contract, options: Options) -> RunnerResult<u8> {
    let session = read_session(&options.session_path)?;
    require_requested_host(&session, options.host.as_deref())?;
    let output = if options.output_explicit {
        options.output.clone()
    } else {
        repo_path(root, &session.report_path)?
    };
    invoke_host_action(root, &session, "resume")?;
    let local_capture = options.session_path.with_extension("capture.json");
    require_zero(
        "retrieve installed Runtime power-loss capture",
        scp_from(
            root,
            &session.host,
            &format!(
                "{}/power-loss-state/resume-capture.json",
                session.remote_run_root
            ),
            &local_capture,
        )?,
    )?;
    let capture: HostCapture = read_json_file(&local_capture)?;
    validate_capture(&capture)?;
    remove_run_root(root, &session.host, &session.remote_run_root)?;
    let report = build_report(
        capture,
        generated_at_unix_ms()?,
        &contract.retention.forbidden_content,
    )?;
    write_durable_json(&output, &report)?;
    remove_local_file(&local_capture)?;
    remove_local_file(&options.session_path)?;
    println!(
        "installed Runtime physical reboot qualification passed: {}",
        display(root, &output)
    );
    Ok(0)
}

fn cleanup(root: &Path, options: Options) -> RunnerResult<u8> {
    let session = read_session(&options.session_path)?;
    require_requested_host(&session, options.host.as_deref())?;
    invoke_host_action(root, &session, "cleanup")?;
    remove_run_root(root, &session.host, &session.remote_run_root)?;
    remove_local_file(&options.session_path)?;
    let local_capture = options.session_path.with_extension("capture.json");
    if local_capture.exists() {
        remove_local_file(&local_capture)?;
    }
    println!("installed Runtime power-loss remote state removed");
    Ok(0)
}

fn invoke_host_action(root: &Path, session: &Session, action: &str) -> RunnerResult<()> {
    require_zero(
        &format!("run installed Runtime power-loss host {action}"),
        ssh_status(root, &session.host, remote_host_command(session, action))?,
    )
}

fn remote_host_command(session: &Session, action: &str) -> String {
    let run_root = remote_shell_path(&session.remote_run_root);
    let version = shell_escape(&session.package_version);
    format!(
        "set -euo pipefail; run_root={run_root}; runner=\"$run_root/bin/kyuubiki-script-runner\"; runtime=\"$run_root/xdg/kyuubiki/runtime/versions/{version_path}\"; source_root=\"$run_root/source\"; harness=\"$run_root/harness-repo\"; test -x \"$runner\"; test -d \"$runtime\"; test ! -e \"$source_root\"; KYUUBIKI_REPO_ROOT=\"$harness\" \"$runner\" installed-runtime-power-loss-host {action} --managed-root \"$run_root\" --runtime-root \"$runtime\" --detached-source-root \"$source_root\" --package-version {version}",
        version_path = session.package_version
    )
}

fn host_action(action: &str) -> String {
    format!(
        "KYUUBIKI_REPO_ROOT=\"$harness\" \"$runner\" installed-runtime-power-loss-host {action} --managed-root \"$run_root\" --runtime-root \"$runtime\" --detached-source-root \"$source_root\" --package-version \"$package_version\""
    )
}

fn remote_intent_probe(session: &Session) -> String {
    let run_root = remote_shell_path(&session.remote_run_root);
    format!(
        "set -eu; run_root={run_root}; test -f \"$run_root/power-loss-state/intent.json\"; test -x \"$run_root/bin/kyuubiki-script-runner\""
    )
}

fn read_session(path: &Path) -> RunnerResult<Session> {
    let session: Session = read_json_file(path)?;
    validate_session(&session)?;
    Ok(session)
}

fn validate_session(session: &Session) -> RunnerResult<()> {
    if session.schema_version != SESSION_SCHEMA
        || !valid_ssh_alias(&session.host)
        || !session
            .remote_run_root
            .starts_with("~/.kyuubiki/lab-runs/installed-runtime-power-loss-")
        || session.remote_run_root.contains("..")
        || !valid_version(&session.package_version)
        || session.report_path.is_empty()
        || Path::new(&session.report_path).is_absolute()
        || Path::new(&session.report_path)
            .components()
            .any(|component| {
                matches!(
                    component,
                    Component::ParentDir | Component::RootDir | Component::Prefix(_)
                )
            })
        || session.session_sha256 != session_digest(session)?
    {
        return Err("installed Runtime power-loss remote session is invalid".into());
    }
    Ok(())
}

fn session_digest(session: &Session) -> RunnerResult<String> {
    let payload = (
        &session.schema_version,
        &session.host,
        &session.remote_run_root,
        &session.package_version,
        &session.report_path,
    );
    let bytes = serde_json::to_vec(&payload).map_err(|error| error.to_string())?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn require_requested_host(session: &Session, requested: Option<&str>) -> RunnerResult<()> {
    if requested.is_some_and(|host| host != session.host) {
        return Err("requested host does not match the sealed remote session".into());
    }
    Ok(())
}

fn repo_path(root: &Path, relative: &str) -> RunnerResult<PathBuf> {
    let path = Path::new(relative);
    if path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(format!("qualification path escapes repository: {relative}"));
    }
    Ok(root.join(path))
}

fn ensure_tmp_scope(root: &Path, path: &Path) -> RunnerResult<()> {
    let tmp = fs::canonicalize(root.join("tmp"))
        .map_err(|error| format!("failed to resolve repository tmp: {error}"))?;
    let mut ancestor = path.parent().ok_or("session path has no parent")?;
    while !ancestor.exists() {
        ancestor = ancestor
            .parent()
            .ok_or("session path has no existing ancestor")?;
    }
    let ancestor = fs::canonicalize(ancestor)
        .map_err(|error| format!("failed to resolve session parent: {error}"))?;
    if !ancestor.starts_with(&tmp) {
        return Err("power-loss remote session resolves outside repository tmp".into());
    }
    Ok(())
}

fn write_durable_json(path: &Path, value: &impl Serialize) -> RunnerResult<()> {
    let parent = path.parent().ok_or("output path has no parent")?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or("output path has an invalid name")?;
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
        .map_err(|error| format!("failed to sync output directory: {error}"))
}

fn read_json_file<T: for<'de> Deserialize<'de>>(path: &Path) -> RunnerResult<T> {
    let bytes =
        fs::read(path).map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid JSON {}: {error}", path.display()))
}

fn remove_local_file(path: &Path) -> RunnerResult<()> {
    fs::remove_file(path).map_err(|error| format!("failed to remove {}: {error}", path.display()))
}

fn next_string(args: &mut impl Iterator<Item = OsString>, option: &str) -> RunnerResult<String> {
    args.next()
        .and_then(|value| value.into_string().ok())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{option} requires a UTF-8 value"))
}

fn require_zero(label: &str, status: u8) -> RunnerResult<()> {
    if status == 0 {
        Ok(())
    } else {
        Err(format!("{label} failed with status {status}"))
    }
}

fn display(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn print_usage() {
    println!(
        "usage: kyuubiki-script-runner qualify-installed-runtime-power-loss-remote prepare [--host SSH_ALIAS] [--session path] [--out report]\n       kyuubiki-script-runner qualify-installed-runtime-power-loss-remote reboot [--session path] --confirm-physical-reboot\n       kyuubiki-script-runner qualify-installed-runtime-power-loss-remote resume [--session path] [--out report]\n       kyuubiki-script-runner qualify-installed-runtime-power-loss-remote cleanup [--session path]"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reboot_requires_explicit_confirmation() {
        let options = Options {
            host: None,
            session_path: PathBuf::from("missing"),
            output: PathBuf::from("report"),
            output_explicit: false,
            confirm_reboot: false,
        };
        assert!(reboot(Path::new("."), options).is_err());
    }

    #[test]
    fn host_action_uses_only_installed_runner_after_detach() {
        let action = host_action("prepare");
        assert!(action.contains("installed-runtime-power-loss-host prepare"));
        assert!(action.contains("$runner"));
        assert!(!action.contains("cargo "));
        assert!(!action.contains("node "));
    }

    #[test]
    fn remote_session_digest_rejects_mutation() {
        let mut session = Session {
            schema_version: SESSION_SCHEMA.into(),
            host: "lab".into(),
            remote_run_root: "~/.kyuubiki/lab-runs/installed-runtime-power-loss-test".into(),
            package_version: "2.19.0".into(),
            report_path: "tmp/report.json".into(),
            session_sha256: String::new(),
        };
        session.session_sha256 = session_digest(&session).expect("digest");
        assert!(validate_session(&session).is_ok());
        session.report_path = "tmp/forged.json".into();
        assert!(validate_session(&session).is_err());
    }
}
