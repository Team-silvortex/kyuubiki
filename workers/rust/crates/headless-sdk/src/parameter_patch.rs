use crate::HeadlessExecutionBatch;
use kyuubiki_protocol::canonical_json_sha256;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;

pub const HEADLESS_PARAMETER_PATCH_SCHEMA_VERSION: &str = "kyuubiki.headless-parameter-patch/v1";
pub const HEADLESS_PARAMETER_PATCH_RECEIPT_SCHEMA_VERSION: &str =
    "kyuubiki.headless-parameter-patch-receipt/v1";

const MAX_PATCH_CHANGES: usize = 256;
const MAX_PATCH_PATH_BYTES: usize = 1_024;
const MAX_PATCH_SERIALIZED_BYTES: usize = 8 * 1_024 * 1_024;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeadlessParameterPatch {
    pub schema_version: String,
    pub patch_id: String,
    pub workflow_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template_id: Option<String>,
    pub changes: Vec<HeadlessParameterChange>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeadlessParameterChange {
    pub path: String,
    pub expected: Value,
    pub value: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeadlessParameterPatchReceipt {
    pub schema_version: String,
    pub patch_id: String,
    pub workflow_id: String,
    pub change_count: usize,
    pub before_sha256: String,
    pub after_sha256: String,
}

pub fn apply_parameter_patch(
    batch: &mut HeadlessExecutionBatch,
    patch: &HeadlessParameterPatch,
) -> Result<HeadlessParameterPatchReceipt, String> {
    validate_patch_header(batch, patch)?;
    let mut paths = BTreeSet::new();
    let before = serde_json::to_value(&*batch)
        .map_err(|error| format!("failed to encode headless batch before patch: {error}"))?;
    let before_sha256 = canonical_json_sha256(&before);
    let mut candidate = before.clone();

    for (index, change) in patch.changes.iter().enumerate() {
        validate_change_path(batch, index, &change.path)?;
        if !paths.insert(change.path.as_str()) {
            return Err(format!(
                "parameter patch {} contains duplicate path {}",
                patch.patch_id, change.path
            ));
        }
        let actual = candidate.pointer(&change.path).ok_or_else(|| {
            format!(
                "parameter patch {} change {} path does not exist: {}",
                patch.patch_id,
                index + 1,
                change.path
            )
        })?;
        if actual != &change.expected {
            return Err(format!(
                "parameter patch {} change {} baseline mismatch at {}: expected {}, actual {}",
                patch.patch_id,
                index + 1,
                change.path,
                describe_value(&change.expected),
                describe_value(actual)
            ));
        }
        if actual == &change.value {
            return Err(format!(
                "parameter patch {} change {} is a no-op at {}",
                patch.patch_id,
                index + 1,
                change.path
            ));
        }
        *candidate.pointer_mut(&change.path).ok_or_else(|| {
            format!(
                "parameter patch {} change {} path is not writable: {}",
                patch.patch_id,
                index + 1,
                change.path
            )
        })? = change.value.clone();
    }

    let after_sha256 = canonical_json_sha256(&candidate);
    let mut patched = serde_json::from_value::<HeadlessExecutionBatch>(candidate)
        .map_err(|error| format!("parameter patch produced an invalid headless batch: {error}"))?;
    let receipt = HeadlessParameterPatchReceipt {
        schema_version: HEADLESS_PARAMETER_PATCH_RECEIPT_SCHEMA_VERSION.to_string(),
        patch_id: patch.patch_id.clone(),
        workflow_id: batch.workflow_id.clone(),
        change_count: patch.changes.len(),
        before_sha256,
        after_sha256,
    };
    patched.warnings.push(format!(
        "parameter_patch:{} changes={} before_sha256={} after_sha256={}",
        receipt.patch_id, receipt.change_count, receipt.before_sha256, receipt.after_sha256
    ));
    *batch = patched;
    Ok(receipt)
}

fn validate_patch_header(
    batch: &HeadlessExecutionBatch,
    patch: &HeadlessParameterPatch,
) -> Result<(), String> {
    let encoded_size = serde_json::to_vec(patch)
        .map_err(|error| format!("failed to encode headless parameter patch: {error}"))?
        .len();
    if encoded_size > MAX_PATCH_SERIALIZED_BYTES {
        return Err(format!(
            "headless parameter patch exceeds the {MAX_PATCH_SERIALIZED_BYTES}-byte limit"
        ));
    }
    if patch.schema_version != HEADLESS_PARAMETER_PATCH_SCHEMA_VERSION {
        return Err(format!(
            "unsupported headless parameter patch schema: {}",
            patch.schema_version
        ));
    }
    if patch.patch_id.trim().is_empty()
        || patch.patch_id.len() > 128
        || !patch
            .patch_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return Err("headless parameter patch_id must be a safe non-empty identifier".to_string());
    }
    if patch.workflow_id != batch.workflow_id {
        return Err(format!(
            "parameter patch workflow mismatch: patch={}, batch={}",
            patch.workflow_id, batch.workflow_id
        ));
    }
    if let Some(template_id) = &patch.template_id {
        if batch.template_id.as_ref() != Some(template_id) {
            return Err(format!(
                "parameter patch template mismatch: patch={}, batch={}",
                template_id,
                batch.template_id.as_deref().unwrap_or("none")
            ));
        }
    }
    if patch.changes.is_empty() || patch.changes.len() > MAX_PATCH_CHANGES {
        return Err(format!(
            "headless parameter patch must contain between 1 and {MAX_PATCH_CHANGES} changes"
        ));
    }
    Ok(())
}

fn validate_change_path(
    batch: &HeadlessExecutionBatch,
    change_index: usize,
    path: &str,
) -> Result<(), String> {
    if path.len() > MAX_PATCH_PATH_BYTES {
        return Err(format!(
            "parameter patch change {} path exceeds {MAX_PATCH_PATH_BYTES} bytes",
            change_index + 1
        ));
    }
    let segments = path.split('/').collect::<Vec<_>>();
    if path.bytes().any(|byte| byte < b' ' || byte == 0x7f)
        || segments.len() < 5
        || segments[0] != ""
        || segments[1] != "steps"
        || segments[2..]
            .iter()
            .any(|segment| !is_canonical_pointer_segment(segment))
    {
        return Err(restricted_path_error(change_index, path));
    }
    let step_text = segments[2];
    let step_index = step_text
        .parse::<usize>()
        .map_err(|_| restricted_path_error(change_index, path))?;
    if step_index >= batch.steps.len() || segments[3] != "payload" || segments[4].is_empty() {
        return Err(restricted_path_error(change_index, path));
    }
    Ok(())
}

fn is_canonical_pointer_segment(segment: &str) -> bool {
    if segment.is_empty() || segment.starts_with('+') {
        return false;
    }
    if segment.bytes().all(|byte| byte.is_ascii_digit())
        && segment.len() > 1
        && segment.starts_with('0')
    {
        return false;
    }
    let bytes = segment.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'~' {
            index += 1;
            if index >= bytes.len() || !matches!(bytes[index], b'0' | b'1') {
                return false;
            }
        }
        index += 1;
    }
    true
}

fn restricted_path_error(change_index: usize, path: &str) -> String {
    format!(
        "parameter patch change {} may only replace an existing /steps/<zero-based-index>/payload/... path, got {}",
        change_index + 1,
        path
    )
}

fn describe_value(value: &Value) -> String {
    match value {
        Value::Null | Value::Bool(_) | Value::Number(_) => {
            serde_json::to_string(value).unwrap_or_else(|_| "<unprintable>".to_string())
        }
        Value::String(text) => format!(
            "<string bytes={} sha256={}>",
            text.len(),
            canonical_json_sha256(value)
        ),
        Value::Array(items) => format!(
            "<array items={} sha256={}>",
            items.len(),
            canonical_json_sha256(value)
        ),
        Value::Object(fields) => format!(
            "<object fields={} sha256={}>",
            fields.len(),
            canonical_json_sha256(value)
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{HeadlessExecutionBatchStep, HeadlessRisk};
    use serde_json::json;

    fn batch() -> HeadlessExecutionBatch {
        HeadlessExecutionBatch {
            schema_version: "kyuubiki.headless-execution-batch/v1".to_string(),
            exported_at: "1970-01-01T00:00:00.000Z".to_string(),
            language: "en".to_string(),
            workflow_id: "template.direct_thermal_frame_3d".to_string(),
            template_id: Some("direct_thermal_frame_3d".to_string()),
            steps: vec![HeadlessExecutionBatchStep {
                index: 1,
                action: "solve_thermal_frame_3d".to_string(),
                risk: HeadlessRisk::Normal,
                payload: json!({"model": {"nodes": [{"load_y": 0.0}, {"load_y": -1000.0}]}}),
            }],
            warnings: vec![],
        }
    }

    fn patch(expected: Value, value: Value) -> HeadlessParameterPatch {
        HeadlessParameterPatch {
            schema_version: HEADLESS_PARAMETER_PATCH_SCHEMA_VERSION.to_string(),
            patch_id: "thermal-load-round-2".to_string(),
            workflow_id: "template.direct_thermal_frame_3d".to_string(),
            template_id: Some("direct_thermal_frame_3d".to_string()),
            changes: vec![HeadlessParameterChange {
                path: "/steps/0/payload/model/nodes/1/load_y".to_string(),
                expected,
                value,
            }],
        }
    }

    #[test]
    fn applies_guarded_payload_change_and_records_distinct_fingerprints() {
        let mut batch = batch();
        let receipt = apply_parameter_patch(&mut batch, &patch(json!(-1000.0), json!(-1250.0)))
            .expect("patch");

        assert_eq!(
            batch.steps[0].payload["model"]["nodes"][1]["load_y"],
            -1250.0
        );
        assert_ne!(receipt.before_sha256, receipt.after_sha256);
        assert_eq!(receipt.change_count, 1);
        assert!(batch.warnings[0].contains("thermal-load-round-2"));
    }

    #[test]
    fn rejects_baseline_drift_no_op_and_non_payload_paths() {
        let mut drift = batch();
        assert!(
            apply_parameter_patch(&mut drift, &patch(json!(-900.0), json!(-1250.0)))
                .expect_err("drift")
                .contains("baseline mismatch")
        );

        let mut no_op = batch();
        assert!(
            apply_parameter_patch(&mut no_op, &patch(json!(-1000.0), json!(-1000.0)))
                .expect_err("no-op")
                .contains("no-op")
        );

        let mut restricted = batch();
        let mut invalid = patch(json!("solve_thermal_frame_3d"), json!("service_health"));
        invalid.changes[0].path = "/steps/0/action".to_string();
        assert!(
            apply_parameter_patch(&mut restricted, &invalid)
                .expect_err("restricted path")
                .contains("may only replace")
        );

        let mut aliased = batch();
        let mut leading_zero = patch(json!(-1000.0), json!(-1250.0));
        leading_zero.changes[0].path = "/steps/0/payload/model/nodes/01/load_y".to_string();
        assert!(
            apply_parameter_patch(&mut aliased, &leading_zero)
                .expect_err("non-canonical path")
                .contains("may only replace")
        );

        let mut duplicated = batch();
        let mut duplicate = patch(json!(-1000.0), json!(-1250.0));
        duplicate.changes.push(duplicate.changes[0].clone());
        assert!(
            apply_parameter_patch(&mut duplicated, &duplicate)
                .expect_err("duplicate")
                .contains("duplicate path")
        );

        let mut redacted = batch();
        let secret = apply_parameter_patch(
            &mut redacted,
            &patch(json!("private-baseline"), json!(-1250.0)),
        )
        .expect_err("string mismatch");
        assert!(!secret.contains("private-baseline"));
        assert!(secret.contains("<string bytes="));
    }

    #[test]
    fn rejects_wrong_target_and_missing_path_without_partial_mutation() {
        let original = batch();
        let mut target = original.clone();
        let mut wrong_workflow = patch(json!(-1000.0), json!(-1250.0));
        wrong_workflow.workflow_id = "template.other".to_string();
        assert!(
            apply_parameter_patch(&mut target, &wrong_workflow)
                .expect_err("workflow mismatch")
                .contains("workflow mismatch")
        );
        assert_eq!(target, original);

        let mut missing = patch(json!(-1000.0), json!(-1250.0));
        missing.changes[0].path = "/steps/0/payload/model/nodes/0/missing".to_string();
        assert!(
            apply_parameter_patch(&mut target, &missing)
                .expect_err("missing")
                .contains("does not exist")
        );
        assert_eq!(target, original);
    }

    #[test]
    fn schemas_and_example_share_the_runtime_contract() {
        let patch_schema: Value = serde_json::from_str(include_str!(
            "../../../../../schemas/headless-parameter-patch.schema.json"
        ))
        .expect("patch schema");
        let receipt_schema: Value = serde_json::from_str(include_str!(
            "../../../../../schemas/headless-parameter-patch-receipt.schema.json"
        ))
        .expect("receipt schema");
        let example: HeadlessParameterPatch = serde_json::from_str(include_str!(
            "../../../../../schemas/examples.headless-parameter-patch.json"
        ))
        .expect("patch example");

        assert_eq!(
            patch_schema["properties"]["schema_version"]["const"],
            HEADLESS_PARAMETER_PATCH_SCHEMA_VERSION
        );
        assert_eq!(
            receipt_schema["properties"]["schema_version"]["const"],
            HEADLESS_PARAMETER_PATCH_RECEIPT_SCHEMA_VERSION
        );
        let mut example_batch = batch();
        let receipt = apply_parameter_patch(&mut example_batch, &example).expect("example applies");
        assert_eq!(
            receipt.schema_version,
            HEADLESS_PARAMETER_PATCH_RECEIPT_SCHEMA_VERSION
        );
    }
}
