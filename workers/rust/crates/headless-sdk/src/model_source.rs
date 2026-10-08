use crate::execution_input::{ExactJson, exact_json_sha256};
use crate::service_executor::validate_path_segment;
use serde::ser::SerializeMap;
use serde::{Deserialize, Serialize, Serializer};
use serde_json::Value;

pub const HEADLESS_MODEL_SOURCE_SCHEMA_VERSION: &str = "kyuubiki.headless-model-source/v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HeadlessModelSourceKind {
    Model,
    ModelVersion,
}

/// The fetched saved snapshot, not a signed receipt of engine execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HeadlessModelSource {
    pub schema_version: String,
    pub source_kind: HeadlessModelSourceKind,
    pub project_id: String,
    pub model_id: String,
    pub model_version_id: Option<String>,
    pub kind: String,
    pub sha256: String,
}

impl HeadlessModelSource {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != HEADLESS_MODEL_SOURCE_SCHEMA_VERSION {
            return Err("unsupported model source schema".into());
        }
        for (key, id) in [
            ("project_id", &self.project_id),
            ("model_id", &self.model_id),
        ] {
            validate_path_segment(id, key).map_err(|_| format!("unusable model source {key}"))?;
        }
        if let Some(id) = &self.model_version_id {
            validate_path_segment(id, "model_version_id")
                .map_err(|_| "unusable model source model_version_id".to_string())?;
        } else if self.source_kind == HeadlessModelSourceKind::ModelVersion {
            return Err("version source requires model_version_id".into());
        }
        usable_kind(&self.kind)?;
        if self.sha256.len() != 64
            || !self
                .sha256
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err("model source requires lowercase SHA-256".into());
        }
        Ok(())
    }

    pub(crate) fn decode(value: &Value) -> Result<Self, String> {
        let source =
            Self::deserialize(value).map_err(|_| "invalid model source descriptor".to_string())?;
        source.validate()?;
        Ok(source)
    }
}

/// Fingerprint an explicit `model` or `version` record, not its HTTP envelope.
/// Timestamps and display names are excluded; payload, material, schema and
/// reference identities are bound. No full model serialization is retained.
pub fn headless_saved_model_source(
    record: &Value,
    source_kind: HeadlessModelSourceKind,
) -> Result<HeadlessModelSource, String> {
    let record = record
        .as_object()
        .ok_or("model source requires an explicit object record")?;
    let identity = |key: &str| -> Result<String, String> {
        let id = record
            .get(key)
            .and_then(Value::as_str)
            .ok_or_else(|| format!("model source requires string {key}"))?;
        validate_path_segment(id, key).map_err(|_| format!("unusable model source {key}"))?;
        Ok(id.into())
    };
    let kind = record
        .get("kind")
        .and_then(Value::as_str)
        .ok_or("model source requires a usable kind")?;
    usable_kind(kind)?;
    let payload = record
        .get("payload")
        .filter(|value| value.is_object())
        .ok_or("model source requires an object payload")?;
    let model_version_id = match source_kind {
        HeadlessModelSourceKind::ModelVersion => Some(identity("version_id")?),
        HeadlessModelSourceKind::Model => match record.get("latest_version_id") {
            Some(value) if !value.is_null() => Some(identity("latest_version_id")?),
            _ => None,
        },
    };
    let mut source = HeadlessModelSource {
        schema_version: HEADLESS_MODEL_SOURCE_SCHEMA_VERSION.into(),
        source_kind,
        project_id: identity("project_id")?,
        model_id: identity("model_id")?,
        model_version_id,
        kind: kind.into(),
        sha256: String::new(),
    };
    source.sha256 = exact_json_sha256(
        HEADLESS_MODEL_SOURCE_SCHEMA_VERSION,
        &SourceInput {
            source: &source,
            payload,
            material: record.get("material"),
            model_schema_version: record.get("model_schema_version"),
        },
    )?;
    Ok(source)
}

fn usable_kind(kind: &str) -> Result<(), String> {
    if kind.is_empty() || kind != kind.trim() || kind.chars().any(char::is_control) {
        return Err("model source requires a usable kind".into());
    }
    Ok(())
}

struct SourceInput<'a> {
    source: &'a HeadlessModelSource,
    payload: &'a Value,
    material: Option<&'a Value>,
    model_schema_version: Option<&'a Value>,
}

impl Serialize for SourceInput<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let source = self.source;
        let mut map = serializer.serialize_map(None)?;
        map.serialize_entry("kind", &source.kind)?;
        if let Some(material) = self.material {
            map.serialize_entry("material", &ExactJson(material))?;
        }
        map.serialize_entry("model_id", &source.model_id)?;
        if let Some(schema) = self.model_schema_version {
            map.serialize_entry("model_schema_version", &ExactJson(schema))?;
        }
        map.serialize_entry("model_version_id", &source.model_version_id)?;
        map.serialize_entry("payload", &ExactJson(self.payload))?;
        map.serialize_entry("project_id", &source.project_id)?;
        map.serialize_entry("schema_version", &source.schema_version)?;
        map.serialize_entry("source_kind", &source.source_kind)?;
        map.end()
    }
}

#[cfg(test)]
#[path = "model_source_tests.rs"]
mod tests;
