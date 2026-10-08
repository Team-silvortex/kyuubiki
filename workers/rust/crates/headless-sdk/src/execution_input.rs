use crate::{HeadlessExecutionBatch, HeadlessExecutionBatchStep};
use serde::ser::{SerializeMap, SerializeSeq};
use serde::{Deserialize, Serialize, Serializer};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::io::{self, BufWriter, Write};

pub const HEADLESS_EXECUTION_INPUT_SCHEMA_VERSION: &str = "kyuubiki.headless-execution-input/v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HeadlessExecutionInputFingerprint {
    pub schema_version: String,
    pub sha256: String,
}

/// Hash the effective batch before dispatch or report compaction. Warnings are
/// diagnostic only; all other batch fields and lossless JSON numbers are bound.
/// This is input identity, not an authenticated execution receipt.
pub fn headless_execution_input_fingerprint(
    batch: &HeadlessExecutionBatch,
) -> Result<HeadlessExecutionInputFingerprint, String> {
    Ok(HeadlessExecutionInputFingerprint {
        schema_version: HEADLESS_EXECUTION_INPUT_SCHEMA_VERSION.to_string(),
        sha256: exact_json_sha256(HEADLESS_EXECUTION_INPUT_SCHEMA_VERSION, &BatchInput(batch))?,
    })
}

pub(crate) fn exact_json_sha256<T: Serialize>(domain: &str, input: &T) -> Result<String, String> {
    let mut digest = Sha256::new();
    digest.update(domain.as_bytes());
    digest.update([0]);
    let mut writer = BufWriter::with_capacity(16 * 1024, HashWriter(digest));
    serde_json::to_writer(&mut writer, input)
        .map_err(|error| format!("failed to encode exact input fingerprint: {error}"))?;
    let digest = writer
        .into_inner()
        .map_err(|error| format!("failed to finish exact input fingerprint: {error}"))?
        .0;
    Ok(format!("{:x}", digest.finalize()))
}

struct HashWriter(Sha256);

impl Write for HashWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.update(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

struct BatchInput<'a>(&'a HeadlessExecutionBatch);

impl Serialize for BatchInput<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        // Exhaustive destructuring makes new batch fields a deliberate contract change.
        let HeadlessExecutionBatch {
            schema_version,
            exported_at,
            language,
            workflow_id,
            template_id,
            steps,
            warnings: _,
        } = self.0;
        let mut map = serializer.serialize_map(Some(6 + usize::from(template_id.is_some())))?;
        map.serialize_entry("exported_at", exported_at)?;
        map.serialize_entry("language", language)?;
        map.serialize_entry("schema_version", schema_version)?;
        map.serialize_entry("steps", &BatchSteps(steps))?;
        if let Some(template_id) = template_id {
            map.serialize_entry("template_id", template_id)?;
        }
        map.serialize_entry("warnings", &[] as &[String])?;
        map.serialize_entry("workflow_id", workflow_id)?;
        map.end()
    }
}

struct BatchSteps<'a>(&'a [HeadlessExecutionBatchStep]);

impl Serialize for BatchSteps<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut sequence = serializer.serialize_seq(Some(self.0.len()))?;
        for step in self.0 {
            sequence.serialize_element(&BatchStep(step))?;
        }
        sequence.end()
    }
}

struct BatchStep<'a>(&'a HeadlessExecutionBatchStep);

impl Serialize for BatchStep<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let HeadlessExecutionBatchStep {
            index,
            action,
            risk,
            payload,
        } = self.0;
        let mut map = serializer.serialize_map(Some(4))?;
        map.serialize_entry("action", action)?;
        map.serialize_entry("index", index)?;
        map.serialize_entry("payload", &ExactJson(payload))?;
        map.serialize_entry("risk", risk)?;
        map.end()
    }
}

pub(crate) struct ExactJson<'a>(pub(crate) &'a Value);

impl Serialize for ExactJson<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self.0 {
            Value::Array(values) => {
                let mut sequence = serializer.serialize_seq(Some(values.len()))?;
                for value in values {
                    sequence.serialize_element(&ExactJson(value))?;
                }
                sequence.end()
            }
            Value::Object(values) => {
                let mut map = serializer.serialize_map(Some(values.len()))?;
                let sorted = values
                    .keys()
                    .zip(values.keys().skip(1))
                    .all(|(a, b)| a <= b);
                if sorted {
                    for (key, value) in values {
                        map.serialize_entry(key, &ExactJson(value))?;
                    }
                } else {
                    let mut entries = values.iter().collect::<Vec<_>>();
                    entries.sort_unstable_by(|(a, _), (b, _)| a.cmp(b));
                    for (key, value) in entries {
                        map.serialize_entry(key, &ExactJson(value))?;
                    }
                }
                map.end()
            }
            value => value.serialize(serializer),
        }
    }
}

#[cfg(test)]
#[path = "execution_input_tests.rs"]
mod tests;
