use crate::HeadlessExecutorError;
use crate::service_executor::MAX_INLINE_JSON_BYTES;
use crate::service_executor_artifact_http::request_file;
use kyuubiki_protocol::model_artifact_max_bytes;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

const MODEL_ARTIFACT_ROUTE: &str = "/api/v1/model-artifacts";
const MODEL_ARTIFACT_MEDIA_TYPE: &str = "application/vnd.kyuubiki.model+json";
const MODEL_ARTIFACT_SCHEMA: &str = "kyuubiki.model-artifact-ref/v1";
pub(crate) const INVALID_UPLOAD_RECEIPT: &str = "model_artifact_receipt_invalid:";
const LARGE_MODEL_ENTITY_PREFLIGHT: usize = 250_000;
static NEXT_TEMPORARY_ARTIFACT: AtomicU64 = AtomicU64::new(1);

pub(crate) struct PreparedDirectFemModel {
    pub body: Value,
    pub upload: Option<Value>,
}

pub(crate) fn prepare_direct_fem_request_body(
    base_url: &str,
    api_token: Option<&str>,
    model: &Value,
) -> Result<PreparedDirectFemModel, HeadlessExecutorError> {
    let entity_count = model_entity_count(model);
    if entity_count >= LARGE_MODEL_ENTITY_PREFLIGHT {
        reject_known_frontend_proxy(base_url, format!("entity_count={entity_count}"))?;
    }
    let size_bytes = serialized_json_size(model)?;
    if size_bytes <= MAX_INLINE_JSON_BYTES {
        return Ok(PreparedDirectFemModel {
            body: model.clone(),
            upload: None,
        });
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
    let reference = upload_model_artifact(base_url, api_token, &artifact)?;
    Ok(PreparedDirectFemModel {
        body: json!({"model_artifact_ref":reference}),
        upload: Some(reference),
    })
}

fn upload_model_artifact(
    base_url: &str,
    api_token: Option<&str>,
    artifact: &TemporaryModelArtifact,
) -> Result<Value, HeadlessExecutorError> {
    let uploaded = request_file(
        base_url,
        api_token,
        "POST",
        MODEL_ARTIFACT_ROUTE,
        MODEL_ARTIFACT_MEDIA_TYPE,
        &artifact.path,
    )?;
    if uploaded.size_bytes != artifact.size_bytes as u64 || uploaded.sha256 != artifact.sha256 {
        return Err(invalid_upload(
            "prepared model differs from transmitted artifact bytes",
        ));
    }
    validate_upload_receipt(&uploaded.envelope, &uploaded.sha256, uploaded.size_bytes)
}

fn validate_upload_receipt(
    envelope: &Value,
    sha256: &str,
    size_bytes: u64,
) -> Result<Value, HeadlessExecutorError> {
    let reference = envelope
        .get("artifact")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            invalid_upload("upload acknowledgement requires an explicit artifact object")
        })?;
    for (key, expected) in [
        ("schema_version", MODEL_ARTIFACT_SCHEMA),
        ("artifact_id", sha256),
        ("sha256", sha256),
        ("media_type", MODEL_ARTIFACT_MEDIA_TYPE),
    ] {
        if reference.get(key).and_then(Value::as_str) != Some(expected) {
            return Err(invalid_upload(&format!(
                "upload acknowledgement contradicts {key}"
            )));
        }
    }
    if reference.get("size_bytes").and_then(Value::as_u64) != Some(size_bytes)
        || reference.get("immutable").and_then(Value::as_bool) != Some(true)
    {
        return Err(invalid_upload(
            "upload acknowledgement requires matching size and immutability",
        ));
    }
    // Forward only the verified protocol fields, not unrelated reply extensions.
    Ok(
        json!({"schema_version":MODEL_ARTIFACT_SCHEMA,"artifact_id":sha256,"sha256":sha256,
        "size_bytes":size_bytes,"media_type":MODEL_ARTIFACT_MEDIA_TYPE,"immutable":true}),
    )
}

fn invalid_upload(detail: &str) -> HeadlessExecutorError {
    HeadlessExecutorError {
        message: format!("{INVALID_UPLOAD_RECEIPT} {detail}"),
    }
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
    sha256: String,
}

impl TemporaryModelArtifact {
    fn serialize(value: &Value) -> Result<Self, HeadlessExecutorError> {
        let (path, file) = create_temporary_artifact()?;
        let result = (|| {
            let mut writer = BufWriter::new(ArtifactWriter {
                file,
                digest: Sha256::new(),
                size_bytes: 0,
            });
            serde_json::to_writer(&mut writer, value).map_err(|error| HeadlessExecutorError {
                message: format!("failed to serialize direct FEM model artifact: {error}"),
            })?;
            let writer = writer.into_inner().map_err(|error| HeadlessExecutorError {
                message: format!("failed to flush direct FEM model artifact: {error}"),
            })?;
            Ok(Self {
                path: path.clone(),
                size_bytes: writer.size_bytes,
                sha256: format!("{:x}", writer.digest.finalize()),
            })
        })();
        if result.is_err() {
            let _ = fs::remove_file(path);
        }
        result
    }
}

struct ArtifactWriter {
    file: File,
    digest: Sha256,
    size_bytes: usize,
}

impl Write for ArtifactWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let written = self.file.write(bytes)?;
        self.digest.update(&bytes[..written]);
        self.size_bytes = self
            .size_bytes
            .checked_add(written)
            .ok_or_else(|| std::io::Error::other("model artifact size overflow"))?;
        Ok(written)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.file.flush()
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
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        match options.open(&path) {
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
#[path = "service_executor_artifact_tests.rs"]
mod tests;
