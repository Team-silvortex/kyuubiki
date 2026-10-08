use crate::{HeadlessExecutorError, service_executor_result_download::download_result};
use serde_json::{Value, json};
use std::time::Duration;

pub(crate) const READBACK_FAILURE: &str = "result_artifact_readback_failed:";
pub(crate) const RESULT_MEDIA_TYPE: &str = "application/vnd.kyuubiki.result+json";
const REFERENCE_SCHEMA: &str = "kyuubiki.solver-result-reference/v1";
const ARTIFACT_SCHEMA: &str = "kyuubiki.result-artifact-ref/v1";

#[derive(Clone, Copy)]
pub(crate) struct ResultReadPolicy {
    pub resolve: bool,
    pub max_bytes: u64,
    pub timeout: Duration,
}

impl ResultReadPolicy {
    pub fn parse(payload: &Value) -> Result<Self, HeadlessExecutorError> {
        let resolve = match payload.get("resolve_result_artifact") {
            None => true,
            Some(Value::Bool(value)) => *value,
            Some(_) => return Err(failure("resolve_result_artifact must be boolean")),
        };
        Ok(Self {
            resolve,
            max_bytes: bounded_integer(
                payload,
                "result_artifact_max_bytes",
                64 * 1024 * 1024,
                512 * 1024 * 1024,
            )?,
            timeout: Duration::from_millis(bounded_integer(
                payload,
                "result_artifact_timeout_ms",
                600_000,
                600_000,
            )?),
        })
    }
}

fn bounded_integer(
    payload: &Value,
    key: &str,
    default: u64,
    maximum: u64,
) -> Result<u64, HeadlessExecutorError> {
    match payload.get(key) {
        None => Ok(default),
        Some(value) => value
            .as_u64()
            .filter(|value| (1..=maximum).contains(value))
            .ok_or_else(|| failure(format!("{key} requires an integer in 1..={maximum}"))),
    }
}

pub(crate) struct ResultArtifact {
    pub id: String,
    pub size: u64,
    pub solver_method: String,
}

impl ResultArtifact {
    fn parse(value: &Value) -> Result<Option<Self>, HeadlessExecutorError> {
        if value.get("result_artifact_ref").is_none()
            && value.get("schema_version").and_then(Value::as_str) != Some(REFERENCE_SCHEMA)
        {
            return Ok(None);
        }
        if value["schema_version"] != REFERENCE_SCHEMA
            || value["storage_mode"] != "orchestra_content_addressed"
        {
            return Err(failure("unsupported solver result reference contract"));
        }
        let method = value["solver_method"]
            .as_str()
            .filter(|value| {
                !value.is_empty()
                    && value.len() <= 128
                    && value.bytes().all(|byte| {
                        byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_'
                    })
            })
            .ok_or_else(|| failure("invalid solver method in result reference"))?;
        let reference = value["result_artifact_ref"]
            .as_object()
            .ok_or_else(|| failure("missing result artifact descriptor"))?;
        let id = reference
            .get("artifact_id")
            .and_then(Value::as_str)
            .filter(|id| {
                id.len() == 64
                    && id
                        .bytes()
                        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            })
            .ok_or_else(|| failure("artifact identity requires lowercase SHA-256"))?;
        if reference.get("sha256").and_then(Value::as_str) != Some(id)
            || reference.get("schema_version").and_then(Value::as_str) != Some(ARTIFACT_SCHEMA)
            || reference.get("media_type").and_then(Value::as_str) != Some(RESULT_MEDIA_TYPE)
            || reference.get("immutable").and_then(Value::as_bool) != Some(true)
        {
            return Err(failure(
                "result artifact descriptor identity or contract mismatch",
            ));
        }
        let size = reference
            .get("size_bytes")
            .and_then(Value::as_u64)
            .filter(|size| *size > 0)
            .ok_or_else(|| failure("result artifact size requires a positive integer"))?;
        Ok(Some(Self {
            id: id.into(),
            size,
            solver_method: method.into(),
        }))
    }

    fn receipt(&self, policy: ResultReadPolicy) -> Value {
        json!({
            "schema_version":"kyuubiki.headless-result-artifact-readback/v1",
            "verified":true,"solver_method":self.solver_method,
            "max_bytes":policy.max_bytes,"timeout_ms":policy.timeout.as_millis() as u64,
            "artifact":{
                "schema_version":ARTIFACT_SCHEMA,"artifact_id":self.id,"sha256":self.id,
                "size_bytes":self.size,"media_type":RESULT_MEDIA_TYPE,"immutable":true
            }
        })
    }
}

pub(crate) fn resolve_result_artifact(
    base_url: &str,
    token: Option<&str>,
    mut outcome: Value,
    policy: ResultReadPolicy,
) -> Result<Value, HeadlessExecutorError> {
    let Some(artifact) = ResultArtifact::parse(&outcome["result"])? else {
        return Ok(outcome);
    };
    // Reference-only mode still validates the descriptor, but performs no download.
    if !policy.resolve {
        return Ok(outcome);
    }
    if artifact.size > policy.max_bytes {
        return Err(failure(
            "result artifact exceeds the configured byte budget; use reference-only mode or an explicit larger budget",
        ));
    }
    let result = download_result(base_url, token, &artifact, policy)?;
    if !result.is_object() || ResultArtifact::parse(&result)?.is_some() {
        return Err(failure(
            "result content must be a JSON object, not a nested result reference",
        ));
    }
    outcome["result"] = result;
    outcome
        .as_object_mut()
        .expect("normalized result object")
        .insert("result_artifact_readback".into(), artifact.receipt(policy));
    Ok(outcome)
}

pub(crate) fn failure(detail: impl AsRef<str>) -> HeadlessExecutorError {
    HeadlessExecutorError {
        message: format!("{READBACK_FAILURE} {}", detail.as_ref()),
    }
}
