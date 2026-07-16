use crate::operator_qualification_evidence_kits::load_qualification_evidence_kits;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

type RunnerResult<T> = Result<T, String>;

const MANIFEST_PATH: &str = "config/operator-reliability-manifest.json";
const ROADMAP_PATH: &str = "config/operator-qualification-roadmap.json";
const EVIDENCE_KITS_PATH: &str = "config/operator-qualification-evidence-kits.json";
const MANIFEST_SCHEMA_PATH: &str = "schemas/operator-reliability-manifest.schema.json";
const SHARD_SCHEMA_PATH: &str = "schemas/operator-reliability-shard.schema.json";
const ROADMAP_SCHEMA_PATH: &str = "schemas/operator-qualification-roadmap.schema.json";
const EVIDENCE_KITS_SCHEMA_PATH: &str = "schemas/operator-qualification-evidence-kits.schema.json";
const RELEASE_RECORDS_PATH: &str = "releases/qualification-records/1.20.0.json";
const RELEASE_RECORDS_SCHEMA_PATH: &str =
    "schemas/operator-qualification-release-records.schema.json";
const MAKE_FILES: &[&str] = &[
    "Makefile",
    "make/checks.mk",
    "make/benchmarks.mk",
    "make/tests.mk",
    "make/help.mk",
];

pub(crate) fn run_check_operator_reliability_schemas(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    if args.iter().any(|arg| arg == "--self-test") {
        run_self_test()?;
        println!("operator reliability schema smoke self-test passed");
        return Ok(0);
    }
    if !args.is_empty() {
        return Err("check-operator-reliability-schemas only accepts --self-test".to_string());
    }
    if let Some(issue) = check_all(root)? {
        eprintln!("operator reliability schema check failed: {issue}");
        return Ok(1);
    }
    println!("operator reliability schema smoke passed");
    Ok(0)
}

fn check_all(root: &Path) -> RunnerResult<Option<String>> {
    for contract in schema_contracts() {
        if let Some(issue) = check_schema_contract(root, &contract)? {
            return Ok(Some(issue));
        }
    }
    if let Some(issue) = check_reliability_shards(root)? {
        return Ok(Some(issue));
    }
    check_qualification_roadmap_closure(root)
}

struct SchemaContract {
    config: &'static str,
    schema: &'static str,
}

fn schema_contracts() -> Vec<SchemaContract> {
    vec![
        SchemaContract {
            config: MANIFEST_PATH,
            schema: MANIFEST_SCHEMA_PATH,
        },
        SchemaContract {
            config: ROADMAP_PATH,
            schema: ROADMAP_SCHEMA_PATH,
        },
        SchemaContract {
            config: EVIDENCE_KITS_PATH,
            schema: EVIDENCE_KITS_SCHEMA_PATH,
        },
        SchemaContract {
            config: RELEASE_RECORDS_PATH,
            schema: RELEASE_RECORDS_SCHEMA_PATH,
        },
    ]
}

fn check_schema_contract(root: &Path, contract: &SchemaContract) -> RunnerResult<Option<String>> {
    let config = read_json(root, contract.config)?;
    let schema = read_json(root, contract.schema)?;
    if let Some(issue) = required_field_errors(&config, &schema, contract.config)
        .into_iter()
        .next()
    {
        return Ok(Some(issue));
    }
    let Some(expected_schema_version) = schema
        .pointer("/properties/schema_version/const")
        .and_then(Value::as_str)
    else {
        return Ok(Some(format!(
            "{}: missing schema_version const",
            contract.schema
        )));
    };
    if field(&config, "schema_version") != expected_schema_version {
        return Ok(Some(format!(
            "{}: schema_version must match {}",
            contract.config, contract.schema
        )));
    }
    Ok(None)
}

fn check_reliability_shards(root: &Path) -> RunnerResult<Option<String>> {
    let manifest = read_json(root, MANIFEST_PATH)?;
    let shard_schema = read_json(root, SHARD_SCHEMA_PATH)?;
    let Some(expected_schema_version) = shard_schema
        .pointer("/properties/schema_version/const")
        .and_then(Value::as_str)
    else {
        return Ok(Some(format!(
            "{SHARD_SCHEMA_PATH}: missing schema_version const"
        )));
    };
    let Some(shards) = manifest.get("shards").and_then(Value::as_array) else {
        return Ok(Some(format!("{MANIFEST_PATH}: shards must be non-empty")));
    };
    if shards.is_empty() {
        return Ok(Some(format!("{MANIFEST_PATH}: shards must be non-empty")));
    }
    for shard_path in shards.iter().filter_map(Value::as_str) {
        let shard = read_json(root, shard_path)?;
        if let Some(issue) = required_field_errors(&shard, &shard_schema, shard_path)
            .into_iter()
            .next()
        {
            return Ok(Some(issue));
        }
        if field(&shard, "schema_version") != expected_schema_version {
            return Ok(Some(format!(
                "{shard_path}: schema_version must match reliability shard schema"
            )));
        }
    }
    Ok(None)
}

fn check_qualification_roadmap_closure(root: &Path) -> RunnerResult<Option<String>> {
    let roadmap = read_json(root, ROADMAP_PATH)?;
    let evidence_kits = load_qualification_evidence_kits(root)?;
    if field(&roadmap, "version_line") != field(&evidence_kits, "version_line") {
        return Ok(Some(format!(
            "{ROADMAP_PATH}: version_line must match evidence kits"
        )));
    }
    let mut candidates = HashMap::new();
    for candidate in array(&roadmap, "candidates") {
        let candidate_id = field(candidate, "candidate_id");
        if candidates.contains_key(candidate_id) {
            return Ok(Some(format!(
                "{ROADMAP_PATH}: duplicate candidate {candidate_id}"
            )));
        }
        candidates.insert(candidate_id.to_string(), candidate);
    }
    let mut kits = HashMap::new();
    for kit in array(&evidence_kits, "kits") {
        let candidate_id = field(kit, "candidate_id");
        if kits.contains_key(candidate_id) {
            return Ok(Some(format!(
                "{EVIDENCE_KITS_PATH}: duplicate kit {candidate_id}"
            )));
        }
        kits.insert(candidate_id.to_string(), kit);
    }
    for (candidate_id, candidate) in &candidates {
        let Some(kit) = kits.get(candidate_id) else {
            return Ok(Some(format!("{candidate_id}: missing evidence kit")));
        };
        if sorted_strings(candidate.get("operator_ids")) != sorted_strings(kit.get("operator_ids"))
        {
            return Ok(Some(format!(
                "{candidate_id}: roadmap operator_ids must match evidence kit operator_ids"
            )));
        }
        if array(kit, "artifact_requirements").len() < array(candidate, "required_artifacts").len()
        {
            return Ok(Some(format!(
                "{candidate_id}: evidence kit must not have fewer artifact requirements than roadmap required_artifacts"
            )));
        }
    }
    for candidate_id in kits.keys() {
        if !candidates.contains_key(candidate_id) {
            return Ok(Some(format!(
                "{candidate_id}: evidence kit has no roadmap candidate"
            )));
        }
    }
    check_artifact_requirements(root, &kits, &list_make_targets(root)?)
}

fn check_artifact_requirements(
    root: &Path,
    kits: &HashMap<String, &Value>,
    make_targets: &HashSet<String>,
) -> RunnerResult<Option<String>> {
    for kit in kits.values() {
        for artifact in array(kit, "artifact_requirements") {
            let candidate_id = field(kit, "candidate_id");
            let artifact_id = field(artifact, "artifact_id");
            let artifact_path = field(artifact, "artifact_path");
            if !artifact_path.is_empty() {
                if artifact_path.starts_with('/') || artifact_path.contains("..") {
                    return Ok(Some(format!(
                        "{candidate_id}/{artifact_id}: artifact_path must be repository-relative"
                    )));
                }
                if matches!(field(kit, "status"), "collecting" | "ready_for_review")
                    && !repo_path(root, artifact_path)?.exists()
                {
                    return Ok(Some(format!(
                        "{candidate_id}/{artifact_id}: collecting artifact_path does not exist"
                    )));
                }
            }
            let command = field(artifact, "artifact_command");
            if !command.is_empty() {
                let Some(target) = make_target(command) else {
                    return Ok(Some(format!(
                        "{candidate_id}/{artifact_id}: artifact_command must be a make target"
                    )));
                };
                if !make_targets.contains(target) {
                    return Ok(Some(format!(
                        "{candidate_id}/{artifact_id}: unknown make target {target}"
                    )));
                }
            }
            let check_command = field(artifact, "artifact_check_command");
            if !check_command.is_empty() {
                let Some(target) = make_target(check_command) else {
                    return Ok(Some(format!(
                        "{candidate_id}/{artifact_id}: artifact_check_command must be a make target"
                    )));
                };
                if !make_targets.contains(target) {
                    return Ok(Some(format!(
                        "{candidate_id}/{artifact_id}: unknown check make target {target}"
                    )));
                }
            }
        }
    }
    Ok(None)
}

fn make_target(command: &str) -> Option<&str> {
    let target = command.strip_prefix("make ")?;
    (target.split_whitespace().count() == 1).then_some(target)
}

fn required_field_errors(value: &Value, schema: &Value, context: &str) -> Vec<String> {
    let mut errors = Vec::new();
    if !schema.is_object() {
        return errors;
    }
    if let (Some(required), Some(object)) = (
        schema.get("required").and_then(Value::as_array),
        value.as_object(),
    ) {
        for field in required.iter().filter_map(Value::as_str) {
            if !object.contains_key(field) {
                errors.push(format!("{context}: missing required field {field}"));
            }
        }
    }
    if let (Some(properties), Some(object)) = (
        schema.get("properties").and_then(Value::as_object),
        value.as_object(),
    ) {
        for (field, field_schema) in properties {
            if let Some(child) = object.get(field) {
                errors.extend(required_field_errors(
                    child,
                    field_schema,
                    &format!("{context}.{field}"),
                ));
            }
        }
    }
    if let (Some(item_schema), Some(items)) = (schema.get("items"), value.as_array()) {
        for (index, item) in items.iter().enumerate() {
            errors.extend(required_field_errors(
                item,
                item_schema,
                &format!("{context}[{index}]"),
            ));
        }
    }
    errors
}

fn run_self_test() -> RunnerResult<()> {
    let schema = serde_json::json!({
        "type": "object",
        "required": ["schema_version", "items"],
        "properties": {
            "schema_version": { "const": "self-test/v1" },
            "items": {
                "type": "array",
                "items": {
                    "type": "object",
                    "required": ["id", "nested"],
                    "properties": {
                        "id": { "type": "string" },
                        "nested": {
                            "type": "object",
                            "required": ["value"],
                            "properties": { "value": { "type": "string" } }
                        }
                    }
                }
            }
        }
    });
    let errors = required_field_errors(
        &serde_json::json!({ "items": [{ "id": "ok", "nested": {} }] }),
        &schema,
        "self",
    );
    for expected in [
        "self: missing required field schema_version",
        "self.items[0].nested: missing required field value",
    ] {
        if !errors.iter().any(|error| error == expected) {
            return Err(format!(
                "self-test did not report expected error: {expected}"
            ));
        }
    }
    if !required_field_errors(
        &serde_json::json!({ "schema_version": "self-test/v1", "items": [] }),
        &schema,
        "self",
    )
    .is_empty()
    {
        return Err("self-test valid sample should not report required-field errors".to_string());
    }
    if sorted_vec(vec!["b".to_string(), "a".to_string()]) != vec!["a", "b"] {
        return Err("self-test sorted string comparison failed".to_string());
    }
    if make_target("make check-sample") != Some("check-sample")
        || make_target("make check-sample EXTRA=1").is_some()
        || make_target("cargo test").is_some()
    {
        return Err("self-test make target parser failed".to_string());
    }
    Ok(())
}

fn list_make_targets(root: &Path) -> RunnerResult<HashSet<String>> {
    let mut targets = HashSet::new();
    for file in MAKE_FILES {
        let path = root.join(file);
        if !path.exists() {
            continue;
        }
        let text =
            fs::read_to_string(&path).map_err(|error| format!("failed to read {file}: {error}"))?;
        for line in text.lines() {
            let Some((target, _rest)) = line.split_once(':') else {
                continue;
            };
            if target
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.' | '-'))
                && !target.is_empty()
            {
                targets.insert(target.to_string());
            }
        }
    }
    Ok(targets)
}

fn repo_path(root: &Path, relative_path: &str) -> RunnerResult<PathBuf> {
    let absolute = root.join(relative_path);
    let relative = absolute
        .strip_prefix(root)
        .map_err(|_| format!("path escapes repository: {relative_path}"))?;
    if relative.starts_with("..") || relative.is_absolute() {
        return Err(format!("path escapes repository: {relative_path}"));
    }
    Ok(absolute)
}

fn read_json(root: &Path, relative_path: &str) -> RunnerResult<Value> {
    let text = fs::read_to_string(root.join(relative_path))
        .map_err(|error| format!("failed to read {relative_path}: {error}"))?;
    serde_json::from_str(&text).map_err(|error| format!("{relative_path}: invalid json: {error}"))
}

fn array<'a>(value: &'a Value, key: &str) -> Vec<&'a Value> {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(|items| items.iter().collect())
        .unwrap_or_default()
}

fn sorted_strings(value: Option<&Value>) -> Vec<String> {
    sorted_vec(
        value
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(ToString::to_string)
            .collect(),
    )
}

fn sorted_vec(mut values: Vec<String>) -> Vec<String> {
    values.sort();
    values
}

fn field<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{make_target, required_field_errors, sorted_vec};

    #[test]
    fn required_field_errors_recurse_into_arrays() {
        let schema = serde_json::json!({
            "type": "object",
            "required": ["items"],
            "properties": {
                "items": {
                    "items": {
                        "type": "object",
                        "required": ["id"]
                    }
                }
            }
        });
        let errors = required_field_errors(&serde_json::json!({"items": [{}]}), &schema, "self");
        assert_eq!(errors, vec!["self.items[0]: missing required field id"]);
    }

    #[test]
    fn sorted_vec_orders_strings() {
        assert_eq!(sorted_vec(vec!["b".into(), "a".into()]), vec!["a", "b"]);
    }

    #[test]
    fn make_target_rejects_extra_arguments() {
        assert_eq!(make_target("make check-sample"), Some("check-sample"));
        assert_eq!(make_target("make check-sample EXTRA=1"), None);
        assert_eq!(make_target("cargo test"), None);
    }
}
