use crate::HeadlessExecutorError;
use crate::service_executor::{request_json, validate_path_segment};
use serde_json::{Map, Value};

pub(crate) const INVALID_REFERENCE: &str = "model_reference_invalid:";

#[derive(Clone, Copy)]
pub(crate) enum ModelReference {
    Model,
    Version,
}

pub(crate) struct LoadedModelReference {
    pub resolved: Value,
    pub kind: String,
}

pub(crate) fn load_model_reference(
    base_url: &str,
    api_token: Option<&str>,
    payload: &Value,
    reference: ModelReference,
) -> Result<LoadedModelReference, HeadlessExecutorError> {
    if payload.get("input").is_some() || payload.get("model_payload").is_some() {
        return Err(invalid(
            "saved reference cannot be combined with inline input",
        ));
    }
    let (route, envelope_key, id_key, source_key, aliases): (_, _, _, _, &[&str]) = match reference
    {
        ModelReference::Model => (
            "models",
            "model",
            "model_id",
            "model_id",
            &["model_id", "modelId"],
        ),
        ModelReference::Version => (
            "model-versions",
            "version",
            "model_version_id",
            "version_id",
            &["model_version_id", "modelVersionId"],
        ),
    };
    let id = requested_identity(payload, aliases)?
        .ok_or_else(|| invalid("missing saved reference identity"))?;
    let envelope = request_json(
        base_url,
        api_token,
        "GET",
        &format!("/api/v1/{route}/{id}"),
        None,
    )?;
    let record = envelope
        .get(envelope_key)
        .and_then(Value::as_object)
        .ok_or_else(|| invalid("missing explicit saved record"))?;
    let actual = record_identity(&envelope, record, source_key)?;
    if actual != id {
        return Err(invalid("saved record identity does not match request"));
    }
    if id_key != source_key {
        for alias in [envelope.get(id_key), record.get(id_key)] {
            if alias.is_some_and(|value| value.as_str() != Some(id)) {
                return Err(invalid(
                    "saved version alias contradicts canonical identity",
                ));
            }
        }
    }
    let project_id = record_identity(&envelope, record, "project_id")?;
    require_parent(payload, &["project_id", "projectId"], project_id)?;
    let model_id = match reference {
        ModelReference::Model => id,
        ModelReference::Version => {
            let parent = record_identity(&envelope, record, "model_id")?;
            require_parent(payload, &["model_id", "modelId"], parent)?;
            parent
        }
    };
    let kind = record
        .get("kind")
        .and_then(Value::as_str)
        .filter(|kind| {
            !kind.is_empty() && *kind == kind.trim() && !kind.chars().any(char::is_control)
        })
        .ok_or_else(|| invalid("saved record requires a usable kind"))?;
    let model = record
        .get("payload")
        .filter(|value| value.is_object())
        .ok_or_else(|| invalid("saved record requires an object payload"))?;
    let mut resolved = payload.as_object().cloned().unwrap_or_default();
    resolved.insert(id_key.into(), Value::String(id.into()));
    resolved.insert("model_id".into(), Value::String(model_id.into()));
    resolved.insert("project_id".into(), Value::String(project_id.into()));
    resolved.insert("model_payload".into(), model.clone());
    if !resolved.contains_key("study_kind") && !resolved.contains_key("studyKind") {
        resolved.insert("study_kind".into(), Value::String(kind.into()));
    }
    Ok(LoadedModelReference {
        resolved: Value::Object(resolved),
        kind: kind.into(),
    })
}

fn record_identity<'a>(
    envelope: &Value,
    record: &'a Map<String, Value>,
    key: &str,
) -> Result<&'a str, HeadlessExecutorError> {
    let id = record
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| invalid(&format!("saved record requires string {key}")))?;
    validate_path_segment(id, key).map_err(|_| invalid(&format!("unusable saved {key}")))?;
    if envelope
        .get(key)
        .is_some_and(|value| value.as_str() != Some(id))
    {
        return Err(invalid(&format!("contradictory saved {key}")));
    }
    Ok(id)
}

fn requested_identity<'a>(
    payload: &'a Value,
    keys: &[&str],
) -> Result<Option<&'a str>, HeadlessExecutorError> {
    let mut selected = None;
    for key in keys {
        if let Some(value) = payload.get(*key) {
            let id = value
                .as_str()
                .ok_or_else(|| invalid(&format!("reference requires string {key}")))?;
            validate_path_segment(id, key)
                .map_err(|_| invalid(&format!("unusable reference {key}")))?;
            if selected.is_some_and(|previous| previous != id) {
                return Err(invalid("contradictory reference aliases"));
            }
            selected = Some(id);
        }
    }
    Ok(selected)
}

fn require_parent(
    payload: &Value,
    keys: &[&str],
    actual: &str,
) -> Result<(), HeadlessExecutorError> {
    if requested_identity(payload, keys)?.is_some_and(|expected| expected != actual) {
        return Err(invalid("saved record parent does not match request"));
    }
    Ok(())
}

fn invalid(detail: &str) -> HeadlessExecutorError {
    HeadlessExecutorError {
        message: format!("{INVALID_REFERENCE} {detail}"),
    }
}
