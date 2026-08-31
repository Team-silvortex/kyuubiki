use crate::operational_agent_support::{
    available_local_port, query_agent_descriptor_value, wait_endpoint_closed,
};
use kyuubiki_installer::{
    AgentUpdateActivationRecord, AgentUpdatePackageManifest, Platform, active_agent_binary_in,
    agent_update_status_in, install_agent_update_package_into, prepare_agent_update_package,
};
use std::fs::{self, File};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

type RunnerResult<T> = Result<T, String>;

pub(crate) const HIGH_AGENT_ID: &str = "fleet-high-capacity";
pub(crate) const LOW_AGENT_ID: &str = "fleet-low-capacity";

#[derive(Debug)]
pub(crate) struct InstallationCapture {
    pub(crate) package: AgentUpdatePackageManifest,
    pub(crate) high_activation: AgentUpdateActivationRecord,
    pub(crate) low_activation: AgentUpdateActivationRecord,
    pub(crate) high_active_version: String,
    pub(crate) low_active_version: String,
}

#[derive(Debug)]
pub(crate) struct CleanupCapture {
    pub(crate) high_agent_stopped: bool,
    pub(crate) low_agent_stopped: bool,
    pub(crate) high_port_closed: bool,
    pub(crate) low_port_closed: bool,
    pub(crate) managed_install_root_removed: bool,
}

pub(crate) struct InstalledFleet {
    work_root: PathBuf,
    high: ManagedAgent,
    low: ManagedAgent,
    installation: InstallationCapture,
    high_initial_pid: u32,
    high_restarted_pid: Option<u32>,
    cleaned: bool,
}

impl InstalledFleet {
    pub(crate) fn prepare(
        agent_binary: &Path,
        work_root: &Path,
        package_version: &str,
        timeout: Duration,
    ) -> RunnerResult<Self> {
        if Platform::current() != Platform::Linux {
            return Err("fleet scheduling host capture requires Linux".to_string());
        }
        prepare_empty_root(work_root)?;
        let package_root = work_root.join("package");
        let high_store = work_root.join("stores/high");
        let low_store = work_root.join("stores/low");
        let logs = work_root.join("logs");
        fs::create_dir_all(&logs)
            .map_err(|error| format!("failed to create fleet logs: {error}"))?;

        let package = prepare_agent_update_package(
            agent_binary,
            &package_root,
            package_version,
            Platform::Linux,
        )?;
        let high_activation =
            install_agent_update_package_into(&package_root, &high_store, Platform::Linux)?;
        let low_activation =
            install_agent_update_package_into(&package_root, &low_store, Platform::Linux)?;
        let high_status = agent_update_status_in(&high_store)?;
        let low_status = agent_update_status_in(&low_store)?;
        let high_binary = active_agent_binary_in(&high_store, Platform::Linux)?;
        let low_binary = active_agent_binary_in(&low_store, Platform::Linux)?;

        let mut high = ManagedAgent::new(
            HIGH_AGENT_ID,
            high_binary,
            logs.join("high-agent.log"),
            available_local_port()?,
        );
        let mut low = ManagedAgent::new(
            LOW_AGENT_ID,
            low_binary,
            logs.join("low-agent.log"),
            available_distinct_port(high.port)?,
        );
        high.start(timeout)?;
        if let Err(error) = low.start(timeout) {
            let _ = high.stop();
            return Err(error);
        }
        let high_initial_pid = high.pid()?;

        Ok(Self {
            work_root: work_root.to_path_buf(),
            high,
            low,
            installation: InstallationCapture {
                package,
                high_activation,
                low_activation,
                high_active_version: high_status.active_version.unwrap_or_default(),
                low_active_version: low_status.active_version.unwrap_or_default(),
            },
            high_initial_pid,
            high_restarted_pid: None,
            cleaned: false,
        })
    }

    pub(crate) fn installation(&self) -> &InstallationCapture {
        &self.installation
    }

    pub(crate) fn high_port(&self) -> u16 {
        self.high.port
    }

    pub(crate) fn low_port(&self) -> u16 {
        self.low.port
    }

    pub(crate) fn stop_high_for_fault(&mut self) -> RunnerResult<()> {
        self.high.stop()?;
        wait_endpoint_closed(self.high.address(), Duration::from_secs(5))
    }

    pub(crate) fn restart_high(&mut self, timeout: Duration) -> RunnerResult<()> {
        self.high.start(timeout)?;
        self.high_restarted_pid = Some(self.high.pid()?);
        Ok(())
    }

    pub(crate) fn high_process_changed(&self) -> bool {
        self.high_restarted_pid
            .is_some_and(|pid| pid != self.high_initial_pid)
    }

    pub(crate) fn both_ready(&mut self) -> RunnerResult<bool> {
        Ok(self.high.alive()? && self.low.alive()?)
    }

    pub(crate) fn cleanup(&mut self) -> RunnerResult<CleanupCapture> {
        if self.cleaned {
            return Err("fleet scheduling installation was already cleaned".to_string());
        }
        let mut errors = Vec::new();
        let high_agent_stopped = stop_with_error(&mut self.high, &mut errors);
        let low_agent_stopped = stop_with_error(&mut self.low, &mut errors);
        let high_port_closed =
            wait_endpoint_closed(self.high.address(), Duration::from_secs(5)).is_ok();
        let low_port_closed =
            wait_endpoint_closed(self.low.address(), Duration::from_secs(5)).is_ok();
        if !high_port_closed {
            errors.push("high-capacity Agent port remained open".to_string());
        }
        if !low_port_closed {
            errors.push("low-capacity Agent port remained open".to_string());
        }
        let managed_install_root_removed = match fs::remove_dir_all(&self.work_root) {
            Ok(()) => !self.work_root.exists(),
            Err(error) => {
                errors.push(format!(
                    "failed to remove managed fleet install root: {error}"
                ));
                false
            }
        };
        if !errors.is_empty() {
            return Err(errors.join("; "));
        }
        self.cleaned = true;
        Ok(CleanupCapture {
            high_agent_stopped,
            low_agent_stopped,
            high_port_closed,
            low_port_closed,
            managed_install_root_removed,
        })
    }
}

impl Drop for InstalledFleet {
    fn drop(&mut self) {
        if !self.cleaned {
            let _ = self.cleanup();
        }
    }
}

struct ManagedAgent {
    id: &'static str,
    binary: PathBuf,
    log_path: PathBuf,
    port: u16,
    child: Option<Child>,
}

impl ManagedAgent {
    fn new(id: &'static str, binary: PathBuf, log_path: PathBuf, port: u16) -> Self {
        Self {
            id,
            binary,
            log_path,
            port,
            child: None,
        }
    }

    fn address(&self) -> SocketAddr {
        SocketAddr::from(([127, 0, 0, 1], self.port))
    }

    fn start(&mut self, timeout: Duration) -> RunnerResult<()> {
        if self.child.is_some() {
            return Err(format!("{} is already running", self.id));
        }
        let stdout = File::create(&self.log_path)
            .map_err(|error| format!("failed to create {} log: {error}", self.id))?;
        let stderr = stdout
            .try_clone()
            .map_err(|error| format!("failed to clone {} log: {error}", self.id))?;
        let child = Command::new(&self.binary)
            .args([
                "agent",
                "--host",
                "127.0.0.1",
                "--port",
                &self.port.to_string(),
                "--agent-id",
                self.id,
                "--watchdog-scan-interval-ms",
                "250",
                "--watchdog-stale-execution-ms",
                "10000",
            ])
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr))
            .spawn()
            .map_err(|error| format!("failed to start installed {}: {error}", self.id))?;
        self.child = Some(child);
        self.wait_ready(timeout)
    }

    fn wait_ready(&mut self, timeout: Duration) -> RunnerResult<()> {
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            if query_agent_descriptor_value(self.address())
                .ok()
                .and_then(|value| {
                    value
                        .get("program")
                        .and_then(|value| value.as_str())
                        .map(str::to_string)
                })
                .as_deref()
                == Some("kyuubiki-rust-agent")
            {
                return Ok(());
            }
            if let Some(status) = self
                .child
                .as_mut()
                .ok_or_else(|| format!("{} process is missing", self.id))?
                .try_wait()
                .map_err(|error| format!("failed to inspect {}: {error}", self.id))?
            {
                return Err(format!(
                    "installed {} exited before readiness: {status}; {}",
                    self.id,
                    log_excerpt(&self.log_path)
                ));
            }
            thread::sleep(Duration::from_millis(50));
        }
        Err(format!(
            "installed {} readiness timed out; {}",
            self.id,
            log_excerpt(&self.log_path)
        ))
    }

    fn alive(&mut self) -> RunnerResult<bool> {
        match self.child.as_mut() {
            Some(child) => child
                .try_wait()
                .map(|status| status.is_none())
                .map_err(|error| format!("failed to inspect {}: {error}", self.id)),
            None => Ok(false),
        }
    }

    fn pid(&self) -> RunnerResult<u32> {
        self.child
            .as_ref()
            .map(Child::id)
            .ok_or_else(|| format!("{} process is not running", self.id))
    }

    fn stop(&mut self) -> RunnerResult<()> {
        let Some(mut child) = self.child.take() else {
            return Ok(());
        };
        if child
            .try_wait()
            .map_err(|error| format!("failed to inspect {}: {error}", self.id))?
            .is_none()
        {
            child
                .kill()
                .map_err(|error| format!("failed to stop {}: {error}", self.id))?;
        }
        child
            .wait()
            .map_err(|error| format!("failed to reap {}: {error}", self.id))?;
        Ok(())
    }
}

fn stop_with_error(agent: &mut ManagedAgent, errors: &mut Vec<String>) -> bool {
    agent.stop().map(|_| true).unwrap_or_else(|error| {
        errors.push(error);
        false
    })
}

fn prepare_empty_root(path: &Path) -> RunnerResult<()> {
    if path.exists()
        && fs::read_dir(path)
            .map_err(|error| format!("failed to inspect {}: {error}", path.display()))?
            .next()
            .is_some()
    {
        return Err("fleet scheduling work root must be empty".to_string());
    }
    fs::create_dir_all(path)
        .map_err(|error| format!("failed to create {}: {error}", path.display()))
}

fn available_distinct_port(other: u16) -> RunnerResult<u16> {
    for _ in 0..8 {
        let port = available_local_port()?;
        if port != other {
            return Ok(port);
        }
    }
    Err("failed to reserve distinct fleet Agent ports".to_string())
}

fn log_excerpt(path: &Path) -> String {
    fs::read_to_string(path)
        .map(|contents| {
            let mut lines = contents.lines().rev().take(12).collect::<Vec<_>>();
            lines.reverse();
            lines.join(" | ")
        })
        .unwrap_or_else(|_| "Agent log unavailable".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capacity_agent_ids_are_stable_and_distinct() {
        assert_ne!(HIGH_AGENT_ID, LOW_AGENT_ID);
        assert_eq!(HIGH_AGENT_ID, "fleet-high-capacity");
    }
}
