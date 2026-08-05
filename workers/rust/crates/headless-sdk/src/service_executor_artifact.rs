use crate::HeadlessExecutorError;
use crate::service_executor::{MAX_INLINE_JSON_BYTES, request_bytes};
use serde_json::{Value, json};

const MODEL_ARTIFACT_ROUTE: &str = "/api/v1/model-artifacts";
const MODEL_ARTIFACT_MEDIA_TYPE: &str = "application/vnd.kyuubiki.model+json";
const MAX_MODEL_ARTIFACT_BYTES: usize = 536_870_912;
const LARGE_MODEL_ENTITY_PREFLIGHT: usize = 250_000;

pub(crate) fn prepare_direct_fem_request_body(
    base_url: &str,
    api_token: Option<&str>,
    model: &Value,
) -> Result<Value, HeadlessExecutorError> {
    let entity_count = model_entity_count(model);
    if entity_count >= LARGE_MODEL_ENTITY_PREFLIGHT {
        reject_known_frontend_proxy(base_url, format!("entity_count={entity_count}"))?;
    }
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
    reject_known_frontend_proxy(base_url, format!("size_bytes={}", bytes.len()))?;
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

fn reject_known_frontend_proxy(
    base_url: &str,
    model_detail: String,
) -> Result<(), HeadlessExecutorError> {
    let base_url = base_url.trim_end_matches('/');
    if !matches!(base_url, "http://127.0.0.1:3000" | "http://localhost:3000") {
        return Ok(());
    }
    Err(HeadlessExecutorError {
        message: format!(
            "frontend_proxy_artifact_limit: direct FEM model requires artifact transport: {model_detail}; connect headless to the runtime control-plane endpoint (default http://127.0.0.1:4000), not the local GUI frontend"
        ),
    })
}

fn model_entity_count(model: &Value) -> usize {
    ["nodes", "elements"]
        .into_iter()
        .filter_map(|key| model.get(key).and_then(Value::as_array))
        .map(Vec::len)
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_small_models_inline() {
        let model = json!({"nodes": [], "elements": []});
        let prepared = prepare_direct_fem_request_body("http://127.0.0.1:3000", None, &model)
            .expect("small model does not need a live service");
        assert_eq!(prepared, model);
    }

    #[test]
    fn large_artifacts_fail_fast_on_known_local_frontend_proxies() {
        for base_url in ["http://127.0.0.1:3000", "http://localhost:3000/"] {
            let error = reject_known_frontend_proxy(base_url, "size_bytes=42000000".to_string())
                .expect_err("known GUI proxy must not receive large artifacts");
            assert!(error.message.contains("frontend_proxy_artifact_limit"));
            assert!(error.message.contains("size_bytes=42000000"));
            assert!(error.message.contains("127.0.0.1:4000"));
        }
        assert!(
            reject_known_frontend_proxy("http://127.0.0.1:4000", "size_bytes=42000000".to_string())
                .is_ok()
        );
        assert!(
            reject_known_frontend_proxy(
                "http://runtime.example:3000",
                "size_bytes=42000000".to_string()
            )
            .is_ok()
        );
    }

    #[test]
    fn counts_large_model_entities_without_serializing_the_model() {
        let model = json!({"nodes": [1, 2, 3], "elements": [4, 5]});
        assert_eq!(model_entity_count(&model), 5);
    }
}
