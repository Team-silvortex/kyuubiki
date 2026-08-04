use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::agent_artifact::transport_config;
use crate::agent_http::{cluster_auth_headers, post_file};

const RESULT_MEDIA_TYPE: &str = "application/vnd.kyuubiki.result+json";

pub(crate) fn upload<T: Serialize>(method: &str, result: &T) -> Result<Value, String> {
    let config = transport_config()?;
    let temporary_path = temporary_result_path()?;
    let outcome = serialize_and_upload(method, result, &temporary_path, config);
    let _ = fs::remove_file(&temporary_path);
    outcome
}

fn serialize_and_upload<T: Serialize>(
    method: &str,
    result: &T,
    temporary_path: &Path,
    config: &crate::agent_artifact::ArtifactTransportConfig,
) -> Result<Value, String> {
    let file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(temporary_path)
        .map_err(|error| format!("failed to create temporary result artifact: {error}"))?;
    let mut writer = DigestWriter::new(BufWriter::new(file));
    serde_json::to_writer(&mut writer, result)
        .map_err(|error| format!("failed to serialize result artifact: {error}"))?;
    let digest = writer.finish()?;
    let size_bytes = File::open(temporary_path)
        .and_then(|file| file.metadata())
        .map_err(|error| format!("failed to inspect result artifact: {error}"))?
        .len();

    let url = format!("{}/api/v1/result-artifacts", config.orchestrator_url);
    let response = post_file(
        &url,
        RESULT_MEDIA_TYPE,
        temporary_path,
        cluster_auth_headers(
            config.cluster_api_token.as_deref(),
            &config.agent_id,
            config.cluster_id.as_deref(),
            config.agent_fingerprint.as_deref(),
        ),
    )?;
    let response: Value = serde_json::from_slice(&response)
        .map_err(|error| format!("failed to decode result artifact response: {error}"))?;
    let artifact = response
        .get("artifact")
        .cloned()
        .ok_or_else(|| "result artifact response omitted artifact descriptor".to_string())?;
    validate_descriptor(&artifact, &digest, size_bytes)?;

    Ok(serde_json::json!({
        "schema_version": "kyuubiki.solver-result-reference/v1",
        "solver_method": method,
        "storage_mode": "orchestra_content_addressed",
        "result_artifact_ref": artifact
    }))
}

fn validate_descriptor(artifact: &Value, digest: &str, size_bytes: u64) -> Result<(), String> {
    if artifact.get("sha256").and_then(Value::as_str) != Some(digest) {
        return Err("result artifact server returned a mismatched digest".to_string());
    }
    if artifact.get("size_bytes").and_then(Value::as_u64) != Some(size_bytes) {
        return Err("result artifact server returned a mismatched size".to_string());
    }
    Ok(())
}

fn temporary_result_path() -> Result<PathBuf, String> {
    let directory = std::env::temp_dir().join("kyuubiki-agent-result-artifacts");
    fs::create_dir_all(&directory)
        .map_err(|error| format!("failed to create result artifact directory: {error}"))?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    Ok(directory.join(format!("result-{}-{nonce}.json", std::process::id())))
}

struct DigestWriter<W> {
    inner: W,
    hasher: Sha256,
}

impl<W: Write> DigestWriter<W> {
    fn new(inner: W) -> Self {
        Self {
            inner,
            hasher: Sha256::new(),
        }
    }

    fn finish(mut self) -> Result<String, String> {
        self.inner
            .flush()
            .map_err(|error| format!("failed to flush result artifact: {error}"))?;
        Ok(format!("{:x}", self.hasher.finalize()))
    }
}

impl<W: Write> Write for DigestWriter<W> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let written = self.inner.write(bytes)?;
        self.hasher.update(&bytes[..written]);
        Ok(written)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::validate_descriptor;
    use serde_json::json;

    #[test]
    fn validates_uploaded_result_descriptor() {
        let digest = "a".repeat(64);
        let artifact = json!({"sha256": digest, "size_bytes": 42});
        assert!(validate_descriptor(&artifact, &digest, 42).is_ok());
        assert!(validate_descriptor(&artifact, &digest, 41).is_err());
    }
}
