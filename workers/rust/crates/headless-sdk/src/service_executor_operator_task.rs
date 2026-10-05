use crate::execution_observability::HEADLESS_FAILURE_RECEIPT_SCHEMA_VERSION;
use crate::{
    HeadlessExecutionStepReport, HeadlessExecutorError, HeadlessExecutorOutcome,
    HeadlessFailureReceipt,
};
use serde_json::{Value, json};

pub(crate) fn operator_task_execution_outcome(
    prepared: &Value,
    receipt: Value,
) -> Result<HeadlessExecutorOutcome, HeadlessExecutorError> {
    verify_identity(prepared, &receipt)?;
    let mut status = completion_status(&receipt, "status")?;
    let failure = validate_agent_failure(prepared, &receipt, status)?;
    // Orchestra wraps Agent receipts in its own executed envelope. Dispatch
    // completion alone does not prove the nested task actually executed.
    if let Some(agent) = receipt.get("result")
        && agent.get("operator_task_ir_status").is_some()
    {
        verify_identity(prepared, agent)?;
        let nested = completion_status(agent, "operator_task_ir_status")?;
        let nested_failure = validate_agent_failure(prepared, agent, nested)?;
        if let (Some(outer), Some(inner)) = (failure, nested_failure)
            && outer != inner
        {
            return Err(invalid("failure_receipt disagrees with the Agent failure"));
        }
        if status == "executed" {
            status = nested;
        } else if nested == "executed" {
            return Err(invalid("contradictory outer and Agent completion states"));
        }
    }
    Ok(HeadlessExecutorOutcome {
        status: status.into(),
        result: receipt,
    })
}

fn validate_agent_failure<'a>(
    prepared: &Value,
    receipt: &'a Value,
    status: &str,
) -> Result<Option<&'a Value>, HeadlessExecutorError> {
    let Some(failure) = receipt.get("failure_receipt") else {
        return Ok(None);
    };
    if failure.is_null() && status != "failed" {
        return Ok(None);
    }
    if status != "failed" || !failure.is_object() {
        return Err(invalid("failure_receipt is not a failed task receipt"));
    }
    for (field, expected) in [
        ("schema_version", "kyuubiki.agent-operator-task-failure/v1"),
        ("failure_owner", "agent_runtime"),
    ] {
        if failure.get(field).and_then(Value::as_str) != Some(expected) {
            return Err(invalid(format!("failure_receipt.{field}")));
        }
    }
    for field in ["task_id", "task_digest", "operator_id", "program_id"] {
        if field == "program_id" && failure.get(field).is_none() {
            continue;
        }
        let value = failure.get(field).and_then(Value::as_str);
        if value.is_none_or(|value| value.is_empty() || value.len() > 1024)
            || value != prepared.get(field).and_then(Value::as_str)
        {
            return Err(invalid(format!("failure_receipt.{field}")));
        }
    }
    for (field, limit) in [
        ("reason_code", 128),
        ("failure_stage", 128),
        ("message", 4096),
    ] {
        if failure
            .get(field)
            .and_then(Value::as_str)
            .is_none_or(|value| value.is_empty() || value.len() > limit)
        {
            return Err(invalid(format!("failure_receipt.{field}")));
        }
    }
    if receipt.get("error_code") != failure.get("reason_code") {
        return Err(invalid("failure_receipt.reason_code"));
    }
    let recovery = &failure["recovery"];
    for field in ["retryable", "safe_to_continue_other_tasks"] {
        if recovery.get(field).and_then(Value::as_bool).is_none() {
            return Err(invalid(format!("failure_receipt.recovery.{field}")));
        }
    }
    if recovery
        .get("required_action")
        .and_then(Value::as_str)
        .is_none_or(|value| value.is_empty() || value.len() > 128)
    {
        return Err(invalid("failure_receipt.recovery.required_action"));
    }
    let readiness = &receipt["execution_readiness"];
    if readiness["status"] != "blocked" || readiness["ready_to_dispatch"] != false {
        return Err(invalid("failure_receipt.execution_readiness"));
    }
    for (field, expected) in [
        ("current_stage", &failure["failure_stage"]),
        ("blocking_stage", &failure["failure_stage"]),
        ("blocking_reason", &failure["reason_code"]),
        ("blocking_owner", &failure["failure_owner"]),
        ("required_action", &recovery["required_action"]),
    ] {
        if readiness.get(field) != Some(expected) {
            return Err(invalid(format!(
                "failure_receipt.execution_readiness.{field}"
            )));
        }
    }
    Ok(Some(failure))
}

pub(crate) fn agent_failure_summary(
    step: &HeadlessExecutionStepReport,
) -> Option<HeadlessFailureReceipt> {
    if step.status != "failed" || step.action != "operator_task_execute" {
        return None;
    }
    let task = step.payload.get("task")?;
    let prepared = json!({"task_id":task["task_id"],"task_digest":task["integrity"]["task_digest"],
        "operator_id":task["operator"]["id"],"program_id":task["execution_program"]["program_id"]});
    verify_identity(&prepared, &step.result_preview).ok()?;
    let failure = validate_agent_failure(&prepared, &step.result_preview, "failed").ok()??;
    let retryable = failure["recovery"]["retryable"].as_bool()?;
    // Preserve the bounded native receipt in the preview; summarize recovery using
    // the existing Headless contract without guessing from human-readable text.
    Some(HeadlessFailureReceipt {
        schema_version: HEADLESS_FAILURE_RECEIPT_SCHEMA_VERSION.into(),
        error_code: "kyuubiki.headless.runtime_failure".into(),
        category: "runtime_failure".into(),
        stage: "execution".into(),
        step_index: step.index,
        action: step.action.clone(),
        message: failure["message"].as_str()?.into(),
        retryable,
        retry_strategy: if retryable {
            "bounded_exponential_backoff"
        } else {
            "none"
        }
        .into(),
        recommended_action: failure["recovery"]["required_action"].as_str()?.into(),
    })
}

fn verify_identity(prepared: &Value, receipt: &Value) -> Result<(), HeadlessExecutorError> {
    for field in ["task_id", "task_digest", "operator_id", "program_id"] {
        let expected = prepared.get(field).and_then(Value::as_str);
        if expected.is_none() || receipt.get(field).and_then(Value::as_str) != expected {
            return Err(invalid(format!(
                "{field} does not match the submitted task"
            )));
        }
    }
    for field in ["provenance_receipt", "validation_receipt"] {
        if let Some(nested) = receipt.get(field) {
            if !nested.is_object() {
                return Err(invalid(format!("malformed {field}")));
            }
            for identity in ["task_id", "task_digest", "operator_id", "program_id"] {
                if let Some(value) = nested.get(identity)
                    && Some(value) != prepared.get(identity)
                {
                    return Err(invalid(format!(
                        "{field}.{identity} disagrees with the task"
                    )));
                }
            }
            if nested
                .get("digest_verified")
                .is_some_and(|value| value != true)
            {
                return Err(invalid(format!("{field} did not verify the task digest")));
            }
        }
    }
    Ok(())
}

fn completion_status(
    receipt: &Value,
    status_field: &str,
) -> Result<&'static str, HeadlessExecutorError> {
    let state = receipt.get(status_field).and_then(Value::as_str);
    let status = match state {
        Some("executed") => "executed",
        Some(
            "verified_pending_execution"
            | "verified_pending_engine_execution"
            | "ready_for_package_resolution"
            | "blocked",
        ) => "blocked",
        Some("failed" | "error" | "cancelled") => "failed",
        _ => return Err(invalid(format!("missing or unknown {status_field}"))),
    };
    if receipt.get("ok").is_some_and(|value| value != true)
        || receipt.get("error").is_some_and(|value| !value.is_null())
    {
        return Err(invalid("response contains an explicit execution failure"));
    }
    if let Some(readiness) = receipt.get("execution_readiness") {
        let state = readiness.get("status").and_then(Value::as_str);
        let gate = match state {
            Some("executed") => "executed",
            Some("blocked" | "ready_for_package_resolution") => "blocked",
            _ => return Err(invalid("missing or unknown execution_readiness.status")),
        };
        if status == "executed" && gate == "blocked" {
            return Ok("blocked");
        }
        if status != "executed" && gate == "executed" {
            return Err(invalid("unexecuted task claims executed readiness"));
        }
        if gate == "executed" && readiness.get("ready_to_dispatch") != Some(&Value::Bool(true)) {
            return Err(invalid("executed readiness is not ready_to_dispatch"));
        }
        if gate == "executed" {
            for field in ["blocking_stage", "blocking_reason", "required_action"] {
                if readiness.get(field).is_some_and(|value| !value.is_null()) {
                    return Err(invalid(format!(
                        "executed readiness still declares {field}"
                    )));
                }
            }
        }
    }
    if status == "executed" {
        if receipt
            .get("blocked_stage")
            .is_some_and(|value| !value.is_null())
        {
            return Err(invalid("executed task still declares a blocking stage"));
        }
        if receipt.get("result").is_none_or(Value::is_null) {
            return Err(invalid("executed task has no published result"));
        }
    }
    Ok(status)
}

fn invalid(detail: impl AsRef<str>) -> HeadlessExecutorError {
    HeadlessExecutorError {
        message: format!(
            "operator task execution receipt invalid: {}",
            detail.as_ref()
        ),
    }
}

#[cfg(test)]
#[path = "service_executor_operator_task_tests.rs"]
mod tests;
