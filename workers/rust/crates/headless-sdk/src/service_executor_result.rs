use crate::service_executor::request_json;
use crate::service_executor_job_read::{JobReadRequest, prefer_job_result};
use crate::service_executor_job_receipt::{
    require_completed_job, require_result_object, validate_result_envelope,
};
use crate::{HeadlessExecutorError, HeadlessExecutorOutcome};
use serde_json::{Map, Value};

pub(crate) fn execute_result_fetch(
    base_url: &str,
    api_token: Option<&str>,
    payload: &Value,
) -> Result<HeadlessExecutorOutcome, HeadlessExecutorError> {
    let read = JobReadRequest::parse(payload)?;
    let job_id = read.job_id;
    let prefer_job_result = prefer_job_result(payload)?;
    // Non-preferred reads use metadata only; large solver results are fetched once.
    let suffix = if prefer_job_result { "" } else { "/status" };
    let envelope = request_json(
        base_url,
        api_token,
        "GET",
        &format!("/api/v1/jobs/{job_id}{suffix}"),
        None,
    )?;
    require_completed_job(job_id, &envelope)?;
    read.validate_context(&envelope)?;
    if prefer_job_result && envelope.get("result").is_some() {
        require_result_object(&envelope)?;
        return Ok(HeadlessExecutorOutcome {
            status: "executed".into(),
            result: normalize_preferred_job_result(job_id, envelope),
        });
    }
    let result = request_json(
        base_url,
        api_token,
        "GET",
        &format!("/api/v1/results/{job_id}"),
        None,
    )?;
    validate_result_envelope(job_id, &result)?;
    read.validate_result_context(&envelope["job"], &result)?;
    Ok(HeadlessExecutorOutcome {
        status: "executed".into(),
        result: normalize_result_fetch_result(job_id, result),
    })
}

pub(crate) fn normalize_preferred_job_result(job_id: &str, envelope: Value) -> Value {
    // Called only after completed-job, context and object-result validation.
    let Value::Object(mut envelope) = envelope else {
        unreachable!("validated job result envelope must be an object")
    };
    let result = envelope.remove("result").expect("validated object result");
    let job = envelope.remove("job").expect("validated job receipt");
    let status = job.get("status").cloned().unwrap_or(Value::Null);
    Value::Object(Map::from_iter([
        ("job_id".into(), Value::String(job_id.into())),
        ("status".into(), status),
        ("job".into(), job),
        ("result".into(), result),
    ]))
}

pub(crate) fn normalize_result_fetch_result(job_id: &str, result: Value) -> Value {
    let result = match result {
        Value::Object(mut envelope) => envelope.remove("result").unwrap_or(Value::Object(envelope)),
        value => value,
    };
    Value::Object(Map::from_iter([
        ("job_id".into(), Value::String(job_id.into())),
        ("result".into(), result),
    ]))
}
