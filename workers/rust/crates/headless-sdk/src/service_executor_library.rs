use crate::service_executor::{request_json, required_path_segment, validate_path_segment};
use crate::service_executor_response::after_send_failure;
use crate::{HeadlessExecutorError, HeadlessExecutorOutcome};
use serde_json::{Map, Value};

pub(crate) fn execute_project_create(
    base_url: &str,
    api_token: Option<&str>,
    payload: &Value,
) -> Result<HeadlessExecutorOutcome, HeadlessExecutorError> {
    execute_json(
        base_url,
        api_token,
        "POST",
        "/api/v1/projects",
        select_fields(payload, &["name", "description"]),
        |result| normalize_project_result(result, None),
    )
}

pub(crate) fn execute_project_update(
    base_url: &str,
    api_token: Option<&str>,
    payload: &Value,
) -> Result<HeadlessExecutorOutcome, HeadlessExecutorError> {
    let project_id = required_path_segment(payload, &["project_id", "projectId"])?;
    execute_json(
        base_url,
        api_token,
        "PATCH",
        &format!("/api/v1/projects/{project_id}"),
        select_fields(payload, &["name", "description"]),
        |result| normalize_project_result(result, Some(project_id)),
    )
}

pub(crate) fn execute_project_delete(
    base_url: &str,
    api_token: Option<&str>,
    payload: &Value,
) -> Result<HeadlessExecutorOutcome, HeadlessExecutorError> {
    let project_id = required_path_segment(payload, &["project_id", "projectId"])?;
    let result = request_json(
        base_url,
        api_token,
        "DELETE",
        &format!("/api/v1/projects/{project_id}"),
        None,
    )?;
    normalize_project_result(result, Some(project_id))
        .map(outcome)
        .map_err(|error| after_send_failure("DELETE", error))
}

pub(crate) fn execute_model_create(
    base_url: &str,
    api_token: Option<&str>,
    payload: &Value,
) -> Result<HeadlessExecutorOutcome, HeadlessExecutorError> {
    let project_id = required_path_segment(payload, &["project_id", "projectId"])?;
    execute_json(
        base_url,
        api_token,
        "POST",
        &format!("/api/v1/projects/{project_id}/models"),
        select_fields(
            payload,
            &[
                "name",
                "kind",
                "payload",
                "material",
                "model_schema_version",
            ],
        ),
        |result| normalize_model_result(result, project_id),
    )
}

pub(crate) fn execute_model_version_create(
    base_url: &str,
    api_token: Option<&str>,
    payload: &Value,
) -> Result<HeadlessExecutorOutcome, HeadlessExecutorError> {
    let model_id = required_path_segment(payload, &["model_id", "modelId"])?;
    execute_json(
        base_url,
        api_token,
        "POST",
        &format!("/api/v1/models/{model_id}/versions"),
        select_fields(
            payload,
            &[
                "payload",
                "name",
                "kind",
                "material",
                "model_schema_version",
            ],
        ),
        |result| normalize_version_result(result, model_id),
    )
}

fn execute_json(
    base_url: &str,
    api_token: Option<&str>,
    method: &str,
    path: &str,
    body: Value,
    normalize: impl FnOnce(Value) -> Result<Value, HeadlessExecutorError>,
) -> Result<HeadlessExecutorOutcome, HeadlessExecutorError> {
    let result = request_json(base_url, api_token, method, path, Some(body))?;
    normalize(result)
        .map(outcome)
        .map_err(|error| after_send_failure(method, error))
}

fn outcome(result: Value) -> HeadlessExecutorOutcome {
    HeadlessExecutorOutcome {
        status: "executed".to_string(),
        result,
    }
}

fn select_fields(payload: &Value, keys: &[&str]) -> Value {
    Value::Object(
        keys.iter()
            .filter_map(|key| {
                payload
                    .get(*key)
                    .cloned()
                    .map(|value| ((*key).to_string(), value))
            })
            .collect::<Map<_, _>>(),
    )
}

fn normalize_project_result(
    result: Value,
    expected_id: Option<&str>,
) -> Result<Value, HeadlessExecutorError> {
    let record = receipt_record(&result, "project")?;
    let id = receipt_identity(&result, record, "project_id")?;
    require_expected(id, expected_id, "project_id")?;
    Ok(normalize_record_result(
        result,
        "project",
        "project_id",
        "project_id",
    ))
}

fn normalize_model_result(result: Value, project_id: &str) -> Result<Value, HeadlessExecutorError> {
    let record = receipt_record(&result, "model")?;
    receipt_identity(&result, record, "model_id")?;
    let parent = receipt_identity(&result, record, "project_id")?;
    require_expected(parent, Some(project_id), "project_id")?;
    receipt_identity(&result, record, "latest_version_id")?;
    Ok(normalize_record_result(
        result, "model", "model_id", "model_id",
    ))
}

fn normalize_version_result(result: Value, model_id: &str) -> Result<Value, HeadlessExecutorError> {
    let record = receipt_record(&result, "version")?;
    let id = receipt_identity(&result, record, "version_id")?;
    for alias in [
        result.get("model_version_id"),
        record.get("model_version_id"),
    ] {
        if alias.is_some_and(|value| value.as_str() != Some(id)) {
            return Err(invalid_receipt("model_version_id contradicts version_id"));
        }
    }
    let parent = receipt_identity(&result, record, "model_id")?;
    require_expected(parent, Some(model_id), "model_id")?;
    receipt_identity(&result, record, "project_id")?;
    Ok(normalize_record_result(
        result,
        "version",
        "version_id",
        "model_version_id",
    ))
}

fn receipt_record<'a>(
    result: &'a Value,
    key: &str,
) -> Result<&'a Map<String, Value>, HeadlessExecutorError> {
    result
        .get(key)
        .and_then(Value::as_object)
        .ok_or_else(|| invalid_receipt(&format!("missing explicit {key} object")))
}

fn receipt_identity<'a>(
    result: &Value,
    record: &'a Map<String, Value>,
    key: &str,
) -> Result<&'a str, HeadlessExecutorError> {
    let id = record
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| invalid_receipt(&format!("missing string {key}")))?;
    // Never trim or repair a receipt identifier before exposing it as a binding.
    validate_path_segment(id, key).map_err(|_| invalid_receipt(&format!("unusable {key}")))?;
    if result
        .get(key)
        .is_some_and(|value| value.as_str() != Some(id))
    {
        return Err(invalid_receipt(&format!("contradictory {key}")));
    }
    Ok(id)
}

fn require_expected(
    actual: &str,
    expected: Option<&str>,
    key: &str,
) -> Result<(), HeadlessExecutorError> {
    if expected.is_some_and(|expected| expected != actual) {
        return Err(invalid_receipt(&format!("{key} does not match request")));
    }
    Ok(())
}

fn invalid_receipt(detail: &str) -> HeadlessExecutorError {
    HeadlessExecutorError {
        message: format!("library_receipt_invalid: {detail}"),
    }
}

fn normalize_record_result(
    result: Value,
    envelope_key: &str,
    source_id_key: &str,
    output_id_key: &str,
) -> Value {
    let Some(record) = result.get(envelope_key).and_then(Value::as_object) else {
        return result;
    };
    let mut normalized = record.clone();
    if output_id_key != source_id_key {
        normalized.insert(
            output_id_key.to_string(),
            record.get(source_id_key).cloned().unwrap_or(Value::Null),
        );
    }
    normalized.insert(envelope_key.to_string(), Value::Object(record.clone()));
    normalized.insert("raw".to_string(), result);
    Value::Object(normalized)
}
