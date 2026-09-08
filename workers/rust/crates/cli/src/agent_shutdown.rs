use std::io::Write;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use crate::{agent_lifecycle, agent_watchdog};

const TIMEOUT_ENV: &str = "KYUUBIKI_AGENT_SHUTDOWN_TIMEOUT_MS";
static CONFIGURED_TIMEOUT_MS: AtomicU64 = AtomicU64::new(30_000);

struct ShutdownRequest {
    started: Instant,
    cause: &'static str,
    error: Option<String>,
}

#[derive(Clone)]
pub(crate) struct ShutdownTrigger {
    sender: mpsc::SyncSender<ShutdownRequest>,
    requested: Arc<AtomicBool>,
}

impl ShutdownTrigger {
    pub(crate) fn request(&self, cause: &'static str, error: Option<String>) {
        self.request_with(cause, error, agent_lifecycle::begin_shutdown);
    }

    fn request_with(
        &self,
        cause: &'static str,
        error: Option<String>,
        close_admission: impl FnOnce() -> Result<(), String>,
    ) {
        if self.requested.swap(true, Ordering::SeqCst) {
            return;
        }
        let started = Instant::now();
        let admission_error = close_admission().err();
        let _ = self.sender.try_send(ShutdownRequest {
            started,
            cause,
            error: error.or(admission_error),
        });
    }
}

pub(crate) struct AgentShutdown {
    trigger: ShutdownTrigger,
    receiver: mpsc::Receiver<ShutdownRequest>,
    timeout: Duration,
}

impl AgentShutdown {
    pub(crate) fn install() -> Result<Self, String> {
        let value = match std::env::var(TIMEOUT_ENV) {
            Ok(value) => Some(value),
            Err(std::env::VarError::NotPresent) => None,
            Err(_) => return Err(format!("{TIMEOUT_ENV} must be valid Unicode")),
        };
        let timeout = parse_timeout(value.as_deref())?;
        CONFIGURED_TIMEOUT_MS.store(timeout.as_millis() as u64, Ordering::Relaxed);
        let (sender, receiver) = mpsc::sync_channel(1);
        let trigger = ShutdownTrigger {
            sender,
            requested: Arc::new(AtomicBool::new(false)),
        };
        let signal_trigger = trigger.clone();
        // ctrlc invokes this on its dedicated thread, not in an async signal handler.
        ctrlc::try_set_handler(move || signal_trigger.request("termination_signal", None))
            .map_err(|error| format!("failed to install Agent termination handler: {error}"))?;
        Ok(Self {
            trigger,
            receiver,
            timeout,
        })
    }

    pub(crate) fn trigger(&self) -> ShutdownTrigger {
        self.trigger.clone()
    }

    pub(crate) fn finish(self, cleanup: impl FnOnce() + Send + 'static) -> Result<(), String> {
        let request = self
            .receiver
            .recv()
            .map_err(|_| "Agent shutdown channel closed")?;
        let deadline = request.started + self.timeout;
        emit("draining", "agent_shutdown_requested", &request);
        loop {
            if agent_lifecycle::snapshot().safe_to_replace {
                break;
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                emit("draining", "agent_shutdown_timeout", &request);
                return Err(
                    "agent_shutdown_timeout: in-flight execution or result delivery remains".into(),
                );
            }
            thread::sleep(remaining.min(Duration::from_millis(10)));
        }
        emit("cleanup", "agent_shutdown_quiescent", &request);
        if let Err(error) = bounded_cleanup(deadline, cleanup) {
            let code = if error == "agent_shutdown_timeout" {
                "agent_shutdown_timeout"
            } else {
                "agent_shutdown_cleanup_failed"
            };
            emit("cleanup", code, &request);
            return Err(format!(
                "{error}: background cleanup did not finish successfully"
            ));
        }
        if let Some(error) = &request.error {
            emit("failed", "agent_shutdown_failed", &request);
            return Err(error.clone());
        }
        emit("completed", "agent_shutdown_complete", &request);
        Ok(())
    }
}

fn bounded_cleanup(
    deadline: Instant,
    cleanup: impl FnOnce() + Send + 'static,
) -> Result<(), String> {
    let remaining = deadline.saturating_duration_since(Instant::now());
    if remaining.is_zero() {
        return Err("agent_shutdown_timeout".into());
    }
    let (sender, receiver) = mpsc::sync_channel(1);
    let handle = thread::Builder::new()
        .name("kyuubiki-agent-cleanup".into())
        .spawn(move || {
            cleanup();
            let _ = sender.send(());
        })
        .map_err(|error| format!("failed to start Agent cleanup: {error}"))?;
    receiver
        .recv_timeout(deadline.saturating_duration_since(Instant::now()))
        .map_err(|error| match error {
            mpsc::RecvTimeoutError::Timeout => "agent_shutdown_timeout".to_string(),
            mpsc::RecvTimeoutError::Disconnected => "agent_shutdown_cleanup_failed".to_string(),
        })?;
    handle
        .join()
        .map_err(|_| "agent_shutdown_cleanup_failed".to_string())
}

pub(crate) fn snapshot() -> Value {
    json!({
        "schema_version":"kyuubiki.agent-shutdown-policy/v1",
        "configuration_env":TIMEOUT_ENV,
        "timeout_ms":CONFIGURED_TIMEOUT_MS.load(Ordering::Relaxed),
        "deadline_scope":"drain_and_background_cleanup",
        "admission":"irreversible_until_process_exit",
        "timeout_exit_code":1,
        "signal_source":if cfg!(unix) { "SIGINT_SIGTERM_SIGHUP" } else { "platform_ctrlc_events" },
        "hard_kill_recovery":false
    })
}

fn emit(phase: &str, reason_code: &str, request: &ShutdownRequest) {
    let watchdog = agent_watchdog::snapshot();
    let event = json!({
        "schema_version":"kyuubiki.agent-shutdown/v1", "phase":phase,
        "reason_code":reason_code, "cause":request.cause, "error":request.error,
        "elapsed_ms":request.started.elapsed().as_millis(), "policy":snapshot(),
        "lifecycle":agent_lifecycle::snapshot(),
        "unfinished_executions":watchdog.active_executions,
        "recent_execution_failures":watchdog.recent_failures
    });
    let mut stderr = std::io::stderr().lock();
    let _ = serde_json::to_writer(&mut stderr, &event);
    let _ = writeln!(stderr);
}

fn parse_timeout(value: Option<&str>) -> Result<Duration, String> {
    let millis = match value {
        None => 30_000,
        Some(value) => value
            .parse::<u64>()
            .ok()
            .filter(|value| (1..=300_000).contains(value))
            .ok_or_else(|| format!("{TIMEOUT_ENV} must be within 1..=300000"))?,
    };
    Ok(Duration::from_millis(millis))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeated_shutdown_signals_do_not_reset_the_deadline_or_close_admission_twice() {
        let (sender, receiver) = mpsc::sync_channel(1);
        let trigger = ShutdownTrigger {
            sender,
            requested: Arc::new(AtomicBool::new(false)),
        };
        trigger.request_with("first", None, || Ok(()));
        let first = receiver.recv().unwrap();
        trigger.request_with("second", None, || {
            panic!("admission must be latched only once")
        });
        assert!(receiver.try_recv().is_err());
        assert_eq!(first.cause, "first");
    }

    #[test]
    fn shutdown_budget_never_silently_disables_the_bound() {
        assert_eq!(parse_timeout(None).unwrap(), Duration::from_secs(30));
        assert_eq!(
            parse_timeout(Some("300000")).unwrap(),
            Duration::from_secs(300)
        );
        for value in [
            "0",
            "-1",
            "1.5",
            "300001",
            "forever",
            "18446744073709551616",
        ] {
            assert!(parse_timeout(Some(value)).is_err(), "{value}");
        }
    }

    #[test]
    fn background_cleanup_cannot_extend_the_total_shutdown_budget() {
        let (release, wait) = mpsc::channel();
        let (finished, done) = mpsc::channel();
        let result = bounded_cleanup(Instant::now() + Duration::from_millis(20), move || {
            let _ = wait.recv();
            let _ = finished.send(());
        });
        assert_eq!(result.unwrap_err(), "agent_shutdown_timeout");
        if release.send(()).is_ok() {
            done.recv_timeout(Duration::from_secs(3)).unwrap();
        }
    }

    #[test]
    fn successful_cleanup_finishes_without_waiting_for_the_budget() {
        bounded_cleanup(Instant::now() + Duration::from_secs(3), || {}).unwrap();
    }
}
