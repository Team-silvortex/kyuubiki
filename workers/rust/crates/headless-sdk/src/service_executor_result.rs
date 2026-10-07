use crate::service_executor::{request_json, required_path_segment};
use crate::service_executor_job_receipt::{
    require_completed_job, require_result_object, validate_result_envelope,
};
use crate::{HeadlessExecutorError, HeadlessExecutorOutcome};
use serde_json::{Value, json};

pub(crate) fn execute_result_fetch(
    base_url: &str,
    api_token: Option<&str>,
    payload: &Value,
) -> Result<HeadlessExecutorOutcome, HeadlessExecutorError> {
    let job_id = required_path_segment(payload, &["job_id", "jobId"])?;
    let prefer_job_result = payload
        .get("prefer_job_result")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    // Non-preferred reads use metadata only; large solver results are fetched once.
    let suffix = if prefer_job_result { "" } else { "/status" };
    let mut envelope = request_json(
        base_url,
        api_token,
        "GET",
        &format!("/api/v1/jobs/{job_id}{suffix}"),
        None,
    )?;
    require_completed_job(job_id, &envelope)?;
    if prefer_job_result && envelope.get("result").is_some() {
        require_result_object(&envelope)?;
        let result = envelope.as_object_mut().unwrap().remove("result").unwrap();
        return Ok(HeadlessExecutorOutcome {
            status: "executed".into(),
            result: json!({
                "job_id":job_id, "status":envelope.pointer("/job/status"),
                "job":envelope.get("job"), "result":result
            }),
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
    Ok(HeadlessExecutorOutcome {
        status: "executed".into(),
        result: normalize_result_fetch_result(job_id, result),
    })
}

pub(crate) fn normalize_result_fetch_result(job_id: &str, result: Value) -> Value {
    let result = match result {
        Value::Object(mut envelope) => envelope.remove("result").unwrap_or(Value::Object(envelope)),
        value => value,
    };
    json!({ "job_id":job_id, "result":result })
}
