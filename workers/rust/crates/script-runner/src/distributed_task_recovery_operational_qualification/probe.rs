use serde_json::Value;
use std::fs::{self, File};
use std::net::Ipv4Addr;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

type RunnerResult<T> = Result<T, String>;

pub(crate) struct ProbeRun {
    scenario: String,
    child: Child,
    report_path: PathBuf,
    log_path: PathBuf,
}

impl ProbeRun {
    pub(crate) fn spawn(
        root: &Path,
        work_root: &Path,
        scenario: &str,
        remote_ip: Ipv4Addr,
        remote_port: u16,
        local_port: u16,
    ) -> RunnerResult<Self> {
        let report_path = work_root.join(format!("probe-{scenario}.json"));
        let log_path = work_root.join(format!("probe-{scenario}.log"));
        let stdout = File::create(&log_path)
            .map_err(|error| format!("failed to create {scenario} probe log: {error}"))?;
        let stderr = stdout
            .try_clone()
            .map_err(|error| format!("failed to clone {scenario} probe log: {error}"))?;
        let child = Command::new("mix")
            .args([
                "run",
                "-e",
                "KyuubikiWeb.Orchestra.DistributedRecoveryOperationalProbe.run_from_env!()",
            ])
            .current_dir(root.join("apps/web"))
            .env("MIX_ENV", "dev")
            .env("KYUUBIKI_QUAL_SCENARIO", scenario)
            .env("KYUUBIKI_QUAL_PRIMARY_HOST", remote_ip.to_string())
            .env("KYUUBIKI_QUAL_PRIMARY_PORT", remote_port.to_string())
            .env("KYUUBIKI_QUAL_FALLBACK_PORT", local_port.to_string())
            .env("KYUUBIKI_QUAL_REPORT_PATH", &report_path)
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr))
            .spawn()
            .map_err(|error| format!("failed to start {scenario} Orchestra probe: {error}"))?;
        Ok(Self {
            scenario: scenario.to_string(),
            child,
            report_path,
            log_path,
        })
    }

    pub(crate) fn ensure_running(&mut self) -> RunnerResult<()> {
        if let Some(status) = self
            .child
            .try_wait()
            .map_err(|error| format!("failed to inspect {} probe: {error}", self.scenario))?
        {
            return Err(format!(
                "{} probe exited before fault injection: {status}; {}",
                self.scenario,
                self.log_excerpt()
            ));
        }
        Ok(())
    }

    pub(crate) fn finish(mut self, timeout: Duration) -> RunnerResult<Value> {
        let deadline = Instant::now() + timeout;
        let status =
            loop {
                if let Some(status) = self.child.try_wait().map_err(|error| {
                    format!("failed to inspect {} probe: {error}", self.scenario)
                })? {
                    break status;
                }
                if Instant::now() >= deadline {
                    let _ = self.child.kill();
                    let _ = self.child.wait();
                    return Err(format!(
                        "{} probe timed out; {}",
                        self.scenario,
                        self.log_excerpt()
                    ));
                }
                thread::sleep(Duration::from_millis(50));
            };
        if !status.success() {
            return Err(format!(
                "{} probe failed: {status}; {}",
                self.scenario,
                self.log_excerpt()
            ));
        }
        let bytes = fs::read(&self.report_path).map_err(|error| {
            format!(
                "failed to read {} probe report {}: {error}",
                self.scenario,
                self.report_path.display()
            )
        })?;
        serde_json::from_slice(&bytes)
            .map_err(|error| format!("invalid {} probe report: {error}", self.scenario))
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
