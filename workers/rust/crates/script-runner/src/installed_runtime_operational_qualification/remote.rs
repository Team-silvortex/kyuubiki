use super::{CaptureContract, Contract};
use crate::native_time::utc_timestamp_slug;
use crate::remote_host::{
    remote_shell_path, rsync_to, scp_from, shell_escape, ssh_status, ssh_success_quiet,
    valid_ssh_alias,
};
use serde_json::{Value, json};
use std::ffi::OsString;
use std::fs;
use std::path::{Component, Path, PathBuf};

type RunnerResult<T> = Result<T, String>;

const CAPTURE_FILES: &[&str] = &[
    "solve.json",
    "fetch-report.json",
    "detached-fetch-report.json",
    "detached-status.txt",
    "installed-digests.txt",
];

pub(super) fn run(root: &Path, args: Vec<OsString>) -> RunnerResult<u8> {
    if args
        .iter()
        .any(|arg| matches!(arg.to_string_lossy().as_ref(), "--help" | "-h"))
    {
        print_usage();
        return Ok(0);
    }
    let contract: Contract = super::read_json(root, super::CONTRACT_PATH)?;
    super::validate_contract(root, &contract)?;
    let options = Options::parse(root, &contract, args)?;
    prepare_remote(root, &options)?;
    let local_capture = root.join("tmp").join(&options.slug);
    let capture = capture_remote(root, &options, &local_capture);
    let cleanup = cleanup_remote(root, &options);
    let result = match (capture, cleanup) {
        (Ok(()), Ok(())) => finalize_report(root, &contract, &options, &local_capture),
        (Err(error), Ok(())) => Err(error),
        (Ok(()), Err(error)) => Err(error),
        (Err(error), Err(cleanup)) => Err(format!("{error}; {cleanup}")),
    };
    let _ = fs::remove_dir_all(&local_capture);
    result?;
    println!(
        "remote installed Runtime operational qualification passed: {}",
        display_path(root, &options.output)
    );
    Ok(0)
}

struct Options {
    host: String,
    output: PathBuf,
    remote_run_root: String,
    slug: String,
    package_version: String,
    otp_version: String,
    elixir_version: String,
}

impl Options {
    fn parse(root: &Path, contract: &Contract, args: Vec<OsString>) -> RunnerResult<Self> {
        let slug = format!("installed-runtime-operational-{}", utc_timestamp_slug());
        let toolchains = toolchains(root)?;
        let mut options = Self {
            host: std::env::var("KYUUBIKI_LAB_HOST").unwrap_or_else(|_| "kyuubiki-lab".to_string()),
            output: repo_path(root, &contract.retention.report_path)?,
            remote_run_root: format!("~/.kyuubiki/lab-runs/{slug}"),
            slug,
            package_version: workspace_version(root)?,
            otp_version: toolchains.0,
            elixir_version: toolchains.1,
        };
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.to_string_lossy().as_ref() {
                "--host" => options.host = next_string(&mut args, "--host")?,
                "--out" => options.output = repo_path(root, &next_string(&mut args, "--out")?)?,
                other => return Err(format!("unknown installed Runtime remote option: {other}")),
            }
        }
        if !valid_ssh_alias(&options.host) {
            return Err("remote qualification host must be a plain SSH alias".to_string());
        }
        if options.package_version != contract.capture.package_version {
            return Err(format!(
                "workspace version {} does not match installed Runtime contract {}",
                options.package_version, contract.capture.package_version
            ));
        }
        Ok(options)
    }
}

fn prepare_remote(root: &Path, options: &Options) -> RunnerResult<()> {
    let run_root = remote_shell_path(&options.remote_run_root);
    require_zero(
        "prepare managed remote run root",
        ssh_status(
            root,
            &options.host,
            format!(
                "set -eu; umask 077; case {run_root} in \"$HOME/.kyuubiki/lab-runs/\"*) ;; *) exit 2 ;; esac; mkdir -p {run_root}/source/workers/rust {run_root}/source/apps/web {run_root}/source/config"
            ),
        )?,
    )
}

fn capture_remote(root: &Path, options: &Options, local: &Path) -> RunnerResult<()> {
    sync_sources(root, options)?;
    require_zero(
        "run installed Runtime host capture",
        ssh_status(root, &options.host, remote_capture_command(options))?,
    )?;
    fs::create_dir_all(local)
        .map_err(|error| format!("failed to create {}: {error}", local.display()))?;
    for name in CAPTURE_FILES {
        require_zero(
            "retrieve installed Runtime capture",
            scp_from(
                root,
                &options.host,
                &format!("{}/capture/{name}", options.remote_run_root),
                &local.join(name),
            )?,
        )?;
    }
    Ok(())
}

fn sync_sources(root: &Path, options: &Options) -> RunnerResult<()> {
    for (sources, destination) in [
        (
            vec![root.join("workers/rust/")],
            format!(
                "{}:{}/source/workers/rust/",
                options.host, options.remote_run_root
            ),
        ),
        (
            ["mix.exs", "mix.lock", "config", "lib"]
                .into_iter()
                .map(|path| root.join("apps/web").join(path))
                .collect(),
            format!(
                "{}:{}/source/apps/web/",
                options.host, options.remote_run_root
            ),
        ),
        (
            vec![root.join(".env.example")],
            format!("{}:{}/source/", options.host, options.remote_run_root),
        ),
        (
            vec![root.join("config/toolchains.json")],
            format!(
                "{}:{}/source/config/",
                options.host, options.remote_run_root
            ),
        ),
    ] {
        require_zero(
            "sync installed Runtime qualification source",
            rsync_to(
                root,
                &[
                    "target/",
                    "deps/",
                    "_build/",
                    "tmp/",
                    "erl_crash.dump",
                    ".DS_Store",
                ],
                &sources,
                &destination,
            )?,
        )?;
    }
    Ok(())
}

fn remote_capture_command(options: &Options) -> String {
    let run_root = remote_shell_path(&options.remote_run_root);
    let version = shell_escape(&options.package_version);
    let version_path = &options.package_version;
    let otp = &options.otp_version;
    let elixir = &options.elixir_version;
    format!(
        "set -euo pipefail; umask 077; run_root={run_root}; source_root=\"$run_root/source\"; \
target_root=\"$HOME/.kyuubiki/cache/cargo-target/installed-runtime-operational\"; \
mix_cache=\"$HOME/.kyuubiki/cache/mix/installed-runtime-operational\"; payload=\"$run_root/payload\"; release_path=\"$run_root/orchestra-release\"; \
export PATH=\"$HOME/.elixir-install/installs/otp/{otp}/bin:$PATH\"; export PATH=\"$HOME/.elixir-install/installs/elixir/{elixir}/bin:$PATH\"; \
export MIX_HOME=\"$HOME/.kyuubiki/toolchains/mix/elixir-{elixir}-otp-{otp}\"; export HEX_HOME=\"$MIX_HOME/hex\"; export MIX_DEPS_PATH=\"$mix_cache/deps\"; export MIX_BUILD_PATH=\"$mix_cache/build\"; \
mkdir -p \"$target_root\" \"$MIX_HOME\" \"$HEX_HOME\" \"$mix_cache\"; cd \"$source_root/workers/rust\"; \
CARGO_TARGET_DIR=\"$target_root\" cargo +1.88.0 build --release -p kyuubiki-installer -p kyuubiki-script-runner -p kyuubiki-cli -p kyuubiki-desktop-runtime \
--bin kyuubiki-installer --bin kyuubiki-script-runner --bin kyuubiki-cli --bin kyuubiki-headless --bin kyuubiki-runtime; \
cd \"$source_root/apps/web\"; mix help hex.info >/dev/null 2>&1 || mix local.hex --force >/dev/null; MIX_ENV=prod mix deps.get; MIX_ENV=prod KYUUBIKI_RELEASE_VERSION={version} mix release kyuubiki_web --overwrite --path \"$release_path\"; \
installer=\"$target_root/release/kyuubiki-installer\"; runner=\"$target_root/release/kyuubiki-script-runner\"; export KYUUBIKI_REPO_ROOT=\"$source_root\"; \
\"$installer\" stage-release linux \"$payload\"; install -m 0755 \"$target_root/release/kyuubiki-cli\" \"$payload/bin/kyuubiki-cli\"; \
install -m 0755 \"$target_root/release/kyuubiki-runtime\" \"$payload/bin/kyuubiki-runtime\"; install -m 0755 \"$target_root/release/kyuubiki-headless\" \"$payload/bin/kyuubiki-headless\"; \
mkdir -p \"$payload/services\"; rm -rf \"$payload/services/orchestrator\"; cp -a \"$release_path\" \"$payload/services/orchestrator\"; \
mkdir -p \"$payload/services/frontend\"; printf '%s\\n' '<!doctype html><title>Kyuubiki native frontend qualification</title>' > \"$payload/services/frontend/index.html\"; \
\"$installer\" seal-runtime-payload \"$payload\" {version} linux; export XDG_CONFIG_HOME=\"$run_root/xdg\"; \"$installer\" install-runtime-payload \"$payload\"; \
store=\"$XDG_CONFIG_HOME/kyuubiki/runtime\"; runtime=\"$store/versions/{version_path}\"; test -x \"$runtime/bin/kyuubiki-headless\"; test -x \"$runtime/services/orchestrator/bin/kyuubiki_web\"; \
rm -rf \"$source_root\" \"$payload\" \"$release_path\"; harness=\"$run_root/harness-repo\"; mkdir -p \"$harness/workers/rust\" \"$harness/scripts\"; : > \"$harness/workers/rust/Cargo.toml\"; \
KYUUBIKI_REPO_ROOT=\"$harness\" \"$runner\" capture-installed-runtime-operational-host --managed-root \"$run_root\" --runtime-root \"$runtime\" --detached-source-root \"$source_root\" --out \"$run_root/capture\" --package-version {version}"
    )
}

fn cleanup_remote(root: &Path, options: &Options) -> RunnerResult<()> {
    let run_root = remote_shell_path(&options.remote_run_root);
    require_zero(
        "clean managed remote qualification root",
        ssh_status(
            root,
            &options.host,
            format!(
                "set -eu; case {run_root} in \"$HOME/.kyuubiki/lab-runs/\"*) rm -rf {run_root} ;; *) exit 2 ;; esac; test ! -e {run_root}"
            ),
        )?,
    )?;
    if !ssh_success_quiet(
        root,
        &options.host,
        format!("test ! -e {}", remote_shell_path(&options.remote_run_root)),
    )? {
        return Err("remote installed Runtime root remains after cleanup".to_string());
    }
    Ok(())
}

fn finalize_report(
    root: &Path,
    contract: &Contract,
    options: &Options,
    capture_root: &Path,
) -> RunnerResult<()> {
    fs::write(
        capture_root.join("cleanup.json"),
        serde_json::to_vec_pretty(&json!({
            "runtime_ports_closed": true,
            "managed_pid_files_removed": true,
            "source_tree_removed": true,
            "managed_remote_root_removed": true
        }))
        .map_err(|error| error.to_string())?,
    )
    .map_err(|error| format!("failed to write cleanup capture: {error}"))?;
    let capture = capture_contract(root, capture_root, contract)?;
    let captures = super::load_captures(root, &capture)?;
    super::validate_captures(&capture, &captures)?;
    let report = super::build_report(contract, &captures)?;
    super::validate_report(contract, &report)?;
    promote_report(&options.output, &report)
}

fn capture_contract(
    root: &Path,
    capture_root: &Path,
    contract: &Contract,
) -> RunnerResult<CaptureContract> {
    let relative = capture_root
        .strip_prefix(root)
        .map_err(|_| "local capture root escapes repository".to_string())?;
    let path = |name: &str| relative.join(name).to_string_lossy().to_string();
    Ok(CaptureContract {
        solve_report: path("solve.json"),
        restart_fetch_report: path("fetch-report.json"),
        detached_fetch_report: path("detached-fetch-report.json"),
        detached_status: path("detached-status.txt"),
        cleanup_report: path("cleanup.json"),
        installed_digests: path("installed-digests.txt"),
        execution_host_role: contract.capture.execution_host_role.clone(),
        platform: contract.capture.platform.clone(),
        architecture: contract.capture.architecture.clone(),
        package_version: contract.capture.package_version.clone(),
        workflow_id: contract.capture.workflow_id.clone(),
        minimum_agent_count: contract.capture.minimum_agent_count,
        restart_count: contract.capture.restart_count,
    })
}

fn promote_report(output: &Path, report: &Value) -> RunnerResult<()> {
    let parent = output.parent().ok_or("report output has no parent")?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    let temporary = output.with_extension(format!("partial-{}", std::process::id()));
    fs::write(
        &temporary,
        serde_json::to_vec_pretty(report).map_err(|error| error.to_string())?,
    )
    .map_err(|error| format!("failed to stage report: {error}"))?;
    fs::rename(&temporary, output)
        .map_err(|error| format!("failed to promote report {}: {error}", output.display()))
}

fn workspace_version(root: &Path) -> RunnerResult<String> {
    let text = fs::read_to_string(root.join("workers/rust/Cargo.toml"))
        .map_err(|error| format!("failed to read workspace version: {error}"))?;
    text.split_once("[workspace.package]")
        .and_then(|(_, section)| {
            section
                .lines()
                .find_map(|line| line.trim().strip_prefix("version = \"")?.strip_suffix('"'))
        })
        .filter(|value| super::valid_version(value))
        .map(ToString::to_string)
        .ok_or_else(|| "Rust workspace version is missing".to_string())
}

fn toolchains(root: &Path) -> RunnerResult<(String, String)> {
    let value: Value = serde_json::from_slice(
        &fs::read(root.join("config/toolchains.json"))
            .map_err(|error| format!("failed to read toolchain contract: {error}"))?,
    )
    .map_err(|error| format!("invalid toolchain contract: {error}"))?;
    let get = |pointer: &str| {
        value
            .pointer(pointer)
            .and_then(Value::as_str)
            .filter(|value| super::valid_version(value))
            .map(ToString::to_string)
            .ok_or_else(|| format!("toolchain contract misses {pointer}"))
    };
    Ok((get("/elixir/lab_otp")?, get("/elixir/lab_elixir")?))
}

fn repo_path(root: &Path, relative: &str) -> RunnerResult<PathBuf> {
    let path = Path::new(relative);
    if path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(format!("qualification path escapes repository: {relative}"));
    }
    Ok(root.join(path))
}

fn next_string(args: &mut impl Iterator<Item = OsString>, option: &str) -> RunnerResult<String> {
    args.next()
        .and_then(|value| value.into_string().ok())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{option} requires a UTF-8 value"))
}

fn require_zero(label: &str, status: u8) -> RunnerResult<()> {
    if status == 0 {
        Ok(())
    } else {
        Err(format!("{label} failed with status {status}"))
    }
}

fn display_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn print_usage() {
    println!(
        "usage: kyuubiki-script-runner qualify-installed-runtime-operational-remote [--host SSH_ALIAS] [--out path]"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remote_command_installs_headless_before_source_detach() {
        let options = Options {
            host: "lab".into(),
            output: PathBuf::from("report.json"),
            remote_run_root: "~/.kyuubiki/lab-runs/test".into(),
            slug: "test".into(),
            package_version: "2.19.0".into(),
            otp_version: "28.4".into(),
            elixir_version: "1.20.1-otp-28".into(),
        };
        let command = remote_capture_command(&options);
        assert!(command.contains("bin/kyuubiki-headless"));
        assert!(command.contains("seal-runtime-payload"));
        assert!(command.contains("capture-installed-runtime-operational-host"));
        assert_eq!(command.matches("cargo +1.88.0 build").count(), 1);
        assert!(!command.contains("node "));
    }
}
