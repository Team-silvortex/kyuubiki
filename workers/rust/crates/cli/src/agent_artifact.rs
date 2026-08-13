use std::any::Any;
use std::fs::{self, File, OpenOptions};
use std::io::{BufReader, BufWriter, Write};
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::de::DeserializeOwned;
use serde_json::Value;
use sha2::{Digest, Sha256};

use kyuubiki_protocol::{
    SolveElectrostaticPlaneQuad2dRequest, SolveElectrostaticPlaneTriangle2dRequest,
    SolveHeatPlaneQuad2dRequest, SolveHeatPlaneTriangle2dRequest, model_artifact_max_bytes,
};

use crate::agent_http::{cluster_auth_headers, get_to_writer, normalize_base_url};
use crate::config::AgentConfig;

#[derive(Clone)]
pub(crate) struct ArtifactTransportConfig {
    pub(crate) orchestrator_url: String,
    pub(crate) cluster_api_token: Option<String>,
    pub(crate) agent_id: String,
    pub(crate) cluster_id: Option<String>,
    pub(crate) agent_fingerprint: Option<String>,
}

static FETCH_CONFIG: OnceLock<ArtifactTransportConfig> = OnceLock::new();

pub(crate) fn configure(config: &AgentConfig) {
    let Some(orchestrator_url) = config.orchestrator_url.as_ref() else {
        return;
    };
    let _ = FETCH_CONFIG.set(ArtifactTransportConfig {
        orchestrator_url: normalize_base_url(orchestrator_url),
        cluster_api_token: config.cluster_api_token.clone(),
        agent_id: config
            .agent_id
            .clone()
            .unwrap_or_else(|| format!("local-agent-{}", config.port)),
        cluster_id: config.cluster_id.clone(),
        agent_fingerprint: config.agent_fingerprint.clone(),
    });
}

pub(crate) fn transport_config() -> Result<&'static ArtifactTransportConfig, String> {
    FETCH_CONFIG
        .get()
        .ok_or_else(|| "agent artifact transport requires KYUUBIKI_ORCHESTRATOR_URL".to_string())
}

pub(crate) fn decode_solver_params<T: DeserializeOwned + 'static>(
    params: Value,
) -> Result<T, String> {
    let Some(reference) = params.get("model_artifact_ref") else {
        let mut request = serde_json::from_value(params)
            .map_err(|error| format!("failed to decode inline solver parameters: {error}"))?;
        normalize_compatibility_ids(&mut request);
        return Ok(request);
    };
    let config = transport_config()?;
    decode_artifact_reference(reference, config)
}

fn decode_artifact_reference<T: DeserializeOwned + 'static>(
    reference: &Value,
    config: &ArtifactTransportConfig,
) -> Result<T, String> {
    let artifact_id = required_digest(reference, "artifact_id")?;
    let expected_digest = reference
        .get("sha256")
        .and_then(Value::as_str)
        .unwrap_or(&artifact_id)
        .to_ascii_lowercase();
    if expected_digest != artifact_id {
        return Err("model artifact reference digest does not match artifact_id".to_string());
    }
    let declared_size = reference
        .get("size_bytes")
        .and_then(Value::as_u64)
        .and_then(|size| usize::try_from(size).ok())
        .ok_or_else(|| "model artifact reference requires size_bytes".to_string())?;
    let max_artifact_bytes = model_artifact_max_bytes();
    if declared_size == 0 || declared_size > max_artifact_bytes {
        return Err(format!(
            "model artifact size is outside the supported range: size_bytes={declared_size} limit_bytes={max_artifact_bytes}"
        ));
    }

    let temporary_path = temporary_artifact_path(&artifact_id)?;
    let result = fetch_verify_and_decode(
        reference,
        config,
        &artifact_id,
        &expected_digest,
        declared_size,
        max_artifact_bytes,
        &temporary_path,
    );
    let _ = fs::remove_file(&temporary_path);
    result
}

fn fetch_verify_and_decode<T: DeserializeOwned + 'static>(
    _reference: &Value,
    config: &ArtifactTransportConfig,
    artifact_id: &str,
    expected_digest: &str,
    declared_size: usize,
    max_artifact_bytes: usize,
    temporary_path: &PathBuf,
) -> Result<T, String> {
    let file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(temporary_path)
        .map_err(|error| format!("failed to create temporary model artifact: {error}"))?;
    let mut writer = DigestWriter::new(BufWriter::new(file));
    let url = format!(
        "{}/api/v1/model-artifacts/{artifact_id}/content",
        config.orchestrator_url
    );
    let received = get_to_writer(
        &url,
        cluster_auth_headers(
            config.cluster_api_token.as_deref(),
            &config.agent_id,
            config.cluster_id.as_deref(),
            config.agent_fingerprint.as_deref(),
        ),
        max_artifact_bytes,
        &mut writer,
    )?;
    let digest = writer.finish()?;
    if received != declared_size {
        return Err(format!(
            "model artifact size mismatch: received_bytes={received} declared_bytes={declared_size}"
        ));
    }
    if digest != expected_digest {
        return Err("model artifact SHA-256 verification failed".to_string());
    }

    let file = File::open(temporary_path)
        .map_err(|error| format!("failed to reopen model artifact: {error}"))?;
    let mut request = serde_json::from_reader(BufReader::new(file))
        .map_err(|error| format!("failed to decode model artifact: {error}"))?;
    normalize_compatibility_ids(&mut request);
    Ok(request)
}

fn normalize_compatibility_ids<T: 'static>(request: &mut T) {
    let request = request as &mut dyn Any;
    if let Some(model) = request.downcast_mut::<SolveHeatPlaneQuad2dRequest>() {
        fill_model_ids(&mut model.nodes, &mut model.elements);
    } else if let Some(model) = request.downcast_mut::<SolveHeatPlaneTriangle2dRequest>() {
        fill_model_ids(&mut model.nodes, &mut model.elements);
    } else if let Some(model) = request.downcast_mut::<SolveElectrostaticPlaneQuad2dRequest>() {
        fill_model_ids(&mut model.nodes, &mut model.elements);
    } else if let Some(model) = request.downcast_mut::<SolveElectrostaticPlaneTriangle2dRequest>() {
        fill_model_ids(&mut model.nodes, &mut model.elements);
    }
}

fn fill_model_ids<Node, Element>(nodes: &mut [Node], elements: &mut [Element])
where
    Node: EntityWithId,
    Element: EntityWithId,
{
    fill_entity_ids(nodes, "n");
    fill_entity_ids(elements, "e");
}

fn fill_entity_ids<T: EntityWithId>(entities: &mut [T], prefix: &str) {
    for (index, entity) in entities.iter_mut().enumerate() {
        if entity.id().trim().is_empty() {
            *entity.id_mut() = format!("{prefix}{index}");
        }
    }
}

trait EntityWithId {
    fn id(&self) -> &str;
    fn id_mut(&mut self) -> &mut String;
}

macro_rules! entity_with_id {
    ($type:ty) => {
        impl EntityWithId for $type {
            fn id(&self) -> &str {
                &self.id
            }

            fn id_mut(&mut self) -> &mut String {
                &mut self.id
            }
        }
    };
}

entity_with_id!(kyuubiki_protocol::HeatPlaneNodeInput);
entity_with_id!(kyuubiki_protocol::HeatPlaneQuadElementInput);
entity_with_id!(kyuubiki_protocol::HeatPlaneTriangleElementInput);
entity_with_id!(kyuubiki_protocol::ElectrostaticPlaneNodeInput);
entity_with_id!(kyuubiki_protocol::ElectrostaticPlaneQuadElementInput);
entity_with_id!(kyuubiki_protocol::ElectrostaticPlaneTriangleElementInput);

fn required_digest(reference: &Value, field: &str) -> Result<String, String> {
    let value = reference
        .get(field)
        .and_then(Value::as_str)
        .map(str::to_ascii_lowercase)
        .ok_or_else(|| format!("model artifact reference requires {field}"))?;
    if value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Ok(value)
    } else {
        Err(format!("model artifact reference has invalid {field}"))
    }
}

fn temporary_artifact_path(artifact_id: &str) -> Result<PathBuf, String> {
    let directory = std::env::temp_dir().join("kyuubiki-agent-model-artifacts");
    fs::create_dir_all(&directory)
        .map_err(|error| format!("failed to create agent artifact directory: {error}"))?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    Ok(directory.join(format!("{artifact_id}-{}-{nonce}.json", std::process::id())))
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
            .map_err(|error| format!("failed to flush model artifact: {error}"))?;
        Ok(self
            .hasher
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect())
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
    use super::{decode_solver_params, normalize_compatibility_ids, required_digest};
    use kyuubiki_protocol::{
        SolveElectrostaticPlaneQuad2dRequest, SolveElectrostaticPlaneTriangle2dRequest,
        SolveHeatPlaneQuad2dRequest,
    };
    use serde_json::json;

    #[test]
    fn validates_content_addressed_artifact_ids() {
        let digest = "a".repeat(64);
        assert_eq!(
            required_digest(&json!({"artifact_id": digest}), "artifact_id").unwrap(),
            digest
        );
        assert!(required_digest(&json!({"artifact_id": "../model"}), "artifact_id").is_err());
    }

    #[test]
    fn restores_graph_ids_for_canonical_heat_artifacts() {
        let mut request: SolveHeatPlaneQuad2dRequest = serde_json::from_value(json!({
            "nodes": [
                {"x": 0.0, "y": 0.0, "fix_temperature": true},
                {"id": "kept", "x": 1.0, "y": 0.0, "fix_temperature": false}
            ],
            "elements": [{
                "node_i": 0, "node_j": 1, "node_k": 1, "node_l": 0,
                "thickness": 1.0, "conductivity": 1.0
            }]
        }))
        .unwrap();

        normalize_compatibility_ids(&mut request);

        assert_eq!(request.nodes[0].id, "n0");
        assert_eq!(request.nodes[1].id, "kept");
        assert_eq!(request.elements[0].id, "e0");
    }

    #[test]
    fn restores_graph_ids_for_inline_electrostatic_models() {
        let quad: SolveElectrostaticPlaneQuad2dRequest = decode_solver_params(json!({
            "nodes": [
                {"x": 0.0, "y": 0.0, "fix_potential": true},
                {"id": "kept", "x": 1.0, "y": 0.0, "fix_potential": false}
            ],
            "elements": [{
                "node_i": 0, "node_j": 1, "node_k": 1, "node_l": 0,
                "thickness": 1.0, "permittivity": 1.0
            }]
        }))
        .unwrap();
        let triangle: SolveElectrostaticPlaneTriangle2dRequest = decode_solver_params(json!({
            "nodes": [
                {"x": 0.0, "y": 0.0, "fix_potential": true},
                {"x": 1.0, "y": 0.0, "fix_potential": false},
                {"x": 0.0, "y": 1.0, "fix_potential": false}
            ],
            "elements": [{
                "node_i": 0, "node_j": 1, "node_k": 2,
                "thickness": 1.0, "permittivity": 1.0
            }]
        }))
        .unwrap();

        assert_eq!(quad.nodes[0].id, "n0");
        assert_eq!(quad.nodes[1].id, "kept");
        assert_eq!(quad.elements[0].id, "e0");
        assert_eq!(triangle.nodes[2].id, "n2");
        assert_eq!(triangle.elements[0].id, "e0");
    }
}
