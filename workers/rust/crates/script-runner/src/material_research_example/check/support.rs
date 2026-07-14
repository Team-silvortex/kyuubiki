use serde_json::Value;
use std::fs;
use std::path::Path;

use super::{RunnerResult, validate_optimization_objectives};

pub(super) fn validate_next_round_lineage(lineage: &Value, manifest: &Value) -> RunnerResult<()> {
    let expected = value(manifest, "expected");
    assert_eq_value(
        lineage.get("schema_version"),
        expected.get("next_round_lineage_schema_version"),
        "run-next lineage schema",
    )?;
    assert_eq_value(
        lineage.get("source_iteration"),
        expected.get("initial_iteration"),
        "lineage source_iteration",
    )?;
    assert_eq_value(
        lineage.get("planned_iteration"),
        expected.get("next_run_iteration"),
        "lineage planned_iteration",
    )?;
    validate_optimization_objectives(
        value(lineage, "optimization_objectives"),
        manifest,
        "run-next.lineage",
    )
}

pub(super) fn validate_convergence_assessment(
    assessment: &Value,
    manifest: &Value,
) -> RunnerResult<()> {
    let expected = value(manifest, "expected");
    assert_eq_value(
        assessment.get("schema_version"),
        expected.get("chain_convergence_schema_version"),
        "chain convergence schema",
    )?;
    assert_eq_value(
        assessment.get("state"),
        expected.get("chain_convergence_state"),
        "chain convergence state",
    )?;
    assert_eq_value(
        assessment.get("winner_stable"),
        Some(&Value::Bool(true)),
        "chain convergence winner_stable",
    )?;
    assert_finite(
        assessment.get("winner_score_delta"),
        "chain convergence winner_score_delta",
    )?;
    if field(assessment, "recommendation").is_empty() {
        return Err("chain convergence assessment must expose recommendation".to_string());
    }
    Ok(())
}

pub(super) fn validate_documentation(root: &Path, manifest: &Value) -> RunnerResult<()> {
    let doc_path = field(manifest, "documentation");
    let markdown = fs::read_to_string(root.join(doc_path))
        .map_err(|error| format!("failed to read {doc_path}: {error}"))?;
    for (context, value) in [
        ("output path", field(manifest, "output_path")),
        ("verify target", field(manifest, "verify_target")),
        (
            "expected winner",
            pointer_str(manifest, "/expected/winner_candidate_id"),
        ),
        (
            "optimization id",
            pointer_str(manifest, "/expected/optimization_id"),
        ),
        (
            "reliability posture",
            pointer_str(manifest, "/expected/reliability_posture"),
        ),
        (
            "next round schema",
            pointer_str(manifest, "/expected/next_round_schema_version"),
        ),
        (
            "chain schema",
            pointer_str(manifest, "/expected/chain_schema_version"),
        ),
    ] {
        if !markdown.contains(value) {
            return Err(format!("documentation missing {context}: {value}"));
        }
    }
    Ok(())
}

pub(super) fn read_json_path(path: &Path, label: &str) -> RunnerResult<Value> {
    let text =
        fs::read_to_string(path).map_err(|error| format!("failed to read {label}: {error}"))?;
    serde_json::from_str(&text).map_err(|error| format!("{label}: invalid json: {error}"))
}

pub(super) fn assert_no_absolute_repo_path(
    root: &Path,
    value: &Value,
    context: &str,
) -> RunnerResult<()> {
    if let Some(text) = value.as_str() {
        if text.contains(root.to_string_lossy().as_ref()) {
            return Err(format!(
                "{context}: contains local absolute repository path"
            ));
        }
    }
    if let Some(items) = value.as_array() {
        for (index, item) in items.iter().enumerate() {
            assert_no_absolute_repo_path(root, item, &format!("{context}[{index}]"))?;
        }
    }
    if let Some(object) = value.as_object() {
        for (key, nested) in object {
            assert_no_absolute_repo_path(root, nested, &format!("{context}.{key}"))?;
        }
    }
    Ok(())
}

pub(super) fn assert_finite(value: Option<&Value>, context: &str) -> RunnerResult<()> {
    if value.and_then(Value::as_f64).is_some_and(f64::is_finite) {
        Ok(())
    } else {
        Err(format!("{context}: expected finite number"))
    }
}

pub(super) fn assert_eq_str(actual: &str, expected: &str, context: &str) -> RunnerResult<()> {
    if actual == expected {
        Ok(())
    } else {
        Err(format!("{context}: expected {expected:?}, got {actual:?}"))
    }
}

pub(super) fn assert_eq_value(
    actual: Option<&Value>,
    expected: Option<&Value>,
    context: &str,
) -> RunnerResult<()> {
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "{context}: expected {}, got {}",
            expected.unwrap_or(&Value::Null),
            actual.unwrap_or(&Value::Null)
        ))
    }
}

pub(super) fn array<'a>(value: &'a Value, key: &str) -> Vec<&'a Value> {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(|items| items.iter().collect())
        .unwrap_or_default()
}

pub(super) fn string_array(value: Option<&Value>) -> Vec<&str> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect()
}

pub(super) fn value<'a>(value: &'a Value, key: &str) -> &'a Value {
    value.get(key).unwrap_or(&Value::Null)
}

fn pointer_str<'a>(value: &'a Value, pointer: &str) -> &'a str {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .unwrap_or_default()
}

pub(super) fn field<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or_default()
}
