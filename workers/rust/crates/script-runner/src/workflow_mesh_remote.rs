use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

type RunnerResult<T> = Result<T, String>;

struct Options {
    local_log_path: PathBuf,
    local_output_dir: PathBuf,
    output_slug: String,
    remote_dir: String,
    remote_elixir_version: String,
    remote_host: String,
    remote_log_path: String,
    remote_otp_version: String,
    remote_output_dir: String,
    remote_pg_bin_dir: String,
    remote_pg_db: String,
    remote_pg_port: String,
    remote_pg_user: String,
    sync_to_remote: bool,
}

pub(crate) fn run_workflow_mesh_remote(root: &Path, args: Vec<OsString>) -> RunnerResult<u8> {
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_usage();
        return Ok(0);
    }
    if !args.is_empty() {
        return Err("workflow-mesh-regression-remote does not accept positional arguments".into());
    }
    let options = Options::from_env(root)?;
    std::fs::create_dir_all(&options.local_output_dir).map_err(|error| {
        format!(
            "failed to create {}: {error}",
            options.local_output_dir.display()
        )
    })?;

    if options.sync_to_remote {
        sync_workflow_mesh_sources(root, &options)?;
    }
    sync_workflow_mesh_tests(root, &options)?;

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

    for status in [
        copy_remote(
            &options,
            &options.remote_log_path,
            &options.local_log_path,
            root,
        )?,
        copy_remote(
            &options,
            &format!(
                "{}/{}/summary.json",
                options.remote_dir, options.remote_output_dir
            ),
            &options.local_output_dir.join("summary.json"),
            root,
        )?,
        copy_remote(
            &options,
            &format!(
                "{}/{}/README.md",
                options.remote_dir, options.remote_output_dir
            ),
            &options.local_output_dir.join("README.md"),
            root,
        )?,
        run_node(
            root,
            "build-workflow-mesh-regression-index.mjs",
            vec![
                "--root".into(),
                root.join("tmp/workflow-mesh-regression").into_os_string(),
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
        "remote workflow mesh regression completed on {}",
        options.remote_host
    );
    println!("local output dir: {}", options.local_output_dir.display());
    println!("local log: {}", options.local_log_path.display());
    Ok(0)
}

impl Options {
    fn from_env(root: &Path) -> RunnerResult<Self> {
        let toolchain = toolchain_env(root)?;
        let output_slug = env::var("OUTPUT_SLUG").unwrap_or_else(|_| {
            format!(
                "workflow-mesh-{}",
                timestamp_slug().unwrap_or_else(|| "manual".to_string())
            )
        });
        let remote_dir =
            env::var("KYUUBIKI_LAB_WORKFLOW_MESH_DIR").unwrap_or_else(|_| "~/kyuubiki".to_string());
        let remote_output_dir = env::var("REMOTE_OUTPUT_DIR")
            .unwrap_or_else(|_| format!("tmp/workflow-mesh-regression/{output_slug}"));
        let local_output_dir = env_path_or(
            "LOCAL_OUTPUT_DIR",
            root.join("tmp/workflow-mesh-regression").join(&output_slug),
        );
        Ok(Self {
            local_log_path: env_path_or("LOCAL_LOG_PATH", local_output_dir.join("run.log")),
            local_output_dir,
            output_slug,
            remote_elixir_version: env::var("REMOTE_ELIXIR_VERSION")
                .ok()
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| toolchain.remote_elixir_version),
            remote_host: env::var("KYUUBIKI_LAB_HOST").unwrap_or_else(|_| "kyuubiki-lab".into()),
            remote_log_path: env::var("REMOTE_LOG_PATH")
                .unwrap_or_else(|_| format!("{remote_dir}/{remote_output_dir}/run.log")),
            remote_otp_version: env::var("REMOTE_OTP_VERSION")
                .ok()
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| toolchain.remote_otp_version),
            remote_dir,
            remote_output_dir,
            remote_pg_bin_dir: env::var("REMOTE_PG_BIN_DIR")
                .unwrap_or_else(|_| "/usr/lib/postgresql/16/bin".to_string()),
            remote_pg_db: env::var("REMOTE_PG_DB")
                .unwrap_or_else(|_| "kyuubiki_mesh_test".to_string()),
            remote_pg_port: env::var("REMOTE_PG_PORT").unwrap_or_else(|_| "55432".to_string()),
            remote_pg_user: env::var("REMOTE_PG_USER").unwrap_or_else(|_| "kyuubiki".to_string()),
            sync_to_remote: env::var("SYNC_TO_REMOTE").unwrap_or_else(|_| "1".into()) == "1",
        })
    }
}

struct ToolchainEnv {
    remote_elixir_version: String,
    remote_otp_version: String,
}

fn toolchain_env(root: &Path) -> RunnerResult<ToolchainEnv> {
    let output = Command::new("node")
        .args([root.join("scripts/toolchain-env.mjs"), "--json".into()])
        .current_dir(root)
        .output()
        .map_err(|error| format!("failed to run scripts/toolchain-env.mjs: {error}"))?;
    if !output.status.success() {
        return Err("scripts/toolchain-env.mjs --json failed".to_string());
    }
    let json = String::from_utf8_lossy(&output.stdout);
    Ok(ToolchainEnv {
        remote_elixir_version: json_string_value(&json, "KYUUBIKI_REMOTE_ELIXIR_VERSION")
            .unwrap_or_else(|| "1.20.1-otp-28".to_string()),
        remote_otp_version: json_string_value(&json, "KYUUBIKI_REMOTE_OTP_VERSION")
            .unwrap_or_else(|| "28.4".to_string()),
    })
}

fn sync_workflow_mesh_sources(root: &Path, options: &Options) -> RunnerResult<()> {
    for status in [
        rsync(
            root,
            &[],
            &[root.join("Makefile")],
            &format!("{}:{}/Makefile", options.remote_host, options.remote_dir),
        )?,
        rsync(
            root,
            &[],
            &[
                root.join("scripts/build-workflow-mesh-regression-index.mjs"),
                root.join("scripts/build-workflow-mesh-regression-summary.mjs"),
                root.join("scripts/build-nightly-artifact-overview.mjs"),
                root.join("scripts/kyuubiki-runtime.mjs"),
                root.join("scripts/run-workflow-mesh-regression.sh"),
                root.join("scripts/run-workflow-mesh-regression-remote.sh"),
            ],
            &format!("{}:{}/scripts/", options.remote_host, options.remote_dir),
        )?,
        rsync(
            root,
            &[],
            &[root.join("apps/frontend/public/models/")],
            &format!(
                "{}:{}/apps/frontend/public/models/",
                options.remote_host, options.remote_dir
            ),
        )?,
        rsync(
            root,
            &["_build/", "deps/"],
            &[root.join("apps/web/")],
            &format!("{}:{}/apps/web/", options.remote_host, options.remote_dir),
        )?,
        rsync(
            root,
            &["target/"],
            &[root.join("workers/rust/")],
            &format!(
                "{}:{}/workers/rust/",
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

fn sync_workflow_mesh_tests(root: &Path, options: &Options) -> RunnerResult<()> {
    let status = rsync(
        root,
        &[],
        &[
            root.join("tests/integration/workflow-distributed-smoke.test.mjs"),
            root.join("tests/integration/workflow-offline-mesh-smoke.test.mjs"),
            root.join("tests/integration/workflow-offline-mesh-branch-diagnostics-smoke.test.mjs"),
        ],
        &format!(
            "{}:{}/tests/integration/",
            options.remote_host, options.remote_dir
        ),
    )?;
    if status == 0 {
        Ok(())
    } else {
        Err(format!("rsync failed with status {status}"))
    }
}

fn remote_command(options: &Options) -> String {
    let workspace = remote_shell_path(&options.remote_dir);
    let pg_bin = shell_escape(&options.remote_pg_bin_dir);
    let output_dir = shell_double_fragment(&options.remote_output_dir);
    let pg_port = shell_double_fragment(&options.remote_pg_port);
    let url_db = shell_double_fragment(&options.remote_pg_db);
    let url_port = shell_double_fragment(&options.remote_pg_port);
    let url_user = shell_double_fragment(&options.remote_pg_user);
    format!(
        "set -euo pipefail; \
remote_elixir_installs_dir=${{REMOTE_ELIXIR_INSTALLS_DIR:-$HOME/.elixir-install/installs}}; \
remote_workspace_root={workspace}; \
export PATH=\"$remote_elixir_installs_dir/otp/{}/bin:$PATH\"; \
export PATH=\"$remote_elixir_installs_dir/elixir/{}/bin:$PATH\"; \
remote_pg_root=\"$remote_workspace_root/{output_dir}/postgres\"; \
remote_pg_data=\"$remote_pg_root/data\"; \
remote_pg_socket=\"/tmp\"; \
mkdir -p \"$remote_workspace_root/{output_dir}\" \"$remote_pg_root\"; \
if [ ! -f \"$remote_pg_data/PG_VERSION\" ]; then \
  {pg_bin}/initdb -D \"$remote_pg_data\" -U {} --auth-local=trust --auth-host=trust >/dev/null; \
fi; \
{pg_bin}/pg_ctl -D \"$remote_pg_data\" -o \"-F -p {pg_port} -k $remote_pg_socket -h 127.0.0.1\" -l \"$remote_pg_root/postgres.log\" start >/dev/null; \
trap '{pg_bin}/pg_ctl -D \"$remote_pg_data\" stop -m fast >/dev/null 2>&1 || true' EXIT; \
{pg_bin}/createdb -h 127.0.0.1 -p {} -U {} {} >/dev/null 2>&1 || true; \
export DATABASE_URL=\"ecto://{url_user}@127.0.0.1:{url_port}/{url_db}\"; \
export OUTPUT_SLUG={}; \
export OUTPUT_DIR=\"$remote_workspace_root/{output_dir}\"; \
export LOG_PATH={}; \
cd \"$remote_workspace_root\" && make test-integration-workflow-mesh 2>&1 | tee {}",
        shell_double_fragment(&options.remote_otp_version),
        shell_double_fragment(&options.remote_elixir_version),
        shell_escape(&options.remote_pg_user),
        shell_escape(&options.remote_pg_port),
        shell_escape(&options.remote_pg_user),
        shell_escape(&options.remote_pg_db),
        shell_escape(&options.output_slug),
        remote_log_shell_path(&options.remote_log_path),
        remote_log_shell_path(&options.remote_log_path),
    )
}

fn rsync(
    root: &Path,
    excludes: &[&str],
    sources: &[PathBuf],
    destination: &str,
) -> RunnerResult<u8> {
    let mut args = vec![OsString::from("-az")];
    for exclude in excludes {
        args.push(OsString::from("--exclude"));
        args.push(OsString::from(exclude));
    }
    args.extend(sources.iter().map(|path| path.clone().into_os_string()));
    args.push(OsString::from(destination));
    run_status("rsync", args, root)
}

fn copy_remote(
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

fn json_string_value(json: &str, key: &str) -> Option<String> {
    let (_, rest) = json.split_once(&format!("\"{key}\""))?;
    let (_, rest) = rest.split_once(':')?;
    let rest = rest.trim_start();
    let value = rest.strip_prefix('"')?.split('"').next()?;
    Some(value.to_string())
}

fn shell_escape(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn shell_double_fragment(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('$', "\\$")
        .replace('`', "\\`")
}

fn remote_shell_path(value: &str) -> String {
    value
        .strip_prefix("~/")
        .map(|rest| format!("$HOME/{}", shell_escape(rest)))
        .unwrap_or_else(|| shell_escape(value))
}

fn remote_log_shell_path(value: &str) -> String {
    if let Some(rest) = value.strip_prefix("~/") {
        return format!("$HOME/{}", shell_escape(rest));
    }
    shell_escape(value)
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
        "Usage:\n  ./scripts/kyuubiki workflow-mesh-regression-remote\n\n\
Syncs workflow mesh runtime/test inputs to the shared lab machine, runs the\n\
distributed workflow mesh regression trio there, and pulls local artifacts.\n\n\
Environment:\n  KYUUBIKI_LAB_HOST\n  KYUUBIKI_LAB_WORKFLOW_MESH_DIR\n  OUTPUT_SLUG\n  LOCAL_OUTPUT_DIR\n  REMOTE_OUTPUT_DIR\n  LOCAL_LOG_PATH\n  REMOTE_LOG_PATH\n  REMOTE_OTP_VERSION\n  REMOTE_ELIXIR_VERSION\n  REMOTE_PG_BIN_DIR\n  REMOTE_PG_PORT\n  REMOTE_PG_USER\n  REMOTE_PG_DB\n  SYNC_TO_REMOTE\n"
    );
}
