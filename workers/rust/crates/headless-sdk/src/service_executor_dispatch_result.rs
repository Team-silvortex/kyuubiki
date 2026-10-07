use crate::HeadlessExecutorError;
use crate::service_executor::request_json_with_timeout;
use crate::service_executor_operator_task::operator_task_execution_outcome;
use serde_json::{Value, json};
use std::time::{Duration, Instant};

const TIMEOUT: Duration = Duration::from_secs(15);
const REQUIRED_FIELDS: &[&str] = &[
    "schema_version",
    "task_id",
    "task_digest",
    "attempt_id",
    "status",
    "outcome",
    "completion",
    "automatic_replay_authorized",
    "publication_performed",
];
const INCARNATION_FIELDS: &[&str] = &["request_id", "process_instance_id", "generation"];

pub(crate) fn fetch(
    base_url: &str,
    token: Option<&str>,
    task: &Value,
    attempt_id: &str,
) -> Result<Value, HeadlessExecutorError> {
    if attempt_id.len() != 32
        || !attempt_id
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(invalid("attempt identity"));
    }
    let prepared = crate::prepare_operator_task_payload(&json!({"task":task}))
        .map_err(|message| HeadlessExecutorError { message })?;
    let result = request_json_with_timeout(
        base_url,
        token,
        "POST",
        "/api/v1/operator-tasks/fetch-dispatch-result",
        Some(
            json!({"task_id":prepared["task_id"], "task_digest":prepared["task_digest"], "attempt_id":attempt_id}),
        ),
        Some(Instant::now() + TIMEOUT),
        TIMEOUT,
    )?;
    validate(&result, &prepared, attempt_id)?;
    Ok(result)
}

fn validate(
    result: &Value,
    prepared: &Value,
    attempt_id: &str,
) -> Result<(), HeadlessExecutorError> {
    let fields = result
        .as_object()
        .ok_or_else(|| invalid("receipt fields"))?;
    if REQUIRED_FIELDS
        .iter()
        .any(|field| !fields.contains_key(*field))
        || fields.keys().any(|field| {
            !REQUIRED_FIELDS.contains(&field.as_str())
                && !INCARNATION_FIELDS.contains(&field.as_str())
        })
    {
        return Err(invalid("receipt fields"));
    }
    for field in INCARNATION_FIELDS {
        if let Some(value) = fields.get(*field) {
            let valid = match *field {
                "generation" => value.as_u64().is_some_and(|v| v > 0),
                "process_instance_id" => valid_id(value) && value != "unavailable",
                _ => valid_id(value),
            };
            if !valid {
                return Err(invalid("execution incarnation"));
            }
        }
    }
    if result["schema_version"] != "kyuubiki.operator-task-dispatch-result/v1"
        || result["task_id"] != prepared["task_id"]
        || result["task_digest"] != prepared["task_digest"]
        || result["attempt_id"] != attempt_id
        || result["automatic_replay_authorized"] != false
        || result["publication_performed"] != false
    {
        return Err(invalid("identity or authority"));
    }
    if result["status"] == "receipt_recovered" {
        if INCARNATION_FIELDS
            .iter()
            .any(|field| !fields.contains_key(*field))
            || !result["completion"]["execution_readiness"].is_object()
            || !result["completion"]["result"].is_object()
            || !result["completion"]["result"]["operator_task_ir_status"].is_string()
        {
            return Err(invalid("execution incarnation"));
        }
        let completion = operator_task_execution_outcome(prepared, result["completion"].clone())?;
        if result["outcome"] != completion.status {
            return Err(invalid("completion outcome"));
        }
    } else if !matches!(
        result["status"].as_str(),
        Some(
            "no_retained_dispatch"
                | "not_dispatched"
                | "original_endpoint_not_configured"
                | "original_endpoint_unreachable"
                | "agent_receipt_invalid"
                | "pending"
                | "not_retained"
                | "result_not_retained"
                | "attempt_identity_ambiguous"
        )
    ) || result["outcome"] != "unknown"
        || !result["completion"].is_null()
    {
        return Err(invalid("unknown outcome"));
    }
    Ok(())
}

fn valid_id(value: &Value) -> bool {
    value
        .as_str()
        .is_some_and(|id| !id.is_empty() && id.len() <= 256)
}

fn invalid(field: &str) -> HeadlessExecutorError {
    HeadlessExecutorError {
        message: format!("invalid original operator task result: {field}"),
    }
}

#[cfg(test)]
#[path = "service_executor_dispatch_result_tests.rs"]
mod tests;
