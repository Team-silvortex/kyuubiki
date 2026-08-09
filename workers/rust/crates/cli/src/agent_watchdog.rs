use std::collections::{HashMap, VecDeque};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;
use serde_json::{Value, json};

const RECENT_FAILURE_LIMIT: usize = 16;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct WatchdogPolicySnapshot {
    pub(crate) enabled: bool,
    pub(crate) scan_interval_ms: u64,
    pub(crate) stale_execution_ms: u64,
    pub(crate) monitored_scope: String,
}

impl Default for WatchdogPolicySnapshot {
    fn default() -> Self {
        Self {
            enabled: false,
            scan_interval_ms: 0,
            stale_execution_ms: 0,
            monitored_scope: "job_bound_execution".to_string(),
        }
    }
}

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
    pub(crate) policy: WatchdogPolicySnapshot,
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
    policy: WatchdogPolicySnapshot,
    active: HashMap<String, ExecutionRecord>,
    recent_failures: VecDeque<FailureReport>,
}

pub(crate) fn configure_policy(scan_interval_ms: u64, stale_execution_ms: u64) {
    configure_policy_in(watchdog_state(), scan_interval_ms, stale_execution_ms);
}

fn configure_policy_in(
    state: &Mutex<WatchdogState>,
    scan_interval_ms: u64,
    stale_execution_ms: u64,
) {
    if let Ok(mut state) = state.lock() {
        state.policy = WatchdogPolicySnapshot {
            enabled: scan_interval_ms > 0 && stale_execution_ms > 0,
            scan_interval_ms,
            stale_execution_ms,
            monitored_scope: "job_bound_execution".to_string(),
        };
    }
}

pub(crate) fn begin_execution(
    request_id: String,
    job_id: Option<String>,
    method: String,
) -> ExecutionGuard {
    begin_execution_in(watchdog_state(), request_id, job_id, method)
}

fn begin_execution_in(
    state: &Mutex<WatchdogState>,
    request_id: String,
    job_id: Option<String>,
    method: String,
) -> ExecutionGuard {
    begin_execution_at(state, request_id, job_id, method, unix_now_ms())
}

fn begin_execution_at(
    state: &Mutex<WatchdogState>,
    request_id: String,
    job_id: Option<String>,
    method: String,
    now: u128,
) -> ExecutionGuard {
    if let Ok(mut state) = state.lock() {
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
    complete_execution_in(watchdog_state(), guard);
}

pub(crate) fn mark_progress(request_id: &str) -> bool {
    mark_progress_at(watchdog_state(), request_id, unix_now_ms())
}

fn mark_progress_at(state: &Mutex<WatchdogState>, request_id: &str, now: u128) -> bool {
    if let Ok(mut state) = state.lock()
        && let Some(record) = state.active.get_mut(request_id)
    {
        record.last_progress_unix_ms = now;
        return true;
    }
    false
}

fn complete_execution_in(state: &Mutex<WatchdogState>, guard: ExecutionGuard) {
    if let Ok(mut state) = state.lock() {
        state.active.remove(&guard.request_id);
    }
}

pub(crate) fn fail_execution(
    guard: ExecutionGuard,
    reason_code: &str,
    message: impl Into<String>,
) -> FailureReport {
    fail_execution_in(watchdog_state(), guard, reason_code, message)
}

fn fail_execution_in(
    state: &Mutex<WatchdogState>,
    guard: ExecutionGuard,
    reason_code: &str,
    message: impl Into<String>,
) -> FailureReport {
    let message = message.into();
    let now = unix_now_ms();

    if let Ok(mut state) = state.lock() {
        if let Some(record) = state.active.remove(&guard.request_id) {
            let report = failure_from_record(record, reason_code, message, now);
            retain_failure(&mut state, report.clone());
            return report;
        }
        if let Some(existing) = state
            .recent_failures
            .iter()
            .find(|failure| failure.request_id == guard.request_id)
        {
            return existing.clone();
        }

        let report = failure_from_record(
            ExecutionRecord {
                request_id: guard.request_id,
                job_id: None,
                method: "unknown".to_string(),
                started_unix_ms: now,
                last_progress_unix_ms: now,
            },
            reason_code,
            message,
            now,
        );
        retain_failure(&mut state, report.clone());
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
    }
}

fn failure_from_record(
    record: ExecutionRecord,
    reason_code: &str,
    message: String,
    now: u128,
) -> FailureReport {
    FailureReport {
        request_id: record.request_id,
        job_id: record.job_id,
        method: record.method,
        reason_code: reason_code.to_string(),
        message,
        elapsed_ms: now.saturating_sub(record.started_unix_ms),
        occurred_unix_ms: now,
    }
}

fn retain_failure(state: &mut WatchdogState, report: FailureReport) {
    state.recent_failures.push_front(report);
    while state.recent_failures.len() > RECENT_FAILURE_LIMIT {
        state.recent_failures.pop_back();
    }
}

pub(crate) fn scan_stale_executions() -> Vec<FailureReport> {
    scan_stale_executions_at(watchdog_state(), unix_now_ms())
}

fn scan_stale_executions_at(state: &Mutex<WatchdogState>, now: u128) -> Vec<FailureReport> {
    let Ok(mut state) = state.lock() else {
        return vec![];
    };
    if !state.policy.enabled {
        return vec![];
    }

    let stale_after_ms = u128::from(state.policy.stale_execution_ms);
    let stale_request_ids = state
        .active
        .iter()
        .filter(|(_, record)| {
            record.job_id.is_some()
                && now.saturating_sub(record.last_progress_unix_ms) >= stale_after_ms
        })
        .map(|(request_id, _)| request_id.clone())
        .collect::<Vec<_>>();

    let mut reports = Vec::with_capacity(stale_request_ids.len());
    for request_id in stale_request_ids {
        let Some(record) = state.active.remove(&request_id) else {
            continue;
        };
        let idle_ms = now.saturating_sub(record.last_progress_unix_ms);
        let report = failure_from_record(
            record,
            "watchdog_timeout",
            format!(
                "agent watchdog detected {idle_ms} ms without progress; stale budget is {stale_after_ms} ms"
            ),
            now,
        );
        retain_failure(&mut state, report.clone());
        reports.push(report);
    }
    reports
}

pub(crate) fn snapshot() -> WatchdogSnapshot {
    snapshot_from(watchdog_state())
}

fn snapshot_from(state: &Mutex<WatchdogState>) -> WatchdogSnapshot {
    snapshot_from_at(state, unix_now_ms())
}

fn snapshot_from_at(state: &Mutex<WatchdogState>, now: u128) -> WatchdogSnapshot {
    if let Ok(state) = state.lock() {
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
            policy: state.policy.clone(),
            active_execution_count: state.active.len(),
            recent_failure_count: state.recent_failures.len(),
            active_executions,
            recent_failures: state.recent_failures.iter().cloned().collect(),
        }
    } else {
        WatchdogSnapshot {
            state: "unknown".to_string(),
            policy: WatchdogPolicySnapshot::default(),
            active_execution_count: 0,
            recent_failure_count: 0,
            active_executions: vec![],
            recent_failures: vec![],
        }
    }
}

#[allow(dead_code)]
pub fn run_fault_injection_probe() -> Result<Value, String> {
    let probe_state = Mutex::new(WatchdogState::default());

    let failed_guard = begin_execution_in(
        &probe_state,
        "watchdog-injected-failure".to_string(),
        Some("watchdog-job-failure".to_string()),
        "solve_bar_1d".to_string(),
    );
    let failure = fail_execution_in(
        &probe_state,
        failed_guard,
        "invalid_params",
        "injected invalid solver parameters",
    );
    let after_failure = snapshot_from(&probe_state);

    let healthy_guard = begin_execution_in(
        &probe_state,
        "watchdog-healthy-after".to_string(),
        Some("watchdog-job-healthy".to_string()),
        "solve_bar_1d".to_string(),
    );
    complete_execution_in(&probe_state, healthy_guard);
    let after_healthy = snapshot_from(&probe_state);

    let mut observations = json!({
        "failure_recorded": failure.reason_code == "invalid_params",
        "failure_request_id": failure.request_id,
        "failure_job_id": failure.job_id,
        "failure_method": failure.method,
        "failure_reason_code": failure.reason_code,
        "watchdog_state_after_failure": after_failure.state,
        "slot_released_after_failure": after_failure.active_execution_count == 0,
        "recent_failure_count_after_failure": after_failure.recent_failure_count,
        "healthy_execution_completed": true,
        "slot_released_after_healthy": after_healthy.active_execution_count == 0,
        "recent_failure_retained": after_healthy.recent_failures.iter().any(|entry| {
            entry.request_id == "watchdog-injected-failure"
                && entry.reason_code == "invalid_params"
        }),
        "new_failure_after_healthy": after_healthy.recent_failure_count
            != after_failure.recent_failure_count
    });
    let recovery_invariants_hold = observations
        .get("failure_recorded")
        .and_then(Value::as_bool)
        == Some(true)
        && observations
            .get("slot_released_after_failure")
            .and_then(Value::as_bool)
            == Some(true)
        && observations
            .get("slot_released_after_healthy")
            .and_then(Value::as_bool)
            == Some(true)
        && observations
            .get("recent_failure_retained")
            .and_then(Value::as_bool)
            == Some(true)
        && observations
            .get("new_failure_after_healthy")
            .and_then(Value::as_bool)
            == Some(false);

    reset_state(&probe_state);
    let after_cleanup = snapshot_from(&probe_state);
    observations["probe_cleanup_completed"] =
        json!(after_cleanup.active_execution_count == 0 && after_cleanup.recent_failure_count == 0);

    if !recovery_invariants_hold
        || observations
            .get("probe_cleanup_completed")
            .and_then(Value::as_bool)
            != Some(true)
    {
        return Err("agent watchdog fault injection violated recovery invariants".to_string());
    }
    Ok(observations)
}

#[allow(dead_code)]
pub fn run_timeout_fault_injection_probe() -> Result<Value, String> {
    let probe_state = Mutex::new(WatchdogState::default());
    configure_policy_in(&probe_state, 10, 100);

    let timed_guard = begin_execution_at(
        &probe_state,
        "watchdog-stale-request".to_string(),
        Some("watchdog-stale-job".to_string()),
        "solve_heat_bar_1d".to_string(),
        1_000,
    );
    let progress_refreshed = mark_progress_at(&probe_state, "watchdog-stale-request", 1_050);
    let before_budget = scan_stale_executions_at(&probe_state, 1_149);
    let timed_out = scan_stale_executions_at(&probe_state, 1_150);
    let after_timeout = snapshot_from_at(&probe_state, 1_150);
    let timeout = timed_out.first().cloned();

    let late_failure = fail_execution_in(
        &probe_state,
        timed_guard,
        "cancelled",
        "late solver completion observed cancellation",
    );
    let after_late_failure = snapshot_from_at(&probe_state, 1_151);

    let healthy_guard = begin_execution_at(
        &probe_state,
        "watchdog-timeout-follow-up".to_string(),
        Some("watchdog-timeout-follow-up-job".to_string()),
        "solve_heat_bar_1d".to_string(),
        1_200,
    );
    complete_execution_in(&probe_state, healthy_guard);
    let after_healthy = snapshot_from_at(&probe_state, 1_201);

    let mut observations = json!({
        "policy_enabled": after_timeout.policy.enabled,
        "stale_execution_ms": after_timeout.policy.stale_execution_ms,
        "progress_refreshed": progress_refreshed,
        "expired_before_budget": !before_budget.is_empty(),
        "timeout_count": timed_out.len(),
        "timeout_reason_code": timeout.as_ref().map(|report| report.reason_code.as_str()),
        "timeout_job_id": timeout.as_ref().and_then(|report| report.job_id.as_deref()),
        "timeout_method": timeout.as_ref().map(|report| report.method.as_str()),
        "timeout_elapsed_ms": timeout.as_ref().map(|report| report.elapsed_ms),
        "timeout_message_has_budget": timeout.as_ref().is_some_and(|report| {
            report.message.contains("stale budget is 100 ms")
        }),
        "slot_released_after_timeout": after_timeout.active_execution_count == 0,
        "timeout_failure_recorded": after_timeout.recent_failure_count == 1,
        "late_failure_reused_timeout": late_failure.reason_code == "watchdog_timeout",
        "duplicate_failure_created": after_late_failure.recent_failure_count != 1,
        "healthy_follow_up_completed": after_healthy.active_execution_count == 0,
        "timeout_reason_retained": after_healthy.recent_failures.iter().any(|report| {
            report.request_id == "watchdog-stale-request"
                && report.reason_code == "watchdog_timeout"
        })
    });
    let timeout_invariants_hold = observations.get("policy_enabled").and_then(Value::as_bool)
        == Some(true)
        && observations
            .get("progress_refreshed")
            .and_then(Value::as_bool)
            == Some(true)
        && observations
            .get("expired_before_budget")
            .and_then(Value::as_bool)
            == Some(false)
        && observations.get("timeout_count").and_then(Value::as_u64) == Some(1)
        && observations
            .get("slot_released_after_timeout")
            .and_then(Value::as_bool)
            == Some(true)
        && observations
            .get("late_failure_reused_timeout")
            .and_then(Value::as_bool)
            == Some(true)
        && observations
            .get("duplicate_failure_created")
            .and_then(Value::as_bool)
            == Some(false)
        && observations
            .get("healthy_follow_up_completed")
            .and_then(Value::as_bool)
            == Some(true);

    reset_state(&probe_state);
    let after_cleanup = snapshot_from(&probe_state);
    observations["probe_cleanup_completed"] =
        json!(after_cleanup.active_execution_count == 0 && after_cleanup.recent_failure_count == 0);
    if !timeout_invariants_hold
        || observations
            .get("probe_cleanup_completed")
            .and_then(Value::as_bool)
            != Some(true)
    {
        return Err("agent watchdog timeout injection violated recovery invariants".to_string());
    }
    Ok(observations)
}

#[cfg(test)]
#[allow(dead_code)]
pub(crate) fn reset_for_tests() {
    reset_state(watchdog_state());
}

fn reset_state(state: &Mutex<WatchdogState>) {
    if let Ok(mut state) = state.lock() {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fault_injection_releases_slot_and_preserves_reason() {
        let report = run_fault_injection_probe().expect("watchdog probe should pass");
        assert_eq!(report["failure_reason_code"], "invalid_params");
        assert_eq!(report["slot_released_after_failure"], true);
        assert_eq!(report["slot_released_after_healthy"], true);
        assert_eq!(report["new_failure_after_healthy"], false);
        assert_eq!(report["probe_cleanup_completed"], true);
    }

    #[test]
    fn timeout_injection_releases_slot_and_deduplicates_late_failure() {
        let report =
            run_timeout_fault_injection_probe().expect("watchdog timeout probe should pass");
        assert_eq!(report["timeout_reason_code"], "watchdog_timeout");
        assert_eq!(report["expired_before_budget"], false);
        assert_eq!(report["slot_released_after_timeout"], true);
        assert_eq!(report["late_failure_reused_timeout"], true);
        assert_eq!(report["duplicate_failure_created"], false);
    }
}
