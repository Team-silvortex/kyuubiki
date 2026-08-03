use crate::HeadlessExecutorError;
use crate::service_executor::{MAX_INLINE_JSON_BYTES, request_bytes};
use serde_json::{Value, json};

const MODEL_ARTIFACT_ROUTE: &str = "/api/v1/model-artifacts";
const MODEL_ARTIFACT_MEDIA_TYPE: &str = "application/vnd.kyuubiki.model+json";
const MAX_MODEL_ARTIFACT_BYTES: usize = 536_870_912;

pub(crate) fn prepare_direct_fem_request_body(
    base_url: &str,
    api_token: Option<&str>,
    model: &Value,
) -> Result<Value, HeadlessExecutorError> {
    let bytes = serde_json::to_vec(model).map_err(|error| HeadlessExecutorError {
        message: format!("failed to serialize direct FEM model: {error}"),
    })?;
    if bytes.len() <= MAX_INLINE_JSON_BYTES {
        return Ok(model.clone());
    }
    if bytes.len() > MAX_MODEL_ARTIFACT_BYTES {
        return Err(HeadlessExecutorError {
            message: format!(
                "direct FEM model exceeds artifact transport limit: size_bytes={} limit_bytes={MAX_MODEL_ARTIFACT_BYTES}",
                bytes.len()
            ),
        });
    }
    let envelope = request_bytes(
        base_url,
        api_token,
        "POST",
        MODEL_ARTIFACT_ROUTE,
        MODEL_ARTIFACT_MEDIA_TYPE,
        &bytes,
    )?;
    let reference = envelope
        .get("artifact")
        .filter(|value| value.get("artifact_id").and_then(Value::as_str).is_some())
        .cloned()
        .ok_or_else(|| HeadlessExecutorError {
            message: "model artifact upload returned an invalid reference".to_string(),
        })?;
    Ok(json!({ "model_artifact_ref": reference }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_small_models_inline() {
        let model = json!({"nodes": [], "elements": []});
        let prepared = prepare_direct_fem_request_body("http://127.0.0.1:1", None, &model)
            .expect("small model does not need a live service");
        assert_eq!(prepared, model);
    }
}
