use crate::benchmark_profile_remote_summary::write_profile_outputs;
use crate::native_time::utc_timestamp_slug;
use crate::remote_host::{rsync_to, scp_from, shell_escape, ssh_status};
use serde_json::json;
use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

type RunnerResult<T> = Result<T, String>;

struct Options {
    case_filter: Option<String>,
    local_json_path: PathBuf,
    local_md_path: PathBuf,
    local_output_dir: PathBuf,
    local_progress_path: PathBuf,
    local_summary_path: PathBuf,
    matrix: String,
    profile: String,
    remote_dir: String,
    remote_host: String,
    remote_json_path: String,
    remote_progress_path: String,
    remote_timeout_seconds: u64,
    report_only: bool,
    repeat: String,
    rustup_toolchain: String,
    solver_preconditioner: String,
    sync_to_remote: bool,
}

pub(crate) fn run_benchmark_profile_remote(root: &Path, args: Vec<OsString>) -> RunnerResult<u8> {
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage();
        return Ok(0);
    }
    if !args.is_empty() {
        return Err("benchmark-profile-remote does not accept positional arguments".into());
    }
    let options = Options::from_env(root)?;
    std::fs::create_dir_all(&options.local_output_dir).map_err(|error| {
        format!(
            "failed to create {}: {error}",
            options.local_output_dir.display()
        )
    })?;

    if options.report_only {
        write_profile_outputs(
            &options.local_json_path,
            &options.local_md_path,
            &options.local_summary_path,
        )?;
        println!(
            "benchmark profile summary regenerated from {}",
            options.local_json_path.display()
        );
        println!("summary: {}", options.local_md_path.display());
        println!("summary json: {}", options.local_summary_path.display());
        return Ok(0);
    }

    if options.sync_to_remote {
        sync_benchmark_sources(root, &options)?;
    }

    let status = ssh_status(root, &options.remote_host, remote_command(&options))?;
    if status != 0 {
        copy_progress_log(root, &options);
        let receipt = write_failure_receipt(&options, "remote-execution", status)?;
        eprintln!(
            "remote benchmark profile failed; receipt: {}",
            receipt.display()
        );
        return Ok(status);
    }

    let scp_status = scp_from(
        root,
        &options.remote_host,
        &options.remote_json_path,
        &options.local_json_path,
    )?;
    if scp_status != 0 {
        copy_progress_log(root, &options);
        let receipt = write_failure_receipt(&options, "artifact-copy", scp_status)?;
        eprintln!(
            "remote benchmark artifact copy failed; receipt: {}",
            receipt.display()
        );
        return Ok(scp_status);
    }

    copy_progress_log(root, &options);

    write_profile_outputs(
        &options.local_json_path,
        &options.local_md_path,
        &options.local_summary_path,
    )?;
    println!(
        "remote benchmark profile completed on {}",
        options.remote_host
    );
    println!("json: {}", options.local_json_path.display());
    println!("summary: {}", options.local_md_path.display());
    println!("summary json: {}", options.local_summary_path.display());
    Ok(0)
}

fn write_failure_receipt(options: &Options, phase: &str, exit_code: u8) -> RunnerResult<PathBuf> {
    let path = options.local_output_dir.join("failure.json");
    let payload = json!({
        "schema_version": "moxi.benchmark-profile-failure.v1",
        "phase": phase,
        "exit_code": exit_code,
        "failure_kind": failure_kind(exit_code),
        "timed_out": exit_code == 124,
        "profile": options.profile,
        "matrix": options.matrix,
        "case": options.case_filter,
        "repeat": options.repeat,
        "remote_host": options.remote_host,
        "remote_json_path": options.remote_json_path,
        "remote_timeout_seconds": options.remote_timeout_seconds,
        "solver_preconditioner": options.solver_preconditioner,
        "progress_log": "progress.log",
        "progress_tail": read_progress_tail(&options.local_progress_path),
    });
    let content = serde_json::to_string_pretty(&payload).map_err(|error| {
        format!("failed to serialize remote benchmark failure receipt: {error}")
    })?;
    std::fs::write(&path, format!("{content}\n"))
        .map_err(|error| format!("failed to write {}: {error}", path.display()))?;
    Ok(path)
}

fn failure_kind(exit_code: u8) -> &'static str {
    match exit_code {
        2 => "configuration",
        124 => "timeout",
        _ => "execution",
    }
}

impl Options {
    fn from_env(root: &Path) -> RunnerResult<Self> {
        let profile = env::var("PROFILE").unwrap_or_else(|_| "200k".to_string());
        let matrix = env::var("MATRIX").unwrap_or_else(|_| "thermal-core".to_string());
        let case_filter = env::var("CASE")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        let output_name = output_name(&matrix, &profile, case_filter.as_deref());
        let output_slug = env::var("OUTPUT_SLUG")
            .unwrap_or_else(|_| format!("benchmark-profile-{}", utc_timestamp_slug()));
        let remote_dir = env::var("KYUUBIKI_LAB_BENCH_DIR")
            .unwrap_or_else(|_| "/tmp/kyuubiki-server-test".to_string());
        let remote_output_dir = env::var("REMOTE_OUTPUT_DIR")
            .unwrap_or_else(|_| format!("/tmp/kyuubiki-benchmark-profile/{output_slug}"));
        let remote_json_path = if remote_output_dir.starts_with('/') {
            format!("{remote_output_dir}/{output_name}.json")
        } else {
            format!("{remote_dir}/{remote_output_dir}/{output_name}.json")
        };
        let local_output_dir = env_path_or(
            "LOCAL_OUTPUT_DIR",
            root.join("tmp/benchmark-profile").join(&output_slug),
        );
        let local_summary_path = local_output_dir.join("summary.json");
        let local_progress_path = local_output_dir.join("progress.log");
        let local_json_path = env_path_or(
            "LOCAL_JSON_PATH",
            local_output_dir.join(format!("{output_name}.json")),
        );
        let remote_timeout_seconds = env::var("REMOTE_TIMEOUT_SECONDS")
            .unwrap_or_else(|_| "900".to_string())
            .parse::<u64>()
            .map_err(|_| "REMOTE_TIMEOUT_SECONDS must be a positive integer".to_string())?;
        if remote_timeout_seconds == 0 {
            return Err("REMOTE_TIMEOUT_SECONDS must be greater than zero".to_string());
        }

        Ok(Self {
            case_filter,
            local_json_path,
            local_md_path: local_output_dir.join("README.md"),
            local_output_dir,
            local_progress_path,
            local_summary_path,
            matrix,
            profile,
            remote_dir,
            remote_host: env::var("KYUUBIKI_LAB_HOST").unwrap_or_else(|_| "kyuubiki-lab".into()),
            remote_progress_path: format!("{}/progress.log", dirname(&remote_json_path)),
            remote_json_path,
            remote_timeout_seconds,
            report_only: env::var("REPORT_ONLY").unwrap_or_else(|_| "0".into()) == "1",
            repeat: env::var("REPEAT").unwrap_or_else(|_| "3".to_string()),
            rustup_toolchain: env::var("RUSTUP_TOOLCHAIN_OVERRIDE")
                .unwrap_or_else(|_| "stable".to_string()),
            solver_preconditioner: env::var("SOLVER_PRECONDITIONER")
                .unwrap_or_else(|_| "auto".to_string()),
            sync_to_remote: env::var("SYNC_TO_REMOTE").unwrap_or_else(|_| "1".into()) == "1",
        })
    }
}

fn copy_progress_log(root: &Path, options: &Options) {
    match scp_from(
        root,
        &options.remote_host,
        &options.remote_progress_path,
        &options.local_progress_path,
    ) {
        Ok(0) => {}
        Ok(status) => eprintln!("remote benchmark progress log copy exited {status}"),
        Err(error) => eprintln!("failed to copy remote benchmark progress log: {error}"),
    }
}

fn read_progress_tail(path: &Path) -> Vec<String> {
    const MAX_LINES: usize = 8;
    std::fs::read_to_string(path)
        .map(|content| {
            content
                .lines()
                .rev()
                .take(MAX_LINES)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn sync_benchmark_sources(root: &Path, options: &Options) -> RunnerResult<()> {
    ensure_remote_sync_dirs(root, options)?;
    let status = rsync_to(
        root,
        &["target/"],
        &[root.join("workers/rust/")],
        &format!(
            "{}:{}/workers/rust/",
            options.remote_host, options.remote_dir
        ),
    )?;
    if status != 0 {
        return Err(format!("rsync failed with status {status}"));
    }
    Ok(())
}

fn ensure_remote_sync_dirs(root: &Path, options: &Options) -> RunnerResult<()> {
    let status = ssh_status(
        root,
        &options.remote_host,
        format!(
            "mkdir -p {}",
            shell_escape(&format!("{}/workers", options.remote_dir))
        ),
    )?;
    if status != 0 {
        return Err(format!("remote mkdir failed with status {status}"));
    }
    Ok(())
}

fn remote_command(options: &Options) -> String {
    let case_arg = options
        .case_filter
        .as_deref()
        .map(|case| format!(" --case {}", shell_escape(case)))
        .unwrap_or_default();
    format!(
        "set -euo pipefail; mkdir -p {}; cd {}/workers/rust; RUSTUP_TOOLCHAIN={} timeout --signal=INT --kill-after=30s {}s cargo run --release -q -p kyuubiki-benchmark -- --profile {} --matrix {} --repeat {} --format json --solver-preconditioner {} --progress{} > {} 2> {}",
        shell_escape(&dirname(&options.remote_json_path)),
        shell_escape(&options.remote_dir),
        shell_escape(&options.rustup_toolchain),
        options.remote_timeout_seconds,
        shell_escape(&options.profile),
        shell_escape(&options.matrix),
        shell_escape(&options.repeat),
        shell_escape(&options.solver_preconditioner),
        case_arg,
        shell_escape(&options.remote_json_path),
        shell_escape(&options.remote_progress_path)
    )
}

fn env_path_or(name: &str, fallback: PathBuf) -> PathBuf {
    env::var_os(name).map(PathBuf::from).unwrap_or(fallback)
}

fn output_name(matrix: &str, profile: &str, case_filter: Option<&str>) -> String {
    match case_filter {
        Some(case) => format!("{matrix}-{profile}-{}", sanitize_file_stem(case)),
        None => format!("{matrix}-{profile}"),
    }
}

fn sanitize_file_stem(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '-'
            }
        })
        .collect()
}

fn dirname(path: &str) -> String {
    path.rsplit_once('/')
        .map(|(dir, _)| dir.to_string())
        .unwrap_or_else(|| ".".to_string())
}

fn print_usage() {
    println!(
        "Usage:\n  ./scripts/kyuubiki benchmark-profile-remote\n\n\
Runs one Rust benchmark profile/matrix on the shared lab machine without a\n\
checked baseline, then copies JSON back and writes a Markdown summary.\n\n\
Environment:\n  KYUUBIKI_LAB_HOST\n  KYUUBIKI_LAB_BENCH_DIR\n  PROFILE\n  MATRIX\n  CASE\n  REPEAT\n  RUSTUP_TOOLCHAIN_OVERRIDE\n  SOLVER_PRECONDITIONER (default: auto)\n  REMOTE_TIMEOUT_SECONDS (default: 900)\n  OUTPUT_SLUG\n  LOCAL_OUTPUT_DIR\n  LOCAL_JSON_PATH\n  REMOTE_OUTPUT_DIR\n  SYNC_TO_REMOTE\n  REPORT_ONLY (1 regenerates local summary without SSH; also set PROFILE, MATRIX, and CASE, or LOCAL_JSON_PATH)\n"
    );
}

#[cfg(test)]
mod tests {
    use super::failure_kind;

    #[test]
    fn classifies_remote_failure_exit_codes() {
        assert_eq!(failure_kind(2), "configuration");
        assert_eq!(failure_kind(124), "timeout");
        assert_eq!(failure_kind(1), "execution");
    }
}
