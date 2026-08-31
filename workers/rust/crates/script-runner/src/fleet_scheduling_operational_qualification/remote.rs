use super::report;
use crate::native_time::utc_timestamp_slug;
use crate::remote_host::{remote_shell_path, rsync_to, scp_from, shell_escape, ssh_status};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

type RunnerResult<T> = Result<T, String>;

const WEB_EXCLUDES: &[&str] = &[
    "_build/",
    "cover/",
    "deps/",
    ".elixir_ls/",
    ".mix/",
    "tmp/",
    "erl_crash.dump",
];
const RUST_EXCLUDES: &[&str] = &["target/", "tmp/"];

pub(crate) fn capture(
    root: &Path,
    host: &str,
    output: &Path,
    package_version: &str,
    timeout: Duration,
) -> RunnerResult<u8> {
    let slug = format!("fleet-scheduling-{}", utc_timestamp_slug());
    let run_root = format!("~/.kyuubiki/lab-runs/{slug}");
    prepare_remote(root, host, &run_root)?;
    let captured = capture_inner(root, host, output, package_version, timeout, &run_root);
    let cleaned = cleanup_remote(root, host, &run_root);
    match (captured, cleaned) {
        (Ok(status), Ok(())) => Ok(status),
        (Err(error), Ok(())) => Err(error),
        (Ok(status), Err(cleanup_error)) => Err(format!(
            "fleet scheduling capture exited with status {status}; {cleanup_error}"
        )),
        (Err(error), Err(cleanup_error)) => Err(format!("{error}; {cleanup_error}")),
    }
}

fn capture_inner(
    root: &Path,
    host: &str,
    output: &Path,
    package_version: &str,
    timeout: Duration,
    run_root: &str,
) -> RunnerResult<u8> {
    sync_sources(root, host, run_root)?;
    let toolchain = toolchain(root)?;
    let status = ssh_status(
        root,
        host,
        remote_command(run_root, package_version, timeout, &toolchain),
    )?;
    if status != 0 {
        return Ok(status);
    }
    let temporary = temporary_report_path(output);
    if let Some(parent) = temporary.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    let copied = scp_from(root, host, &format!("{run_root}/report.json"), &temporary)?;
    if copied != 0 {
        let _ = fs::remove_file(&temporary);
        return Ok(copied);
    }
    let qualification = match report::read_path(&temporary) {
        Ok(report) => report,
        Err(error) => {
            let _ = fs::remove_file(&temporary);
            return Err(error);
        }
    };
    if let Err(error) = report::validate(root, &qualification) {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    promote_report(&temporary, output)?;
    Ok(0)
}

fn prepare_remote(root: &Path, host: &str, run_root: &str) -> RunnerResult<()> {
    let run_root = remote_shell_path(run_root);
    let status = ssh_status(
        root,
        host,
        format!(
            "set -eu; umask 077; run_root={run_root}; mkdir -p \"$run_root/source/apps\" \"$run_root/source/workers\" \"$run_root/source/scripts\" \"$run_root/source/config/architecture\" \"$run_root/source/schemas\""
        ),
    )?;
    if status == 0 {
        Ok(())
    } else {
        Err(format!(
            "failed to prepare managed fleet scheduling run root: status {status}"
        ))
    }
}

fn sync_sources(root: &Path, host: &str, run_root: &str) -> RunnerResult<()> {
    let source = format!("{host}:{run_root}/source");
    let transfers = [
        rsync_to(
            root,
            WEB_EXCLUDES,
            &[root.join("apps/web/")],
            &format!("{source}/apps/web/"),
        )?,
        rsync_to(
            root,
            RUST_EXCLUDES,
            &[root.join("workers/rust/")],
            &format!("{source}/workers/rust/"),
        )?,
        rsync_to(
            root,
            &[],
            &[root.join("scripts/kyuubiki")],
            &format!("{source}/scripts/kyuubiki"),
        )?,
        rsync_to(
            root,
            &[],
            &[root.join(report::CONTRACT_PATH)],
            &format!("{source}/config/architecture/"),
        )?,
        rsync_to(
            root,
            &[],
            &[
                root.join(
                    "schemas/fleet-scheduling-operational-qualification-contract.schema.json",
                ),
                root.join("schemas/fleet-scheduling-operational-qualification-report.schema.json"),
            ],
            &format!("{source}/schemas/"),
        )?,
    ];
    if let Some(status) = transfers.into_iter().find(|status| *status != 0) {
        Err(format!(
            "fleet scheduling source synchronization failed with status {status}"
        ))
    } else {
        Ok(())
    }
}

fn remote_command(
    run_root: &str,
    package_version: &str,
    timeout: Duration,
    toolchain: &Toolchain,
) -> String {
    let run_root = remote_shell_path(run_root);
    let version = shell_escape(package_version);
    let timeout_seconds = timeout.as_secs();
    let otp = shell_fragment(&toolchain.otp);
    let elixir = shell_fragment(&toolchain.elixir);
    format!(
        "set -eu; umask 077; run_root={run_root}; source_root=\"$run_root/source\"; \
remote_elixir_installs_dir=${{REMOTE_ELIXIR_INSTALLS_DIR:-$HOME/.elixir-install/installs}}; \
export PATH=\"$remote_elixir_installs_dir/otp/{otp}/bin:$PATH\"; \
export PATH=\"$remote_elixir_installs_dir/elixir/{elixir}/bin:$PATH\"; \
remote_mix_home=\"$HOME/.kyuubiki/toolchains/mix/elixir-{elixir}-otp-{otp}\"; \
export MIX_HOME=\"$remote_mix_home\"; export HEX_HOME=\"$remote_mix_home/hex\"; \
mkdir -p \"$MIX_HOME\" \"$HEX_HOME\"; \
if ! mix hex.info >/dev/null 2>&1; then mix local.hex --force >/dev/null; fi; \
cd \"$source_root/apps/web\"; mix deps.get >/dev/null; mix compile >/dev/null; \
target_root=\"$HOME/.kyuubiki/cache/cargo-target/fleet-scheduling-operational\"; \
mkdir -p \"$target_root\"; cd \"$source_root/workers/rust\"; \
CARGO_TARGET_DIR=\"$target_root\" cargo build --locked --release -p kyuubiki-cli -p kyuubiki-script-runner; \
KYUUBIKI_REPO_ROOT=\"$source_root\" \"$target_root/release/kyuubiki-script-runner\" \
capture-fleet-scheduling-operational-host \
--agent-binary \"$target_root/release/kyuubiki-cli\" \
--work-root \"$run_root/qualification-work\" --out \"$run_root/report.json\" \
--package-version {version} --timeout-secs {timeout_seconds}"
    )
}

fn cleanup_remote(root: &Path, host: &str, run_root: &str) -> RunnerResult<()> {
    let run_root = remote_shell_path(run_root);
    let status = ssh_status(
        root,
        host,
        format!(
            "set -eu; run_root={run_root}; case \"$run_root\" in \
\"$HOME/.kyuubiki/lab-runs/\"*) rm -rf \"$run_root\" ;; \
*) echo 'refusing unmanaged fleet scheduling cleanup root' >&2; exit 2 ;; esac"
        ),
    )?;
    if status == 0 {
        Ok(())
    } else {
        Err(format!(
            "managed fleet scheduling run cleanup failed with status {status}"
        ))
    }
}

struct Toolchain {
    elixir: String,
    otp: String,
}

fn toolchain(root: &Path) -> RunnerResult<Toolchain> {
    let path = root.join("config/toolchains.json");
    let value: Value = serde_json::from_slice(
        &fs::read(&path).map_err(|error| format!("failed to read {}: {error}", path.display()))?,
    )
    .map_err(|error| format!("invalid {}: {error}", path.display()))?;
    Ok(Toolchain {
        elixir: required_toolchain(&value, "/elixir/lab_elixir")?,
        otp: required_toolchain(&value, "/elixir/lab_otp")?,
    })
}

fn required_toolchain(value: &Value, pointer: &str) -> RunnerResult<String> {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .filter(|entry| !entry.is_empty() && entry.bytes().all(toolchain_byte))
        .map(str::to_string)
        .ok_or_else(|| format!("config/toolchains.json misses portable {pointer}"))
}

fn toolchain_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-')
}

fn shell_fragment(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('$', "\\$")
        .replace('`', "\\`")
}

fn temporary_report_path(output: &Path) -> PathBuf {
    let name = output
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("fleet-scheduling-operational.json");
    output.with_file_name(format!(".{name}.{}.tmp", std::process::id()))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_sync_excludes_generated_trees() {
        assert!(WEB_EXCLUDES.contains(&"_build/"));
        assert!(RUST_EXCLUDES.contains(&"target/"));
    }

    #[test]
    fn toolchain_values_reject_shell_metacharacters() {
        assert!("1.20.1-otp-28".bytes().all(toolchain_byte));
        assert!(!"1.20;rm".bytes().all(toolchain_byte));
    }
}
