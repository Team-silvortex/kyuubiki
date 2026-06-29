use std::collections::{HashMap, VecDeque};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

const RECENT_FAILURE_LIMIT: usize = 16;

#[derive(Debug, Clone)]
pub(crate) struct ExecutionGuard {
    request_id: String,
}

#[derive(Debug, Clone)]
struct ExecutionRecord {
    request_id: String,
    job_id: Option<String>,
    method: String,
    started_unix_ms: u128,
    last_progress_unix_ms: u128,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct FailureReport {
    pub(crate) request_id: String,
    pub(crate) job_id: Option<String>,
    pub(crate) method: String,
    pub(crate) reason_code: String,
    pub(crate) message: String,
    pub(crate) elapsed_ms: u128,
    pub(crate) occurred_unix_ms: u128,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct WatchdogSnapshot {
    pub(crate) state: String,
    pub(crate) active_execution_count: usize,
    pub(crate) recent_failure_count: usize,
    pub(crate) active_executions: Vec<ActiveExecutionSnapshot>,
    pub(crate) recent_failures: Vec<FailureReport>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ActiveExecutionSnapshot {
    pub(crate) request_id: String,
    pub(crate) job_id: Option<String>,
    pub(crate) method: String,
    pub(crate) elapsed_ms: u128,
    pub(crate) idle_ms: u128,
}

#[derive(Debug, Default)]
struct WatchdogState {
    active: HashMap<String, ExecutionRecord>,
    recent_failures: VecDeque<FailureReport>,
}

pub(crate) fn begin_execution(
    request_id: String,
    job_id: Option<String>,
    method: String,
) -> ExecutionGuard {
    let now = unix_now_ms();

    if let Ok(mut state) = watchdog_state().lock() {
        state.active.insert(
            request_id.clone(),
            ExecutionRecord {
                request_id: request_id.clone(),
                job_id,
                method,
                started_unix_ms: now,
                last_progress_unix_ms: now,
            },
        );
    }

    ExecutionGuard { request_id }
}

pub(crate) fn complete_execution(guard: ExecutionGuard) {
    if let Ok(mut state) = watchdog_state().lock() {
        state.active.remove(&guard.request_id);
    }
}

pub(crate) fn fail_execution(
    guard: ExecutionGuard,
    reason_code: &str,
    message: impl Into<String>,
) -> FailureReport {
    let message = message.into();
    let now = unix_now_ms();

    let report = if let Ok(mut state) = watchdog_state().lock() {
        let record = state
            .active
            .remove(&guard.request_id)
            .unwrap_or_else(|| ExecutionRecord {
                request_id: guard.request_id.clone(),
                job_id: None,
                method: "unknown".to_string(),
                started_unix_ms: now,
                last_progress_unix_ms: now,
            });

        let report = FailureReport {
            request_id: record.request_id,
            job_id: record.job_id,
            method: record.method,
            reason_code: reason_code.to_string(),
            message,
            elapsed_ms: now.saturating_sub(record.started_unix_ms),
            occurred_unix_ms: now,
        };

        state.recent_failures.push_front(report.clone());
        while state.recent_failures.len() > RECENT_FAILURE_LIMIT {
            state.recent_failures.pop_back();
        }

        report
    } else {
        FailureReport {
            request_id: guard.request_id,
            job_id: None,
            method: "unknown".to_string(),
            reason_code: reason_code.to_string(),
            message,
            elapsed_ms: 0,
            occurred_unix_ms: now,
        }
    };

    report
}

pub(crate) fn snapshot() -> WatchdogSnapshot {
    let now = unix_now_ms();

    if let Ok(state) = watchdog_state().lock() {
        let active_executions = state
            .active
            .values()
            .map(|record| ActiveExecutionSnapshot {
                request_id: record.request_id.clone(),
                job_id: record.job_id.clone(),
                method: record.method.clone(),
                elapsed_ms: now.saturating_sub(record.started_unix_ms),
                idle_ms: now.saturating_sub(record.last_progress_unix_ms),
            })
            .collect::<Vec<_>>();

        WatchdogSnapshot {
            state: if state.recent_failures.is_empty() {
                "healthy".to_string()
            } else {
                "watch".to_string()
            },
            active_execution_count: state.active.len(),
            recent_failure_count: state.recent_failures.len(),
            active_executions,
            recent_failures: state.recent_failures.iter().cloned().collect(),
        }
    } else {
        WatchdogSnapshot {
            state: "unknown".to_string(),
            active_execution_count: 0,
            recent_failure_count: 0,
            active_executions: vec![],
            recent_failures: vec![],
        }
    }
}

#[cfg(test)]
pub(crate) fn reset_for_tests() {
    if let Ok(mut state) = watchdog_state().lock() {
        *state = WatchdogState::default();
    }
}

fn watchdog_state() -> &'static Mutex<WatchdogState> {
    static STATE: OnceLock<Mutex<WatchdogState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(WatchdogState::default()))
}

fn unix_now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}
