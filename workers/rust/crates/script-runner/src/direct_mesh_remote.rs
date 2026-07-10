use crate::native_time::utc_timestamp_slug;
use crate::remote_host::{
    remote_shell_path, scp_from, shell_escape, ssh_output, ssh_status, ssh_success_quiet,
};
use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

type RunnerResult<T> = Result<T, String>;

struct Options {
    artifact_dir: String,
    baseline_path: PathBuf,
    benchmark_wrapper: String,
    compare_json_local: PathBuf,
    compare_md_local: PathBuf,
    current_summary_local: PathBuf,
    docker_run_network: String,
    elapsed_threshold: String,
    output_dir_remote: String,
    proxy: Proxy,
    remote_dir: String,
    remote_host: String,
    repeat: String,
    rss_threshold: String,
}

#[derive(Default)]
struct Proxy {
    http: Option<String>,
    https: Option<String>,
    no_proxy: Option<String>,
}

pub(crate) fn run_direct_mesh_benchmark_regression(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage();
        return Ok(0);
    }
    if !args.is_empty() {
        return Err("direct-mesh-benchmark-regression does not accept positional arguments".into());
    }
    let options = Options::from_env(root);
    if let Some(parent) = options.current_summary_local.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }

    if !remote_wrapper_has_passwordless_sudo(root, &options)? {
        eprintln!(
            "passwordless sudo is not configured for {} on {}",
            options.benchmark_wrapper, options.remote_host
        );
        eprintln!(
            "configure a narrow NOPASSWD sudoers rule for the benchmark wrapper before using this regression wrapper"
        );
        return Ok(1);
    }

    let status = ssh_status(
        root,
        &options.remote_host,
        remote_benchmark_command(&options),
    )?;
    if status != 0 {
        return Ok(status);
    }

    let remote_summary_path = find_remote_summary(&options, root)?;
    let scp_status = scp_from(
        root,
        &options.remote_host,
        &remote_summary_path,
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
    println!("remote summary source: {remote_summary_path}");
    println!("comparison json: {}", options.compare_json_local.display());
    println!("comparison report: {}", options.compare_md_local.display());
    Ok(0)
}

impl Options {
    fn from_env(root: &Path) -> Self {
        let output_slug =
            env::var("OUTPUT_SLUG").unwrap_or_else(|_| format!("nightly-{}", utc_timestamp_slug()));
        let remote_dir = env::var("KYUUBIKI_LAB_BENCH_DIR")
            .unwrap_or_else(|_| "~/kyuubiki-bench-709b8c9".to_string());
        Self {
            artifact_dir: env::var("KYUUBIKI_LAB_ARTIFACT_DIR")
                .unwrap_or_else(|_| remote_dir.clone()),
            baseline_path: env_path_or(
                "BASELINE_PATH",
                root.join("tests/integration/benchmarks/direct-mesh-docker-baseline.json"),
            ),
            benchmark_wrapper: env::var("KYUUBIKI_LAB_BENCHMARK_WRAPPER")
                .unwrap_or_else(|_| "/usr/local/bin/kyuubiki-direct-mesh-benchmark".to_string()),
            compare_json_local: env_path_or(
                "COMPARE_JSON_LOCAL",
                root.join("tmp/direct-mesh-benchmark-container")
                    .join(&output_slug)
                    .join("compare.json"),
            ),
            compare_md_local: env_path_or(
                "COMPARE_MD_LOCAL",
                root.join("tmp/direct-mesh-benchmark-container")
                    .join(&output_slug)
                    .join("compare.md"),
            ),
            current_summary_local: env_path_or(
                "CURRENT_SUMMARY_LOCAL",
                root.join("tmp/direct-mesh-benchmark-container")
                    .join(&output_slug)
                    .join("summary.json"),
            ),
            docker_run_network: env::var("DOCKER_RUN_NETWORK").unwrap_or_else(|_| "host".into()),
            elapsed_threshold: env::var("DIRECT_MESH_ELAPSED_THRESHOLD")
                .unwrap_or_else(|_| "15".to_string()),
            output_dir_remote: env::var("OUTPUT_DIR_REMOTE")
                .unwrap_or_else(|_| format!("tmp/direct-mesh-benchmark-container/{output_slug}")),
            proxy: Proxy::from_env(),
            remote_dir,
            remote_host: env::var("KYUUBIKI_LAB_HOST").unwrap_or_else(|_| "kyuubiki-lab".into()),
            repeat: env::var("REPEAT").unwrap_or_else(|_| "3".to_string()),
            rss_threshold: env::var("DIRECT_MESH_RSS_THRESHOLD")
                .unwrap_or_else(|_| "20".to_string()),
        }
    }
}

impl Proxy {
    fn from_env() -> Self {
        Self {
            http: env_nonempty("HTTP_PROXY").or_else(|| env_nonempty("http_proxy")),
            https: env_nonempty("HTTPS_PROXY").or_else(|| env_nonempty("https_proxy")),
            no_proxy: env_nonempty("NO_PROXY").or_else(|| env_nonempty("no_proxy")),
        }
    }

    fn env_pairs(&self, docker_run_network: &str) -> Vec<(&'static str, String)> {
        let mut pairs = vec![("DOCKER_RUN_NETWORK", docker_run_network.to_string())];
        if let Some(value) = &self.http {
            pairs.push(("HTTP_PROXY", value.clone()));
            pairs.push(("http_proxy", value.clone()));
        }
        if let Some(value) = &self.https {
            pairs.push(("HTTPS_PROXY", value.clone()));
            pairs.push(("https_proxy", value.clone()));
        }
        if let Some(value) = &self.no_proxy {
            pairs.push(("NO_PROXY", value.clone()));
            pairs.push(("no_proxy", value.clone()));
        }
        pairs
    }
}

fn remote_wrapper_has_passwordless_sudo(root: &Path, options: &Options) -> RunnerResult<bool> {
    ssh_success_quiet(
        root,
        &options.remote_host,
        format!(
            "sudo -n {} --help",
            shell_escape(&options.benchmark_wrapper)
        ),
    )
}

fn remote_benchmark_command(options: &Options) -> String {
    let env_pairs = options.proxy.env_pairs(&options.docker_run_network);
    let preserve_env_arg = format!(
        "--preserve-env={}",
        env_pairs
            .iter()
            .map(|(name, _)| *name)
            .collect::<Vec<_>>()
            .join(",")
    );
    let remote_exports = env_pairs
        .iter()
        .map(|(name, value)| shell_escape(&format!("{name}={value}")))
        .map(|pair| format!(" export {pair};"))
        .collect::<String>();
    format!(
        "cd {} &&{} sudo -n {} {} --skip-build --repeat {} --output-dir {}",
        remote_shell_path(&options.remote_dir),
        remote_exports,
        preserve_env_arg,
        shell_escape(&options.benchmark_wrapper),
        shell_escape(&options.repeat),
        shell_escape(&options.output_dir_remote)
    )
}

fn find_remote_summary(options: &Options, root: &Path) -> RunnerResult<String> {
    let candidates = [
        format!(
            "{}/{}/summary.json",
            options.artifact_dir, options.output_dir_remote
        ),
        format!(
            "{}/{}/summary.json",
            options.remote_dir, options.output_dir_remote
        ),
        format!(
            "$HOME/kyuubiki-bench-709b8c9/{}/summary.json",
            options.output_dir_remote
        ),
        format!("$HOME/kyuubiki/{}/summary.json", options.output_dir_remote),
    ];
    let candidate_words = candidates
        .iter()
        .map(|candidate| shell_escape(candidate))
        .collect::<Vec<_>>()
        .join(" ");
    let script = format!(
        "set -e\nfor candidate in {candidate_words}; do\n  resolved_candidate=$(eval printf '%s' \"$candidate\")\n  if [ -f \"$resolved_candidate\" ]; then\n    printf '%s\\n' \"$resolved_candidate\"\n    exit 0\n  fi\ndone\nexit 1\n"
    );
    ssh_output(root, &options.remote_host, script).map_err(|_| {
        format!(
            "failed to locate remote summary.json under {} on {}",
            options.output_dir_remote, options.remote_host
        )
    })
}

fn run_compare(root: &Path, options: &Options) -> RunnerResult<u8> {
    run_node(
        root,
        "compare-direct-mesh-benchmark.mjs",
        vec![
            "--current".into(),
            options.current_summary_local.clone().into_os_string(),
            "--baseline".into(),
            options.baseline_path.clone().into_os_string(),
            "--json-out".into(),
            options.compare_json_local.clone().into_os_string(),
            "--report-out".into(),
            options.compare_md_local.clone().into_os_string(),
            "--fail-on-elapsed-regression-pct".into(),
            OsString::from(&options.elapsed_threshold),
            "--fail-on-rss-regression-pct".into(),
            OsString::from(&options.rss_threshold),
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

fn env_nonempty(name: &str) -> Option<String> {
    env::var(name).ok().filter(|value| !value.is_empty())
}

fn env_path_or(name: &str, fallback: PathBuf) -> PathBuf {
    env::var_os(name).map(PathBuf::from).unwrap_or(fallback)
}

fn print_usage() {
    println!(
        "Usage:\n  ./scripts/kyuubiki direct-mesh-benchmark-regression\n\n\
Runs the remote direct-mesh Docker benchmark, copies summary.json locally, and\n\
compares it against tests/integration/benchmarks/direct-mesh-docker-baseline.json.\n\n\
Environment:\n  KYUUBIKI_LAB_HOST\n  KYUUBIKI_LAB_BENCH_DIR\n  KYUUBIKI_LAB_ARTIFACT_DIR\n  KYUUBIKI_LAB_BENCHMARK_WRAPPER\n  OUTPUT_SLUG\n  OUTPUT_DIR_REMOTE\n  REPEAT\n  DOCKER_RUN_NETWORK\n  HTTP_PROXY / HTTPS_PROXY / NO_PROXY\n  CURRENT_SUMMARY_LOCAL\n  COMPARE_JSON_LOCAL\n  COMPARE_MD_LOCAL\n  BASELINE_PATH\n  DIRECT_MESH_ELAPSED_THRESHOLD\n  DIRECT_MESH_RSS_THRESHOLD\n"
    );
}
