use crate::model_plan_approval::compute_canonical_json_digest;
use crate::{SdkError, SdkResult};
use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet};

const IDENTITIES: &[(&str, &str)] = &[
    ("task_id", "/task_id"),
    ("task_digest", "/integrity/task_digest"),
    ("operator_id", "/operator/id"),
    ("program_id", "/execution_program/program_id"),
];

pub(crate) fn validate_model_task_completion(
    action: &str,
    payload: &Value,
    receipt: &Value,
) -> SdkResult<()> {
    match action {
        "operator_task_execute" => {
            let task = payload
                .get("task")
                .ok_or_else(|| invalid("missing submitted task"))?;
            verify_identity(task, receipt, true)?;
            verify_completion(task, receipt, "status", "executed")
        }
        "operator_task_batch_execute" => verify_batch(payload, receipt),
        // Preparing a task or submitting an asynchronous job is not synchronous computation.
        _ => Ok(()),
    }
}

fn verify_identity(task: &Value, receipt: &Value, require_program: bool) -> SdkResult<()> {
    for &(field, path) in IDENTITIES {
        let expected = task
            .pointer(path)
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| invalid(format!("submitted task has no {field}")))?;
        if field == "program_id" && !require_program && receipt.get(field).is_none() {
            continue;
        }
        if receipt.get(field).and_then(Value::as_str) != Some(expected) {
            return Err(invalid(format!("{field} does not match submitted task")));
        }
    }
    for field in ["provenance_receipt", "validation_receipt"] {
        if let Some(nested) = receipt.get(field) {
            let nested = nested
                .as_object()
                .ok_or_else(|| invalid(format!("malformed {field}")))?;
            for &(identity, path) in IDENTITIES {
                if nested
                    .get(identity)
                    .is_some_and(|value| Some(value) != task.pointer(path))
                {
                    return Err(invalid(format!("{field}.{identity} disagrees with task")));
                }
            }
            if nested
                .get("digest_verified")
                .is_some_and(|value| value != true)
            {
                return Err(invalid(format!("{field} did not verify task digest")));
            }
        }
    }
    Ok(())
}

fn verify_completion(
    task: &Value,
    receipt: &Value,
    status_field: &str,
    success: &str,
) -> SdkResult<()> {
    match receipt.get(status_field).and_then(Value::as_str) {
        Some(status) if status == success => {}
        Some(
            "blocked"
            | "verified_pending_execution"
            | "verified_pending_engine_execution"
            | "ready_for_package_resolution",
        ) => return Err(blocked()),
        Some("failed" | "error" | "cancelled") => {
            return Err(validation(
                "operator_task_execution_failed: inspect retained task receipt",
            ));
        }
        _ => return Err(invalid(format!("missing or unknown {status_field}"))),
    }
    verify_success_gates(receipt)?;
    let result = receipt
        .get("result")
        .filter(|value| !value.is_null())
        .ok_or_else(|| invalid("success receipt has no published result"))?;
    if result.get("operator_task_ir_status").is_some() {
        verify_identity(task, result, true)?;
        verify_completion(task, result, "operator_task_ir_status", "executed")?;
    }
    Ok(())
}

fn verify_success_gates(receipt: &Value) -> SdkResult<()> {
    if receipt.get("ok").is_some_and(|value| value != true)
        || receipt.get("error").is_some_and(|value| !value.is_null())
        || receipt
            .get("failure_receipt")
            .is_some_and(|value| !value.is_null())
        || receipt
            .get("operator_task_failure_receipt")
            .is_some_and(|value| !value.is_null())
    {
        return Err(invalid("success receipt also declares execution failure"));
    }
    if let Some(readiness) = receipt.get("execution_readiness") {
        match readiness.get("status").and_then(Value::as_str) {
            Some("blocked" | "ready_for_package_resolution") => return Err(blocked()),
            Some("executed") if readiness.get("ready_to_dispatch") == Some(&Value::Bool(true)) => {}
            _ => return Err(invalid("malformed or contradictory execution_readiness")),
        }
        for field in ["blocking_stage", "blocking_reason", "required_action"] {
            if readiness.get(field).is_some_and(|value| !value.is_null()) {
                return Err(invalid(format!(
                    "executed readiness still declares {field}"
                )));
            }
        }
    }
    if receipt
        .get("blocked_stage")
        .is_some_and(|value| !value.is_null())
    {
        return Err(invalid("success receipt still declares blocked_stage"));
    }
    Ok(())
}

fn verify_batch(payload: &Value, receipt: &Value) -> SdkResult<()> {
    let batch = payload
        .get("batch")
        .and_then(Value::as_object)
        .ok_or_else(|| invalid("missing submitted batch"))?;
    if batch
        .get("quality_execution_batch_contract")
        .and_then(Value::as_str)
        != Some("kyuubiki.quality_execution_batch/v1")
        || receipt
            .get("operator_task_batch_execution_contract")
            .and_then(Value::as_str)
            != Some("kyuubiki.operator_task_batch_execution/v1")
    {
        return Err(invalid("unsupported batch execution contract"));
    }
    let tasks = batch
        .get("tasks")
        .and_then(Value::as_array)
        .filter(|tasks| !tasks.is_empty())
        .ok_or_else(|| invalid("submitted batch has no tasks"))?;
    let count = tasks.len() as u64;
    if batch
        .get("task_count")
        .is_some_and(|value| value.as_u64() != Some(count))
    {
        return Err(invalid("submitted batch task_count disagrees with tasks"));
    }
    let fields: Map<_, _> = [
        "quality_execution_batch_contract",
        "operator_id",
        "task_count",
        "case_index",
        "tasks",
    ]
    .into_iter()
    .filter_map(|key| batch.get(key).map(|value| (key.into(), value.clone())))
    .collect();
    let digest = compute_canonical_json_digest(&Value::Object(fields));
    if receipt.get("batch_digest").and_then(Value::as_str) != digest.strip_prefix("sha256:") {
        return Err(invalid("batch_digest does not match submitted batch"));
    }
    if receipt.get("status").and_then(Value::as_str) != Some("executed") {
        return Err(invalid("batch does not report executed status"));
    }
    verify_success_gates(receipt)?;
    for (field, expected) in [
        ("task_count", count),
        ("executed_count", count),
        ("ok_count", count),
        ("error_count", 0),
    ] {
        if receipt.get(field).and_then(Value::as_u64) != Some(expected) {
            return Err(invalid(format!(
                "incomplete or contradictory batch {field}"
            )));
        }
    }
    let results = receipt
        .get("results")
        .and_then(Value::as_array)
        .filter(|results| results.len() == tasks.len())
        .ok_or_else(|| invalid("batch results do not cover every submitted task"))?;
    let mut expected = HashMap::with_capacity(tasks.len());
    for entry in tasks {
        let case_id = required_case_id(entry)?;
        let task = entry
            .get("task_ir")
            .ok_or_else(|| invalid("batch entry has no task_ir"))?;
        if expected.insert(case_id, task).is_some() {
            return Err(invalid("submitted batch repeats case_id"));
        }
    }
    let mut seen = HashSet::with_capacity(results.len());
    for result in results {
        let case_id = required_case_id(result)?;
        let task = expected
            .get(case_id)
            .ok_or_else(|| invalid("batch result case_id was not submitted"))?;
        if !seen.insert(case_id) {
            return Err(invalid("batch result repeats case_id"));
        }
        // Local Orchestra entries carry three identity fields; Agent receipts also carry program_id.
        verify_identity(task, result, false)?;
        verify_completion(task, result, "status", "ok")?;
    }
    for field in ["error_codes", "failure_receipts", "failed_case_ids"] {
        if receipt
            .get(field)
            .is_some_and(|value| value.as_array().is_none_or(|items| !items.is_empty()))
        {
            return Err(invalid(format!("completed batch contains {field}")));
        }
    }
    if receipt
        .get("error_code_counts")
        .is_some_and(|value| value.as_object().is_none_or(|counts| !counts.is_empty()))
    {
        return Err(invalid("completed batch contains error_code_counts"));
    }
    Ok(())
}

fn required_case_id(value: &Value) -> SdkResult<&str> {
    value
        .get("case_id")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| invalid("batch entry has no case_id"))
}

fn blocked() -> SdkError {
    validation(
        "operator_task_execution_blocked: inspect retained execution_readiness.required_action",
    )
}

fn invalid(detail: impl AsRef<str>) -> SdkError {
    validation(format!(
        "operator_task_execution_receipt_invalid: {}",
        detail.as_ref()
    ))
}

fn validation(message: impl Into<String>) -> SdkError {
    SdkError::Validation {
        errors: vec![message.into()],
    }
}
