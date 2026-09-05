use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use kyuubiki_protocol::{
    AGENT_DRAIN_CONTROLLER_ID_MAX_BYTES, AGENT_DRAIN_REASON_MAX_BYTES, AGENT_LIFECYCLE_SCHEMA,
    AgentLifecycleDescriptor,
};
use serde::Serialize;

use crate::agent_watchdog;

pub(crate) use crate::agent_watchdog::{ExecutionAdmissionError, FailureReport};

#[derive(Debug, Clone)]
struct DrainLease {
    generation: u64,
    owner_id: String,
    reason: String,
    started_unix_ms: u128,
}

#[derive(Debug)]
struct LifecycleState {
    process_instance_id: String,
    next_drain_generation: u64,
    active_execution_count: usize,
    drain: Option<DrainLease>,
    last_resumed: Option<(u64, String)>,
}

impl Default for LifecycleState {
    fn default() -> Self {
        Self {
            process_instance_id: next_process_instance_id(),
            next_drain_generation: 0,
            active_execution_count: 0,
            drain: None,
            last_resumed: None,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ExecutionGuard {
    watchdog: agent_watchdog::ExecutionGuard,
    _lifecycle_lease: Arc<ExecutionLease>,
}

#[derive(Debug)]
struct ExecutionLease {
    state: Arc<Mutex<LifecycleState>>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct LifecycleControlError {
    pub(crate) code: String,
    pub(crate) message: String,
    pub(crate) lifecycle: Box<AgentLifecycleDescriptor>,
}

pub(crate) fn begin_execution(
    request_id: String,
    job_id: Option<String>,
    method: String,
) -> Result<ExecutionGuard, ExecutionAdmissionError> {
    let state = lifecycle_state();
    let lifecycle_lease = acquire_execution_slot(&state, &request_id)?;
    let watchdog = match agent_watchdog::begin_execution(request_id, job_id, method) {
        Ok(guard) => guard,
        Err(error) => {
            drop(lifecycle_lease);
            return Err(error);
        }
    };
    Ok(ExecutionGuard {
        watchdog,
        _lifecycle_lease: lifecycle_lease,
    })
}

pub(crate) fn complete_execution(guard: ExecutionGuard) {
    agent_watchdog::complete_execution(guard.watchdog.clone());
    drop(guard);
}

pub(crate) fn fail_execution(
    guard: ExecutionGuard,
    reason_code: &str,
    message: impl Into<String>,
) -> FailureReport {
    let report =
        agent_watchdog::fail_execution(guard.watchdog.clone(), reason_code, message.into());
    drop(guard);
    report
}

pub(crate) fn mark_progress(guard: &ExecutionGuard) -> bool {
    agent_watchdog::mark_progress(&guard.watchdog)
}

pub(crate) fn begin_drain(
    controller_id: &str,
    reason: &str,
) -> Result<AgentLifecycleDescriptor, LifecycleControlError> {
    begin_drain_in(&lifecycle_state(), controller_id, reason)
}

pub(crate) fn resume_admission(
    controller_id: &str,
    drain_generation: u64,
) -> Result<AgentLifecycleDescriptor, LifecycleControlError> {
    resume_admission_in(&lifecycle_state(), controller_id, drain_generation)
}

pub(crate) fn snapshot() -> AgentLifecycleDescriptor {
    snapshot_in(&lifecycle_state())
}

fn acquire_execution_slot(
    state: &Arc<Mutex<LifecycleState>>,
    request_id: &str,
) -> Result<Arc<ExecutionLease>, ExecutionAdmissionError> {
    let mut lifecycle = state.lock().map_err(|_| ExecutionAdmissionError {
        request_id: request_id.to_string(),
        reason_code: "agent_lifecycle_unavailable".to_string(),
        message: "agent lifecycle state is unavailable".to_string(),
    })?;
    if let Some(drain) = lifecycle.drain.as_ref() {
        return Err(ExecutionAdmissionError {
            request_id: request_id.to_string(),
            reason_code: "agent_draining".to_string(),
            message: format!(
                "agent drain generation {} is not accepting new execution",
                drain.generation
            ),
        });
    }
    lifecycle.active_execution_count =
        lifecycle
            .active_execution_count
            .checked_add(1)
            .ok_or_else(|| ExecutionAdmissionError {
                request_id: request_id.to_string(),
                reason_code: "agent_execution_count_exhausted".to_string(),
                message: "agent lifecycle execution count is exhausted".to_string(),
            })?;
    drop(lifecycle);
    Ok(Arc::new(ExecutionLease {
        state: Arc::clone(state),
    }))
}

fn begin_drain_in(
    state: &Arc<Mutex<LifecycleState>>,
    controller_id: &str,
    reason: &str,
) -> Result<AgentLifecycleDescriptor, LifecycleControlError> {
    let controller_id = validate_controller_id(controller_id)
        .map_err(|message| control_error("invalid_controller_id", message, state))?;
    let reason = validate_reason(reason)
        .map_err(|message| control_error("invalid_drain_reason", message, state))?;
    let mut lifecycle = state.lock().map_err(|_| LifecycleControlError {
        code: "agent_lifecycle_unavailable".to_string(),
        message: "agent lifecycle state is unavailable".to_string(),
        lifecycle: Box::new(unavailable_snapshot()),
    })?;

    if let Some(drain) = lifecycle.drain.as_ref() {
        if drain.owner_id == controller_id {
            return Ok(snapshot_locked(&lifecycle));
        }
        return Err(LifecycleControlError {
            code: "agent_drain_owned".to_string(),
            message: format!(
                "agent drain generation {} is owned by another controller",
                drain.generation
            ),
            lifecycle: Box::new(snapshot_locked(&lifecycle)),
        });
    }

    let generation = lifecycle
        .next_drain_generation
        .checked_add(1)
        .ok_or_else(|| LifecycleControlError {
            code: "agent_drain_generation_exhausted".to_string(),
            message: "agent drain generation is exhausted".to_string(),
            lifecycle: Box::new(snapshot_locked(&lifecycle)),
        })?;
    lifecycle.next_drain_generation = generation;
    lifecycle.last_resumed = None;
    lifecycle.drain = Some(DrainLease {
        generation,
        owner_id: controller_id,
        reason,
        started_unix_ms: unix_now_ms(),
    });
    Ok(snapshot_locked(&lifecycle))
}

fn resume_admission_in(
    state: &Arc<Mutex<LifecycleState>>,
    controller_id: &str,
    drain_generation: u64,
) -> Result<AgentLifecycleDescriptor, LifecycleControlError> {
    let controller_id = validate_controller_id(controller_id)
        .map_err(|message| control_error("invalid_controller_id", message, state))?;
    let mut lifecycle = state.lock().map_err(|_| LifecycleControlError {
        code: "agent_lifecycle_unavailable".to_string(),
        message: "agent lifecycle state is unavailable".to_string(),
        lifecycle: Box::new(unavailable_snapshot()),
    })?;

    let Some(drain) = lifecycle.drain.as_ref() else {
        if lifecycle
            .last_resumed
            .as_ref()
            .is_some_and(|(generation, owner)| {
                *generation == drain_generation && owner == &controller_id
            })
        {
            return Ok(snapshot_locked(&lifecycle));
        }
        return Err(LifecycleControlError {
            code: "agent_not_draining".to_string(),
            message: "agent is not draining for this controller generation".to_string(),
            lifecycle: Box::new(snapshot_locked(&lifecycle)),
        });
    };

    if drain.generation != drain_generation || drain.owner_id != controller_id {
        return Err(LifecycleControlError {
            code: "stale_agent_drain_generation".to_string(),
            message: "drain owner or generation does not match the active lease".to_string(),
            lifecycle: Box::new(snapshot_locked(&lifecycle)),
        });
    }

    lifecycle.last_resumed = Some((drain.generation, drain.owner_id.clone()));
    lifecycle.drain = None;
    Ok(snapshot_locked(&lifecycle))
}

fn snapshot_in(state: &Arc<Mutex<LifecycleState>>) -> AgentLifecycleDescriptor {
    state
        .lock()
        .map(|state| snapshot_locked(&state))
        .unwrap_or_else(|_| unavailable_snapshot())
}

fn snapshot_locked(state: &LifecycleState) -> AgentLifecycleDescriptor {
    let quiescent = state.drain.is_some() && state.active_execution_count == 0;
    let lifecycle_state = match (&state.drain, quiescent) {
        (None, _) => "accepting",
        (Some(_), false) => "draining",
        (Some(_), true) => "quiescent",
    };
    AgentLifecycleDescriptor {
        schema_version: AGENT_LIFECYCLE_SCHEMA.to_string(),
        process_instance_id: state.process_instance_id.clone(),
        mutation_control_scope: "host_loopback".to_string(),
        state: lifecycle_state.to_string(),
        drain_generation: state.next_drain_generation,
        accepting_new_work: state.drain.is_none(),
        active_execution_count: state.active_execution_count,
        quiescent,
        safe_to_replace: quiescent,
        drain_owner_id: state.drain.as_ref().map(|drain| drain.owner_id.clone()),
        drain_reason: state.drain.as_ref().map(|drain| drain.reason.clone()),
        drain_started_unix_ms: state.drain.as_ref().map(|drain| drain.started_unix_ms),
    }
}

fn unavailable_snapshot() -> AgentLifecycleDescriptor {
    AgentLifecycleDescriptor {
        process_instance_id: "unavailable".to_string(),
        state: "unavailable".to_string(),
        accepting_new_work: false,
        ..AgentLifecycleDescriptor::default()
    }
}

fn control_error(
    code: &str,
    message: String,
    state: &Arc<Mutex<LifecycleState>>,
) -> LifecycleControlError {
    LifecycleControlError {
        code: code.to_string(),
        message,
        lifecycle: Box::new(snapshot_in(state)),
    }
}

fn validate_controller_id(value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() || value.len() > AGENT_DRAIN_CONTROLLER_ID_MAX_BYTES {
        return Err(format!(
            "controller_id must contain 1..={AGENT_DRAIN_CONTROLLER_ID_MAX_BYTES} bytes"
        ));
    }
    if !value.bytes().all(|byte| {
        byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'/' | b'-')
    }) {
        return Err("controller_id contains unsupported characters".to_string());
    }
    Ok(value.to_string())
}

fn validate_reason(value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() || value.len() > AGENT_DRAIN_REASON_MAX_BYTES {
        return Err(format!(
            "reason must contain 1..={AGENT_DRAIN_REASON_MAX_BYTES} bytes"
        ));
    }
    if value.chars().any(char::is_control) {
        return Err("reason must not contain control characters".to_string());
    }
    Ok(value.to_string())
}

fn lifecycle_state() -> Arc<Mutex<LifecycleState>> {
    static STATE: OnceLock<Arc<Mutex<LifecycleState>>> = OnceLock::new();
    Arc::clone(STATE.get_or_init(|| Arc::new(Mutex::new(LifecycleState::default()))))
}

fn next_process_instance_id() -> String {
    static NEXT_INSTANCE: AtomicU64 = AtomicU64::new(1);
    format!(
        "agent-instance-{}-{}-{}",
        std::process::id(),
        unix_now_ms(),
        NEXT_INSTANCE.fetch_add(1, Ordering::Relaxed)
    )
}

fn unix_now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}

impl Drop for ExecutionLease {
    fn drop(&mut self) {
        if let Ok(mut state) = self.state.lock() {
            state.active_execution_count = state.active_execution_count.saturating_sub(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drain_preserves_existing_slots_and_rejects_new_work() {
        let state = Arc::new(Mutex::new(LifecycleState::default()));
        let existing = acquire_execution_slot(&state, "existing").expect("existing slot");
        let draining =
            begin_drain_in(&state, "installer-1", "rolling replacement").expect("begin drain");
        assert_eq!(draining.state, "draining");
        assert_eq!(draining.active_execution_count, 1);
        assert!(!draining.safe_to_replace);

        let rejected = acquire_execution_slot(&state, "new-work").expect_err("reject new work");
        assert_eq!(rejected.reason_code, "agent_draining");
        drop(existing);

        let quiescent = snapshot_in(&state);
        assert_eq!(quiescent.state, "quiescent");
        assert!(quiescent.safe_to_replace);
        let resumed = resume_admission_in(&state, "installer-1", draining.drain_generation)
            .expect("resume admission");
        assert_eq!(resumed.state, "accepting");
        assert!(resumed.accepting_new_work);
    }

    #[test]
    fn drain_owner_and_generation_fence_stale_controllers() {
        let state = Arc::new(Mutex::new(LifecycleState::default()));
        let first = begin_drain_in(&state, "installer-1", "upgrade").expect("first drain");
        let retry = begin_drain_in(&state, "installer-1", "retry").expect("idempotent retry");
        assert_eq!(retry.drain_generation, first.drain_generation);
        assert_eq!(
            begin_drain_in(&state, "installer-2", "competing upgrade")
                .expect_err("competing owner")
                .code,
            "agent_drain_owned"
        );
        assert_eq!(
            resume_admission_in(&state, "installer-2", first.drain_generation)
                .expect_err("wrong owner")
                .code,
            "stale_agent_drain_generation"
        );

        resume_admission_in(&state, "installer-1", first.drain_generation)
            .expect("matching owner resumes");
        resume_admission_in(&state, "installer-1", first.drain_generation)
            .expect("resume retry is idempotent");
        let second =
            begin_drain_in(&state, "installer-1", "next upgrade").expect("next drain generation");
        assert!(second.drain_generation > first.drain_generation);
        assert_eq!(
            resume_admission_in(&state, "installer-1", first.drain_generation)
                .expect_err("stale generation")
                .code,
            "stale_agent_drain_generation"
        );
    }
}
