use super::{ServiceHeadlessExecutor, request_json_with_timeout};
use crate::{CancelExecutionRequest, HeadlessExecutorError};
use serde_json::{Value, json};
use std::time::{Duration, Instant};

const SCHEMA: &str = "kyuubiki.operator-task-dispatch-cancellation/v1";
const TIMEOUT: Duration = Duration::from_secs(10);
const FIELDS: &[&str] = &[
    "schema_version",
    "task_id",
    "task_digest",
    "attempt_id",
    "execution_target",
    "status",
    "cancel_registered",
    "delivery_attempted",
    "agent_acknowledgement",
    "uncertainty_reason",
    "execution_terminal_confirmed",
    "automatic_replay_authorized",
    "journal_mutation_performed",
    "publication_performed",
    "job_wide_fallback_performed",
];
const AGENT_FIELDS: &[&str] = &[
    "schema_version",
    "execution_target",
    "status",
    "cancel_registered",
    "execution_terminal_confirmed",
    "pending_cancellation_created",
    "operator_package_cleanup_performed",
    "automatic_replay_authorized",
];

impl ServiceHeadlessExecutor {
    /// Request cancellation of an explicitly observed generation through its original endpoint.
    /// Inspect the returned status: registration is not termination; unknown never grants replay.
    pub fn cancel_operator_task_dispatch(
        &self,
        task_id: &str,
        task_digest: &str,
        attempt_id: &str,
        target: &CancelExecutionRequest,
    ) -> Result<Value, HeadlessExecutorError> {
        target.validate().map_err(|_| invalid("target identity"))?;
        if target.job_id != task_id
            || target.process_instance_id == "unavailable"
            || !hex(task_digest, 64)
            || !hex(attempt_id, 32)
        {
            return Err(invalid("query identity"));
        }
        let query = json!({"task_id":task_id, "task_digest":task_digest,
            "attempt_id":attempt_id, "execution_target":target});
        let receipt = request_json_with_timeout(
            &self.base_url, self.api_token.as_deref(), "POST",
            "/api/v1/operator-tasks/cancel-dispatch", Some(query.clone()),
            Some(Instant::now() + TIMEOUT), TIMEOUT,
        ).map_err(|error| HeadlessExecutorError {
            message: format!("cancellation request failed; registration is unconfirmed and automatic retry is not authorized: {}", error.message),
        })?;
        validate(&receipt, &query)?;
        Ok(receipt)
    }
}

fn hex(value: &str, size: usize) -> bool {
    value.len() == size
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn exact_fields(value: &Value, fields: &[&str]) -> bool {
    value.as_object().is_some_and(|map| {
        map.len() == fields.len() && fields.iter().all(|field| map.contains_key(*field))
    })
}

fn validate(receipt: &Value, query: &Value) -> Result<(), HeadlessExecutorError> {
    if !exact_fields(receipt, FIELDS)
        || receipt["schema_version"] != SCHEMA
        || ["task_id", "task_digest", "attempt_id", "execution_target"]
            .iter()
            .any(|field| receipt[field] != query[field])
        || [
            "execution_terminal_confirmed",
            "automatic_replay_authorized",
            "journal_mutation_performed",
            "publication_performed",
            "job_wide_fallback_performed",
        ]
        .iter()
        .any(|field| receipt[field] != false)
    {
        return Err(invalid("identity or authority"));
    }
    match receipt["status"].as_str() {
        Some("requested" | "target_not_observed") => {
            let registered = receipt["status"] == "requested";
            let ack = &receipt["agent_acknowledgement"];
            if receipt["cancel_registered"] != registered
                || receipt["delivery_attempted"] != true
                || !receipt["uncertainty_reason"].is_null()
                || !exact_fields(ack, AGENT_FIELDS)
                || ack["schema_version"] != kyuubiki_protocol::AGENT_EXECUTION_CANCELLATION_SCHEMA
                || ack["execution_target"] != query["execution_target"]
                || ack["status"] != receipt["status"]
                || ack["cancel_registered"] != registered
                || [
                    "execution_terminal_confirmed",
                    "pending_cancellation_created",
                    "operator_package_cleanup_performed",
                    "automatic_replay_authorized",
                ]
                .iter()
                .any(|field| ack[field] != false)
            {
                return Err(invalid("Agent acknowledgement"));
            }
        }
        Some(
            "no_retained_dispatch"
            | "not_dispatched"
            | "retained_terminal_dispatch"
            | "original_endpoint_not_configured",
        ) => {
            if receipt["cancel_registered"] != false
                || receipt["delivery_attempted"] != false
                || !receipt["agent_acknowledgement"].is_null()
                || !receipt["uncertainty_reason"].is_null()
            {
                return Err(invalid("undelivered cancellation"));
            }
        }
        Some("cancellation_outcome_unknown") => {
            if !receipt["cancel_registered"].is_null()
                || receipt["delivery_attempted"] != true
                || !receipt["agent_acknowledgement"].is_null()
                || !matches!(
                    receipt["uncertainty_reason"].as_str(),
                    Some("agent_acknowledgement_invalid" | "original_endpoint_unreachable")
                )
            {
                return Err(invalid("uncertain cancellation"));
            }
        }
        _ => return Err(invalid("status")),
    }
    Ok(())
}

fn invalid(field: &str) -> HeadlessExecutorError {
    HeadlessExecutorError {
        message: format!(
            "invalid operator task dispatch cancellation: {field}; cancellation is not terminal proof or replay authority"
        ),
    }
}

#[cfg(test)]
#[path = "service_executor_dispatch_cancellation_tests.rs"]
mod tests;
