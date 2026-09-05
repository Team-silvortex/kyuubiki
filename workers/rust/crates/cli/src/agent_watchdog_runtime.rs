use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use crate::agent_state::register_execution_cancel;
use crate::agent_watchdog;
use crate::config::AgentConfig;

pub(crate) struct AgentWatchdogRuntimeHandle {
    running: Arc<AtomicBool>,
    join_handle: Option<thread::JoinHandle<()>>,
}

impl AgentWatchdogRuntimeHandle {
    pub(crate) fn maybe_spawn(config: &AgentConfig) -> Result<Option<Self>, String> {
        let scan_interval_ms = config.watchdog_scan_interval_ms;
        let stale_execution_ms = config.watchdog_stale_execution_ms;
        agent_watchdog::configure_policy(scan_interval_ms, stale_execution_ms);
        if scan_interval_ms == 0 || stale_execution_ms == 0 {
            return Ok(None);
        }

        let running = Arc::new(AtomicBool::new(true));
        let running_clone = Arc::clone(&running);
        let join_handle = thread::Builder::new()
            .name("kyuubiki-agent-watchdog".to_string())
            .spawn(move || {
                while running_clone.load(Ordering::SeqCst) {
                    thread::park_timeout(Duration::from_millis(scan_interval_ms));
                    if !running_clone.load(Ordering::SeqCst) {
                        break;
                    }
                    submit_timeout_cancellations(agent_watchdog::scan_stale_executions());
                }
            })
            .map_err(|error| format!("failed to spawn Agent watchdog: {error}"))?;

        Ok(Some(Self {
            running,
            join_handle: Some(join_handle),
        }))
    }

    pub(crate) fn stop(mut self) {
        self.running.store(false, Ordering::SeqCst);
        if let Some(join_handle) = self.join_handle.take() {
            join_handle.thread().unpark();
            let _ = join_handle.join();
        }
    }
}

fn submit_timeout_cancellations(failures: Vec<agent_watchdog::FailureReport>) -> usize {
    failures
        .into_iter()
        .filter(|failure| failure.job_id.is_some())
        .map(|failure| {
            register_execution_cancel(failure.request_id);
            1_usize
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_state::{take_cancelled, take_execution_cancelled};

    #[test]
    fn timeout_bridge_submits_only_job_bound_cancellations() {
        let failures = vec![failure(Some("watchdog-runtime-job")), failure(None)];

        assert_eq!(submit_timeout_cancellations(failures), 1);
        assert!(take_execution_cancelled("watchdog-runtime-request"));
        assert!(!take_cancelled("watchdog-runtime-job"));
    }

    fn failure(job_id: Option<&str>) -> agent_watchdog::FailureReport {
        agent_watchdog::FailureReport {
            request_id: "watchdog-runtime-request".to_string(),
            generation: 1,
            job_id: job_id.map(ToString::to_string),
            method: "solve_bar_1d".to_string(),
            reason_code: "watchdog_timeout".to_string(),
            message: "injected timeout".to_string(),
            elapsed_ms: 100,
            occurred_unix_ms: 1_000,
        }
    }
}
