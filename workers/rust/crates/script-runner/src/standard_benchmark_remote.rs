use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

type RunnerResult<T> = Result<T, String>;

struct Options {
    benchmark_median_threshold: String,
    benchmark_min_baseline_ms: String,
    benchmark_rss_threshold: String,
    local_output_dir: PathBuf,
    merged_report_local: PathBuf,
    profile: String,
    remote_dir: String,
    remote_host: String,
    remote_report_dir: String,
    repeat: String,
    retain_runs: String,
    sync_to_remote: bool,
}

pub(crate) fn run_standard_benchmark_regression(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage();
        return Ok(0);
    }
    if !args.is_empty() {
        return Err("standard-benchmark-regression does not accept positional arguments".into());
    }
    let options = Options::from_env(root);
    std::fs::create_dir_all(&options.local_output_dir).map_err(|error| {
        format!(
            "failed to create {}: {error}",
            options.local_output_dir.display()
        )
    })?;

    if options.sync_to_remote {
        sync_benchmark_sources(root, &options)?;
    }

    let benchmark_status = run_status(
        "ssh",
        [
            OsString::from(&options.remote_host),
            OsString::from(remote_benchmark_command(&options)),
        ],
        root,
    )?;
    if benchmark_status != 0 {
        return Ok(benchmark_status);
    }

    for status in [
        copy_remote_report(
            &options,
            &format!(
                "{}/{}/standard-{}-compare.md",
                options.remote_dir, options.remote_report_dir, options.profile
            ),
            &options.merged_report_local,
            root,
        )?,
        copy_remote_report(
            &options,
            &format!(
                "{}/workers/rust/benchmarks/reports/mechanical-core-{}-compare.md",
                options.remote_dir, options.profile
            ),
            &options
                .local_output_dir
                .join(format!("mechanical-core-{}-compare.md", options.profile)),
            root,
        )?,
        copy_remote_report(
            &options,
            &format!(
                "{}/workers/rust/benchmarks/reports/thermal-core-{}-compare.md",
                options.remote_dir, options.profile
            ),
            &options
                .local_output_dir
                .join(format!("thermal-core-{}-compare.md", options.profile)),
            root,
        )?,
        copy_remote_report(
            &options,
            &format!(
                "{}/workers/rust/benchmarks/reports/compound-core-{}-compare.md",
                options.remote_dir, options.profile
            ),
            &options
                .local_output_dir
                .join(format!("compound-core-{}-compare.md", options.profile)),
            root,
        )?,
        run_node(
            root,
            "build-standard-benchmark-index.mjs",
            vec![
                "--root".into(),
                root.join("tmp/standard-benchmark").into_os_string(),
                "--retain".into(),
                OsString::from(&options.retain_runs),
            ],
        )?,
        run_node(
            root,
            "build-regression-lane-catalog.mjs",
            vec!["--tmp-root".into(), root.join("tmp").into_os_string()],
        )?,
        run_node(
            root,
            "build-regression-gate-report.mjs",
            vec!["--tmp-root".into(), root.join("tmp").into_os_string()],
        )?,
        run_node(
            root,
            "build-nightly-artifact-overview.mjs",
            vec!["--tmp-root".into(), root.join("tmp").into_os_string()],
        )?,
    ] {
        if status != 0 {
            return Ok(status);
        }
    }

    println!(
        "remote standard benchmark regression completed on {}",
        options.remote_host
    );
    println!("local output dir: {}", options.local_output_dir.display());
    println!("merged report: {}", options.merged_report_local.display());
    Ok(0)
}

impl Options {
    fn from_env(root: &Path) -> Self {
        let profile = env::var("PROFILE").unwrap_or_else(|_| "10k".to_string());
        let output_slug = env::var("OUTPUT_SLUG").unwrap_or_else(|_| {
            format!(
                "standard-benchmark-{}",
                timestamp_slug().unwrap_or_else(|| "manual".to_string())
            )
        });
        let local_output_dir = env_path_or(
            "LOCAL_OUTPUT_DIR",
            root.join("tmp/standard-benchmark").join(&output_slug),
        );
        Self {
            benchmark_median_threshold: env::var("BENCHMARK_MEDIAN_THRESHOLD")
                .unwrap_or_else(|_| "25".to_string()),
            benchmark_min_baseline_ms: env::var("BENCHMARK_MIN_BASELINE_MS")
                .unwrap_or_else(|_| "5.0".to_string()),
            benchmark_rss_threshold: env::var("BENCHMARK_RSS_THRESHOLD")
                .unwrap_or_else(|_| "20".to_string()),
            merged_report_local: env_path_or(
                "MERGED_REPORT_LOCAL",
                local_output_dir.join(format!("standard-{profile}-compare.md")),
            ),
            local_output_dir,
            profile,
            remote_dir: env::var("KYUUBIKI_LAB_BENCH_DIR")
                .unwrap_or_else(|_| "~/kyuubiki".to_string()),
            remote_host: env::var("KYUUBIKI_LAB_HOST").unwrap_or_else(|_| "kyuubiki-lab".into()),
            remote_report_dir: env::var("REMOTE_REPORT_DIR")
                .unwrap_or_else(|_| format!("tmp/standard-benchmark/{output_slug}")),
            repeat: env::var("REPEAT").unwrap_or_else(|_| "1".to_string()),
            retain_runs: env::var("RETAIN_RUNS").unwrap_or_else(|_| "12".to_string()),
            sync_to_remote: env::var("SYNC_TO_REMOTE").unwrap_or_else(|_| "1".into()) == "1",
        }
    }
}

fn sync_benchmark_sources(root: &Path, options: &Options) -> RunnerResult<()> {
    for status in [
        rsync(
            root,
            &[root.join("Makefile")],
            &format!("{}:{}/Makefile", options.remote_host, options.remote_dir),
        )?,
        rsync(
            root,
            &[
                root.join("scripts/build-nightly-artifact-overview.mjs"),
                root.join("scripts/build-standard-benchmark-index.mjs"),
                root.join("scripts/build-standard-benchmark-report.mjs"),
                root.join("scripts/run-standard-benchmark-regression.sh"),
            ],
            &format!("{}:{}/scripts/", options.remote_host, options.remote_dir),
        )?,
        rsync(
            root,
            &[root.join("workers/rust/crates/benchmark/src/")],
            &format!(
                "{}:{}/workers/rust/crates/benchmark/src/",
                options.remote_host, options.remote_dir
            ),
        )?,
        rsync(
            root,
            &[root.join("workers/rust/benchmarks/")],
            &format!(
                "{}:{}/workers/rust/benchmarks/",
                options.remote_host, options.remote_dir
            ),
        )?,
    ] {
        if status != 0 {
            return Err(format!("rsync failed with status {status}"));
        }
    }
    Ok(())
}

fn rsync(root: &Path, sources: &[PathBuf], destination: &str) -> RunnerResult<u8> {
    run_status(
        "rsync",
        [OsString::from("-az")]
            .into_iter()
            .chain(sources.iter().map(|path| path.clone().into_os_string()))
            .chain([OsString::from(destination)]),
        root,
    )
}

fn remote_benchmark_command(options: &Options) -> String {
    format!(
        "cd {} && mkdir -p {} && make benchmark-standard-compare PROFILE={} REPEAT={} BENCHMARK_MEDIAN_THRESHOLD={} BENCHMARK_RSS_THRESHOLD={} BENCHMARK_MIN_BASELINE_MS={} && make benchmark-standard-report PROFILE={} REPEAT={} OUTPUT={}",
        remote_shell_path(&options.remote_dir),
        shell_escape(&options.remote_report_dir),
        shell_escape(&options.profile),
        shell_escape(&options.repeat),
        shell_escape(&options.benchmark_median_threshold),
        shell_escape(&options.benchmark_rss_threshold),
        shell_escape(&options.benchmark_min_baseline_ms),
        shell_escape(&options.profile),
        shell_escape(&options.repeat),
        shell_escape(&format!(
            "{}/standard-{}-compare.md",
            options.remote_report_dir, options.profile
        ))
    )
}

fn copy_remote_report(
    options: &Options,
    remote_path: &str,
    local_path: &Path,
    root: &Path,
) -> RunnerResult<u8> {
    if let Some(parent) = local_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    run_status(
        "scp",
        [
            OsString::from(format!("{}:{remote_path}", options.remote_host)),
            local_path.to_path_buf().into_os_string(),
        ],
        root,
    )
}

fn run_node(root: &Path, script: &str, args: Vec<OsString>) -> RunnerResult<u8> {
    run_status(
        "node",
        [root.join("scripts").join(script).into_os_string()]
            .into_iter()
            .chain(args),
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

fn shell_escape(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn remote_shell_path(value: &str) -> String {
    value
        .strip_prefix("~/")
        .map(|rest| format!("$HOME/{}", shell_escape(rest)))
        .unwrap_or_else(|| shell_escape(value))
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
        "Usage:\n  ./scripts/kyuubiki standard-benchmark-regression\n\n\
Runs the standard Rust benchmark regression trio on the shared lab machine,\n\
copies reports back locally, and refreshes local benchmark indexes.\n\n\
Environment:\n  KYUUBIKI_LAB_HOST\n  KYUUBIKI_LAB_BENCH_DIR\n  PROFILE\n  REPEAT\n  OUTPUT_SLUG\n  REMOTE_REPORT_DIR\n  LOCAL_OUTPUT_DIR\n  MERGED_REPORT_LOCAL\n  BENCHMARK_MEDIAN_THRESHOLD\n  BENCHMARK_RSS_THRESHOLD\n  BENCHMARK_MIN_BASELINE_MS\n  SYNC_TO_REMOTE\n  RETAIN_RUNS\n"
    );
}
