use crate::operational_agent_support::available_local_port;
use crate::remote_host::{
    remote_shell_path, rsync_to, shell_escape, ssh_output, ssh_status, ssh_success_quiet,
};
use std::fs::File;
use std::net::{Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

type RunnerResult<T> = Result<T, String>;

const REMOTE_AGENT_ID: &str = "distributed-recovery-remote-primary";
const LOCAL_AGENT_ID: &str = "distributed-recovery-local-fallback";

pub(crate) struct ManagedAgents {
    root: PathBuf,
    host: String,
    run_root: String,
    work_root: PathBuf,
    remote_ip: Ipv4Addr,
    remote_port: Option<u16>,
    local_port: u16,
    local_child: Option<Child>,
    remote_prepared: bool,
    remote_started: bool,
    remote_generation: u8,
}

impl ManagedAgents {
    pub(crate) fn new(
        root: &Path,
        host: &str,
        run_root: String,
        work_root: PathBuf,
        remote_ip: Ipv4Addr,
    ) -> RunnerResult<Self> {
        Ok(Self {
            root: root.to_path_buf(),
            host: host.to_string(),
            run_root,
            work_root,
            remote_ip,
            remote_port: None,
            local_port: available_local_port()?,
            local_child: None,
            remote_prepared: false,
            remote_started: false,
            remote_generation: 0,
        })
    }

    pub(crate) fn prepare(&mut self, port_seed: u16) -> RunnerResult<()> {
        self.prepare_remote()?;
        self.build_local()?;
        self.start_local()?;
        self.start_remote_with_port_selection(port_seed)?;
        Ok(())
    }

    pub(crate) fn prepare_remote_only(&mut self, port_seed: u16) -> RunnerResult<()> {
        self.prepare_remote()?;
        self.start_remote_with_port_selection(port_seed)
    }

    pub(crate) fn local_address(&self) -> SocketAddr {
        SocketAddr::from(([127, 0, 0, 1], self.local_port))
    }

    pub(crate) fn remote_address(&self) -> RunnerResult<SocketAddr> {
        self.remote_port
            .map(|port| SocketAddr::from((self.remote_ip, port)))
            .ok_or_else(|| "remote Agent port is unavailable".to_string())
    }

    pub(crate) fn local_port(&self) -> u16 {
        self.local_port
    }

    pub(crate) fn remote_port(&self) -> RunnerResult<u16> {
        self.remote_port
            .ok_or_else(|| "remote Agent port is unavailable".to_string())
    }

    pub(crate) fn local_alive(&mut self) -> RunnerResult<bool> {
        match self.local_child.as_mut() {
            Some(child) => child
                .try_wait()
                .map(|status| status.is_none())
                .map_err(|error| format!("failed to inspect local fallback Agent: {error}")),
            None => Ok(false),
        }
    }

    pub(crate) fn remote_alive(&self) -> RunnerResult<bool> {
        if !self.remote_started {
            return Ok(false);
        }
        ssh_success_quiet(&self.root, &self.host, remote_alive_command(&self.run_root))
    }

    pub(crate) fn terminate_remote_inflight(&mut self) -> RunnerResult<()> {
        if !self.remote_started {
            return Err("remote primary Agent is not running".to_string());
        }
        let status = ssh_status(
            &self.root,
            &self.host,
            remote_stop_command(&self.run_root, true),
        )?;
        if status != 0 {
            return Err(format!(
                "failed to terminate in-flight remote Agent: status {status}"
            ));
        }
        self.remote_started = false;
        Ok(())
    }

    pub(crate) fn hold_remote_execution(&self, job_id: &str) -> RunnerResult<()> {
        let status = ssh_status(
            &self.root,
            &self.host,
            remote_hold_command(&self.run_root, job_id),
        )?;
        if status == 0 {
            Ok(())
        } else {
            Err(format!(
                "failed to arm remote execution hold: status {status}"
            ))
        }
    }

    pub(crate) fn release_remote_execution(&self) -> RunnerResult<()> {
        let run_root = remote_shell_path(&self.run_root);
        let status = ssh_status(
            &self.root,
            &self.host,
            format!("set -eu; run_root={run_root}; rm -f \"$run_root/execution.hold\""),
        )?;
        if status == 0 {
            Ok(())
        } else {
            Err(format!(
                "failed to release remote execution hold: status {status}"
            ))
        }
    }

    pub(crate) fn pause_remote(&self) -> RunnerResult<()> {
        self.signal_remote("STOP", true)
    }

    pub(crate) fn resume_remote(&self) -> RunnerResult<()> {
        self.signal_remote("CONT", false)
    }

    pub(crate) fn remote_log_tail(&self) -> RunnerResult<String> {
        if !self.remote_prepared {
            return Ok("remote Agent was not prepared".to_string());
        }
        let run_root = remote_shell_path(&self.run_root);
        ssh_output(
            &self.root,
            &self.host,
            format!(
                "set -eu; run_root={run_root}; for file in \"$run_root\"/agent-*.log; do test -f \"$file\" || continue; printf '== %s ==\\n' \"$(basename \"$file\")\"; tail -n 40 \"$file\"; done"
            ),
        )
    }

    pub(crate) fn restart_remote(&mut self) -> RunnerResult<()> {
        let port = self.remote_port()?;
        self.start_remote(port)
    }

    pub(crate) fn stop_remote(&mut self) -> RunnerResult<()> {
        if !self.remote_prepared {
            return Ok(());
        }
        let status = ssh_status(
            &self.root,
            &self.host,
            remote_stop_command(&self.run_root, false),
        )?;
        if status != 0 {
            return Err(format!("failed to stop remote Agent: status {status}"));
        }
        self.remote_started = false;
        Ok(())
    }

    pub(crate) fn stop_local(&mut self) -> RunnerResult<()> {
        if let Some(mut child) = self.local_child.take() {
            if child
                .try_wait()
                .map_err(|error| format!("failed to inspect local Agent: {error}"))?
                .is_none()
            {
                child
                    .kill()
                    .map_err(|error| format!("failed to stop local Agent: {error}"))?;
            }
            child
                .wait()
                .map_err(|error| format!("failed to reap local Agent: {error}"))?;
        }
        Ok(())
    }

    pub(crate) fn remove_remote_root(&mut self) -> RunnerResult<bool> {
        if !self.remote_prepared {
            return Ok(true);
        }
        let run_root = remote_shell_path(&self.run_root);
        let status = ssh_status(
            &self.root,
            &self.host,
            format!(
                "set -eu; run_root={run_root}; case \"$run_root\" in \"$HOME/.kyuubiki/lab-runs/\"*) rm -rf \"$run_root\" ;; *) exit 2 ;; esac"
            ),
        )?;
        if status != 0 {
            return Err(format!("failed to remove remote run root: status {status}"));
        }
        let removed = ssh_success_quiet(
            &self.root,
            &self.host,
            format!("set -eu; run_root={run_root}; test ! -e \"$run_root\""),
        )?;
        if removed {
            self.remote_prepared = false;
        }
        Ok(removed)
    }

    fn prepare_remote(&mut self) -> RunnerResult<()> {
        let run_root = remote_shell_path(&self.run_root);
        let status = ssh_status(
            &self.root,
            &self.host,
            format!("set -eu; umask 077; mkdir -p {run_root}/workers/rust"),
        )?;
        if status != 0 {
            return Err(format!(
                "failed to prepare remote run root: status {status}"
            ));
        }
        self.remote_prepared = true;
        let sync = rsync_to(
            &self.root,
            &["target/", ".DS_Store"],
            &[self.root.join("workers/rust/")],
            &format!("{}:{}/workers/rust/", self.host, self.run_root),
        )?;
        if sync != 0 {
            return Err(format!("failed to synchronize Agent source: status {sync}"));
        }
        let build = ssh_status(&self.root, &self.host, remote_build_command(&self.run_root))?;
        if build != 0 {
            return Err(format!(
                "failed to build remote Release Agent: status {build}"
            ));
        }
        Ok(())
    }

    fn build_local(&self) -> RunnerResult<()> {
        let status = Command::new("cargo")
            .args(["build", "--release", "-p", "kyuubiki-cli"])
            .current_dir(self.root.join("workers/rust"))
            .status()
            .map_err(|error| format!("failed to build local Release Agent: {error}"))?;
        if status.success() {
            Ok(())
        } else {
            Err(format!("local Release Agent build failed: {status}"))
        }
    }

    fn start_local(&mut self) -> RunnerResult<()> {
        let log_path = self.work_root.join("local-fallback-agent.log");
        let stdout = File::create(&log_path)
            .map_err(|error| format!("failed to create local Agent log: {error}"))?;
        let stderr = stdout
            .try_clone()
            .map_err(|error| format!("failed to clone local Agent log: {error}"))?;
        let child = Command::new(self.root.join("workers/rust/target/release/kyuubiki-cli"))
            .args([
                "agent",
                "--host",
                "127.0.0.1",
                "--port",
                &self.local_port.to_string(),
                "--agent-id",
                LOCAL_AGENT_ID,
                "--watchdog-scan-interval-ms",
                "250",
                "--watchdog-stale-execution-ms",
                "10000",
            ])
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr))
            .spawn()
            .map_err(|error| format!("failed to start local fallback Agent: {error}"))?;
        self.local_child = Some(child);
        Ok(())
    }

    fn start_remote_with_port_selection(&mut self, seed: u16) -> RunnerResult<()> {
        let first_port = 46_000 + seed % 12_000;
        let mut last_error = "no remote Agent start attempt completed".to_string();
        for offset in 0..8_u16 {
            let port = first_port + offset;
            match self.start_remote(port) {
                Ok(()) => {
                    self.remote_port = Some(port);
                    return Ok(());
                }
                Err(error) => last_error = error,
            }
            self.stop_remote().map_err(|cleanup| {
                format!("{last_error}; failed to clean partial Agent start: {cleanup}")
            })?;
        }
        Err(format!(
            "could not allocate an isolated remote Agent port: {last_error}"
        ))
    }

    fn start_remote(&mut self, port: u16) -> RunnerResult<()> {
        self.remote_generation = self.remote_generation.saturating_add(1);
        let status = ssh_status(
            &self.root,
            &self.host,
            remote_start_command(&self.run_root, self.remote_ip, port, self.remote_generation),
        )?;
        if status != 0 {
            self.remote_started = false;
            return Err(format!("failed to start remote Agent: status {status}"));
        }
        self.remote_started = true;
        Ok(())
    }

    fn signal_remote(&self, signal: &str, expect_stopped: bool) -> RunnerResult<()> {
        if !self.remote_started {
            return Err("remote primary Agent is not running".to_string());
        }
        let status = ssh_status(
            &self.root,
            &self.host,
            remote_signal_command(&self.run_root, signal, expect_stopped),
        )?;
        if status == 0 {
            Ok(())
        } else {
            Err(format!(
                "failed to send SIG{signal} to remote Agent: status {status}"
            ))
        }
    }
}

fn remote_build_command(run_root: &str) -> String {
    let run_root = remote_shell_path(run_root);
    format!(
        "set -eu; umask 077; run_root={run_root}; source_root=\"$run_root/workers/rust\"; target_root=\"$HOME/.kyuubiki/cache/cargo-target/distributed-task-recovery-operational\"; mkdir -p \"$target_root\"; cd \"$source_root\"; CARGO_TARGET_DIR=\"$target_root\" cargo build --release -p kyuubiki-cli; install -m 700 \"$target_root/release/kyuubiki-cli\" \"$run_root/kyuubiki-agent\""
    )
}

fn remote_start_command(run_root: &str, remote_ip: Ipv4Addr, port: u16, generation: u8) -> String {
    let run_root = remote_shell_path(run_root);
    let advertise_host = shell_escape(&remote_ip.to_string());
    format!(
        "set -eu; umask 077; run_root={run_root}; run_root=$(readlink -f \"$run_root\"); test -x \"$run_root/kyuubiki-agent\"; rm -f \"$run_root/agent.pid\"; KYUUBIKI_AGENT_FAULT_INJECTION_HOLD_FILE=\"$run_root/execution.hold\" nohup \"$run_root/kyuubiki-agent\" agent --host 0.0.0.0 --port {port} --agent-id {REMOTE_AGENT_ID} --advertise-host {advertise_host} --watchdog-scan-interval-ms 250 --watchdog-stale-execution-ms 10000 >\"$run_root/agent-{generation}.log\" 2>&1 </dev/null & pid=$!; printf '%s\\n' \"$pid\" >\"$run_root/agent.pid\"; sleep 1; kill -0 \"$pid\"; test \"$(readlink -f \"/proc/$pid/exe\")\" = \"$run_root/kyuubiki-agent\""
    )
}

fn remote_hold_command(run_root: &str, job_id: &str) -> String {
    let run_root = remote_shell_path(run_root);
    let job_id = shell_escape(job_id);
    format!(
        "set -eu; umask 077; run_root={run_root}; printf '%s\\n' {job_id} >\"$run_root/execution.hold\""
    )
}

fn remote_alive_command(run_root: &str) -> String {
    let run_root = remote_shell_path(run_root);
    format!(
        "set -eu; run_root={run_root}; run_root=$(readlink -f \"$run_root\"); pid=$(cat \"$run_root/agent.pid\"); case \"$pid\" in ''|*[!0-9]*) exit 2 ;; esac; kill -0 \"$pid\"; test \"$(readlink -f \"/proc/$pid/exe\")\" = \"$run_root/kyuubiki-agent\""
    )
}

fn remote_stop_command(run_root: &str, require_running: bool) -> String {
    let run_root = remote_shell_path(run_root);
    let missing_status = if require_running { "exit 3" } else { "exit 0" };
    format!(
        "set -eu; run_root={run_root}; run_root=$(readlink -f \"$run_root\"); if test ! -f \"$run_root/agent.pid\"; then {missing_status}; fi; pid=$(cat \"$run_root/agent.pid\"); case \"$pid\" in ''|*[!0-9]*) exit 2 ;; esac; if ! kill -0 \"$pid\" 2>/dev/null; then rm -f \"$run_root/agent.pid\"; {missing_status}; fi; actual=$(readlink -f \"/proc/$pid/exe\" || true); test \"$actual\" = \"$run_root/kyuubiki-agent\"; kill \"$pid\"; count=0; while kill -0 \"$pid\" 2>/dev/null && test \"$count\" -lt 50; do sleep 0.1; count=$((count + 1)); done; if kill -0 \"$pid\" 2>/dev/null; then kill -9 \"$pid\"; fi; rm -f \"$run_root/agent.pid\""
    )
}

fn remote_signal_command(run_root: &str, signal: &str, expect_stopped: bool) -> String {
    let run_root = remote_shell_path(run_root);
    let expected = if expect_stopped {
        "test \"$state\" = T -o \"$state\" = t"
    } else {
        "test \"$state\" != T -a \"$state\" != t"
    };
    format!(
        "set -eu; run_root={run_root}; run_root=$(readlink -f \"$run_root\"); pid=$(cat \"$run_root/agent.pid\"); case \"$pid\" in ''|*[!0-9]*) exit 2 ;; esac; test \"$(readlink -f \"/proc/$pid/exe\")\" = \"$run_root/kyuubiki-agent\"; kill -{signal} \"$pid\"; count=0; while test \"$count\" -lt 50; do state=$(awk '/^State:/ {{print $2}}' \"/proc/$pid/status\"); if {expected}; then exit 0; fi; sleep 0.02; count=$((count + 1)); done; exit 3"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleanup_is_scoped_to_managed_remote_root() {
        let command = remote_stop_command("~/.kyuubiki/lab-runs/recovery-test", false);
        assert!(command.contains("readlink -f"));
        assert!(command.contains("run_root=$(readlink -f"));
        assert!(command.contains("$run_root/kyuubiki-agent"));
        assert!(!command.contains("pkill"));
        let hold = remote_hold_command(
            "~/.kyuubiki/lab-runs/recovery-test",
            "distributed-recovery-job",
        );
        assert!(hold.contains("execution.hold"));
        assert!(!hold.contains("pkill"));
        let pause = remote_signal_command("~/.kyuubiki/lab-runs/recovery-test", "STOP", true);
        assert!(pause.contains("readlink -f"));
        assert!(pause.contains("kill -STOP"));
        assert!(!pause.contains("pkill"));
    }
}
