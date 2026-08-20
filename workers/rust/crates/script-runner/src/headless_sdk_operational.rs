use crate::headless_sdk_operational_validation::{
    DEFAULT_REPORT, QUALIFICATION_ID, REPORT_SCHEMA, REQUIRED_CHECKS, array_len, bool_at,
    string_at, u64_at, validate_binaries, validate_contract, validate_report, validator_self_test,
};
use crate::native_time::utc_timestamp_slug;
use crate::qualification_support::{generated_at_unix_ms, read_json};
use crate::remote_host::{
    remote_shell_path, rsync_to, scp_from, ssh_output, ssh_status, ssh_success_quiet,
    valid_ssh_alias,
};
use serde_json::{Value, json};
use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

type RunnerResult<T> = Result<T, String>;

const ARTIFACTS: [&str; 8] = [
    "templates.json",
    "workflow.json",
    "validation.json",
    "batch.json",
    "mock-run.json",
    "material.json",
    "failure-report.json",
    "recovery.json",
];

pub(crate) fn run_check_headless_sdk_operational(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    let options = CheckOptions::parse(args)?;
    if options.help {
        println!(
            "usage: kyuubiki-script-runner check-headless-sdk-operational-qualification [--self-test] [--verify-report path]"
        );
        return Ok(0);
    }
    validate_contract(root, !options.self_test || options.report.is_some())?;
    if options.self_test {
        validator_self_test()?;
        println!("Headless SDK operational qualification self-test passed");
        if options.report.is_none() {
            return Ok(0);
        }
    }
    let relative = options.report.as_deref().unwrap_or(DEFAULT_REPORT);
    let report: Value = read_json(root, relative)?;
    validate_report(&report)?;
    println!(
        "Headless SDK operational qualification passed: package {}, {} real-solver candidate(s), {relative}",
        string_at(&report, "/installation/package_version")?,
        u64_at(&report, "/real_solver/candidate_count")?
    );
    Ok(0)
}

pub(crate) fn run_qualify_headless_sdk_operational_remote(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    let options = RemoteOptions::parse(root, args)?;
    if options.help {
        print_remote_usage();
        return Ok(0);
    }
    validate_contract(root, false)?;
    prepare_remote_run(root, &options)?;

    let mut capture = capture_remote_report(root, &options);
    let cleanup = cleanup_remote_run(root, &options);
    let _ = fs::remove_dir_all(local_capture_dir(root, &options));
    match (&mut capture, cleanup) {
        (Ok(report), Ok(())) => {
            report["cleanup"] = json!({
                "scope": "managed-remote-run-root",
                "work_root_removed": true,
                "residue_count": 0
            });
            mark_check(report, "cleanup_complete")?;
            validate_report(report)?;
            promote_report(&options.output, report)?;
            println!(
                "remote installed Headless SDK qualification passed: {}",
                display_path(root, &options.output)
            );
            Ok(0)
        }
        (Ok(_), Err(cleanup_error)) => Err(cleanup_error),
        (Err(error), Ok(())) => Err(error.clone()),
        (Err(error), Err(cleanup_error)) => Err(format!("{error}; {cleanup_error}")),
    }
}

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
                other => return Err(format!("unknown operational check option: {other}")),
            }
        }
        Ok(options)
    }
}

struct RemoteOptions {
    help: bool,
    host: String,
    output: PathBuf,
    remote_run_root: String,
    slug: String,
    package_version: String,
}

impl RemoteOptions {
    fn parse(root: &Path, args: Vec<OsString>) -> RunnerResult<Self> {
        let slug = format!("headless-sdk-operational-{}", utc_timestamp_slug());
        let mut options = Self {
            help: false,
            host: env::var("KYUUBIKI_LAB_HOST").unwrap_or_else(|_| "kyuubiki-lab".to_string()),
            output: root.join(DEFAULT_REPORT),
            remote_run_root: format!("~/.kyuubiki/lab-runs/{slug}"),
            slug,
            package_version: workspace_version(root)?,
        };
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.to_string_lossy().as_ref() {
                "--help" | "-h" => options.help = true,
                "--host" => options.host = next_string(&mut args, "--host")?,
                "--out" => options.output = next_path(&mut args, "--out")?,
                "--package-version" => {
                    options.package_version = next_string(&mut args, "--package-version")?;
                }
                other => return Err(format!("unknown remote qualification option: {other}")),
            }
        }
        options.output = repo_resolve(root, options.output);
        if !valid_ssh_alias(&options.host) {
            return Err("remote qualification host must be a plain SSH alias".to_string());
        }
        Ok(options)
    }
}

fn prepare_remote_run(root: &Path, options: &RemoteOptions) -> RunnerResult<()> {
    let run_root = remote_shell_path(&options.remote_run_root);
    require_zero(
        "prepare managed remote run root",
        ssh_status(
            root,
            &options.host,
            format!("set -eu; umask 077; mkdir -p {run_root}/workers/rust"),
        )?,
    )
}

fn capture_remote_report(root: &Path, options: &RemoteOptions) -> RunnerResult<Value> {
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
        "install Headless SDK binaries",
        ssh_status(root, &options.host, remote_install_command(options))?,
    )?;
    require_zero(
        "detach installed binaries from source tree",
        ssh_status(root, &options.host, remote_detach_command(options))?,
    )?;

    let digests = ssh_output(root, &options.host, remote_digest_command(options))?;
    let platform = ssh_output(root, &options.host, "uname -s; uname -m".to_string())?;
    run_remote_journey(root, options)?;
    let capture_dir = retrieve_artifacts(root, options)?;
    build_report(&capture_dir, &options.package_version, &digests, &platform)
}

fn run_remote_journey(root: &Path, options: &RemoteOptions) -> RunnerResult<()> {
    for (label, command) in remote_success_commands(options) {
        require_zero(label, ssh_status(root, &options.host, command)?)?;
    }
    let failure_status = ssh_status(root, &options.host, remote_failure_command(options))?;
    if failure_status != 1 {
        return Err(format!(
            "expected installed Headless failure to exit 1, got {failure_status}"
        ));
    }
    require_zero(
        "recover after expected failure",
        ssh_status(root, &options.host, remote_recovery_command(options))?,
    )
}

fn remote_install_command(options: &RemoteOptions) -> String {
    let run_root = remote_shell_path(&options.remote_run_root);
    format!(
        "set -eu; umask 077; run_root={run_root}; source_root=\"$run_root/workers/rust\"; \
target_root=\"$HOME/.kyuubiki/cache/cargo-target/headless-sdk-operational\"; \
mkdir -p \"$target_root\"; CARGO_TARGET_DIR=\"$target_root\" cargo install \
--path \"$source_root/crates/cli\" --root \"$run_root/install\" --locked \
--bin kyuubiki-headless --bin kyuubiki-material-explore"
    )
}

fn remote_detach_command(options: &RemoteOptions) -> String {
    let run_root = remote_shell_path(&options.remote_run_root);
    let command = format!(
        "set -eu; run_root={run_root}; test -x \"$run_root/install/bin/kyuubiki-headless\"; \
test -x \"$run_root/install/bin/kyuubiki-material-explore\"; rm -rf \"$run_root/workers\"; \
mkdir -p \"$run_root/isolated-work\" \"$run_root/home\" \"$run_root/artifacts\"; \
test ! -e \"$run_root/workers\"; env -i HOME=\"$run_root/home\" PATH=/usr/bin:/bin \
/bin/sh -c 'test -z \"$(command -v cargo)\" && test -z \"$(command -v rustc)\"'"
    );
    command
        .replace(
            "\"$run_root/artifacts\"",
            "\"$run_root/artifacts\" \"$run_root/empty-bin\"",
        )
        .replace("PATH=/usr/bin:/bin", "PATH=\"$run_root/empty-bin\"")
}

fn remote_digest_command(options: &RemoteOptions) -> String {
    let run_root = remote_shell_path(&options.remote_run_root);
    format!(
        "set -eu; run_root={run_root}; sha256sum \"$run_root/install/bin/kyuubiki-headless\" \
\"$run_root/install/bin/kyuubiki-material-explore\""
    )
}

fn remote_success_commands(options: &RemoteOptions) -> Vec<(&'static str, String)> {
    let prefix = remote_execution_prefix(options);
    vec![
        (
            "discover installed templates",
            format!(
                "{prefix} \"$run_root/install/bin/kyuubiki-headless\" templates --runtime service_only --json > \"$run_root/artifacts/templates.json\""
            ),
        ),
        (
            "initialize installed workflow",
            format!(
                "{prefix} \"$run_root/install/bin/kyuubiki-headless\" init --template direct_bar_1d --workflow-id qualification.headless.operational --out \"$run_root/artifacts/workflow.json\" --json > /dev/null"
            ),
        ),
        (
            "validate installed workflow",
            format!(
                "{prefix} \"$run_root/install/bin/kyuubiki-headless\" validate \"$run_root/artifacts/workflow.json\" --json > \"$run_root/artifacts/validation.json\""
            ),
        ),
        (
            "render installed workflow",
            format!(
                "{prefix} \"$run_root/install/bin/kyuubiki-headless\" render \"$run_root/artifacts/workflow.json\" --out \"$run_root/artifacts/batch.json\" --json > /dev/null"
            ),
        ),
        (
            "execute installed Headless workflow",
            format!(
                "{prefix} \"$run_root/install/bin/kyuubiki-headless\" run \"$run_root/artifacts/workflow.json\" --execute --executor mock --json --report-out \"$run_root/artifacts/mock-run.json\" > /dev/null"
            ),
        ),
        (
            "execute installed real solver study",
            format!(
                "{prefix} \"$run_root/install/bin/kyuubiki-material-explore\" heat-spreader --out \"$run_root/artifacts/material.json\" --json > /dev/null"
            ),
        ),
    ]
}

fn remote_failure_command(options: &RemoteOptions) -> String {
    let prefix = remote_execution_prefix(options).replace("set -eu", "set -u");
    format!(
        "{prefix} \"$run_root/install/bin/kyuubiki-headless\" run missing-workflow.json --execute --executor mock --json --report-out \"$run_root/artifacts/failure-report.json\" > /dev/null 2> \"$run_root/artifacts/failure.stderr\""
    )
}

fn remote_recovery_command(options: &RemoteOptions) -> String {
    let prefix = remote_execution_prefix(options);
    format!(
        "{prefix} \"$run_root/install/bin/kyuubiki-headless\" validate \"$run_root/artifacts/workflow.json\" --json > \"$run_root/artifacts/recovery.json\""
    )
}

fn remote_execution_prefix(options: &RemoteOptions) -> String {
    let run_root = remote_shell_path(&options.remote_run_root);
    format!(
        "set -eu; run_root={run_root}; cd \"$run_root/isolated-work\"; env -i HOME=\"$run_root/home\" PATH=\"$run_root/empty-bin\" LANG=C.UTF-8"
    )
}

fn retrieve_artifacts(root: &Path, options: &RemoteOptions) -> RunnerResult<PathBuf> {
    let local = local_capture_dir(root, options);
    fs::create_dir_all(&local)
        .map_err(|error| format!("failed to create {}: {error}", local.display()))?;
    for name in ARTIFACTS {
        require_zero(
            "retrieve Headless qualification artifact",
            scp_from(
                root,
                &options.host,
                &format!("{}/artifacts/{name}", options.remote_run_root),
                &local.join(name),
            )?,
        )?;
    }
    Ok(local)
}

fn build_report(
    capture: &Path,
    package_version: &str,
    digest_output: &str,
    platform_output: &str,
) -> RunnerResult<Value> {
    let templates = read_capture(capture, "templates.json")?;
    let workflow = read_capture(capture, "workflow.json")?;
    let validation = read_capture(capture, "validation.json")?;
    let batch = read_capture(capture, "batch.json")?;
    let run = read_capture(capture, "mock-run.json")?;
    let material = read_capture(capture, "material.json")?;
    let failure = read_capture(capture, "failure-report.json")?;
    let recovery = read_capture(capture, "recovery.json")?;
    let (platform, architecture) = parse_platform(platform_output)?;
    let binaries = parse_binary_digests(digest_output)?;

    let report = json!({
        "schema_version": REPORT_SCHEMA,
        "generated_at_unix_ms": generated_at_unix_ms()?,
        "status": "pass",
        "qualification_id": QUALIFICATION_ID,
        "execution_host_role": "remote-linux-qualification-host",
        "platform": platform,
        "architecture": architecture,
        "installation": {
            "package_id": "kyuubiki-cli",
            "package_version": package_version,
            "method": "cargo-install-path",
            "build_profile": "release",
            "isolated_prefix": true,
            "source_removed_before_execution": true,
            "runtime_path_mode": "isolated-empty",
            "binaries": binaries
        },
        "workflow": {
            "template_count": u64_at(&templates, "/template_count")?,
            "workflow_schema": string_at(&workflow, "/schema_version")?,
            "workflow_id": string_at(&workflow, "/workflow/id")?,
            "template_id": string_at(&workflow, "/template/id")?,
            "step_count": array_len(&workflow, "/workflow/steps")?,
            "validation_ok": bool_at(&validation, "/ok")?,
            "rendered_schema": string_at(&batch, "/schema_version")?,
            "execution_report_schema": string_at(&run, "/schema_version")?,
            "execution_mode": string_at(&run, "/mode")?,
            "execution_status": string_at(&run, "/status")?,
            "executed_step_count": u64_at(&run, "/executed_step_count")?
        },
        "real_solver": {
            "schema_version": string_at(&material, "/schema_version")?,
            "study": string_at(&material, "/study")?,
            "candidate_count": u64_at(&material, "/candidate_count")?,
            "winner_candidate_id": string_at(&material, "/report/winner_candidate_id")?,
            "execution_class": string_at(&material, "/execution_authority/execution_class")?,
            "executor_id": string_at(&material, "/execution_authority/executor_id")?,
            "runtime": string_at(&material, "/execution_authority/runtime")?,
            "result_origin": string_at(&material, "/execution_authority/result_origin")?,
            "mock_execution": bool_at(&material, "/execution_authority/mock_execution")?,
            "fallback_used": bool_at(&material, "/execution_authority/fallback_used")?,
            "production_eligible": bool_at(&material, "/execution_authority/production_eligible")?
        },
        "failure_recovery": {
            "expected_failure_exit_code": 1,
            "failure_report_schema": string_at(&failure, "/schema_version")?,
            "failure_status": string_at(&failure, "/status")?,
            "executed_step_count": u64_at(&failure, "/executed_step_count")?,
            "failure_category": string_at(&failure, "/execution_summary/failure/category")?,
            "failure_stage": string_at(&failure, "/execution_summary/failure/stage")?,
            "retryable": bool_at(&failure, "/execution_summary/failure/retryable")?,
            "recovery_validation_ok": bool_at(&recovery, "/ok")?
        },
        "cleanup": {
            "scope": "managed-remote-run-root",
            "work_root_removed": false,
            "residue_count": 1
        },
        "checks": REQUIRED_CHECKS.iter().map(|id| json!({
            "id": id,
            "ok": *id != "cleanup_complete"
        })).collect::<Vec<_>>()
    });
    Ok(report)
}

fn cleanup_remote_run(root: &Path, options: &RemoteOptions) -> RunnerResult<()> {
    let run_root = remote_shell_path(&options.remote_run_root);
    require_zero(
        "clean managed remote run root",
        ssh_status(
            root,
            &options.host,
            format!(
                "set -eu; run_root={run_root}; case \"$run_root\" in \
\"$HOME/.kyuubiki/lab-runs/\"*) rm -rf \"$run_root\" ;; \
*) echo 'refusing unmanaged cleanup root' >&2; exit 2 ;; esac"
            ),
        )?,
    )?;
    if !ssh_success_quiet(
        root,
        &options.host,
        format!("test ! -e {}", remote_shell_path(&options.remote_run_root)),
    )? {
        return Err("remote Headless qualification root still exists after cleanup".to_string());
    }
    Ok(())
}

fn parse_binary_digests(output: &str) -> RunnerResult<Vec<Value>> {
    let mut binaries = output
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let digest = fields.next()?;
            let path = fields.next()?;
            let id = path.rsplit('/').next()?;
            Some(json!({"id": id, "sha256": digest}))
        })
        .collect::<Vec<_>>();
    binaries.sort_by(|left, right| {
        left["id"]
            .as_str()
            .unwrap_or_default()
            .cmp(right["id"].as_str().unwrap_or_default())
    });
    let probe = json!({"installation": {"binaries": binaries}});
    validate_binaries(&probe)?;
    Ok(probe["installation"]["binaries"]
        .as_array()
        .cloned()
        .unwrap_or_default())
}

fn parse_platform(output: &str) -> RunnerResult<(String, String)> {
    let mut lines = output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty());
    let os = lines.next().unwrap_or_default().to_ascii_lowercase();
    let architecture = lines.next().unwrap_or_default().to_string();
    if os != "linux" || architecture.is_empty() || lines.next().is_some() {
        return Err("operational capture requires a Linux platform and architecture".to_string());
    }
    Ok((os, architecture))
}

fn mark_check(report: &mut Value, id: &str) -> RunnerResult<()> {
    let checks = report
        .pointer_mut("/checks")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| "operational report misses checks".to_string())?;
    let check = checks
        .iter_mut()
        .find(|check| check.pointer("/id").and_then(Value::as_str) == Some(id))
        .ok_or_else(|| format!("operational report misses check {id}"))?;
    check["ok"] = Value::Bool(true);
    Ok(())
}

fn read_capture(root: &Path, name: &str) -> RunnerResult<Value> {
    let path = root.join(name);
    let bytes =
        fs::read(&path).map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid capture artifact {name}: {error}"))
}

fn workspace_version(root: &Path) -> RunnerResult<String> {
    let path = root.join("workers/rust/Cargo.toml");
    let text = fs::read_to_string(&path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    text.split_once("[workspace.package]")
        .and_then(|(_, section)| {
            section
                .lines()
                .find_map(|line| line.trim().strip_prefix("version = \"")?.strip_suffix('"'))
        })
        .map(str::to_string)
        .ok_or_else(|| "Rust workspace version is missing".to_string())
}

fn require_zero(label: &str, status: u8) -> RunnerResult<()> {
    if status == 0 {
        Ok(())
    } else {
        Err(format!("{label} failed with status {status}"))
    }
}

fn promote_report(path: &Path, report: &Value) -> RunnerResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    let temporary = path.with_extension(format!("json.{}.tmp", std::process::id()));
    let rendered = serde_json::to_string_pretty(report)
        .map_err(|error| format!("failed to encode operational report: {error}"))?;
    fs::write(&temporary, format!("{rendered}\n"))
        .map_err(|error| format!("failed to write {}: {error}", temporary.display()))?;
    fs::rename(&temporary, path)
        .map_err(|error| format!("failed to promote {}: {error}", path.display()))
}

fn local_capture_dir(root: &Path, options: &RemoteOptions) -> PathBuf {
    root.join("tmp/headless-sdk-operational-capture")
        .join(&options.slug)
}

fn next_path(args: &mut impl Iterator<Item = OsString>, option: &str) -> RunnerResult<PathBuf> {
    args.next()
        .map(PathBuf::from)
        .ok_or_else(|| format!("{option} requires a path"))
}

fn next_string(args: &mut impl Iterator<Item = OsString>, option: &str) -> RunnerResult<String> {
    args.next()
        .and_then(|value| value.into_string().ok())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{option} requires a UTF-8 value"))
}

fn repo_resolve(root: &Path, path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        path
    } else {
        root.join(path)
    }
}

fn display_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn print_remote_usage() {
    println!(
        "usage: kyuubiki-script-runner qualify-headless-sdk-operational-remote [--host SSH_ALIAS] [--out path] [--package-version version]\n\nInstalls the Rust Headless SDK tools into an isolated Linux prefix, removes source before execution, runs workflow, real-solver, failure, and recovery journeys, retains a sanitized report, and cleans the managed remote run root."
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digest_parser_requires_both_installed_binaries() {
        let output = format!(
            "{}  /remote/kyuubiki-headless\n{}  /remote/kyuubiki-material-explore",
            "a".repeat(64),
            "b".repeat(64)
        );
        let binaries = parse_binary_digests(&output).expect("binary digests");
        assert_eq!(binaries.len(), 2);
        assert!(
            parse_binary_digests(&format!("{}  /remote/kyuubiki-headless", "a".repeat(64)))
                .is_err()
        );
    }

    #[test]
    fn remote_cleanup_is_scoped_to_managed_lab_runs() {
        let options = RemoteOptions {
            help: false,
            host: "lab".to_string(),
            output: PathBuf::from("tmp/report.json"),
            remote_run_root: "~/.kyuubiki/lab-runs/test".to_string(),
            slug: "test".to_string(),
            package_version: "2.7.0".to_string(),
        };
        let command = remote_detach_command(&options);
        assert!(command.contains("rm -rf \"$run_root/workers\""));
        assert!(command.contains("PATH=\"$run_root/empty-bin\""));
        assert!(!command.contains("node"));
    }

    #[test]
    fn relative_outputs_resolve_from_repository_root() {
        assert_eq!(
            repo_resolve(Path::new("/repo"), PathBuf::from("tmp/report.json")),
            PathBuf::from("/repo/tmp/report.json")
        );
    }
}
