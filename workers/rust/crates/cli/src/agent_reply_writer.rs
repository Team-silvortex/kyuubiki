use std::net::TcpStream;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::agent_lifecycle::{self, ExecutionAdmissionError, ExecutionGuard};

pub(crate) const REPLY_TIMEOUT_ENV: &str = "KYUUBIKI_AGENT_REPLY_TIMEOUT_MS";
static CONFIGURED_TIMEOUT_MS: AtomicU64 = AtomicU64::new(10_000);
pub(crate) type SharedReplyWriter = Arc<ReplyWriter>;

pub(crate) struct ReplyWriter {
    pub(crate) stream: Mutex<TcpStream>,
    pub(crate) timeout: Duration,
    pending: Mutex<Option<ExecutionGuard>>,
}

impl ReplyWriter {
    pub(crate) fn new(stream: TcpStream, timeout: Duration) -> Self {
        Self {
            stream: Mutex::new(stream),
            timeout,
            pending: Mutex::new(None),
        }
    }

    fn retain_execution(&self, guard: &ExecutionGuard) -> Result<(), String> {
        let mut pending = self
            .pending
            .lock()
            .map_err(|_| "reply state is unavailable")?;
        if pending.is_some() {
            return Err("reply writer already owns an execution".into());
        }
        *pending = Some(guard.clone());
        Ok(())
    }

    pub(crate) fn finish(&self, result: Result<(), String>) -> Result<(), String> {
        let (guard, result) = match self.pending.lock() {
            Ok(mut pending) => (pending.take(), result),
            Err(poisoned) => (
                poisoned.into_inner().take(),
                Err("reply state became unavailable during delivery".into()),
            ),
        };
        if let Some(guard) = guard {
            match &result {
                Ok(()) => agent_lifecycle::complete_execution(guard),
                Err(error) => retain_failure(guard, "result_delivery_failed", error),
            }
        }
        result
    }
}

impl Drop for ReplyWriter {
    fn drop(&mut self) {
        // Even unwinding or an abandoned response must release the exact lease.
        let pending = self
            .pending
            .get_mut()
            .unwrap_or_else(|error| error.into_inner());
        if let Some(guard) = pending.take() {
            retain_failure(
                guard,
                "result_delivery_aborted",
                "response writer closed before final delivery completed",
            );
        }
    }
}

fn retain_failure(guard: ExecutionGuard, code: &str, message: &str) {
    let failure = agent_lifecycle::fail_execution(guard, code, message);
    eprintln!(
        "agent result delivery: {}",
        serde_json::to_string(&failure).expect("execution failure should serialize")
    );
}

pub(crate) fn begin_execution(
    writer: Option<&SharedReplyWriter>,
    request_id: String,
    job_id: Option<String>,
    method: String,
) -> Result<ExecutionGuard, ExecutionAdmissionError> {
    let guard = agent_lifecycle::begin_execution(request_id.clone(), job_id, method)?;
    if let Some(writer) = writer
        && let Err(message) = writer.retain_execution(&guard)
    {
        agent_lifecycle::fail_execution(guard, "result_delivery_unavailable", &message);
        return Err(ExecutionAdmissionError {
            request_id,
            reason_code: "result_delivery_unavailable".into(),
            message,
        });
    }
    Ok(guard)
}

pub(crate) fn complete_execution(writer: Option<&SharedReplyWriter>, guard: ExecutionGuard) {
    if writer.is_none() {
        agent_lifecycle::complete_execution(guard);
    }
    // Network executions complete only when ReplyWriter::finish releases its clone.
}

pub(crate) fn timeout_from_env() -> Result<Duration, String> {
    let timeout = match std::env::var(REPLY_TIMEOUT_ENV) {
        Ok(value) => parse_timeout(Some(&value)),
        Err(std::env::VarError::NotPresent) => parse_timeout(None),
        Err(_) => Err(format!("{REPLY_TIMEOUT_ENV} must be valid Unicode")),
    }?;
    CONFIGURED_TIMEOUT_MS.store(timeout.as_millis() as u64, Ordering::Relaxed);
    Ok(timeout)
}

pub(crate) fn snapshot() -> serde_json::Value {
    serde_json::json!({
        "schema_version":"kyuubiki.agent-reply-delivery/v1",
        "timeout_ms":CONFIGURED_TIMEOUT_MS.load(Ordering::Relaxed),
        "configuration_env":REPLY_TIMEOUT_ENV,
        "deadline_scope":"progress_and_final_response",
        "completion_boundary":"final_response_written_to_transport",
        "durable_receiver_acknowledgement":false
    })
}

fn parse_timeout(value: Option<&str>) -> Result<Duration, String> {
    let millis = match value {
        None => 10_000,
        Some(value) => value
            .parse::<u64>()
            .ok()
            .filter(|value| (1..=300_000).contains(value))
            .ok_or_else(|| format!("{REPLY_TIMEOUT_ENV} must be within 1..=300000"))?,
    };
    Ok(Duration::from_millis(millis))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn response_deadline_configuration_never_silently_disables_the_bound() {
        assert_eq!(parse_timeout(None).unwrap(), Duration::from_secs(10));
        assert_eq!(parse_timeout(Some("1")).unwrap(), Duration::from_millis(1));
        assert_eq!(
            parse_timeout(Some("300000")).unwrap(),
            Duration::from_secs(300)
        );
        for value in [
            "",
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
}
