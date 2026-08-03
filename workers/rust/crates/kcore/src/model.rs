use std::collections::{BTreeMap, HashSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::canonical;

pub const FORMAT_SCHEMA_VERSION: &str = "kyuubiki.kcore/v1";
pub const EXPORT_SCHEMA_VERSION: &str = "kyuubiki.kcore-export/v1";
pub const MEDIA_TYPE: &str = "application/vnd.kyuubiki.kcore";
pub const FORMAT_VERSION: u32 = 1;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Producer {
    pub name: String,
    pub version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SchemaReference {
    pub schema: String,
    pub version: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ContractBinding {
    pub name: String,
    pub schema_version: String,
    pub artifact_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub purpose: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExportArtifact {
    pub id: String,
    pub role: String,
    pub media_type: String,
    pub source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema_ref: Option<SchemaReference>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encoding: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub shape: Vec<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub metadata: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExportSpec {
    pub schema_version: String,
    pub core_id: String,
    pub title: String,
    pub kind: String,
    pub producer: Producer,
    pub artifacts: Vec<ExportArtifact>,
    pub contracts: Vec<ContractBinding>,
    pub entrypoints: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(default)]
    pub provenance: Value,
    #[serde(default)]
    pub metadata: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Artifact {
    pub id: String,
    pub role: String,
    pub media_type: String,
    pub object_path: String,
    pub byte_length: u64,
    pub sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema_ref: Option<SchemaReference>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encoding: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub shape: Vec<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub metadata: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Integrity {
    pub algorithm: String,
    pub core_digest_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema_version: String,
    pub format: String,
    pub format_version: u32,
    pub core_id: String,
    pub title: String,
    pub kind: String,
    pub producer: Producer,
    pub artifacts: Vec<Artifact>,
    pub contracts: Vec<ContractBinding>,
    pub entrypoints: Vec<String>,
    pub integrity: Integrity,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(default)]
    pub provenance: Value,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub metadata: BTreeMap<String, Value>,
}

impl Manifest {
    pub fn object_path(sha256: &str) -> String {
        format!("objects/{}/{}", &sha256[..2], sha256)
    }

    pub fn compute_core_digest(&self) -> Result<String, String> {
        let mut value = serde_json::to_value(self)
            .map_err(|error| format!("failed to serialize kcore manifest: {error}"))?;
        value["integrity"]["core_digest_sha256"] = Value::String(String::new());
        Ok(canonical::sha256(canonical::json(&value).as_bytes()))
    }

    pub fn seal(&mut self) -> Result<(), String> {
        self.integrity.core_digest_sha256 = self.compute_core_digest()?;
        Ok(())
    }

    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut issues = Vec::new();
        require_eq(
            &self.schema_version,
            FORMAT_SCHEMA_VERSION,
            "schema_version",
            &mut issues,
        );
        require_eq(&self.format, "kcore", "format", &mut issues);
        if self.format_version != FORMAT_VERSION {
            issues.push(format!("format_version must be {FORMAT_VERSION}"));
        }
        require_token(&self.core_id, "core_id", &mut issues);
        require_text(&self.title, "title", &mut issues);
        require_token(&self.kind, "kind", &mut issues);
        require_text(&self.producer.name, "producer.name", &mut issues);
        require_text(&self.producer.version, "producer.version", &mut issues);
        if self.artifacts.is_empty() {
            issues.push("artifacts must not be empty".to_string());
        }
        if self.entrypoints.is_empty() {
            issues.push("entrypoints must not be empty".to_string());
        }
        validate_artifacts(self, &mut issues);
        validate_references(self, &mut issues);
        validate_integrity(self, &mut issues);
        if let Ok(value) = serde_json::to_value(self) {
            reject_host_paths(&value, "$", &mut issues);
        }
        if issues.is_empty() {
            Ok(())
        } else {
            Err(issues)
        }
    }
}

pub(crate) fn validate_export_spec(spec: &ExportSpec) -> Result<(), Vec<String>> {
    let mut issues = Vec::new();
    require_eq(
        &spec.schema_version,
        EXPORT_SCHEMA_VERSION,
        "schema_version",
        &mut issues,
    );
    require_token(&spec.core_id, "core_id", &mut issues);
    require_text(&spec.title, "title", &mut issues);
    require_token(&spec.kind, "kind", &mut issues);
    require_text(&spec.producer.name, "producer.name", &mut issues);
    require_text(&spec.producer.version, "producer.version", &mut issues);
    if spec.artifacts.is_empty() {
        issues.push("artifacts must not be empty".to_string());
    }
    if spec.entrypoints.is_empty() {
        issues.push("entrypoints must not be empty".to_string());
    }
    let mut ids = HashSet::new();
    for (index, artifact) in spec.artifacts.iter().enumerate() {
        let base = format!("artifacts[{index}]");
        require_token(&artifact.id, &format!("{base}.id"), &mut issues);
        require_token(&artifact.role, &format!("{base}.role"), &mut issues);
        require_media_type(
            &artifact.media_type,
            &format!("{base}.media_type"),
            &mut issues,
        );
        require_text(&artifact.source, &format!("{base}.source"), &mut issues);
        if !ids.insert(artifact.id.as_str()) {
            issues.push(format!("duplicate artifact id: {}", artifact.id));
        }
    }
    validate_reference_ids(&ids, &spec.entrypoints, &spec.contracts, &mut issues);
    if issues.is_empty() {
        Ok(())
    } else {
        Err(issues)
    }
}

fn validate_artifacts(manifest: &Manifest, issues: &mut Vec<String>) {
    let mut ids = HashSet::new();
    let mut objects = BTreeMap::<&str, u64>::new();
    let mut total = 0_u64;
    for (index, artifact) in manifest.artifacts.iter().enumerate() {
        let base = format!("artifacts[{index}]");
        require_token(&artifact.id, &format!("{base}.id"), issues);
        require_token(&artifact.role, &format!("{base}.role"), issues);
        require_media_type(&artifact.media_type, &format!("{base}.media_type"), issues);
        if let Some(name) = &artifact.name {
            require_text(name, &format!("{base}.name"), issues);
        }
        if let Some(schema_ref) = &artifact.schema_ref {
            require_text(
                &schema_ref.schema,
                &format!("{base}.schema_ref.schema"),
                issues,
            );
            require_text(
                &schema_ref.version,
                &format!("{base}.schema_ref.version"),
                issues,
            );
        }
        if let Some(encoding) = &artifact.encoding {
            require_text(encoding, &format!("{base}.encoding"), issues);
        }
        if let Some(unit) = &artifact.unit {
            require_text(unit, &format!("{base}.unit"), issues);
        }
        if !is_sha256(&artifact.sha256) {
            issues.push(format!("{base}.sha256 must be lowercase SHA-256"));
        } else if artifact.object_path != Manifest::object_path(&artifact.sha256) {
            issues.push(format!("{base}.object_path is not content-addressed"));
        }
        if !ids.insert(artifact.id.as_str()) {
            issues.push(format!("duplicate artifact id: {}", artifact.id));
        }
        if let Some(length) = objects.insert(&artifact.sha256, artifact.byte_length)
            && length != artifact.byte_length
        {
            issues.push(format!(
                "{base} conflicts with the shared object byte length"
            ));
        }
        match total.checked_add(artifact.byte_length) {
            Some(next) => total = next,
            None => issues.push("artifact byte lengths overflow u64".to_string()),
        }
    }
}

fn validate_references(manifest: &Manifest, issues: &mut Vec<String>) {
    let ids = manifest
        .artifacts
        .iter()
        .map(|item| item.id.as_str())
        .collect();
    validate_reference_ids(&ids, &manifest.entrypoints, &manifest.contracts, issues);
}

fn validate_reference_ids(
    ids: &HashSet<&str>,
    entrypoints: &[String],
    contracts: &[ContractBinding],
    issues: &mut Vec<String>,
) {
    let mut entrypoint_ids = HashSet::new();
    for id in entrypoints {
        if !entrypoint_ids.insert(id) {
            issues.push(format!("duplicate entrypoint: {id}"));
        }
        if !ids.contains(id.as_str()) {
            issues.push(format!("entrypoint references unknown artifact: {id}"));
        }
    }
    let mut contract_names = HashSet::new();
    for contract in contracts {
        require_token(&contract.name, "contracts[].name", issues);
        require_text(
            &contract.schema_version,
            "contracts[].schema_version",
            issues,
        );
        if !contract_names.insert(contract.name.as_str()) {
            issues.push(format!("duplicate contract name: {}", contract.name));
        }
        if !ids.contains(contract.artifact_id.as_str()) {
            issues.push(format!(
                "contract {} references unknown artifact: {}",
                contract.name, contract.artifact_id
            ));
        }
    }
}

fn validate_integrity(manifest: &Manifest, issues: &mut Vec<String>) {
    if manifest.integrity.algorithm != "sha256" {
        issues.push("integrity.algorithm must be sha256".to_string());
    }
    if !is_sha256(&manifest.integrity.core_digest_sha256) {
        issues.push("integrity.core_digest_sha256 must be lowercase SHA-256".to_string());
        return;
    }
    match manifest.compute_core_digest() {
        Ok(actual) if actual != manifest.integrity.core_digest_sha256 => {
            issues.push("integrity.core_digest_sha256 does not match manifest".to_string());
        }
        Err(error) => issues.push(error),
        _ => {}
    }
}

fn reject_host_paths(value: &Value, path: &str, issues: &mut Vec<String>) {
    match value {
        Value::String(text) if looks_like_host_path(text) => {
            issues.push(format!("{path} contains a host-absolute path"));
        }
        Value::Array(values) => {
            for (index, value) in values.iter().enumerate() {
                reject_host_paths(value, &format!("{path}[{index}]"), issues);
            }
        }
        Value::Object(values) => {
            for (key, value) in values {
                reject_host_paths(value, &format!("{path}.{key}"), issues);
            }
        }
        _ => {}
    }
}

fn looks_like_host_path(value: &str) -> bool {
    let normalized = value.replace('\\', "/");
    let lowercase = normalized.to_ascii_lowercase();
    normalized.starts_with('/')
        || normalized.starts_with("~/")
        || normalized.starts_with("$HOME/")
        || normalized.starts_with("${HOME}/")
        || lowercase.starts_with("file://")
        || lowercase.starts_with("%userprofile%/")
        || normalized.as_bytes().get(1) == Some(&b':')
            && normalized.as_bytes().get(2) == Some(&b'/')
}

fn require_eq(value: &str, expected: &str, field: &str, issues: &mut Vec<String>) {
    if value != expected {
        issues.push(format!("{field} must be {expected}"));
    }
}

fn require_text(value: &str, field: &str, issues: &mut Vec<String>) {
    if value.trim().is_empty() {
        issues.push(format!("{field} must not be empty"));
    }
}

fn require_token(value: &str, field: &str, issues: &mut Vec<String>) {
    let valid = !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'));
    if !valid {
        issues.push(format!("{field} must be a portable token"));
    }
}

fn require_media_type(value: &str, field: &str, issues: &mut Vec<String>) {
    if value.trim() != value
        || !value.contains('/')
        || value.bytes().any(|byte| byte.is_ascii_whitespace())
    {
        issues.push(format!("{field} must be a media type"));
    }
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn manifest() -> Manifest {
        let mut manifest = Manifest {
            schema_version: FORMAT_SCHEMA_VERSION.to_string(),
            format: "kcore".to_string(),
            format_version: FORMAT_VERSION,
            core_id: "study-1".to_string(),
            title: "Study".to_string(),
            kind: "simulation-result".to_string(),
            producer: Producer {
                name: "test".to_string(),
                version: "1".to_string(),
                runtime: None,
            },
            artifacts: vec![Artifact {
                id: "result".to_string(),
                role: "result".to_string(),
                media_type: "application/json".to_string(),
                object_path: Manifest::object_path(&"a".repeat(64)),
                byte_length: 2,
                sha256: "a".repeat(64),
                name: None,
                schema_ref: None,
                encoding: Some("json".to_string()),
                shape: vec![],
                unit: None,
                metadata: BTreeMap::new(),
            }],
            contracts: vec![],
            entrypoints: vec!["result".to_string()],
            integrity: Integrity {
                algorithm: "sha256".to_string(),
                core_digest_sha256: String::new(),
            },
            created_at: None,
            provenance: json!({}),
            metadata: BTreeMap::new(),
        };
        manifest.seal().expect("seal manifest");
        manifest
    }

    #[test]
    fn sealed_manifest_validates() {
        manifest().validate().expect("valid manifest");
    }

    #[test]
    fn changed_manifest_breaks_core_digest() {
        let mut manifest = manifest();
        manifest.title = "Tampered".to_string();
        assert!(
            manifest
                .validate()
                .expect_err("must reject")
                .iter()
                .any(|issue| issue.contains("does not match"))
        );
    }

    #[test]
    fn host_paths_are_rejected() {
        let mut manifest = manifest();
        manifest.provenance = json!({"source": "/Volumes/lab/private.json"});
        manifest.seal().expect("reseal");
        assert!(
            manifest
                .validate()
                .expect_err("must reject")
                .iter()
                .any(|issue| issue.contains("host-absolute"))
        );
    }
}
