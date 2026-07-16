use crate::material_research_bundle_index::validate_material_research_bundle_index_value;
use serde_json::Value;
use std::ffi::OsString;
use std::fs;
use std::path::Path;

type RunnerResult<T> = Result<T, String>;

const SCHEMA_PATH: &str = "schemas/material-research-bundle-index.schema.json";
const EXAMPLE_PATH: &str = "schemas/examples.material-research-bundle-index.json";
const SCHEMAS_README_PATH: &str = "schemas/README.md";
const SCRIPTS_README_PATH: &str = "scripts/README.md";
const DOCS_PATH: &str = "docs/automated-material-research-example.md";
const INDEX_SCHEMA_VERSION: &str = "kyuubiki.material-research-bundle-index/v1";

pub(crate) fn run_check_material_research_bundle_index_contract(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    if args.iter().any(|arg| arg == "--self-test") {
        run_self_test(root)?;
        println!("material research bundle index contract check self-test passed");
        return Ok(0);
    }
    if !args.is_empty() {
        return Err(
            "check-material-research-bundle-index-contract only accepts --self-test".to_string(),
        );
    }
    let mut issues = Vec::new();
    check_schema(&read_json(root, SCHEMA_PATH)?, &mut issues);
    check_example(&read_json(root, EXAMPLE_PATH)?, &mut issues);
    check_documentation(root, &mut issues)?;
    if let Some(issue) = issues.first() {
        eprintln!("material research bundle index contract check failed: {issue}");
        Ok(1)
    } else {
        println!("material research bundle index contract check passed");
        Ok(0)
    }
}

fn run_self_test(root: &Path) -> RunnerResult<()> {
    let mut bad = read_json(root, EXAMPLE_PATH)?;
    bad["winner_changed_in_chain_count"] = Value::from(0);
    let mut issues = Vec::new();
    check_example(&bad, &mut issues);
    if issues.is_empty() {
        return Err("self-test did not reject winner drift count mismatch".to_string());
    }
    Ok(())
}

fn check_schema(schema: &Value, issues: &mut Vec<String>) {
    if schema
        .pointer("/properties/schema_version/const")
        .and_then(Value::as_str)
        != Some(INDEX_SCHEMA_VERSION)
    {
        issues.push(format!(
            "{SCHEMA_PATH}: schema_version const must be {INDEX_SCHEMA_VERSION}"
        ));
    }
    for field in [
        "bundle_count",
        "studies",
        "winner_changed_in_chain_count",
        "reliability_decision_counts",
        "next_round_decision_counts",
        "bundles",
    ] {
        if !required_fields(schema)
            .iter()
            .any(|required| *required == field)
        {
            issues.push(format!("{SCHEMA_PATH}: missing required field {field}"));
        }
    }
    let entry_required = schema
        .pointer("/$defs/bundleIndexEntry/required")
        .and_then(Value::as_array)
        .map(|items| items.iter().filter_map(Value::as_str).collect::<Vec<_>>())
        .unwrap_or_default();
    for field in [
        "winner_candidate_id",
        "final_winner_candidate_id",
        "winner_changed_in_chain",
        "primary_metric_ids",
        "violated_quality_gate_ids",
        "focus_candidate_ids",
        "chain_trace_round_count",
        "validation_posture",
        "external_validation_required",
        "baseline_ref_count",
        "acceptance_criteria_count",
        "candidate_confidence_counts",
        "validation_readiness_decision",
        "validation_readiness_score",
        "validation_blocking_reasons",
        "next_validation_action_count",
    ] {
        if !entry_required.iter().any(|required| *required == field) {
            issues.push(format!(
                "{SCHEMA_PATH}: bundleIndexEntry missing required {field}"
            ));
        }
    }
}

fn check_example(example: &Value, issues: &mut Vec<String>) {
    if let Err(issue) = validate_material_research_bundle_index_value(example) {
        issues.push(format!("{EXAMPLE_PATH}: {issue}"));
    }
    if example.get("schema_version").and_then(Value::as_str) != Some(INDEX_SCHEMA_VERSION) {
        issues.push(format!(
            "{EXAMPLE_PATH}: schema_version must be {INDEX_SCHEMA_VERSION}"
        ));
    }
    if example
        .get("bundles")
        .and_then(Value::as_array)
        .is_none_or(Vec::is_empty)
    {
        issues.push(format!("{EXAMPLE_PATH}: bundles must be non-empty"));
    }
}

fn check_documentation(root: &Path, issues: &mut Vec<String>) -> RunnerResult<()> {
    let schemas_readme = read_text(root, SCHEMAS_README_PATH)?;
    for required_path in [SCHEMA_PATH, EXAMPLE_PATH] {
        let file_name = Path::new(required_path)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(required_path);
        if !schemas_readme.contains(file_name) {
            issues.push(format!(
                "{SCHEMAS_README_PATH}: missing entry for {file_name}"
            ));
        }
    }
    for doc_path in [SCRIPTS_README_PATH, DOCS_PATH] {
        let text = read_text(root, doc_path)?;
        for required_path in [SCHEMA_PATH, EXAMPLE_PATH] {
            if !text.contains(required_path) {
                issues.push(format!("{doc_path}: missing reference to {required_path}"));
            }
        }
    }
    Ok(())
}

fn required_fields(schema: &Value) -> Vec<&str> {
    schema
        .get("required")
        .and_then(Value::as_array)
        .map(|fields| fields.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default()
}

fn read_json(root: &Path, relative_path: &str) -> RunnerResult<Value> {
    let text = read_text(root, relative_path)?;
    serde_json::from_str(&text).map_err(|error| format!("{relative_path}: invalid json: {error}"))
}

fn read_text(root: &Path, relative_path: &str) -> RunnerResult<String> {
    fs::read_to_string(root.join(relative_path))
        .map_err(|error| format!("failed to read {relative_path}: {error}"))
}
