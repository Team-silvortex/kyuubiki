use crate::native_time::utc_timestamp_slug;
use crate::remote_host::{
    remote_shell_path, rsync_to, scp_from, shell_escape, ssh_status, valid_ssh_alias,
};
use kyuubiki_installer::{
    RuntimePayloadQualificationReport, RuntimePayloadQualificationSummary,
    validate_runtime_payload_qualification_report,
};
use serde_json::Value;
use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

type RunnerResult<T> = Result<T, String>;

pub(crate) fn run_check_runtime_payload_operational_qualification(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    let options = CheckOptions::parse(root, args)?;
    if options.help {
        print_check_usage();
        return Ok(0);
    }
    let report_path = options
        .report
        .unwrap_or(default_report_path(root, &development_version(root)?));
    let summary = verify_report(&report_path, options.require_remote_linux)?;
    println!(
        "Runtime payload operational qualification passed: {} -> {} -> {}, {} service probes, {}",
        summary.first_version,
        summary.second_version,
        summary.first_version,
        summary.executed_probe_count,
        display_path(root, &report_path)
    );
    Ok(0)
}

pub(crate) fn run_qualify_runtime_payload_operational_remote(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    let options = RemoteOptions::parse(root, args)?;
    if options.help {
        print_remote_usage();
        return Ok(0);
    }
    prepare_remote_run(root, &options)?;
    let capture = capture_remote_report(root, &options);
    let cleanup = cleanup_remote_run(root, &options);
    match (capture, cleanup) {
        (Ok(0), Ok(())) => {
            println!(
                "remote Installer-managed Runtime payload qualification passed: {}",
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

struct CheckOptions {
    help: bool,
    report: Option<PathBuf>,
    require_remote_linux: bool,
}

impl CheckOptions {
    fn parse(root: &Path, args: Vec<OsString>) -> RunnerResult<Self> {
        let mut options = Self {
            help: false,
            report: None,
            require_remote_linux: false,
        };
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.to_string_lossy().as_ref() {
                "--help" | "-h" => options.help = true,
                "--verify-report" | "--in" => {
                    options.report =
                        Some(repo_resolve(root, next_path(&mut args, "--verify-report")?));
                }
                "--require-remote-linux" => options.require_remote_linux = true,
                other => return Err(format!("unknown qualification check option: {other}")),
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
    evidence_slug: String,
    first_version: String,
    second_version: String,
}

impl RemoteOptions {
    fn parse(root: &Path, args: Vec<OsString>) -> RunnerResult<Self> {
        let second_version = development_version(root)?;
        let first_version = previous_version(&second_version)?;
        let slug = format!("runtime-payload-{}", utc_timestamp_slug());
        let mut options = Self {
            help: false,
            host: env::var("KYUUBIKI_LAB_HOST").unwrap_or_else(|_| "kyuubiki-lab".to_string()),
            output: default_report_path(root, &second_version),
            remote_run_root: format!("~/.kyuubiki/lab-runs/{slug}"),
            evidence_slug: slug,
            first_version,
            second_version,
        };
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.to_string_lossy().as_ref() {
                "--help" | "-h" => options.help = true,
                "--host" => options.host = next_string(&mut args, "--host")?,
                "--out" => options.output = repo_resolve(root, next_path(&mut args, "--out")?),
                "--first-version" => {
                    options.first_version = next_string(&mut args, "--first-version")?;
                }
                "--second-version" => {
                    options.second_version = next_string(&mut args, "--second-version")?;
                }
                other => return Err(format!("unknown remote qualification option: {other}")),
            }
        }
        if !valid_ssh_alias(&options.host) {
            return Err("remote qualification host must be a plain SSH alias".to_string());
        }
        validate_version_pair(&options.first_version, &options.second_version)?;
        Ok(options)
    }
}

fn capture_remote_report(root: &Path, options: &RemoteOptions) -> RunnerResult<u8> {
    let sync = rsync_to(
        root,
        &["target/", ".DS_Store"],
        &[root.join("workers/rust/")],
        &format!("{}:{}/workers/rust/", options.host, options.remote_run_root),
    )?;
    if sync != 0 {
        return Ok(sync);
    }
    let status = ssh_status(root, &options.host, remote_qualification_command(options))?;
    if status != 0 {
        return Ok(status);
    }
    let temporary = temporary_report_path(&options.output);
    let copy = scp_from(
        root,
        &options.host,
        &format!("{}/report.json", options.remote_run_root),
        &temporary,
    )?;
    if copy != 0 {
        let _ = fs::remove_file(&temporary);
        return Ok(copy);
    }
    if let Err(error) = verify_report(&temporary, true) {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    promote_report(&temporary, &options.output)?;
    Ok(0)
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
    let first = shell_escape(&options.first_version);
    let second = shell_escape(&options.second_version);
    let evidence_file = shell_escape(&format!("{}.json", options.evidence_slug));
    format!(
        "set -eu; umask 077; run_root={run_root}; source_root=\"$run_root/workers/rust\"; \
target_root=\"$HOME/.kyuubiki/cache/cargo-target/runtime-payload-operational\"; \
evidence_root=\"$HOME/.kyuubiki/lab-evidence/runtime-payload-operational\"; \
mkdir -p \"$target_root\" \"$evidence_root\"; cd \"$source_root\"; \
CARGO_TARGET_DIR=\"$target_root\" cargo build -p kyuubiki-cli -p kyuubiki-installer; \
CARGO_TARGET_DIR=\"$target_root\" cargo build --release -p kyuubiki-cli; \
\"$target_root/debug/kyuubiki-installer\" qualify-runtime-payload \
\"$target_root/debug/kyuubiki-cli\" \"$target_root/release/kyuubiki-cli\" \
\"$run_root/qualification-work\" {first} {second} \"$run_root/report.json\"; \
cp \"$run_root/report.json\" \"$evidence_root\"/{evidence_file}"
    )
}

fn cleanup_remote_run(root: &Path, options: &RemoteOptions) -> RunnerResult<()> {
    let run_root = remote_shell_path(&options.remote_run_root);
    let status = ssh_status(
        root,
        &options.host,
        format!(
            "set -eu; run_root={run_root}; case \"$run_root\" in \
\"$HOME/.kyuubiki/lab-runs/\"*) rm -rf \"$run_root\" ;; \
*) echo 'refusing unmanaged cleanup root' >&2; exit 2 ;; esac"
        ),
    )?;
    if status == 0 {
        Ok(())
    } else {
        Err(format!(
            "managed remote run cleanup failed with status {status}"
        ))
    }
}

fn verify_report(
    path: &Path,
    require_remote_linux: bool,
) -> RunnerResult<RuntimePayloadQualificationSummary> {
    let bytes =
        fs::read(path).map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let report: RuntimePayloadQualificationReport = serde_json::from_slice(&bytes)
        .map_err(|error| format!("{}: invalid report: {error}", path.display()))?;
    let summary = validate_runtime_payload_qualification_report(&report)
        .map_err(|errors| format!("{}: {}", path.display(), errors.join("; ")))?;
    if require_remote_linux && summary.execution_host_role != "remote-linux-qualification-host" {
        return Err(format!(
            "{}: remote Linux evidence is required",
            path.display()
        ));
    }
    Ok(summary)
}

fn development_version(root: &Path) -> RunnerResult<String> {
    let path = root.join("docs/book-manifest.json");
    let value: Value = serde_json::from_slice(
        &fs::read(&path).map_err(|error| format!("failed to read {}: {error}", path.display()))?,
    )
    .map_err(|error| format!("{}: invalid JSON: {error}", path.display()))?;
    value
        .get("current_development_version")
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| "book manifest misses current_development_version".to_string())
}

fn previous_version(version: &str) -> RunnerResult<String> {
    let mut parts = version.split('.');
    let major = parse_part(parts.next(), version)?;
    let minor = parse_part(parts.next(), version)?;
    let patch = parse_part(parts.next(), version)?;
    if parts.next().is_some() || (minor == 0 && patch == 0) {
        return Err(format!(
            "cannot derive previous moxi version from {version}"
        ));
    }
    Ok(if patch > 0 {
        format!("{major}.{minor}.{}", patch - 1)
    } else {
        format!("{major}.{}.9", minor - 1)
    })
}

fn parse_part(value: Option<&str>, version: &str) -> RunnerResult<u64> {
    value
        .and_then(|part| part.parse().ok())
        .ok_or_else(|| format!("invalid development version {version}"))
}

fn validate_version_pair(first: &str, second: &str) -> RunnerResult<()> {
    if first == second || !first.bytes().all(version_byte) || !second.bytes().all(version_byte) {
        return Err("qualification versions must be distinct portable identifiers".to_string());
    }
    Ok(())
}

fn version_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-')
}

fn default_report_path(root: &Path, version: &str) -> PathBuf {
    root.join(format!(
        "releases/usability-evidence/{version}/runtime-payload-operational-qualification.json"
    ))
}

fn temporary_report_path(output: &Path) -> PathBuf {
    let file = output
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("runtime-payload-operational.json");
    output.with_file_name(format!(".{file}.{}.tmp", std::process::id()))
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

fn repo_resolve(root: &Path, path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        path
    } else {
        root.join(path)
    }
}

fn next_string(args: &mut impl Iterator<Item = OsString>, flag: &str) -> RunnerResult<String> {
    args.next()
        .map(|value| value.to_string_lossy().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{flag} requires a value"))
}

fn next_path(args: &mut impl Iterator<Item = OsString>, flag: &str) -> RunnerResult<PathBuf> {
    args.next()
        .map(PathBuf::from)
        .ok_or_else(|| format!("{flag} requires a path"))
}

fn display_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn print_check_usage() {
    println!(
        "usage: kyuubiki-script-runner check-runtime-payload-operational-qualification [--verify-report path] [--require-remote-linux]"
    );
}

fn print_remote_usage() {
    println!(
        "usage: kyuubiki-script-runner qualify-runtime-payload-operational-remote [--host SSH_ALIAS] [--out path] [--first-version version] [--second-version version]"
    );
}

#[cfg(test)]
mod tests {
    use super::previous_version;

    #[test]
    fn derives_previous_patch_across_minor_boundaries() {
        assert_eq!(previous_version("2.14.4").unwrap(), "2.14.3");
        assert_eq!(previous_version("2.14.0").unwrap(), "2.13.9");
        assert!(previous_version("2.0.0").is_err());
    }
}
