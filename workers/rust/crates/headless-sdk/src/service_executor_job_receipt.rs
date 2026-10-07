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
