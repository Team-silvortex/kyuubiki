use crate::HeadlessExecutorError;
use crate::service_executor::MAX_INLINE_JSON_BYTES;
use crate::service_executor_artifact_http::request_file;
use kyuubiki_protocol::model_artifact_max_bytes;
use serde_json::{Value, json};
use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

const MODEL_ARTIFACT_ROUTE: &str = "/api/v1/model-artifacts";
const MODEL_ARTIFACT_MEDIA_TYPE: &str = "application/vnd.kyuubiki.model+json";
const LARGE_MODEL_ENTITY_PREFLIGHT: usize = 250_000;
static NEXT_TEMPORARY_ARTIFACT: AtomicU64 = AtomicU64::new(1);

pub(crate) fn prepare_direct_fem_request_body(
    base_url: &str,
    api_token: Option<&str>,
    model: &Value,
) -> Result<Value, HeadlessExecutorError> {
    let entity_count = model_entity_count(model);
    if entity_count >= LARGE_MODEL_ENTITY_PREFLIGHT {
        reject_known_frontend_proxy(base_url, format!("entity_count={entity_count}"))?;
    }
    let size_bytes = serialized_json_size(model)?;
    if size_bytes <= MAX_INLINE_JSON_BYTES {
        return Ok(model.clone());
    }
    let limit_bytes = model_artifact_max_bytes();
    if size_bytes > limit_bytes {
        return Err(HeadlessExecutorError {
            message: format!(
                "model_artifact_limit_exceeded: direct FEM model exceeds artifact transport limit: size_bytes={size_bytes} limit_bytes={limit_bytes}"
            ),
        });
    }
    reject_known_frontend_proxy(base_url, format!("size_bytes={size_bytes}"))?;
    let artifact = TemporaryModelArtifact::serialize(model)?;
    if artifact.size_bytes != size_bytes {
        return Err(HeadlessExecutorError {
            message: "direct FEM model changed while preparing artifact transport".to_string(),
        });
    }
    let envelope = request_file(
        base_url,
        api_token,
        "POST",
        MODEL_ARTIFACT_ROUTE,
        MODEL_ARTIFACT_MEDIA_TYPE,
        &artifact.path,
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

fn serialized_json_size(value: &Value) -> Result<usize, HeadlessExecutorError> {
    let mut counter = ByteCounter::default();
    serde_json::to_writer(&mut counter, value).map_err(|error| HeadlessExecutorError {
        message: format!("failed to measure direct FEM model: {error}"),
    })?;
    Ok(counter.size_bytes)
}

#[derive(Default)]
struct ByteCounter {
    size_bytes: usize,
}

impl Write for ByteCounter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.size_bytes = self
            .size_bytes
            .checked_add(bytes.len())
            .ok_or_else(|| std::io::Error::other("serialized model size overflow"))?;
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

struct TemporaryModelArtifact {
    path: PathBuf,
    size_bytes: usize,
}

impl TemporaryModelArtifact {
    fn serialize(value: &Value) -> Result<Self, HeadlessExecutorError> {
        let (path, file) = create_temporary_artifact()?;
        let result = (|| {
            let mut writer = BufWriter::new(file);
            serde_json::to_writer(&mut writer, value).map_err(|error| HeadlessExecutorError {
                message: format!("failed to serialize direct FEM model artifact: {error}"),
            })?;
            writer.flush().map_err(|error| HeadlessExecutorError {
                message: format!("failed to flush direct FEM model artifact: {error}"),
            })?;
            let size_bytes = usize::try_from(
                fs::metadata(&path)
                    .map_err(|error| HeadlessExecutorError {
                        message: format!("failed to inspect direct FEM model artifact: {error}"),
                    })?
                    .len(),
            )
            .map_err(|_| HeadlessExecutorError {
                message: "direct FEM model artifact size exceeds this platform".to_string(),
            })?;
            Ok(Self {
                path: path.clone(),
                size_bytes,
            })
        })();
        if result.is_err() {
            let _ = fs::remove_file(path);
        }
        result
    }
}

impl Drop for TemporaryModelArtifact {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn create_temporary_artifact() -> Result<(PathBuf, File), HeadlessExecutorError> {
    let directory = std::env::temp_dir().join("kyuubiki-headless-model-artifacts");
    fs::create_dir_all(&directory).map_err(|error| HeadlessExecutorError {
        message: format!("failed to create model artifact temporary directory: {error}"),
    })?;
    for _ in 0..16 {
        let id = NEXT_TEMPORARY_ARTIFACT.fetch_add(1, Ordering::Relaxed);
        let path = directory.join(format!("{}-{id}.json", std::process::id()));
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => return Ok((path, file)),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(HeadlessExecutorError {
                    message: format!("failed to create model artifact temporary file: {error}"),
                });
            }
        }
    }
    Err(HeadlessExecutorError {
        message: "failed to allocate a unique model artifact temporary file".to_string(),
    })
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

    #[test]
    fn streaming_size_matches_canonical_json_serialization() {
        let model = json!({"nodes": [{"x": 1.0}], "elements": [], "label": "test"});
        assert_eq!(
            serialized_json_size(&model).expect("measure model"),
            serde_json::to_vec(&model).expect("serialize model").len()
        );

        let artifact = TemporaryModelArtifact::serialize(&model).expect("temporary artifact");
        let path = artifact.path.clone();
        assert_eq!(artifact.size_bytes, serialized_json_size(&model).unwrap());
        assert!(path.is_file());
        drop(artifact);
        assert!(!path.exists());
    }
}
