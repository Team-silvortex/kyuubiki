use super::agent_process::{CleanupCapture, InstallationCapture, InstalledFleet};
use super::probe::{self, RecoveryProbe};
use serde_json::Value;
use std::path::Path;
use std::time::Duration;

type RunnerResult<T> = Result<T, String>;

#[derive(Debug)]
pub(crate) struct Captured {
    pub(crate) architecture: String,
    pub(crate) installation: InstallationCapture,
    pub(crate) baseline_probe: Value,
    pub(crate) recovery_probe: Value,
    pub(crate) high_process_changed: bool,
    pub(crate) cleanup: CleanupCapture,
}

pub(crate) fn capture_host(
    root: &Path,
    agent_binary: &Path,
    work_root: &Path,
    package_version: &str,
    timeout: Duration,
) -> RunnerResult<Captured> {
    if std::env::consts::OS != "linux" || std::env::var_os("SSH_CONNECTION").is_none() {
        return Err("fleet scheduling operational capture requires a remote Linux SSH host".into());
    }
    let mut fleet = InstalledFleet::prepare(agent_binary, work_root, package_version, timeout)?;
    let installation = clone_installation(fleet.installation());
    let journey = run_journey(root, work_root, &mut fleet, timeout);
    let high_process_changed = fleet.high_process_changed();
    let cleanup = fleet.cleanup();

    match (journey, cleanup) {
        (Ok((baseline_probe, recovery_probe)), Ok(cleanup)) => Ok(Captured {
            architecture: std::env::consts::ARCH.to_string(),
            installation,
            baseline_probe,
            recovery_probe,
            high_process_changed,
            cleanup,
        }),
        (Err(error), Ok(_)) => Err(error),
        (Ok(_), Err(cleanup_error)) => Err(cleanup_error),
        (Err(error), Err(cleanup_error)) => {
            Err(format!("{error}; fleet cleanup failed: {cleanup_error}"))
        }
    }
}

fn run_journey(
    root: &Path,
    work_root: &Path,
    fleet: &mut InstalledFleet,
    timeout: Duration,
) -> RunnerResult<(Value, Value)> {
    if !fleet.both_ready()? {
        return Err("installed fleet was not ready before Orchestra dispatch".to_string());
    }
    let baseline_probe = probe::run_baseline(
        root,
        work_root,
        fleet.high_port(),
        fleet.low_port(),
        timeout,
    )?;

    fleet.stop_high_for_fault()?;
    let mut recovery = RecoveryProbe::spawn(root, work_root, fleet.high_port(), fleet.low_port())?;
    recovery.wait_ready(timeout)?;
    fleet.restart_high(timeout)?;
    recovery.release_after_restart()?;
    let recovery_probe = recovery.finish(timeout)?;

    if !fleet.both_ready()? {
        return Err("installed fleet was not healthy after Agent restart".to_string());
    }
    Ok((baseline_probe, recovery_probe))
}

fn clone_installation(capture: &InstallationCapture) -> InstallationCapture {
    InstallationCapture {
        package: capture.package.clone(),
        high_activation: capture.high_activation.clone(),
        low_activation: capture.low_activation.clone(),
        high_active_version: capture.high_active_version.clone(),
        low_active_version: capture.low_active_version.clone(),
    }
}
