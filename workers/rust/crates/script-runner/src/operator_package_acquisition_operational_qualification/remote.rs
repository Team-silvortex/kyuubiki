use super::distribution::ENTRYPOINT_NAME;
use crate::remote_host::{
    remote_shell_path, rsync_to, scp_from, shell_escape, ssh_status, ssh_success_quiet,
};
use std::net::Ipv4Addr;
use std::path::Path;

type RunnerResult<T> = Result<T, String>;

pub(super) const AGENT_ID: &str = "operator-acquisition-agent";
pub(super) const CLUSTER_ID: &str = "operator-acquisition-cluster";
pub(super) const HOST_ID: &str = "operator-acquisition-host";
const RUST_EXCLUDES: &[&str] = &["target/", "tmp/", ".DS_Store"];

pub(super) fn prepare_source(root: &Path, host: &str, run_root: &str) -> RunnerResult<()> {
    let remote_root = remote_shell_path(run_root);
    require_zero(
        "prepare remote qualification root",
        ssh_status(
            root,
            host,
            format!(
                "set -eu; umask 077; run_root={remote_root}; mkdir -p \"$run_root/source/workers\" \"$run_root/source/scripts\""
            ),
        )?,
    )?;
    require_zero(
        "synchronize Rust source",
        rsync_to(
            root,
            RUST_EXCLUDES,
            &[root.join("workers/rust/")],
            &format!("{host}:{run_root}/source/workers/rust/"),
        )?,
    )
}

pub(super) fn build_remote(root: &Path, host: &str, run_root: &str) -> RunnerResult<()> {
    require_zero(
        "build remote Agent, qualification runner, and operator cdylib",
        ssh_status(root, host, build_command(run_root))?,
    )
}

pub(super) fn retrieve_operator(
    root: &Path,
    host: &str,
    run_root: &str,
    destination: &Path,
) -> RunnerResult<()> {
    require_zero(
        "retrieve Linux operator entrypoint",
        scp_from(
            root,
            host,
            &format!("{run_root}/operator-target/release/{ENTRYPOINT_NAME}"),
            destination,
        )?,
    )
}

pub(super) fn remove_remote_operator_build(
    root: &Path,
    host: &str,
    run_root: &str,
) -> RunnerResult<bool> {
    let remote_root = remote_shell_path(run_root);
    require_zero(
        "remove remote operator build artifact",
        ssh_status(
            root,
            host,
            format!(
                "set -eu; run_root={remote_root}; rm -rf \"$run_root/operator-target\"; test ! -e \"$run_root/operator-target\"; test -z \"$(find \"$run_root\" -type f -name {ENTRYPOINT_NAME} -print -quit)\""
            ),
        )?,
    )?;
    ssh_success_quiet(
        root,
        host,
        format!(
            "set -eu; run_root={remote_root}; test ! -e \"$run_root/operator-target\"; test -z \"$(find \"$run_root\" -type f -name {ENTRYPOINT_NAME} -print -quit)\""
        ),
    )
}

pub(super) fn prepare_installed_agent(
    root: &Path,
    host: &str,
    run_root: &str,
    package_version: &str,
) -> RunnerResult<()> {
    let remote_root = remote_shell_path(run_root);
    let version = shell_escape(package_version);
    require_zero(
        "prepare Installer-managed remote Agent",
        ssh_status(
            root,
            host,
            format!(
                "set -eu; umask 077; run_root={remote_root}; source_root=\"$run_root/source\"; KYUUBIKI_REPO_ROOT=\"$source_root\" \"$run_root/kyuubiki-script-runner\" prepare-operator-package-acquisition-host --agent-binary \"$run_root/kyuubiki-agent\" --work-root \"$run_root/qualification-work\" --out \"$run_root/installation.json\" --package-version {version}"
            ),
        )?,
    )
}

pub(super) fn retrieve_installation(
    root: &Path,
    host: &str,
    run_root: &str,
    destination: &Path,
) -> RunnerResult<()> {
    require_zero(
        "retrieve Installer setup evidence",
        scp_from(
            root,
            host,
            &format!("{run_root}/installation.json"),
            destination,
        )?,
    )
}

pub(super) fn transfer_secret(
    root: &Path,
    host: &str,
    run_root: &str,
    local_path: &Path,
) -> RunnerResult<()> {
    require_zero(
        "transfer ephemeral cluster token",
        rsync_to(
            root,
            &[],
            &[local_path.to_path_buf()],
            &format!("{host}:{run_root}/control.env"),
        )?,
    )
}

pub(super) struct AgentStart<'a> {
    pub(super) root: &'a Path,
    pub(super) host: &'a str,
    pub(super) run_root: &'a str,
    pub(super) package_version: &'a str,
    pub(super) local_ip: Ipv4Addr,
    pub(super) remote_ip: Ipv4Addr,
    pub(super) orchestra_port: u16,
    pub(super) agent_port: u16,
}

pub(super) fn start_agent(options: AgentStart<'_>) -> RunnerResult<bool> {
    let command = start_command(
        options.run_root,
        options.package_version,
        options.local_ip,
        options.remote_ip,
        options.orchestra_port,
        options.agent_port,
    );
    let status = ssh_status(options.root, options.host, command)?;
    Ok(status == 0)
}

pub(super) fn agent_alive(
    root: &Path,
    host: &str,
    run_root: &str,
    package_version: &str,
) -> RunnerResult<bool> {
    ssh_success_quiet(root, host, alive_command(run_root, package_version))
}

pub(super) fn stop_agent(
    root: &Path,
    host: &str,
    run_root: &str,
    package_version: &str,
) -> RunnerResult<()> {
    require_zero(
        "stop remote installed Agent",
        ssh_status(root, host, reset_command(run_root, package_version))?,
    )
}

pub(super) fn secret_removed(root: &Path, host: &str, run_root: &str) -> RunnerResult<bool> {
    let remote_root = remote_shell_path(run_root);
    ssh_success_quiet(
        root,
        host,
        format!("set -eu; run_root={remote_root}; test ! -e \"$run_root/control.env\""),
    )
}

pub(super) fn remove_run_root(root: &Path, host: &str, run_root: &str) -> RunnerResult<bool> {
    let remote_root = remote_shell_path(run_root);
    require_zero(
        "remove managed remote qualification root",
        ssh_status(
            root,
            host,
            format!(
                "set -eu; run_root={remote_root}; case \"$run_root\" in \"$HOME/.kyuubiki/lab-runs/\"*) rm -rf \"$run_root\" ;; *) exit 2 ;; esac"
            ),
        )?,
    )?;
    ssh_success_quiet(
        root,
        host,
        format!("set -eu; run_root={remote_root}; test ! -e \"$run_root\""),
    )
}

fn build_command(run_root: &str) -> String {
    let remote_root = remote_shell_path(run_root);
    format!(
        "set -eu; umask 077; run_root={remote_root}; source_root=\"$run_root/source/workers/rust\"; target_root=\"$HOME/.kyuubiki/cache/cargo-target/operator-package-acquisition-operational\"; find \"$source_root\" -type f -exec touch {{}} +; mkdir -p \"$target_root\"; cd \"$source_root\"; CARGO_TARGET_DIR=\"$target_root\" cargo build --locked --release -p kyuubiki-cli -p kyuubiki-script-runner; install -m 700 \"$target_root/release/kyuubiki-cli\" \"$run_root/kyuubiki-agent\"; install -m 700 \"$target_root/release/kyuubiki-script-runner\" \"$run_root/kyuubiki-script-runner\"; rm -rf \"$run_root/operator-target\"; CARGO_TARGET_DIR=\"$run_root/operator-target\" cargo build --locked --release --manifest-path templates/operator-crate-template/Cargo.toml; test -f \"$run_root/operator-target/release/{ENTRYPOINT_NAME}\""
    )
}

fn installed_agent_path(run_root: &str, package_version: &str) -> String {
    let remote_root = remote_shell_path(run_root);
    let version = shell_escape(package_version);
    format!(
        "run_root={remote_root}; version={version}; agent=\"$run_root/qualification-work/agent-store/versions/$version/bin/kyuubiki-agent\""
    )
}

fn start_command(
    run_root: &str,
    package_version: &str,
    local_ip: Ipv4Addr,
    remote_ip: Ipv4Addr,
    orchestra_port: u16,
    agent_port: u16,
) -> String {
    let prefix = installed_agent_path(run_root, package_version);
    let orchestra_url = shell_escape(&format!("http://{local_ip}:{orchestra_port}"));
    let advertise_host = shell_escape(&remote_ip.to_string());
    format!(
        "set -eu; umask 077; {prefix}; test -x \"$agent\"; test -f \"$run_root/control.env\"; set -a; . \"$run_root/control.env\"; set +a; rm -f \"$run_root/agent.pid\"; nohup \"$agent\" agent --host 0.0.0.0 --port {agent_port} --agent-id {AGENT_ID} --advertise-host {advertise_host} --orchestrator-url {orchestra_url} --cluster-id {CLUSTER_ID} --register-interval-ms 250 --operator-package-host-id {HOST_ID} --operator-packages-root \"$run_root/qualification-work/operator-store/packages\" --operator-activated-package-count 0 >\"$run_root/agent.log\" 2>&1 </dev/null & pid=$!; printf '%s\\n' \"$pid\" >\"$run_root/agent.pid\"; rm -f \"$run_root/control.env\"; sleep 1; kill -0 \"$pid\"; test \"$(readlink -f \"/proc/$pid/exe\")\" = \"$(readlink -f \"$agent\")\""
    )
}

fn alive_command(run_root: &str, package_version: &str) -> String {
    let prefix = installed_agent_path(run_root, package_version);
    format!(
        "set -eu; {prefix}; pid=$(cat \"$run_root/agent.pid\"); case \"$pid\" in ''|*[!0-9]*) exit 2 ;; esac; kill -0 \"$pid\"; test \"$(readlink -f \"/proc/$pid/exe\")\" = \"$(readlink -f \"$agent\")\""
    )
}

fn reset_command(run_root: &str, package_version: &str) -> String {
    let prefix = installed_agent_path(run_root, package_version);
    format!(
        "set -eu; {prefix}; if test -f \"$run_root/agent.pid\"; then pid=$(cat \"$run_root/agent.pid\"); case \"$pid\" in ''|*[!0-9]*) exit 2 ;; esac; if kill -0 \"$pid\" 2>/dev/null; then actual=$(readlink -f \"/proc/$pid/exe\" || true); expected=$(readlink -f \"$agent\"); test \"$actual\" = \"$expected\"; kill \"$pid\"; count=0; while kill -0 \"$pid\" 2>/dev/null && test \"$count\" -lt 50; do sleep 0.1; count=$((count + 1)); done; if kill -0 \"$pid\" 2>/dev/null; then kill -9 \"$pid\"; fi; fi; fi; rm -f \"$run_root/agent.pid\" \"$run_root/control.env\""
    )
}

fn require_zero(label: &str, status: u8) -> RunnerResult<()> {
    if status == 0 {
        Ok(())
    } else {
        Err(format!("{label} failed with status {status}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleanup_and_liveness_are_scoped_to_installed_agent() {
        let command = reset_command("~/.kyuubiki/lab-runs/test", "2.19.0");
        assert!(command.contains("qualification-work/agent-store/versions"));
        assert!(command.contains("readlink -f"));
    }

    #[test]
    fn generated_remote_source_excludes_build_outputs() {
        assert!(RUST_EXCLUDES.contains(&"target/"));
        assert!(RUST_EXCLUDES.contains(&"tmp/"));
        let command = build_command("~/.kyuubiki/lab-runs/test");
        assert!(command.contains("-type f -exec touch {} +"));
        assert!(command.contains("cargo build --locked --release"));
        assert!(command.contains("templates/operator-crate-template/Cargo.toml"));
    }
}
