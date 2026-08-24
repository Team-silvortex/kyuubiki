use kyuubiki_protocol::AgentControlLinkDescriptor;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_RETRY_DELAY_MS: u64 = 30_000;

pub(crate) fn configure(base_retry_delay_ms: u64) {
    mutate(|link| {
        *link = AgentControlLinkDescriptor {
            state: "connecting".to_string(),
            operation: "register".to_string(),
            orchestrator_bound: true,
            next_retry_delay_ms: base_retry_delay_ms,
            ..AgentControlLinkDescriptor::default()
        };
    });
}

pub(crate) fn record_attempt(operation: &str) {
    mutate(|link| {
        link.attempt_count = link.attempt_count.saturating_add(1);
        link.operation = operation.to_string();
        if operation == "register" {
            link.state = if link.last_success_unix_ms.is_some() {
                "rejoining"
            } else {
                "connecting"
            }
            .to_string();
        }
    });
}

pub(crate) fn record_success(
    operation: &str,
    base_retry_delay_ms: u64,
) -> AgentControlLinkDescriptor {
    mutate(|link| {
        link.state = "registered".to_string();
        link.operation = operation.to_string();
        link.consecutive_failure_count = 0;
        link.last_success_unix_ms = Some(unix_now_ms());
        link.last_failure_code = None;
        link.last_failure_message = None;
        link.next_retry_delay_ms = base_retry_delay_ms;
        match operation {
            "register" => {
                link.successful_registration_count =
                    link.successful_registration_count.saturating_add(1);
            }
            "heartbeat" => {
                link.successful_heartbeat_count = link.successful_heartbeat_count.saturating_add(1);
            }
            _ => {}
        }
    })
}

pub(crate) fn record_failure(
    operation: &str,
    error: &str,
    base_retry_delay_ms: u64,
) -> AgentControlLinkDescriptor {
    mutate(|link| {
        let (code, message) = classify_failure(error);
        link.state = "degraded".to_string();
        link.operation = operation.to_string();
        link.consecutive_failure_count = link.consecutive_failure_count.saturating_add(1);
        link.last_failure_unix_ms = Some(unix_now_ms());
        link.last_failure_code = Some(code.to_string());
        link.last_failure_message = Some(message.to_string());
        link.next_retry_delay_ms =
            retry_delay_ms(base_retry_delay_ms, link.consecutive_failure_count);
    })
}

pub(crate) fn record_stopped(unregister_error: Option<&str>) {
    mutate(|link| {
        link.state = "stopped".to_string();
        link.operation = "unregister".to_string();
        link.next_retry_delay_ms = 0;
        if let Some(error) = unregister_error {
            let (code, message) = classify_failure(error);
            link.last_failure_unix_ms = Some(unix_now_ms());
            link.last_failure_code = Some(code.to_string());
            link.last_failure_message = Some(message.to_string());
        }
    });
}

pub(crate) fn snapshot() -> AgentControlLinkDescriptor {
    state().lock().map(|link| link.clone()).unwrap_or_default()
}

pub(crate) fn retry_delay_ms(base_retry_delay_ms: u64, failure_count: u32) -> u64 {
    if failure_count == 0 {
        return base_retry_delay_ms;
    }
    let exponent = failure_count.saturating_sub(1).min(6);
    let multiplier = 1_u64 << exponent;
    let cap = MAX_RETRY_DELAY_MS.max(base_retry_delay_ms);
    base_retry_delay_ms.saturating_mul(multiplier).min(cap)
}

fn mutate(update: impl FnOnce(&mut AgentControlLinkDescriptor)) -> AgentControlLinkDescriptor {
    match state().lock() {
        Ok(mut link) => {
            update(&mut link);
            link.clone()
        }
        Err(_) => AgentControlLinkDescriptor::default(),
    }
}

fn state() -> &'static Mutex<AgentControlLinkDescriptor> {
    static STATE: OnceLock<Mutex<AgentControlLinkDescriptor>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(AgentControlLinkDescriptor::default()))
}

fn classify_failure(error: &str) -> (&'static str, &'static str) {
    if error.contains("failed to resolve") {
        (
            "endpoint_resolution_failed",
            "orchestrator endpoint could not be resolved",
        )
    } else if error.contains("failed to connect") {
        (
            "endpoint_unreachable",
            "orchestrator endpoint is unreachable",
        )
    } else if error.contains("unexpected HTTP response") {
        (
            "request_rejected",
            "orchestrator rejected the control-plane request",
        )
    } else if error.contains("URL") || error.contains("HTTP endpoint") {
        (
            "invalid_endpoint",
            "orchestrator endpoint configuration is invalid",
        )
    } else {
        (
            "transport_failed",
            "orchestrator control-plane transport failed",
        )
    }
}

fn unix_now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or(0)
}

#[cfg(test)]
pub(crate) fn test_guard() -> std::sync::MutexGuard<'static, ()> {
    static TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    TEST_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("Agent control-link test lock")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_sanitized_failure_and_recovery_state() {
        let _guard = test_guard();
        configure(1_000);
        record_attempt("register");
        let failed = record_failure(
            "register",
            "unexpected HTTP response from http://secret.invalid: HTTP/1.1 401 token=secret",
            1_000,
        );
        assert_eq!(failed.state, "degraded");
        assert_eq!(
            failed.last_failure_code.as_deref(),
            Some("request_rejected")
        );
        assert!(!failed.last_failure_message.unwrap().contains("secret"));
        assert_eq!(failed.next_retry_delay_ms, 1_000);

        record_attempt("register");
        let recovered = record_success("register", 1_000);
        assert_eq!(recovered.state, "registered");
        assert_eq!(recovered.successful_registration_count, 1);
        assert_eq!(recovered.consecutive_failure_count, 0);
        assert_eq!(recovered.last_failure_code, None);
        record_stopped(None);
    }

    #[test]
    fn retry_delay_is_bounded() {
        assert_eq!(retry_delay_ms(1_000, 0), 1_000);
        assert_eq!(retry_delay_ms(1_000, 1), 1_000);
        assert_eq!(retry_delay_ms(1_000, 2), 2_000);
        assert_eq!(retry_delay_ms(1_000, 6), 30_000);
        assert_eq!(retry_delay_ms(1_000, 20), 30_000);
    }
}
