use super::{BootReceipt, RunnerResult, read_and_validate_receipt};
use std::{
    path::Path,
    process::Child,
    thread,
    time::{Duration, Instant},
};

pub(super) fn wait_for_receipt(
    child: &mut Child,
    path: &Path,
    surface: &str,
    expected_pid: Option<u32>,
    deadline: Instant,
) -> RunnerResult<BootReceipt> {
    let started = Instant::now();
    let mut next_notice = started + Duration::from_secs(5);
    loop {
        check_live_budget(child, surface, deadline)?;
        if path.is_file() {
            let receipt = read_and_validate_receipt(path, surface, expected_pid)?;
            // Reading/validating a receipt must not extend the startup budget or
            // turn a process that has already exited into successful startup.
            check_live_budget(child, surface, deadline)?;
            return Ok(receipt);
        }
        let now = Instant::now();
        if now >= next_notice {
            eprintln!(
                "packaged {surface}: child is running, waiting for UI readiness ({}s elapsed, {}s remaining)",
                now.duration_since(started).as_secs(),
                deadline.saturating_duration_since(now).as_secs(),
            );
            next_notice = now + Duration::from_secs(5);
        }
        thread::sleep(Duration::from_millis(100).min(deadline.saturating_duration_since(now)));
    }
}

fn check_live_budget(child: &mut Child, surface: &str, deadline: Instant) -> RunnerResult<()> {
    if let Some(status) = child
        .try_wait()
        .map_err(|error| format!("failed to inspect {surface} process: {error}"))?
    {
        return Err(format!(
            "packaged {surface} exited before readiness with {status}"
        ));
    }
    if Instant::now() >= deadline {
        return Err(format!(
            "packaged {surface} did not report readiness before its deadline (child still running)"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests;
