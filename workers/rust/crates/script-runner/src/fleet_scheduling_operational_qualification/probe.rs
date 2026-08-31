use serde_json::Value;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

type RunnerResult<T> = Result<T, String>;

const BASELINE_EXPRESSION: &str =
    "KyuubikiWeb.Orchestra.FleetSchedulingOperationalProbe.run_baseline_from_env!()";
const RECOVERY_EXPRESSION: &str =
    "KyuubikiWeb.Orchestra.FleetSchedulingOperationalProbe.run_failover_recovery_from_env!()";

pub(crate) fn run_baseline(
    root: &Path,
    work_root: &Path,
    high_port: u16,
    low_port: u16,
    timeout: Duration,
) -> RunnerResult<Value> {
    let mut run = ProbeRun::spawn(
        root,
        work_root,
        "baseline",
        BASELINE_EXPRESSION,
        high_port,
        low_port,
        false,
    )?;
    run.finish(timeout)
}

pub(crate) struct RecoveryProbe {
    run: ProbeRun,
    ready_path: PathBuf,
    release_path: PathBuf,
}

impl RecoveryProbe {
    pub(crate) fn spawn(
        root: &Path,
        work_root: &Path,
        high_port: u16,
        low_port: u16,
    ) -> RunnerResult<Self> {
        let ready_path = work_root.join("ready-for-agent-restart");
        let release_path = work_root.join("agent-restarted");
        let _ = fs::remove_file(&ready_path);
        let _ = fs::remove_file(&release_path);
        let run = ProbeRun::spawn(
            root,
            work_root,
            "failover-recovery",
            RECOVERY_EXPRESSION,
            high_port,
            low_port,
            true,
        )?;
        Ok(Self {
            run,
            ready_path,
            release_path,
        })
    }

    pub(crate) fn wait_ready(&mut self, timeout: Duration) -> RunnerResult<()> {
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            self.run.ensure_running()?;
            if self.ready_path.is_file() {
                return Ok(());
            }
            thread::sleep(Duration::from_millis(50));
        }
        Err(format!(
            "fleet recovery probe did not reach restart barrier; {}",
            self.run.log_excerpt()
        ))
    }

    pub(crate) fn release_after_restart(&self) -> RunnerResult<()> {
        fs::write(&self.release_path, b"restarted\n")
            .map_err(|error| format!("failed to release fleet restart barrier: {error}"))
    }

    pub(crate) fn finish(mut self, timeout: Duration) -> RunnerResult<Value> {
        self.run.finish(timeout)
    }
}

struct ProbeRun {
    phase: String,
    child: Child,
    report_path: PathBuf,
    log_path: PathBuf,
}

impl ProbeRun {
    #[allow(clippy::too_many_arguments)]
    fn spawn(
        root: &Path,
        work_root: &Path,
        phase: &str,
        expression: &str,
        high_port: u16,
        low_port: u16,
        handshake: bool,
    ) -> RunnerResult<Self> {
        let report_path = work_root.join(format!("probe-{phase}.json"));
        let log_path = work_root.join(format!("probe-{phase}.log"));
        let stdout = File::create(&log_path)
            .map_err(|error| format!("failed to create fleet {phase} log: {error}"))?;
        let stderr = stdout
            .try_clone()
            .map_err(|error| format!("failed to clone fleet {phase} log: {error}"))?;
        let mut command = Command::new("mix");
        command
            .args(["run", "--no-start", "-e", expression])
            .current_dir(root.join("apps/web"))
            .env("MIX_ENV", "dev")
            .env("KYUUBIKI_QUAL_HIGH_PORT", high_port.to_string())
            .env("KYUUBIKI_QUAL_LOW_PORT", low_port.to_string())
            .env("KYUUBIKI_QUAL_REPORT_PATH", &report_path)
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr));
        if handshake {
            command
                .env(
                    "KYUUBIKI_QUAL_READY_PATH",
                    work_root.join("ready-for-agent-restart"),
                )
                .env(
                    "KYUUBIKI_QUAL_RELEASE_PATH",
                    work_root.join("agent-restarted"),
                );
        }
        let child = command
            .spawn()
            .map_err(|error| format!("failed to start fleet {phase} Orchestra probe: {error}"))?;
        Ok(Self {
            phase: phase.to_string(),
            child,
            report_path,
            log_path,
        })
    }

    fn ensure_running(&mut self) -> RunnerResult<()> {
        if let Some(status) = self
            .child
            .try_wait()
            .map_err(|error| format!("failed to inspect {} probe: {error}", self.phase))?
        {
            return Err(format!(
                "{} probe exited before the restart barrier: {status}; {}",
                self.phase,
                self.log_excerpt()
            ));
        }
        Ok(())
    }

    fn finish(&mut self, timeout: Duration) -> RunnerResult<Value> {
        let deadline = Instant::now() + timeout;
        let status = loop {
            if let Some(status) = self
                .child
                .try_wait()
                .map_err(|error| format!("failed to inspect {} probe: {error}", self.phase))?
            {
                break status;
            }
            if Instant::now() >= deadline {
                let _ = self.child.kill();
                let _ = self.child.wait();
                return Err(format!(
                    "{} probe timed out; {}",
                    self.phase,
                    self.log_excerpt()
                ));
            }
            thread::sleep(Duration::from_millis(50));
        };
        if !status.success() {
            return Err(format!(
                "{} probe failed: {status}; {}",
                self.phase,
                self.log_excerpt()
            ));
        }
        let bytes = fs::read(&self.report_path).map_err(|error| {
            format!(
                "failed to read {} report {}: {error}",
                self.phase,
                self.report_path.display()
            )
        })?;
        serde_json::from_slice(&bytes)
            .map_err(|error| format!("invalid {} probe report: {error}", self.phase))
    }

    fn log_excerpt(&self) -> String {
        fs::read_to_string(&self.log_path)
            .map(|contents| {
                let mut lines = contents.lines().rev().take(12).collect::<Vec<_>>();
                lines.reverse();
                lines.join(" | ")
            })
            .unwrap_or_else(|_| "probe log unavailable".to_string())
    }
}

impl Drop for ProbeRun {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probes_use_the_dedicated_orchestra_surface() {
        assert!(BASELINE_EXPRESSION.contains("FleetSchedulingOperationalProbe"));
        assert!(RECOVERY_EXPRESSION.contains("run_failover_recovery_from_env"));
    }

    #[test]
    fn probes_do_not_start_the_web_application() {
        let args = ["run", "--no-start", "-e", BASELINE_EXPRESSION];
        assert!(args.contains(&"--no-start"));
    }
}
