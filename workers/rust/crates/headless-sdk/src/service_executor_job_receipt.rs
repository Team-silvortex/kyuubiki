use crate::HeadlessExecutorError;
use serde_json::{Map, Value};

pub(crate) const INVALID_RECEIPT: &str = "job_receipt_invalid:";
pub(crate) const UNAVAILABLE_RESULT: &str = "job_result_unavailable:";
pub(crate) const JOB_STATUSES: &[&str] = &[
    "queued",
    "preprocessing",
    "partitioning",
    "solving",
    "postprocessing",
    "completed",
    "failed",
    "cancelled",
];

const SUBMISSION_CONTEXT: &[&str] = &["project_id", "model_version_id"];

pub(crate) fn submission_context(
    payload: &Value,
) -> Result<Map<String, Value>, HeadlessExecutorError> {
    let mut context = Map::new();
    for &key in SUBMISSION_CONTEXT {
        if let Some(value) = payload.get(key) {
            let id = value.as_str().ok_or_else(|| HeadlessExecutorError {
                message: format!("submission context requires string {key}"),
            })?;
            crate::service_executor::validate_path_segment(id, key)?;
            context.insert(key.into(), value.clone());
        }
    }
    Ok(context)
}

pub(crate) fn validate_submission_context(
    envelope: &Value,
    job: &Map<String, Value>,
    expected: &Map<String, Value>,
) -> Result<(), HeadlessExecutorError> {
    for &key in SUBMISSION_CONTEXT {
        if expected
            .get(key)
            .is_some_and(|value| job.get(key) != Some(value))
        {
            return Err(invalid(&format!(
                "submission {key} does not match the transmitted context"
            )));
        }
        if envelope
            .get(key)
            .is_some_and(|value| job.get(key) != Some(value))
        {
            return Err(invalid(&format!(
                "submission {key} fields are contradictory"
            )));
        }
    }
    Ok(())
}

pub(crate) fn validate_submission_receipt(
    envelope: &Value,
) -> Result<&Map<String, Value>, HeadlessExecutorError> {
    let job_id = envelope
        .pointer("/job/job_id")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid("submission job identity is missing or not a string"))?;
    crate::service_executor::validate_path_segment(job_id, "submission job identity")
        .map_err(|_| invalid("submission job identity is not a usable path segment"))?;
    validate_job_receipt(job_id, envelope)
}

pub(crate) fn validate_job_receipt<'a>(
    job_id: &str,
    envelope: &'a Value,
) -> Result<&'a Map<String, Value>, HeadlessExecutorError> {
    let job = envelope
        .get("job")
        .and_then(Value::as_object)
        .ok_or_else(|| invalid("job object is missing"))?;
    if job.get("job_id").and_then(Value::as_str) != Some(job_id)
        || envelope
            .get("job_id")
            .is_some_and(|id| id.as_str() != Some(job_id))
    {
        return Err(invalid("job identity does not match the requested job"));
    }
    if !job
        .get("status")
        .and_then(Value::as_str)
        .is_some_and(|status| JOB_STATUSES.contains(&status))
    {
        return Err(invalid("job status is missing or unsupported"));
    }
    if envelope
        .get("status")
        .is_some_and(|status| Some(status) != job.get("status"))
    {
        return Err(invalid("job status fields are contradictory"));
    }
    Ok(job)
}

pub(crate) fn require_completed_job<'a>(
    job_id: &str,
    envelope: &'a Value,
) -> Result<&'a Map<String, Value>, HeadlessExecutorError> {
    let job = validate_job_receipt(job_id, envelope)?;
    if job.get("status").and_then(Value::as_str) != Some("completed") {
        return Err(unavailable(
            "the requested job has not completed successfully",
        ));
    }
    Ok(job)
}

pub(crate) fn validate_result_envelope(
    job_id: &str,
    envelope: &Value,
) -> Result<(), HeadlessExecutorError> {
    if envelope.get("job_id").and_then(Value::as_str) != Some(job_id) {
        return Err(invalid("result identity does not match the requested job"));
    }
    if envelope.get("job").is_some() {
        require_completed_job(job_id, envelope)?;
    }
    if envelope
        .get("status")
        .is_some_and(|status| status.as_str() != Some("completed"))
    {
        return Err(invalid("result status contradicts a completed job"));
    }
    require_result_object(envelope)
}

pub(crate) fn require_result_object(envelope: &Value) -> Result<(), HeadlessExecutorError> {
    if !envelope.get("result").is_some_and(Value::is_object) {
        return Err(unavailable("an explicit object result is not available"));
    }
    Ok(())
}

fn invalid(detail: &str) -> HeadlessExecutorError {
    HeadlessExecutorError {
        message: format!("{INVALID_RECEIPT} {detail}"),
    }
}

fn unavailable(detail: &str) -> HeadlessExecutorError {
    HeadlessExecutorError {
        message: format!("{UNAVAILABLE_RESULT} {detail}"),
    }
}
