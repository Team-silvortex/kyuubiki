use crate::service_executor::{request_json, required_path_segment};
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
        normalize_project_result,
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
        normalize_project_result,
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
    Ok(outcome(normalize_project_result(result)))
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
        normalize_model_result,
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
        normalize_version_result,
    )
}

fn execute_json(
    base_url: &str,
    api_token: Option<&str>,
    method: &str,
    path: &str,
    body: Value,
    normalize: fn(Value) -> Value,
) -> Result<HeadlessExecutorOutcome, HeadlessExecutorError> {
    let result = request_json(base_url, api_token, method, path, Some(body))?;
    Ok(outcome(normalize(result)))
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

fn normalize_project_result(result: Value) -> Value {
    normalize_record_result(result, "project", "project_id", "project_id")
}

fn normalize_model_result(result: Value) -> Value {
    normalize_record_result(result, "model", "model_id", "model_id")
}

fn normalize_version_result(result: Value) -> Value {
    normalize_record_result(result, "version", "version_id", "model_version_id")
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
