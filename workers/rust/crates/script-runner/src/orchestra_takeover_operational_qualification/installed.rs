use super::{installed_report, installed_runtime};
use crate::native_time::utc_timestamp_slug;
use crate::remote_host::{
    remote_shell_path, rsync_to, scp_from, shell_escape, ssh_status, valid_ssh_alias,
};
use serde_json::Value;
use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::time::Duration;

type RunnerResult<T> = Result<T, String>;

const EVIDENCE_ROOT: &str = "~/.kyuubiki/lab-evidence/orchestra-installed-takeover";

pub(crate) fn run_qualify_remote(root: &Path, args: Vec<OsString>) -> RunnerResult<u8> {
    let options = RemoteOptions::parse(root, args)?;
    if options.help {
        print_remote_usage();
        return Ok(0);
    }
    installed_report::validate_contract(root, false)?;
    prepare_remote_run(root, &options)?;
    let capture = capture_remote_report(root, &options);
    let cleanup = cleanup_remote_run(root, &options);
    match (capture, cleanup) {
        (Ok(0), Ok(())) => {
            println!(
                "remote Installer-managed Orchestra takeover qualification passed: {}",
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

pub(crate) fn run_capture_host(args: Vec<OsString>) -> RunnerResult<u8> {
    let options = HostOptions::parse(args)?;
    let (journey, cleanup) = installed_runtime::capture(
        &options.managed_root,
        &options.runtime_root,
        &options.detached_source_root,
        &options.package_version,
        &options.postgres_image,
        Duration::from_secs(options.timeout_seconds),
    )?;
    let report = installed_report::build_report(journey, cleanup)?;
    installed_report::validate_host_capture(&report, &options.package_version)?;
    installed_report::write_path(&options.output, &report)?;
    println!(
        "installed Orchestra host capture passed: {}",
        options.output.display()
    );
    Ok(0)
}

pub(crate) fn run_check(root: &Path, args: Vec<OsString>) -> RunnerResult<u8> {
    let options = CheckOptions::parse(args)?;
    if options.help {
        print_check_usage();
        return Ok(0);
    }
    installed_report::validate_contract(root, false)?;
    if options.self_test {
        installed_report::validator_self_test(root)?;
        println!("Installed Orchestra takeover validator self-test passed");
        if options.report.is_none() {
            return Ok(0);
        }
    }
    let relative = options
        .report
        .unwrap_or_else(|| installed_report::DEFAULT_REPORT.to_string());
    let report = installed_report::read_path(&repo_path(root, &relative)?)?;
    installed_report::validate(root, &report)?;
    println!("Installed Orchestra takeover report passed: {relative}");
    Ok(0)
}

struct RemoteOptions {
    help: bool,
    host: String,
    output: PathBuf,
    remote_run_root: String,
    evidence_name: String,
    run_id: String,
    package_version: String,
    postgres_image: String,
    timeout_seconds: u64,
    otp_version: String,
    elixir_version: String,
    node_version: String,
}

impl RemoteOptions {
    fn parse(root: &Path, args: Vec<OsString>) -> RunnerResult<Self> {
        let package_version = workspace_version(root)?;
        let toolchains = toolchains(root)?;
        let run_id = format!("orchestra-installed-takeover-{}", utc_timestamp_slug());
        let mut options = Self {
            help: false,
            host: env::var("KYUUBIKI_LAB_HOST").unwrap_or_else(|_| "kyuubiki-lab".to_string()),
            output: root.join(installed_report::DEFAULT_CAPTURE),
            remote_run_root: format!("~/.kyuubiki/lab-runs/{run_id}"),
            evidence_name: format!("{run_id}.json"),
            run_id,
            package_version,
            postgres_image: env::var("KYUUBIKI_POSTGRES_QUALIFICATION_IMAGE")
                .unwrap_or_else(|_| "postgres:16-alpine".to_string()),
            timeout_seconds: 120,
            otp_version: toolchains.0,
            elixir_version: toolchains.1,
            node_version: toolchains.2,
        };
        let mut iter = args.into_iter();
        while let Some(arg) = iter.next() {
            match arg.to_string_lossy().as_ref() {
                "--help" | "-h" => options.help = true,
                "--host" => options.host = next_string(&mut iter, "--host")?,
                "--out" => options.output = repo_path(root, &next_string(&mut iter, "--out")?)?,
                "--postgres-image" => {
                    options.postgres_image = next_string(&mut iter, "--postgres-image")?
                }
                "--timeout-secs" => {
                    options.timeout_seconds = next_string(&mut iter, "--timeout-secs")?
                        .parse()
                        .map_err(|_| "--timeout-secs requires an integer".to_string())?;
                }
                other => return Err(format!("unknown installed takeover option: {other}")),
            }
        }
        if !valid_ssh_alias(&options.host) {
            return Err("remote qualification host must be a plain SSH alias".to_string());
        }
        if !valid_image_reference(&options.postgres_image) {
            return Err("PostgreSQL image must be a portable container reference".to_string());
        }
        if !(30..=300).contains(&options.timeout_seconds) {
            return Err("--timeout-secs must be between 30 and 300".to_string());
        }
        Ok(options)
    }
}

struct HostOptions {
    managed_root: PathBuf,
    runtime_root: PathBuf,
    detached_source_root: PathBuf,
    output: PathBuf,
    package_version: String,
    postgres_image: String,
    timeout_seconds: u64,
}

impl HostOptions {
    fn parse(args: Vec<OsString>) -> RunnerResult<Self> {
        let mut managed_root = None;
        let mut runtime_root = None;
        let mut detached_source_root = None;
        let mut output = None;
        let mut package_version = None;
        let mut postgres_image = "postgres:16-alpine".to_string();
        let mut timeout_seconds = 120;
        let mut iter = args.into_iter();
        while let Some(arg) = iter.next() {
            match arg.to_string_lossy().as_ref() {
                "--managed-root" => {
                    managed_root = Some(next_absolute_path(&mut iter, "--managed-root")?)
                }
                "--runtime-root" => {
                    runtime_root = Some(next_absolute_path(&mut iter, "--runtime-root")?)
                }
                "--detached-source-root" => {
                    detached_source_root =
                        Some(next_absolute_path(&mut iter, "--detached-source-root")?)
                }
                "--out" => output = Some(next_absolute_path(&mut iter, "--out")?),
                "--package-version" => {
                    package_version = Some(next_string(&mut iter, "--package-version")?)
                }
                "--postgres-image" => postgres_image = next_string(&mut iter, "--postgres-image")?,
                "--timeout-secs" => {
                    timeout_seconds = next_string(&mut iter, "--timeout-secs")?
                        .parse()
                        .map_err(|_| "--timeout-secs requires an integer".to_string())?;
                }
                other => return Err(format!("unknown installed host capture option: {other}")),
            }
        }
        let options = Self {
            managed_root: managed_root.ok_or("--managed-root is required")?,
            runtime_root: runtime_root.ok_or("--runtime-root is required")?,
            detached_source_root: detached_source_root
                .ok_or("--detached-source-root is required")?,
            output: output.ok_or("--out is required")?,
            package_version: package_version.ok_or("--package-version is required")?,
            postgres_image,
            timeout_seconds,
        };
        if options.output.starts_with(&options.managed_root) {
            return Err("host capture output must survive managed-root cleanup".to_string());
        }
        if !valid_version(&options.package_version)
            || !valid_image_reference(&options.postgres_image)
            || !(30..=300).contains(&options.timeout_seconds)
        {
            return Err("installed host capture options are invalid".to_string());
        }
        Ok(options)
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
        let mut iter = args.into_iter();
        while let Some(arg) = iter.next() {
            match arg.to_string_lossy().as_ref() {
                "--help" | "-h" => options.help = true,
                "--self-test" => options.self_test = true,
                "--verify-report" | "--in" => {
                    options.report = Some(next_string(&mut iter, "--verify-report")?)
                }
                other => return Err(format!("unknown installed takeover check option: {other}")),
            }
        }
        Ok(options)
    }
}

fn prepare_remote_run(root: &Path, options: &RemoteOptions) -> RunnerResult<()> {
    let run_root = remote_shell_path(&options.remote_run_root);
    let status = ssh_status(
        root,
        &options.host,
        format!(
            "set -eu; umask 077; case {run_root} in \"$HOME/.kyuubiki/lab-runs/\"*) ;; *) exit 2 ;; esac; mkdir -p {run_root}/source/workers/rust {run_root}/source/apps/web {run_root}/source/config"
        ),
    )?;
    if status == 0 {
        Ok(())
    } else {
        Err(format!(
            "failed to prepare managed remote run root: {status}"
        ))
    }
}

fn capture_remote_report(root: &Path, options: &RemoteOptions) -> RunnerResult<u8> {
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
        let status = rsync_to(
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
        )?;
        if status != 0 {
            return Ok(status);
        }
    }
    let status = ssh_status(root, &options.host, remote_capture_command(options))?;
    if status != 0 {
        return Ok(status);
    }
    let temporary = temporary_path(&options.output);
    let remote_report = format!("{EVIDENCE_ROOT}/{}", options.evidence_name);
    let copy = scp_from(root, &options.host, &remote_report, &temporary)?;
    if copy != 0 {
        let _ = fs::remove_file(&temporary);
        return Ok(copy);
    }
    let report = installed_report::read_path(&temporary)?;
    installed_report::validate(root, &report)?;
    promote_report(&temporary, &options.output)?;
    Ok(0)
}

fn remote_capture_command(options: &RemoteOptions) -> String {
    let run_root = remote_shell_path(&options.remote_run_root);
    let evidence_root = remote_shell_path(EVIDENCE_ROOT);
    let version = shell_escape(&options.package_version);
    let version_path = &options.package_version;
    let image = shell_escape(&options.postgres_image);
    let timeout = options.timeout_seconds;
    let run_id = shell_escape(&options.run_id);
    let evidence_name = &options.evidence_name;
    let otp = &options.otp_version;
    let elixir = &options.elixir_version;
    let node = &options.node_version;
    format!(
        "set -euo pipefail; umask 077; run_root={run_root}; source_root=\"$run_root/source\"; \
target_root=\"$HOME/.kyuubiki/cache/cargo-target/orchestra-installed-takeover\"; \
mix_cache=\"$HOME/.kyuubiki/cache/mix/orchestra-installed-takeover\"; \
evidence_root={evidence_root}; payload=\"$run_root/payload\"; release_path=\"$run_root/orchestra-release\"; \
export PATH=\"$HOME/.elixir-install/installs/otp/{otp}/bin:$PATH\"; \
export PATH=\"$HOME/.elixir-install/installs/elixir/{elixir}/bin:$PATH\"; \
export MIX_HOME=\"$HOME/.kyuubiki/toolchains/mix/elixir-{elixir}-otp-{otp}\"; \
export HEX_HOME=\"$MIX_HOME/hex\"; export MIX_DEPS_PATH=\"$mix_cache/deps\"; \
export MIX_BUILD_PATH=\"$mix_cache/build\"; mkdir -p \"$target_root\" \"$MIX_HOME\" \"$HEX_HOME\" \"$mix_cache\" \"$evidence_root\"; \
cd \"$source_root/workers/rust\"; CARGO_TARGET_DIR=\"$target_root\" cargo +1.88.0 build --release -p kyuubiki-installer -p kyuubiki-script-runner; \
CARGO_TARGET_DIR=\"$target_root\" cargo +1.88.0 build --release -p kyuubiki-cli --bin kyuubiki-cli; \
cd \"$source_root/apps/web\"; mix help hex.info >/dev/null 2>&1 || mix local.hex --force >/dev/null; \
MIX_ENV=prod mix deps.get; MIX_ENV=prod KYUUBIKI_RELEASE_VERSION={version} mix release kyuubiki_web --overwrite --path \"$release_path\"; \
installer=\"$target_root/release/kyuubiki-installer\"; runner=\"$target_root/release/kyuubiki-script-runner\"; export KYUUBIKI_REPO_ROOT=\"$source_root\"; \
\"$installer\" stage-release linux \"$payload\"; install -m 0755 \"$target_root/release/kyuubiki-cli\" \"$payload/bin/kyuubiki-cli\"; \
mkdir -p \"$payload/services\"; rm -rf \"$payload/services/orchestrator\"; cp -a \"$release_path\" \"$payload/services/orchestrator\"; \
node_a=\"$HOME/.local/kyuubiki-runtimes/node-v{node}-linux-x64/bin/node\"; node_b=\"$HOME/.kyuubiki-toolchains/node-v{node}-linux-x64/bin/node\"; \
if test -x \"$node_a\"; then node_bin=\"$node_a\"; else node_bin=\"$node_b\"; fi; test -x \"$node_bin\"; \
mkdir -p \"$payload/runtimes/linux/node/bin\" \"$payload/services/frontend\"; install -m 0755 \"$node_bin\" \"$payload/runtimes/linux/node/bin/node\"; \
printf '%s\\n' 'process.stdout.write(\"frontend qualification slice\\n\");' > \"$payload/services/frontend/server.js\"; \
\"$installer\" seal-runtime-payload \"$payload\" {version} linux; export XDG_CONFIG_HOME=\"$run_root/xdg\"; \
\"$installer\" install-runtime-payload \"$payload\"; store=\"$XDG_CONFIG_HOME/kyuubiki/runtime\"; runtime=\"$store/versions/{version_path}\"; test -x \"$runtime/services/orchestrator/bin/kyuubiki_web\"; \
rm -rf \"$source_root\" \"$payload\" \"$release_path\"; harness=\"$run_root/harness-repo\"; mkdir -p \"$harness/workers/rust\" \"$harness/scripts\"; : > \"$harness/workers/rust/Cargo.toml\"; \
cd \"$HOME\"; KYUUBIKI_REPO_ROOT=\"$harness\" KYUUBIKI_QUALIFICATION_RUN_ID={run_id} \"$runner\" capture-orchestra-installed-takeover-host \
--managed-root \"$run_root\" --runtime-root \"$runtime\" --detached-source-root \"$source_root\" \
--out \"$evidence_root/{evidence_name}\" --package-version {version} --postgres-image {image} --timeout-secs {timeout}"
    )
}

fn cleanup_remote_run(root: &Path, options: &RemoteOptions) -> RunnerResult<()> {
    let run_root = remote_shell_path(&options.remote_run_root);
    let evidence = remote_shell_path(&format!("{EVIDENCE_ROOT}/{}", options.evidence_name));
    let run_id = shell_escape(&options.run_id);
    let status = ssh_status(
        root,
        &options.host,
        format!(
            "set -eu; for id in $(docker ps -aq --filter label=io.kyuubiki.run={run_id}); do docker rm -f \"$id\" >/dev/null; done; case {run_root} in \"$HOME/.kyuubiki/lab-runs/\"*) rm -rf {run_root} ;; *) exit 2 ;; esac; rm -f {evidence}; test ! -e {run_root}; test -z \"$(docker ps -aq --filter label=io.kyuubiki.run={run_id})\""
        ),
    )?;
    if status == 0 {
        Ok(())
    } else {
        Err(format!(
            "managed remote qualification cleanup failed: {status}"
        ))
    }
}

fn workspace_version(root: &Path) -> RunnerResult<String> {
    let text = fs::read_to_string(root.join("workers/rust/Cargo.toml"))
        .map_err(|error| format!("failed to read workspace version: {error}"))?;
    text.lines()
        .map(str::trim)
        .find_map(|line| line.strip_prefix("version = \"")?.strip_suffix('"'))
        .filter(|version| valid_version(version))
        .map(ToString::to_string)
        .ok_or_else(|| "workspace version is missing or invalid".to_string())
}

fn toolchains(root: &Path) -> RunnerResult<(String, String, String)> {
    let bytes = fs::read(root.join("config/toolchains.json"))
        .map_err(|error| format!("failed to read toolchain contract: {error}"))?;
    let value: Value = serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid toolchain contract: {error}"))?;
    let get = |pointer: &str| {
        value
            .pointer(pointer)
            .and_then(Value::as_str)
            .filter(|text| valid_toolchain_value(text))
            .map(ToString::to_string)
            .ok_or_else(|| format!("toolchain contract misses {pointer}"))
    };
    Ok((
        get("/elixir/lab_otp")?,
        get("/elixir/lab_elixir")?,
        get("/node/preferred")?,
    ))
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

fn next_string(iter: &mut impl Iterator<Item = OsString>, option: &str) -> RunnerResult<String> {
    iter.next()
        .and_then(|value| value.into_string().ok())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{option} requires a UTF-8 value"))
}

fn next_absolute_path(
    iter: &mut impl Iterator<Item = OsString>,
    option: &str,
) -> RunnerResult<PathBuf> {
    let path = PathBuf::from(next_string(iter, option)?);
    if !path.is_absolute() {
        return Err(format!("{option} requires an absolute path"));
    }
    Ok(path)
}

fn valid_version(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'))
}

fn valid_image_reference(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 253
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'/' | b'_' | b'-' | b':')
        })
}

fn valid_toolchain_value(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'))
}

fn temporary_path(output: &Path) -> PathBuf {
    output.with_extension(format!("partial-{}", std::process::id()))
}

fn promote_report(temporary: &Path, output: &Path) -> RunnerResult<()> {
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    fs::rename(temporary, output).map_err(|error| {
        format!(
            "failed to promote {} to {}: {error}",
            temporary.display(),
            output.display()
        )
    })
}

fn display_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn print_remote_usage() {
    println!(
        "usage: kyuubiki-script-runner qualify-orchestra-installed-takeover-operational-remote [--host SSH_ALIAS] [--out path] [--postgres-image image] [--timeout-secs seconds]"
    );
}

fn print_check_usage() {
    println!(
        "usage: kyuubiki-script-runner check-orchestra-installed-takeover-operational-qualification [--self-test] [--verify-report path]"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_capture_requires_absolute_managed_paths() {
        assert!(HostOptions::parse(vec!["--managed-root".into(), "relative".into()]).is_err());
    }

    #[test]
    fn portable_values_reject_shell_injection() {
        assert!(!valid_image_reference("postgres:16;id"));
        assert!(!valid_toolchain_value("28.4;id"));
        assert!(valid_version("2.15.0"));
    }
}
