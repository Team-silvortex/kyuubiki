use crate::native_time::utc_timestamp_slug;
use crate::operator_package_dynamic_smoke::{
    dynamic_smoke_errors, run_operator_package_dynamic_smoke,
};
use crate::qualification_support::{generated_at_unix_ms, read_json, repo_path, write_json};
use crate::remote_host::{
    remote_shell_path, rsync_to, scp_from, ssh_output, ssh_status, valid_ssh_alias,
};
use crate::{RepoPaths, RunnerResult};
use serde_json::{Value, json};
use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

mod validation;

use validation::{
    DEFAULT_REPORT, QUALIFICATION_ID, REPORT_SCHEMA, REQUIRED_CHECKS, REQUIRED_STAGES, digest,
    validate_contract, validate_report, validator_self_test,
};

pub(crate) fn run_check(root: &Path, args: Vec<OsString>) -> RunnerResult<u8> {
    let options = CheckOptions::parse(args)?;
    if options.help {
        println!(
            "usage: kyuubiki-script-runner check-operator-sdk-multihost-operational-qualification [--self-test] [--verify-report path]"
        );
        return Ok(0);
    }
    validate_contract(root, !options.self_test || options.report.is_some())?;
    if options.self_test {
        validator_self_test(root)?;
        println!("Operator SDK multihost qualification self-test passed");
        if options.report.is_none() {
            return Ok(0);
        }
    }
    let report_path = options.report.as_deref().unwrap_or(DEFAULT_REPORT);
    let report: Value = read_json(root, report_path)?;
    validate_report(root, &report)?;
    println!("Operator SDK multihost qualification report passed: {report_path}");
    Ok(0)
}

pub(crate) fn run_qualify_remote(paths: &RepoPaths, args: Vec<OsString>) -> RunnerResult<u8> {
    let options = Options::parse(&paths.root, args)?;
    if options.help {
        println!(
            "usage: kyuubiki-script-runner qualify-operator-sdk-multihost-operational-remote [--host alias] [--out path]"
        );
        return Ok(0);
    }
    validate_contract(&paths.root, false)?;
    validate_local_platform()?;
    fs::create_dir_all(&options.staging_root).map_err(|error| {
        format!(
            "failed to create {}: {error}",
            options.staging_root.display()
        )
    })?;

    let result = capture_and_retain(paths, &options);
    let local_cleanup = fs::remove_dir_all(&options.staging_root)
        .map_err(|error| format!("failed to remove local qualification staging: {error}"));
    match (result, local_cleanup) {
        (Ok(code), Ok(())) => Ok(code),
        (Err(error), Ok(())) => Err(error),
        (Ok(code), Err(cleanup)) => Err(format!(
            "qualification exited with status {code}; {cleanup}"
        )),
        (Err(error), Err(cleanup)) => Err(format!("{error}; {cleanup}")),
    }
}

fn capture_and_retain(paths: &RepoPaths, options: &Options) -> RunnerResult<u8> {
    for parent in [
        options.staged_local_report.parent(),
        options.staged_remote_report.parent(),
    ]
    .into_iter()
    .flatten()
    {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    let local_status = run_operator_package_dynamic_smoke(
        paths,
        vec![
            OsString::from("--out"),
            options.staged_local_report.clone().into_os_string(),
        ],
    )?;
    if local_status != 0 {
        return Ok(local_status);
    }

    prepare_remote_run(&paths.root, options)?;
    let remote_capture = capture_remote(&paths.root, options);
    let remote_cleanup = cleanup_remote_run(&paths.root, options);
    match (remote_capture, remote_cleanup) {
        (Ok(0), Ok(())) => {}
        (Ok(status), Ok(())) => return Ok(status),
        (Err(error), Ok(())) => return Err(error),
        (Ok(status), Err(cleanup)) => {
            return Err(format!(
                "remote capture exited with status {status}; {cleanup}"
            ));
        }
        (Err(error), Err(cleanup)) => return Err(format!("{error}; {cleanup}")),
    }

    let paths_out = RetainedPaths::new(&paths.root, &options.output)?;
    let local_report = normalize_dynamic_report(
        &paths.root,
        &options.staged_local_report,
        &paths_out.local_preflight,
    )?;
    let remote_report = normalize_dynamic_report(
        &paths.root,
        &options.staged_remote_report,
        &paths_out.remote_preflight,
    )?;
    validate_host_reports(&paths.root, &local_report, &remote_report)?;
    promote_capture(
        &options.staged_local_report,
        &paths_out.local_report_absolute,
    )?;
    promote_capture(
        &options.staged_local_preflight,
        &paths_out.local_preflight_absolute,
    )?;
    promote_capture(
        &options.staged_remote_report,
        &paths_out.remote_report_absolute,
    )?;
    promote_capture(
        &options.staged_remote_preflight,
        &paths_out.remote_preflight_absolute,
    )?;

    let report = build_report(&paths.root, &paths_out, &local_report)?;
    write_json(&paths.root, &paths_out.output, &report)?;
    validate_report(&paths.root, &report)?;
    println!(
        "Operator SDK macOS/Linux multihost qualification passed: {}",
        paths_out.output
    );
    Ok(0)
}

fn capture_remote(root: &Path, options: &Options) -> RunnerResult<u8> {
    require_zero(
        "sync native command entrypoints",
        rsync_to(
            root,
            &[".DS_Store"],
            &[root.join("scripts/")],
            &format!("{}:{}/scripts/", options.host, options.remote_run_root),
        )?,
    )?;
    require_zero(
        "sync Rust workspace",
        rsync_to(
            root,
            &["target/", "tmp/", ".DS_Store"],
            &[root.join("workers/rust/")],
            &format!("{}:{}/workers/rust/", options.host, options.remote_run_root),
        )?,
    )?;
    require_zero(
        "execute remote Linux dynamic package journey",
        ssh_status(root, &options.host, remote_capture_command(options))?,
    )?;
    let platform = ssh_output(root, &options.host, "uname -s; uname -m".to_string())?;
    if platform.lines().collect::<Vec<_>>() != ["Linux", "x86_64"] {
        return Err("remote Operator SDK qualification requires Linux x86_64".to_string());
    }
    require_zero(
        "retrieve remote dynamic smoke report",
        scp_from(
            root,
            &options.host,
            &format!("{}/report.json", options.remote_run_root),
            &options.staged_remote_report,
        )?,
    )?;
    require_zero(
        "retrieve remote preflight report",
        scp_from(
            root,
            &options.host,
            &format!(
                "{}/operator-package-dynamic-preflight.json",
                options.remote_run_root
            ),
            &options.staged_remote_preflight,
        )?,
    )?;
    Ok(0)
}

fn prepare_remote_run(root: &Path, options: &Options) -> RunnerResult<()> {
    let run_root = remote_shell_path(&options.remote_run_root);
    require_zero(
        "prepare managed remote qualification root",
        ssh_status(
            root,
            &options.host,
            format!("set -eu; umask 077; mkdir -p {run_root}/scripts {run_root}/workers/rust"),
        )?,
    )
}

fn remote_capture_command(options: &Options) -> String {
    let run_root = remote_shell_path(&options.remote_run_root);
    format!(
        "set -eu; umask 077; run_root={run_root}; \
cd \"$run_root/workers/rust\"; cargo build --locked -p kyuubiki-script-runner; \
KYUUBIKI_REPO_ROOT=\"$run_root\" \"$run_root/workers/rust/target/debug/kyuubiki-script-runner\" \
operator-package-dynamic-smoke --out \"$run_root/report.json\""
    )
}

fn cleanup_remote_run(root: &Path, options: &Options) -> RunnerResult<()> {
    let run_root = remote_shell_path(&options.remote_run_root);
    require_zero(
        "remove managed remote qualification root",
        ssh_status(
            root,
            &options.host,
            format!(
                "set -eu; run_root={run_root}; case \"$run_root\" in \
\"$HOME/.local/state/kyuubiki/lab-runs/\"*) rm -rf \"$run_root\" ;; \
*) echo 'refusing unmanaged cleanup root' >&2; exit 2 ;; esac"
            ),
        )?,
    )
}

fn normalize_dynamic_report(
    root: &Path,
    staged_report: &Path,
    retained_preflight: &str,
) -> RunnerResult<Value> {
    let bytes = fs::read(staged_report)
        .map_err(|error| format!("failed to read {}: {error}", staged_report.display()))?;
    let mut report: Value = serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid dynamic smoke report: {error}"))?;
    report["preflight_report"] = Value::String(retained_preflight.to_string());
    if let Some(error) = dynamic_smoke_errors(root, &report, "multihost capture").first() {
        return Err(error.clone());
    }
    let rendered = serde_json::to_string_pretty(&report)
        .map_err(|error| format!("failed to encode normalized dynamic report: {error}"))?;
    fs::write(staged_report, format!("{rendered}\n"))
        .map_err(|error| format!("failed to normalize {}: {error}", staged_report.display()))?;
    Ok(report)
}

fn validate_host_reports(root: &Path, local: &Value, remote: &Value) -> RunnerResult<()> {
    for (label, report) in [("local macOS", local), ("remote Linux", remote)] {
        if let Some(error) = dynamic_smoke_errors(root, report, label).first() {
            return Err(error.clone());
        }
    }
    for field in [
        "package_id",
        "operator_ids",
        "host_version",
        "sdk_api_version",
    ] {
        if local.get(field) != remote.get(field) {
            return Err(format!("Operator SDK {field} differs across host captures"));
        }
    }
    Ok(())
}

fn build_report(root: &Path, paths: &RetainedPaths, local: &Value) -> RunnerResult<Value> {
    let host = |role: &str,
                platform: &str,
                architecture: &str,
                capture_kind: &str,
                report_path: &str,
                preflight_path: &str|
     -> RunnerResult<Value> {
        let report_bytes = fs::read(repo_path(root, report_path)?)
            .map_err(|error| format!("failed to read {report_path}: {error}"))?;
        let preflight_bytes = fs::read(repo_path(root, preflight_path)?)
            .map_err(|error| format!("failed to read {preflight_path}: {error}"))?;
        Ok(json!({
            "role": role,
            "platform": platform,
            "architecture": architecture,
            "capture_kind": capture_kind,
            "report_path": report_path,
            "report_sha256": digest(&report_bytes),
            "preflight_path": preflight_path,
            "preflight_sha256": digest(&preflight_bytes),
            "stage_ids": REQUIRED_STAGES,
            "stage_count": REQUIRED_STAGES.len(),
            "all_stages_passed": true
        }))
    };
    Ok(json!({
        "schema_version": REPORT_SCHEMA,
        "generated_at_unix_ms": generated_at_unix_ms()?,
        "status": "pass",
        "qualification_id": QUALIFICATION_ID,
        "scope": {
            "completed_platforms": ["macos", "linux"],
            "deferred_platforms": ["windows"],
            "release_complete": false
        },
        "package": {
            "package_id": local["package_id"],
            "operator_ids": local["operator_ids"],
            "host_version": local["host_version"],
            "sdk_api_version": local["sdk_api_version"]
        },
        "hosts": [
            host(
                "local-macos-qualification-host",
                "macos",
                "aarch64",
                "local-native",
                &paths.local_report,
                &paths.local_preflight,
            )?,
            host(
                "remote-linux-qualification-host",
                "linux",
                "x86_64",
                "remote-native",
                &paths.remote_report,
                &paths.remote_preflight,
            )?
        ],
        "cleanup": {
            "remote_run_root_removed": true,
            "local_staging_removed": true,
            "residue_count": 0
        },
        "checks": REQUIRED_CHECKS
            .iter()
            .map(|id| json!({"id": id, "ok": true}))
            .collect::<Vec<_>>()
    }))
}

fn promote_capture(source: &Path, destination: &Path) -> RunnerResult<()> {
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    if destination.exists() {
        fs::remove_file(destination)
            .map_err(|error| format!("failed to replace {}: {error}", destination.display()))?;
    }
    fs::rename(source, destination).map_err(|error| {
        format!(
            "failed to promote {} to {}: {error}",
            source.display(),
            destination.display()
        )
    })
}

fn validate_local_platform() -> RunnerResult<()> {
    if env::consts::OS != "macos" || env::consts::ARCH != "aarch64" {
        return Err(
            "Operator SDK multihost capture currently requires a macOS aarch64 control host"
                .to_string(),
        );
    }
    Ok(())
}

fn require_zero(label: &str, status: u8) -> RunnerResult<()> {
    if status == 0 {
        Ok(())
    } else {
        Err(format!("{label} failed with status {status}"))
    }
}

#[derive(Debug)]
struct CheckOptions {
    help: bool,
    self_test: bool,
    report: Option<String>,
}

impl CheckOptions {
    fn parse(args: Vec<OsString>) -> RunnerResult<Self> {
        let mut options = Self {
            help: false,
            self_test: false,
            report: None,
        };
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.to_string_lossy().as_ref() {
                "--help" | "-h" => options.help = true,
                "--self-test" => options.self_test = true,
                "--verify-report" | "--in" => {
                    options.report = Some(next_string(&mut args, "--verify-report")?);
                }
                other => return Err(format!("unknown multihost check option: {other}")),
            }
        }
        Ok(options)
    }
}

#[derive(Debug)]
struct Options {
    help: bool,
    host: String,
    output: PathBuf,
    remote_run_root: String,
    staging_root: PathBuf,
    staged_local_report: PathBuf,
    staged_local_preflight: PathBuf,
    staged_remote_report: PathBuf,
    staged_remote_preflight: PathBuf,
}

impl Options {
    fn parse(root: &Path, args: Vec<OsString>) -> RunnerResult<Self> {
        let slug = format!("operator-sdk-multihost-{}", utc_timestamp_slug());
        let mut host = env::var("KYUUBIKI_LAB_HOST").unwrap_or_else(|_| "kyuubiki-lab".to_string());
        let mut output = root.join(DEFAULT_REPORT);
        let mut help = false;
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.to_string_lossy().as_ref() {
                "--help" | "-h" => help = true,
                "--host" => host = next_string(&mut args, "--host")?,
                "--out" => output = PathBuf::from(next_string(&mut args, "--out")?),
                other => return Err(format!("unknown multihost qualification option: {other}")),
            }
        }
        if !output.is_absolute() {
            output = root.join(output);
        }
        if !valid_ssh_alias(&host) {
            return Err("remote qualification host must be a plain SSH alias".to_string());
        }
        relative_path(root, &output)?;
        let staging_root = root.join("tmp").join(&slug);
        Ok(Self {
            help,
            host,
            output,
            remote_run_root: format!("~/.local/state/kyuubiki/lab-runs/{slug}"),
            staged_local_report: staging_root.join("local/report.json"),
            staged_local_preflight: staging_root
                .join("local/operator-package-dynamic-preflight.json"),
            staged_remote_report: staging_root.join("remote/report.json"),
            staged_remote_preflight: staging_root
                .join("remote/operator-package-dynamic-preflight.json"),
            staging_root,
        })
    }
}

struct RetainedPaths {
    output: String,
    local_report: String,
    local_preflight: String,
    remote_report: String,
    remote_preflight: String,
    local_report_absolute: PathBuf,
    local_preflight_absolute: PathBuf,
    remote_report_absolute: PathBuf,
    remote_preflight_absolute: PathBuf,
}

impl RetainedPaths {
    fn new(root: &Path, output: &Path) -> RunnerResult<Self> {
        let parent = output
            .parent()
            .ok_or("qualification output must have a parent directory")?;
        let local_report_absolute = parent.join("operator-sdk-macos-agent-qualification.json");
        let local_preflight_absolute = parent.join("operator-sdk-macos-preflight.json");
        let remote_report_absolute = parent.join("operator-sdk-linux-agent-qualification.json");
        let remote_preflight_absolute = parent.join("operator-sdk-linux-preflight.json");
        Ok(Self {
            output: relative_path(root, output)?,
            local_report: relative_path(root, &local_report_absolute)?,
            local_preflight: relative_path(root, &local_preflight_absolute)?,
            remote_report: relative_path(root, &remote_report_absolute)?,
            remote_preflight: relative_path(root, &remote_preflight_absolute)?,
            local_report_absolute,
            local_preflight_absolute,
            remote_report_absolute,
            remote_preflight_absolute,
        })
    }
}

fn relative_path(root: &Path, path: &Path) -> RunnerResult<String> {
    path.strip_prefix(root)
        .map_err(|_| "qualification output must stay inside the repository".to_string())
        .map(|relative| relative.to_string_lossy().to_string())
        .and_then(|relative| {
            repo_path(root, &relative)?;
            Ok(relative)
        })
}

fn next_string(args: &mut impl Iterator<Item = OsString>, flag: &str) -> RunnerResult<String> {
    args.next()
        .map(|value| value.to_string_lossy().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{flag} requires a value"))
}
