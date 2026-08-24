use crate::RunnerResult;
use serde_json::Value;
use std::fs;
use std::path::Path;

const GENERATION_SCHEMA_VERSION: &str = "kyuubiki.agent-operator-generation-execution/v1";
const CACHE_EVICTION_SCHEMA_PATH: &str = "schemas/agent-operator-cache-eviction.schema.json";
const CACHE_EVICTION_EXAMPLE_PATH: &str = "schemas/examples.agent-operator-cache-eviction.json";
const CACHE_EVICTION_SCHEMA_VERSION: &str = "kyuubiki.agent-operator-cache-eviction/v1";
const JOB_RELEASE_SCHEMA_PATH: &str = "schemas/agent-operator-job-cache-release.schema.json";
const JOB_RELEASE_EXAMPLE_PATH: &str = "schemas/examples.agent-operator-job-cache-release.json";
const JOB_RELEASE_SCHEMA_VERSION: &str = "kyuubiki.agent-operator-job-cache-release/v1";

pub(super) fn check_lifecycle_contracts(root: &Path) -> RunnerResult<Option<String>> {
    if let Some(issue) = check_cache_eviction_contract(root)? {
        return Ok(Some(issue));
    }
    check_job_release_contract(root)
}

fn check_cache_eviction_contract(root: &Path) -> RunnerResult<Option<String>> {
    let schema = read_repo_json(root, CACHE_EVICTION_SCHEMA_PATH)?;
    if schema
        .pointer("/properties/schema_version/const")
        .and_then(Value::as_str)
        != Some(CACHE_EVICTION_SCHEMA_VERSION)
    {
        return Ok(Some(format!(
            "{CACHE_EVICTION_SCHEMA_PATH}: schema_version const must match {CACHE_EVICTION_SCHEMA_VERSION}"
        )));
    }
    for field_name in [
        "schema_version",
        "requested_cache_scope",
        "resolved_cache_policy",
        "disposition",
        "package_id",
        "package_version",
        "remaining_activated_package_count",
        "generation",
    ] {
        if !schema_requires(&schema, "/required", field_name) {
            return Ok(Some(format!(
                "{CACHE_EVICTION_SCHEMA_PATH}: must require {field_name}"
            )));
        }
    }
    for (field_name, expected) in [
        ("requested_cache_scope", "none"),
        ("resolved_cache_policy", "task_required_disposable"),
    ] {
        if schema
            .pointer(&format!("/properties/{field_name}/const"))
            .and_then(Value::as_str)
            != Some(expected)
        {
            return Ok(Some(format!(
                "{CACHE_EVICTION_SCHEMA_PATH}: {field_name} const must be {expected}"
            )));
        }
    }
    let dispositions = schema_enum(&schema, "/properties/disposition/enum");
    if dispositions
        != [
            "evicted_after_execution",
            "superseded_generation_released",
            "retained_by_other_scope",
        ]
    {
        return Ok(Some(format!(
            "{CACHE_EVICTION_SCHEMA_PATH}: disposition enum must preserve all safe outcomes"
        )));
    }
    if schema
        .pointer("/properties/generation/$ref")
        .and_then(Value::as_str)
        != Some("agent-operator-generation-execution.schema.json")
    {
        return Ok(Some(format!(
            "{CACHE_EVICTION_SCHEMA_PATH}: generation must reuse the generation execution schema"
        )));
    }

    let example = read_repo_json(root, CACHE_EVICTION_EXAMPLE_PATH)?;
    for (field_name, expected) in [
        ("schema_version", CACHE_EVICTION_SCHEMA_VERSION),
        ("requested_cache_scope", "none"),
        ("resolved_cache_policy", "task_required_disposable"),
        ("disposition", "evicted_after_execution"),
    ] {
        if field(&example, field_name) != expected {
            return Ok(Some(format!(
                "{CACHE_EVICTION_EXAMPLE_PATH}: {field_name} must be {expected}"
            )));
        }
    }
    for field_name in ["package_id", "package_version"] {
        if field(&example, field_name).is_empty() {
            return Ok(Some(format!(
                "{CACHE_EVICTION_EXAMPLE_PATH}: {field_name} must be non-empty"
            )));
        }
    }
    if !has_non_negative_count(&example, "remaining_activated_package_count") {
        return Ok(Some(format!(
            "{CACHE_EVICTION_EXAMPLE_PATH}: remaining_activated_package_count must be non-negative"
        )));
    }
    if !has_generation_receipt(example.get("generation")) {
        return Ok(Some(format!(
            "{CACHE_EVICTION_EXAMPLE_PATH}: generation must be a complete host-lease receipt"
        )));
    }
    Ok(None)
}

fn check_job_release_contract(root: &Path) -> RunnerResult<Option<String>> {
    let schema = read_repo_json(root, JOB_RELEASE_SCHEMA_PATH)?;
    if schema
        .pointer("/properties/schema_version/const")
        .and_then(Value::as_str)
        != Some(JOB_RELEASE_SCHEMA_VERSION)
    {
        return Ok(Some(format!(
            "{JOB_RELEASE_SCHEMA_PATH}: schema_version const must match {JOB_RELEASE_SCHEMA_VERSION}"
        )));
    }
    for field_name in [
        "schema_version",
        "release_boundary",
        "job_id",
        "disposition",
        "released_package_ids",
        "evicted_package_ids",
        "retained_package_ids",
        "remaining_activated_package_count",
        "generation",
    ] {
        if !schema_requires(&schema, "/required", field_name) {
            return Ok(Some(format!(
                "{JOB_RELEASE_SCHEMA_PATH}: must require {field_name}"
            )));
        }
    }
    if schema
        .pointer("/properties/release_boundary/const")
        .and_then(Value::as_str)
        != Some("explicit_job_terminal_rpc")
    {
        return Ok(Some(format!(
            "{JOB_RELEASE_SCHEMA_PATH}: release_boundary must be explicit_job_terminal_rpc"
        )));
    }
    if schema_enum(&schema, "/properties/disposition/enum")
        != [
            "already_released",
            "released_retained_packages",
            "evicted_after_job_release",
        ]
    {
        return Ok(Some(format!(
            "{JOB_RELEASE_SCHEMA_PATH}: disposition enum must preserve all terminal outcomes"
        )));
    }
    let generation_variants = schema
        .pointer("/properties/generation/oneOf")
        .and_then(Value::as_array);
    let generation_contract_is_bounded = generation_variants.is_some_and(|variants| {
        variants.iter().any(|variant| {
            variant.get("$ref").and_then(Value::as_str)
                == Some("agent-operator-generation-execution.schema.json")
        }) && variants
            .iter()
            .any(|variant| variant.get("type").and_then(Value::as_str) == Some("null"))
    });
    if !generation_contract_is_bounded {
        return Ok(Some(format!(
            "{JOB_RELEASE_SCHEMA_PATH}: generation must be a host-lease receipt or null"
        )));
    }

    let example = read_repo_json(root, JOB_RELEASE_EXAMPLE_PATH)?;
    for (field_name, expected) in [
        ("schema_version", JOB_RELEASE_SCHEMA_VERSION),
        ("release_boundary", "explicit_job_terminal_rpc"),
        ("disposition", "evicted_after_job_release"),
    ] {
        if field(&example, field_name) != expected {
            return Ok(Some(format!(
                "{JOB_RELEASE_EXAMPLE_PATH}: {field_name} must be {expected}"
            )));
        }
    }
    if field(&example, "job_id").is_empty()
        || !has_non_empty_string_array(&example, "released_package_ids")
        || !has_non_empty_string_array(&example, "evicted_package_ids")
        || example
            .get("retained_package_ids")
            .and_then(Value::as_array)
            .is_none()
        || !has_non_negative_count(&example, "remaining_activated_package_count")
    {
        return Ok(Some(format!(
            "{JOB_RELEASE_EXAMPLE_PATH}: terminal release identity and package sets must be complete"
        )));
    }
    if !has_generation_receipt(example.get("generation")) {
        return Ok(Some(format!(
            "{JOB_RELEASE_EXAMPLE_PATH}: generation must be a complete host-lease receipt"
        )));
    }
    Ok(None)
}

fn schema_enum<'a>(schema: &'a Value, pointer: &str) -> Vec<&'a str> {
    schema
        .pointer(pointer)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect()
}

fn schema_requires(schema: &Value, pointer: &str, field_name: &str) -> bool {
    schema_enum(schema, pointer).contains(&field_name)
}

fn has_non_empty_string_array(value: &Value, key: &str) -> bool {
    value
        .get(key)
        .and_then(Value::as_array)
        .is_some_and(|items| {
            !items.is_empty()
                && items
                    .iter()
                    .all(|item| item.as_str().is_some_and(|item| !item.is_empty()))
        })
}

fn has_non_negative_count(value: &Value, key: &str) -> bool {
    value.get(key).and_then(Value::as_u64).is_some()
}

fn has_generation_receipt(value: Option<&Value>) -> bool {
    let Some(generation) = value else {
        return false;
    };
    field(generation, "schema_version") == GENERATION_SCHEMA_VERSION
        && field(generation, "retention_policy") == "host_lease"
        && field(generation, "crash_recovery") == "next_session_start"
        && !field(generation, "session_id").is_empty()
        && !field(generation, "generation_id").is_empty()
}

fn read_repo_json(root: &Path, relative_path: &str) -> RunnerResult<Value> {
    let text = fs::read_to_string(root.join(relative_path))
        .map_err(|error| format!("failed to read {relative_path}: {error}"))?;
    serde_json::from_str(&text).map_err(|error| format!("{relative_path}: invalid json: {error}"))
}

fn field<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or_default()
}
