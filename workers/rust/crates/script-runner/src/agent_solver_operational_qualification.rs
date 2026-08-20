use crate::native_time::utc_timestamp_slug;
use crate::remote_host::{
    remote_shell_path, rsync_to, scp_from, shell_escape, ssh_status, valid_ssh_alias,
};
use kyuubiki_installer::validate_agent_solver_operational_qualification_report;
use serde_json::Value;
use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

type RunnerResult<T> = Result<T, String>;

const DEFAULT_REPORT: &str =
    "releases/usability-evidence/2.13.8/agent-solver-operational-qualification.json";
const CONTRACT_PATH: &str = "config/architecture/agent-solver-operational-qualification.json";
const REPORT_SCHEMA: &str = "kyuubiki.agent-solver-operational-qualification/v1";

pub(crate) fn run_check_agent_solver_operational_qualification(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    let options = CheckOptions::parse(root, args)?;
    if options.help {
        println!(
            "usage: kyuubiki-script-runner check-agent-solver-operational-qualification [--self-test] [--verify-report path]"
        );
        return Ok(0);
    }
    validate_contract(root, true)?;
    if options.self_test {
        validator_self_test()?;
        println!("Agent solver operational qualification self-test passed");
        if options.report.is_none() {
            return Ok(0);
        }
    }
    let report_path = options.report.unwrap_or_else(|| root.join(DEFAULT_REPORT));
    let summary = verify_remote_linux_report(&report_path)?;
    println!(
        "Agent solver operational qualification passed: {} run(s), package {}, {}",
        summary.solver_run_count,
        summary.package_version,
        display_path(root, &report_path)
    );
    Ok(0)
}

pub(crate) fn run_qualify_agent_solver_operational_remote(
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

    let capture = capture_remote_report(root, &options);
    let cleanup = cleanup_remote_run(root, &options);
    match (capture, cleanup) {
        (Ok(0), Ok(())) => {
            println!(
                "remote Installer-managed Agent solver qualification passed: {}",
                display_path(root, &options.output)
            );
            Ok(0)
        }
        (Ok(status), Ok(())) => Ok(status),
        (Err(error), Ok(())) => Err(error),
        (Ok(status), Err(cleanup_error)) => Err(format!(
            "remote qualification exited with status {status}; {cleanup_error}"
        )),
        (Err(error), Err(cleanup_error)) => Err(format!("{error}; {cleanup_error}")),
    }
}

fn capture_remote_report(root: &Path, options: &RemoteOptions) -> RunnerResult<u8> {
    let sync_status = rsync_to(
        root,
        &["target/", ".DS_Store"],
        &[root.join("workers/rust/")],
        &format!("{}:{}/workers/rust/", options.host, options.remote_run_root),
    )?;
    if sync_status != 0 {
        return Ok(sync_status);
    }

    let qualification_status =
        ssh_status(root, &options.host, remote_qualification_command(&options))?;
    if qualification_status != 0 {
        return Ok(qualification_status);
    }

    let temporary = temporary_report_path(&options.output);
    if let Some(parent) = temporary.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    let copy_status = scp_from(
        root,
        &options.host,
        &format!("{}/report.json", options.remote_run_root),
        &temporary,
    )?;
    if copy_status != 0 {
        let _ = fs::remove_file(&temporary);
        return Ok(copy_status);
    }
    if let Err(error) = verify_remote_linux_report(&temporary) {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    if let Err(error) = promote_report(&temporary, &options.output) {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    Ok(0)
}

fn cleanup_remote_run(root: &Path, options: &RemoteOptions) -> RunnerResult<()> {
    let status = ssh_status(root, &options.host, remote_cleanup_command(options))?;
    if status == 0 {
        Ok(())
    } else {
        Err(format!(
            "managed remote qualification run-root cleanup failed with status {status}"
        ))
    }
}

struct CheckOptions {
    help: bool,
    self_test: bool,
    report: Option<PathBuf>,
}

impl CheckOptions {
    fn parse(root: &Path, args: Vec<OsString>) -> RunnerResult<Self> {
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
                    options.report = Some(next_path(&mut args, "--verify-report")?);
                }
                other => return Err(format!("unknown qualification check option: {other}")),
            }
        }
        options.report = options.report.map(|path| repo_resolve(root, path));
        Ok(options)
    }
}

struct RemoteOptions {
    help: bool,
    host: String,
    output: PathBuf,
    remote_run_root: String,
    evidence_slug: String,
    package_version: String,
}

impl RemoteOptions {
    fn parse(root: &Path, args: Vec<OsString>) -> RunnerResult<Self> {
        let slug = format!("solver-operational-{}", utc_timestamp_slug());
        let mut options = Self {
            help: false,
            host: env::var("KYUUBIKI_LAB_HOST").unwrap_or_else(|_| "kyuubiki-lab".to_string()),
            output: root.join("tmp/agent-solver-operational-remote.json"),
            remote_run_root: format!("~/.kyuubiki/lab-runs/{slug}"),
            evidence_slug: slug,
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
            return Err(
                "remote qualification host must be a plain SSH alias containing only letters, digits, dots, underscores, and hyphens"
                    .to_string(),
            );
        }
        Ok(options)
    }
}

fn prepare_remote_run(root: &Path, options: &RemoteOptions) -> RunnerResult<()> {
    let run_root = remote_shell_path(&options.remote_run_root);
    let status = ssh_status(
        root,
        &options.host,
        format!("set -eu; umask 077; mkdir -p {run_root}/workers/rust"),
    )?;
    if status == 0 {
        Ok(())
    } else {
        Err(format!(
            "failed to prepare managed remote run root: status {status}"
        ))
    }
}

fn remote_qualification_command(options: &RemoteOptions) -> String {
    let run_root = remote_shell_path(&options.remote_run_root);
    let evidence_root = "$HOME/.kyuubiki/lab-evidence/agent-solver-operational";
    let version = shell_escape(&options.package_version);
    let evidence_file = shell_escape(&format!("{}.json", options.evidence_slug));
    format!(
        "set -eu; umask 077; run_root={run_root}; source_root=\"$run_root/workers/rust\"; \
target_root=\"$HOME/.kyuubiki/cache/cargo-target/agent-solver-operational\"; \
evidence_root=\"{evidence_root}\"; mkdir -p \"$target_root\" \"$evidence_root\"; \
cd \"$source_root\"; CARGO_TARGET_DIR=\"$target_root\" cargo build --release -p kyuubiki-cli -p kyuubiki-installer; \
\"$target_root/release/kyuubiki-installer\" qualify-agent-solver-operational \
\"$target_root/release/kyuubiki-cli\" \"$run_root/qualification-work\" {version} \"$run_root/report.json\"; \
cp \"$run_root/report.json\" \"$evidence_root\"/{evidence_file}"
    )
}

fn remote_cleanup_command(options: &RemoteOptions) -> String {
    let run_root = remote_shell_path(&options.remote_run_root);
    format!(
        "set -eu; run_root={run_root}; case \"$run_root\" in \
\"$HOME/.kyuubiki/lab-runs/\"*) rm -rf \"$run_root\" ;; \
*) echo 'refusing unmanaged cleanup root' >&2; exit 2 ;; esac"
    )
}

fn verify_remote_linux_report(
    path: &Path,
) -> RunnerResult<kyuubiki_installer::AgentSolverOperationalQualificationSummary> {
    let bytes =
        fs::read(path).map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let report: Value = serde_json::from_slice(&bytes)
        .map_err(|error| format!("{}: invalid JSON: {error}", path.display()))?;
    let summary = validate_agent_solver_operational_qualification_report(&report)
        .map_err(|errors| format!("{}: {}", path.display(), errors.join("; ")))?;
    if summary.execution_host_role != "remote-linux-qualification-host"
        || summary.platform != "linux"
    {
        return Err("operational evidence must be captured from a remote Linux host".to_string());
    }
    if summary.solver_run_count != 2 || !summary.process_restart_confirmed {
        return Err(
            "operational evidence does not prove a two-process restart journey".to_string(),
        );
    }
    Ok(summary)
}

fn validator_self_test() -> RunnerResult<()> {
    let invalid = serde_json::json!({
        "schema_version": REPORT_SCHEMA,
        "status": "pass",
        "execution_host_role": "remote-linux-qualification-host",
        "platform": "linux",
        "hostname": "must-not-be-retained"
    });
    let errors = validate_agent_solver_operational_qualification_report(&invalid)
        .expect_err("incomplete and host-identifying evidence must be rejected");
    if !errors.iter().any(|error| error.contains("hostname")) {
        return Err("operational validator self-test did not reject host identity".to_string());
    }
    Ok(())
}

fn validate_contract(root: &Path, require_retained_report: bool) -> RunnerResult<()> {
    let path = root.join(CONTRACT_PATH);
    let bytes =
        fs::read(&path).map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let contract: Value = serde_json::from_slice(&bytes)
        .map_err(|error| format!("{CONTRACT_PATH}: invalid JSON: {error}"))?;
    let expected = [
        (
            "/schema_version",
            "kyuubiki.agent-solver-operational-qualification-contract/v1",
        ),
        (
            "/qualification_id",
            "installer-managed-agent-solver-linux-operational",
        ),
        ("/target_coordinate/module_id", "runtime-engine-solver"),
        ("/target_coordinate/paradigm", "solver_execution"),
        ("/target_coordinate/target_grade", "operational"),
        (
            "/capture/execution_host_role",
            "remote-linux-qualification-host",
        ),
        ("/capture/build_profile", "release"),
        ("/capture/network_bind_scope", "loopback"),
        ("/retention/report_schema", REPORT_SCHEMA),
        ("/retention/report_path", DEFAULT_REPORT),
    ];
    for (pointer, value) in expected {
        if contract.pointer(pointer).and_then(Value::as_str) != Some(value) {
            return Err(format!("{CONTRACT_PATH}: {pointer} must be {value}"));
        }
    }
    if contract
        .pointer("/capture/solver_runs_minimum")
        .and_then(Value::as_u64)
        .is_none_or(|value| value < 2)
        || contract
            .pointer("/capture/process_restarts_minimum")
            .and_then(Value::as_u64)
            .is_none_or(|value| value < 1)
        || contract
            .pointer("/capture/failures_per_run_minimum")
            .and_then(Value::as_u64)
            .is_none_or(|value| value < 2)
    {
        return Err(format!(
            "{CONTRACT_PATH}: operational thresholds are too weak"
        ));
    }
    if contract
        .pointer("/capture/maximum_absolute_error")
        .and_then(Value::as_f64)
        .is_none_or(|value| !(0.0..=1.0e-12).contains(&value))
        || contract
            .pointer("/capture/cleanup_required")
            .and_then(Value::as_bool)
            != Some(true)
    {
        return Err(format!(
            "{CONTRACT_PATH}: numerical or cleanup policy is invalid"
        ));
    }
    let checks = contract
        .pointer("/required_checks")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{CONTRACT_PATH}: required_checks must be an array"))?;
    if checks.len() != 14 {
        return Err(format!(
            "{CONTRACT_PATH}: required_checks must contain 14 checks"
        ));
    }
    for pointer in ["/retention/report_schema_path"] {
        let relative = contract
            .pointer(pointer)
            .and_then(Value::as_str)
            .ok_or_else(|| format!("{CONTRACT_PATH}: {pointer} must be a path"))?;
        if !root.join(relative).is_file() {
            return Err(format!("{CONTRACT_PATH}: missing retained path {relative}"));
        }
    }
    if require_retained_report && !root.join(DEFAULT_REPORT).is_file() {
        return Err(format!(
            "{CONTRACT_PATH}: missing retained report {DEFAULT_REPORT}"
        ));
    }
    Ok(())
}

fn workspace_version(root: &Path) -> RunnerResult<String> {
    let path = root.join("workers/rust/Cargo.toml");
    let text = fs::read_to_string(&path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let workspace_package = text
        .split_once("[workspace.package]")
        .map(|(_, section)| section)
        .ok_or_else(|| "Rust workspace package section is missing".to_string())?;
    workspace_package
        .lines()
        .find_map(|line| line.trim().strip_prefix("version = \"")?.strip_suffix('"'))
        .map(str::to_string)
        .ok_or_else(|| "Rust workspace version is missing".to_string())
}

fn promote_report(temporary: &Path, output: &Path) -> RunnerResult<()> {
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    if output.exists() {
        fs::remove_file(output)
            .map_err(|error| format!("failed to replace {}: {error}", output.display()))?;
    }
    fs::rename(temporary, output)
        .map_err(|error| format!("failed to promote {}: {error}", output.display()))
}

fn temporary_report_path(output: &Path) -> PathBuf {
    let file = output
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("agent-solver-operational.json");
    output.with_file_name(format!(".{file}.{}.tmp", std::process::id()))
}

fn next_path(args: &mut impl Iterator<Item = OsString>, option: &str) -> RunnerResult<PathBuf> {
    args.next()
        .map(PathBuf::from)
        .ok_or_else(|| format!("{option} requires a path"))
}

fn next_string(args: &mut impl Iterator<Item = OsString>, option: &str) -> RunnerResult<String> {
    args.next()
        .and_then(|value| value.into_string().ok())
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
        "usage: kyuubiki-script-runner qualify-agent-solver-operational-remote [--host SSH_ALIAS] [--out path] [--package-version version]\n\nBuilds the Rust Agent and Installer in an isolated Linux lab run, executes the Installer-managed solver/recovery/restart journey, verifies the pulled report, and cleans the managed remote run root."
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remote_cleanup_is_scoped_to_managed_lab_runs() {
        let options = RemoteOptions {
            help: false,
            host: "lab".to_string(),
            output: PathBuf::from("tmp/report.json"),
            remote_run_root: "~/.kyuubiki/lab-runs/test".to_string(),
            evidence_slug: "test".to_string(),
            package_version: "2.7.0".to_string(),
        };
        let command = remote_cleanup_command(&options);
        assert!(command.contains("$HOME/.kyuubiki/lab-runs/"));
        assert!(command.contains("refusing unmanaged cleanup root"));
        let qualification = remote_qualification_command(&options);
        assert!(qualification.contains("\"$evidence_root\"/'test.json'"));
        assert!(!qualification.contains("\"$evidence_root/'test'"));
    }

    #[test]
    fn validator_self_test_rejects_host_identity() {
        validator_self_test().expect("self-test");
    }

    #[test]
    fn relative_outputs_are_resolved_from_repository_root() {
        let root = Path::new("/repo");
        assert_eq!(
            repo_resolve(root, PathBuf::from("tmp/report.json")),
            PathBuf::from("/repo/tmp/report.json")
        );
    }

    #[test]
    fn ssh_aliases_cannot_be_reinterpreted_as_options() {
        assert!(valid_ssh_alias("kyuubiki-lab"));
        assert!(valid_ssh_alias("lab.internal_1"));
        assert!(!valid_ssh_alias("-oProxyCommand=bad"));
        assert!(!valid_ssh_alias("user@host"));
        assert!(!valid_ssh_alias("host name"));
    }
}
