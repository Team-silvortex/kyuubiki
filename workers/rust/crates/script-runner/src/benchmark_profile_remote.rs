use crate::benchmark_profile_remote_summary::write_profile_outputs;
use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

type RunnerResult<T> = Result<T, String>;

struct Options {
    case_filter: Option<String>,
    local_json_path: PathBuf,
    local_md_path: PathBuf,
    local_output_dir: PathBuf,
    local_summary_path: PathBuf,
    matrix: String,
    profile: String,
    remote_dir: String,
    remote_host: String,
    remote_json_path: String,
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
    let options = Options::from_env(root);
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

    let status = run_status(
        "ssh",
        [
            OsString::from(&options.remote_host),
            OsString::from(remote_command(&options)),
        ],
        root,
    )?;
    if status != 0 {
        return Ok(status);
    }

    let scp_status = run_status(
        "scp",
        [
            OsString::from(format!(
                "{}:{}",
                options.remote_host, options.remote_json_path
            )),
            options.local_json_path.clone().into_os_string(),
        ],
        root,
    )?;
    if scp_status != 0 {
        return Ok(scp_status);
    }

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

impl Options {
    fn from_env(root: &Path) -> Self {
        let profile = env::var("PROFILE").unwrap_or_else(|_| "200k".to_string());
        let matrix = env::var("MATRIX").unwrap_or_else(|_| "thermal-core".to_string());
        let case_filter = env::var("CASE")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        let output_name = output_name(&matrix, &profile, case_filter.as_deref());
        let output_slug = env::var("OUTPUT_SLUG").unwrap_or_else(|_| {
            format!(
                "benchmark-profile-{}",
                timestamp_slug().unwrap_or_else(|| "manual".to_string())
            )
        });
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
        let local_json_path = env_path_or(
            "LOCAL_JSON_PATH",
            local_output_dir.join(format!("{output_name}.json")),
        );
        Self {
            case_filter,
            local_json_path,
            local_md_path: local_output_dir.join("README.md"),
            local_output_dir,
            local_summary_path,
            matrix,
            profile,
            remote_dir,
            remote_host: env::var("KYUUBIKI_LAB_HOST").unwrap_or_else(|_| "kyuubiki-lab".into()),
            remote_json_path,
            report_only: env::var("REPORT_ONLY").unwrap_or_else(|_| "0".into()) == "1",
            repeat: env::var("REPEAT").unwrap_or_else(|_| "3".to_string()),
            rustup_toolchain: env::var("RUSTUP_TOOLCHAIN_OVERRIDE")
                .unwrap_or_else(|_| "stable".to_string()),
            solver_preconditioner: env::var("SOLVER_PRECONDITIONER")
                .unwrap_or_else(|_| "auto".to_string()),
            sync_to_remote: env::var("SYNC_TO_REMOTE").unwrap_or_else(|_| "1".into()) == "1",
        }
    }
}

fn sync_benchmark_sources(root: &Path, options: &Options) -> RunnerResult<()> {
    ensure_remote_sync_dirs(root, options)?;
    let status = rsync(
        root,
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
    let status = run_status(
        "ssh",
        [
            OsString::from(&options.remote_host),
            OsString::from(format!(
                "mkdir -p {}",
                shell_escape(&format!("{}/workers", options.remote_dir))
            )),
        ],
        root,
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
        "set -euo pipefail; mkdir -p {}; cd {}/workers/rust; RUSTUP_TOOLCHAIN={} cargo run --release -q -p kyuubiki-benchmark -- --profile {} --matrix {} --repeat {} --format json --solver-preconditioner {} --progress{} > {}",
        shell_escape(&dirname(&options.remote_json_path)),
        shell_escape(&options.remote_dir),
        shell_escape(&options.rustup_toolchain),
        shell_escape(&options.profile),
        shell_escape(&options.matrix),
        shell_escape(&options.repeat),
        shell_escape(&options.solver_preconditioner),
        case_arg,
        shell_escape(&options.remote_json_path)
    )
}

fn rsync(root: &Path, sources: &[PathBuf], destination: &str) -> RunnerResult<u8> {
    run_status(
        "rsync",
        [OsString::from("-az"), OsString::from("--exclude=target/")]
            .into_iter()
            .chain(sources.iter().map(|path| path.clone().into_os_string()))
            .chain([OsString::from(destination)]),
        root,
    )
}

fn run_status<I>(program: &str, args: I, cwd: &Path) -> RunnerResult<u8>
where
    I: IntoIterator<Item = OsString>,
{
    let status = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .status()
        .map_err(|error| format!("failed to run {program}: {error}"))?;
    Ok(status.code().unwrap_or(1) as u8)
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

fn shell_escape(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn timestamp_slug() -> Option<String> {
    let output = Command::new("date")
        .args(["-u", "+%Y%m%dT%H%M%SZ"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn print_usage() {
    println!(
        "Usage:\n  ./scripts/kyuubiki benchmark-profile-remote\n\n\
Runs one Rust benchmark profile/matrix on the shared lab machine without a\n\
checked baseline, then copies JSON back and writes a Markdown summary.\n\n\
Environment:\n  KYUUBIKI_LAB_HOST\n  KYUUBIKI_LAB_BENCH_DIR\n  PROFILE\n  MATRIX\n  CASE\n  REPEAT\n  RUSTUP_TOOLCHAIN_OVERRIDE\n  SOLVER_PRECONDITIONER (default: auto)\n  OUTPUT_SLUG\n  LOCAL_OUTPUT_DIR\n  LOCAL_JSON_PATH\n  REMOTE_OUTPUT_DIR\n  SYNC_TO_REMOTE\n  REPORT_ONLY (1 regenerates local summary without SSH)\n"
    );
}
