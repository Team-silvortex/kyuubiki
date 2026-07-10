use crate::native_time::utc_timestamp_slug;
use crate::remote_host::{remote_shell_path, scp_from, shell_escape, ssh_status};
use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

type RunnerResult<T> = Result<T, String>;

struct Options {
    baseline_path: PathBuf,
    compare_json_local: PathBuf,
    compare_md_local: PathBuf,
    current_summary_local: PathBuf,
    output_path_remote: String,
    remote_dir: String,
    remote_host: String,
    repeat: String,
    workflow_avg_threshold: String,
    workflow_median_threshold: String,
}

pub(crate) fn run_workflow_catalog_remote(root: &Path, args: Vec<OsString>) -> RunnerResult<u8> {
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage();
        return Ok(0);
    }
    if !args.is_empty() {
        return Err(
            "workflow-catalog-benchmark-regression does not accept positional arguments"
                .to_string(),
        );
    }
    let options = Options::from_env(root);
    if let Some(parent) = options.current_summary_local.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }

    let remote_command = format!(
        "export PATH=$HOME/.local/elixir-1.15.7-otp-25/bin:$PATH; cd {}/apps/web && ERL_LIBS=\"$PWD/_build/test/lib\" elixir ../../scripts/workflow-catalog-benchmark.exs --repeat {} --output ../../{}",
        remote_shell_path(&options.remote_dir),
        shell_escape(&options.repeat),
        shell_escape(&options.output_path_remote)
    );
    let ssh_status = ssh_status(root, &options.remote_host, remote_command)?;
    if ssh_status != 0 {
        return Ok(ssh_status);
    }

    let scp_status = scp_from(
        root,
        &options.remote_host,
        &format!("{}/{}", options.remote_dir, options.output_path_remote),
        &options.current_summary_local,
    )?;
    if scp_status != 0 {
        return Ok(scp_status);
    }

    for status in [
        run_compare(root, &options)?,
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
        "remote summary copied to {}",
        options.current_summary_local.display()
    );
    println!("comparison json: {}", options.compare_json_local.display());
    println!("comparison report: {}", options.compare_md_local.display());
    Ok(0)
}

impl Options {
    fn from_env(root: &Path) -> Self {
        let output_slug = env::var("OUTPUT_SLUG")
            .unwrap_or_else(|_| format!("workflow-catalog-{}", utc_timestamp_slug()));
        Self {
            baseline_path: env_path_or(
                "BASELINE_PATH",
                root.join("tests/integration/benchmarks/workflow-catalog-benchmark-baseline.json"),
            ),
            compare_json_local: env_path_or(
                "COMPARE_JSON_LOCAL",
                root.join("tmp/workflow-catalog-benchmark")
                    .join(&output_slug)
                    .join("compare.json"),
            ),
            compare_md_local: env_path_or(
                "COMPARE_MD_LOCAL",
                root.join("tmp/workflow-catalog-benchmark")
                    .join(&output_slug)
                    .join("compare.md"),
            ),
            current_summary_local: env_path_or(
                "CURRENT_SUMMARY_LOCAL",
                root.join("tmp/workflow-catalog-benchmark")
                    .join(&output_slug)
                    .join("summary.json"),
            ),
            output_path_remote: env::var("OUTPUT_PATH_REMOTE")
                .unwrap_or_else(|_| format!("tmp/{output_slug}.json")),
            remote_dir: env::var("KYUUBIKI_LAB_WORKFLOW_BENCH_DIR")
                .unwrap_or_else(|_| "~/kyuubiki".to_string()),
            remote_host: env::var("KYUUBIKI_LAB_HOST")
                .unwrap_or_else(|_| "kyuubiki-lab".to_string()),
            repeat: env::var("REPEAT").unwrap_or_else(|_| "3".to_string()),
            workflow_avg_threshold: env::var("WORKFLOW_AVG_THRESHOLD")
                .unwrap_or_else(|_| "80".to_string()),
            workflow_median_threshold: env::var("WORKFLOW_MEDIAN_THRESHOLD")
                .unwrap_or_else(|_| "50".to_string()),
        }
    }
}

fn run_compare(root: &Path, options: &Options) -> RunnerResult<u8> {
    run_node(
        root,
        "compare-workflow-catalog-benchmark.mjs",
        vec![
            "--current".into(),
            options.current_summary_local.clone().into_os_string(),
            "--baseline".into(),
            options.baseline_path.clone().into_os_string(),
            "--json-out".into(),
            options.compare_json_local.clone().into_os_string(),
            "--report-out".into(),
            options.compare_md_local.clone().into_os_string(),
            "--fail-on-median-regression-pct".into(),
            OsString::from(&options.workflow_median_threshold),
            "--fail-on-avg-regression-pct".into(),
            OsString::from(&options.workflow_avg_threshold),
        ],
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

fn print_usage() {
    println!(
        "Usage:\n  ./scripts/kyuubiki workflow-catalog-benchmark-regression\n\n\
Environment:\n  KYUUBIKI_LAB_HOST\n  KYUUBIKI_LAB_WORKFLOW_BENCH_DIR\n  OUTPUT_SLUG\n  OUTPUT_PATH_REMOTE\n  REPEAT\n  CURRENT_SUMMARY_LOCAL\n  COMPARE_JSON_LOCAL\n  COMPARE_MD_LOCAL\n  BASELINE_PATH\n"
    );
}
