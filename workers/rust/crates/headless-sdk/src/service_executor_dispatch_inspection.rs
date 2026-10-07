use crate::HeadlessExecutorError;
use crate::service_executor::request_json_with_timeout;
use serde_json::{Value, json};
use std::collections::HashSet;
use std::time::{Duration, Instant};

const SCHEMA: &str = "kyuubiki.operator-task-dispatch-inspection/v1";
const TIMEOUT: Duration = Duration::from_secs(20);

pub(crate) fn inspect(
    base_url: &str,
    token: Option<&str>,
    task_id: &str,
    task_digest: &str,
) -> Result<Value, HeadlessExecutorError> {
    if task_id.is_empty()
        || task_id.len() > 1024
        || task_digest.len() != 64
        || !task_digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(invalid("query identity"));
    }
    let receipt = request_json_with_timeout(
        base_url,
        token,
        "POST",
        "/api/v1/operator-tasks/inspect-dispatch",
        Some(json!({"task_id":task_id,"task_digest":task_digest})),
        Some(Instant::now() + TIMEOUT),
        TIMEOUT,
    )?;
    validate(&receipt, task_id, task_digest)?;
    Ok(receipt)
}

fn validate(
    receipt: &Value,
    task_id: &str,
    task_digest: &str,
) -> Result<(), HeadlessExecutorError> {
    if receipt["schema_version"] != SCHEMA
        || receipt["task_id"] != task_id
        || receipt["task_digest"] != task_digest
        || receipt["automatic_replay_authorized"] != false
        || receipt["terminal_result_available"] != false
    {
        return Err(invalid("identity or recovery authority"));
    }
    let attempts = receipt["attempts"]
        .as_array()
        .ok_or_else(|| invalid("attempts"))?;
    let count = receipt["retained_attempt_count"]
        .as_u64()
        .ok_or_else(|| invalid("retained_attempt_count"))?;
    if count > 512
        || attempts.len() != (count as usize).min(128)
        || receipt["truncated"] != (count > 128)
        || receipt["status"]
            != if count == 0 {
                "no_retained_dispatch"
            } else {
                "dispatch_records_found"
            }
    {
        return Err(invalid("coverage"));
    }
    let mut ids = HashSet::new();
    for attempt in attempts {
        let state = attempt["state"].as_str();
        let terminal = matches!(
            state,
            Some("observed_executed" | "observed_failed" | "observed_blocked" | "not_dispatched")
        );
        if !terminal
            && !matches!(
                state,
                Some("dispatch_boundary_unconfirmed" | "outcome_unknown")
            )
        {
            return Err(invalid("attempt state"));
        }
        for (key, limit) in [("attempt_id", 32), ("request_id", 1024)] {
            if attempt[key]
                .as_str()
                .is_none_or(|value| value.is_empty() || value.len() > limit)
            {
                return Err(invalid("attempt identity"));
            }
        }
        let id = attempt["attempt_id"]
            .as_str()
            .ok_or_else(|| invalid("attempt identity"))?;
        if id.len() != 32
            || !id
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            || !ids.insert(id)
        {
            return Err(invalid("duplicate or malformed attempt identity"));
        }
        let created = attempt["created_at_ms"]
            .as_u64()
            .ok_or_else(|| invalid("created_at_ms"))?;
        if attempt["updated_at_ms"]
            .as_u64()
            .is_none_or(|updated| updated < created)
        {
            return Err(invalid("updated_at_ms"));
        }
        let observation = &attempt["observation"];
        if terminal {
            if observation["status"] != "retained_dispatch_observation"
                || observation["outcome"] != attempt["state"]
            {
                return Err(invalid("terminal dispatch observation"));
            }
        } else if observation["outcome"] != "unknown"
            || !matches!(
                observation["status"].as_str(),
                Some(
                    "original_endpoint_not_configured"
                        | "probe_limit_reached"
                        | "original_endpoint_reports_active_request"
                        | "request_not_observed_active"
                        | "agent_observation_invalid"
                        | "original_endpoint_unreachable"
                )
            )
        {
            return Err(invalid("nonterminal dispatch observation"));
        }
        if observation["status"] == "original_endpoint_reports_active_request"
            && (observation["generation"]
                .as_u64()
                .is_none_or(|value| value == 0)
                || observation["cancel_requested"].as_bool().is_none()
                || observation["process_instance_id"]
                    .as_str()
                    .is_none_or(|value| {
                        value.is_empty() || value.len() > 256 || value == "unavailable"
                    }))
        {
            return Err(invalid("active request observation"));
        }
        if !observation["execution_target"].is_null() {
            let target: crate::CancelExecutionRequest =
                serde_json::from_value(observation["execution_target"].clone())
                    .map_err(|_| invalid("execution cancellation target"))?;
            target
                .validate()
                .map_err(|_| invalid("execution cancellation target"))?;
            if observation["status"] != "original_endpoint_reports_active_request"
                || target.job_id != task_id
                || observation["process_instance_id"] != target.process_instance_id
                || observation["generation"] != target.generation
                || attempt["request_id"] != target.request_id
            {
                return Err(invalid("execution cancellation target mirrors"));
            }
        }
    }
    Ok(())
}

fn invalid(field: &str) -> HeadlessExecutorError {
    HeadlessExecutorError {
        message: format!("invalid operator task dispatch inspection: {field}"),
    }
}

#[cfg(test)]
#[path = "service_executor_dispatch_inspection_tests.rs"]
mod tests;
