use serde_json::Value;
use std::ffi::OsString;
use std::fs;
use std::path::Path;

type RunnerResult<T> = Result<T, String>;

const SCHEMA_PATH: &str = "schemas/material-candidate-materialization-plan.schema.json";
const EXAMPLE_PATH: &str = "schemas/examples.material-candidate-materialization-plan.json";
const SDK_README_PATH: &str = "sdks/README.md";
const PLAN_SCHEMA_VERSION: &str = "kyuubiki.material-candidate-materialization-plan/v1";
const SPEC_SCHEMA_VERSION: &str = "kyuubiki.materialized-candidate-spec/v1";
const PLAN_STATUS: &str = "ready_for_solver_rerun";
const SPEC_STATUS: &str = "requires_solver_rerun";

pub(crate) fn run_check_materialization_plan_contract(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    if args.iter().any(|arg| arg == "--self-test") {
        run_self_test(root)?;
        println!("materialization plan contract check self-test passed");
        return Ok(0);
    }
    if !args.is_empty() {
        return Err("check-materialization-plan-contract only accepts --self-test".to_string());
    }
    let issues = check_contracts(root)?;
    if let Some(issue) = issues.first() {
        eprintln!("materialization plan contract check failed: {issue}");
        return Ok(1);
    }
    println!("materialization plan contract check passed");
    Ok(0)
}

fn check_contracts(root: &Path) -> RunnerResult<Vec<String>> {
    let mut issues = Vec::new();
    check_schema(&read_json(root, SCHEMA_PATH)?, &mut issues);
    check_example(&read_json(root, EXAMPLE_PATH)?, &mut issues);
    check_documentation(root, &mut issues)?;
    Ok(issues)
}

fn run_self_test(root: &Path) -> RunnerResult<()> {
    let mut example = read_json(root, EXAMPLE_PATH)?;
    if let Some(count) = example
        .get("materialized_candidate_count")
        .and_then(Value::as_i64)
    {
        example["materialized_candidate_count"] = Value::from(count + 1);
    }
    let mut issues = Vec::new();
    check_example(&example, &mut issues);
    if issues.is_empty() {
        return Err("self-test did not reject candidate count mismatch".to_string());
    }
    Ok(())
}

fn check_schema(schema: &Value, issues: &mut Vec<String>) {
    if schema
        .pointer("/properties/schema_version/const")
        .and_then(Value::as_str)
        != Some(PLAN_SCHEMA_VERSION)
    {
        issues.push(format!(
            "{SCHEMA_PATH}: schema_version const must be {PLAN_SCHEMA_VERSION}"
        ));
    }
    if schema
        .pointer("/properties/status/const")
        .and_then(Value::as_str)
        != Some(PLAN_STATUS)
    {
        issues.push(format!("{SCHEMA_PATH}: status const must be {PLAN_STATUS}"));
    }
    let candidate = schema
        .pointer("/$defs/materializedCandidate")
        .unwrap_or(&Value::Null);
    if candidate
        .pointer("/properties/schema_version/const")
        .and_then(Value::as_str)
        != Some(SPEC_SCHEMA_VERSION)
    {
        issues.push(format!(
            "{SCHEMA_PATH}: candidate schema_version const must be {SPEC_SCHEMA_VERSION}"
        ));
    }
    if candidate
        .pointer("/properties/status/const")
        .and_then(Value::as_str)
        != Some(SPEC_STATUS)
    {
        issues.push(format!(
            "{SCHEMA_PATH}: candidate status const must be {SPEC_STATUS}"
        ));
    }
}

fn check_example(example: &Value, issues: &mut Vec<String>) {
    if field(example, "schema_version") != PLAN_SCHEMA_VERSION {
        issues.push(format!(
            "{EXAMPLE_PATH}: schema_version must be {PLAN_SCHEMA_VERSION}"
        ));
    }
    if field(example, "status") != PLAN_STATUS {
        issues.push(format!("{EXAMPLE_PATH}: status must be {PLAN_STATUS}"));
    }
    let candidates = example
        .get("materialized_candidates")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if !example
        .get("materialized_candidates")
        .is_some_and(Value::is_array)
    {
        issues.push(format!(
            "{EXAMPLE_PATH}: materialized_candidates must be an array"
        ));
    }
    if candidates.is_empty() {
        issues.push(format!(
            "{EXAMPLE_PATH}: materialized_candidates must not be empty"
        ));
    }
    if example
        .get("materialized_candidate_count")
        .and_then(Value::as_u64)
        != Some(candidates.len() as u64)
    {
        issues.push(format!(
            "{EXAMPLE_PATH}: materialized_candidate_count must match candidate length"
        ));
    }
    require_string(
        example,
        "source_request_schema_version",
        EXAMPLE_PATH,
        issues,
    );
    require_string(example, "required_result_schema", EXAMPLE_PATH, issues);
    for (index, candidate) in candidates.iter().enumerate() {
        let context = format!("{EXAMPLE_PATH}#materialized_candidates/{index}");
        if field(candidate, "schema_version") != SPEC_SCHEMA_VERSION {
            issues.push(format!(
                "{context}: schema_version must be {SPEC_SCHEMA_VERSION}"
            ));
        }
        if field(candidate, "status") != SPEC_STATUS {
            issues.push(format!("{context}: status must be {SPEC_STATUS}"));
        }
        for field_name in [
            "candidate_id",
            "source_draft_id",
            "source_candidate_id",
            "strategy",
            "study",
            "required_result_schema",
        ] {
            require_string(candidate, field_name, &context, issues);
        }
    }
}

fn check_documentation(root: &Path, issues: &mut Vec<String>) -> RunnerResult<()> {
    let readme = read_text(root, SDK_README_PATH)?;
    for required in [SCHEMA_PATH, EXAMPLE_PATH] {
        if !readme.contains(required) {
            issues.push(format!("{SDK_README_PATH}: missing link to {required}"));
        }
    }
    Ok(())
}

fn require_string(value: &Value, key: &str, context: &str, issues: &mut Vec<String>) {
    if field(value, key).is_empty() {
        issues.push(format!("{context}: {key} must be a non-empty string"));
    }
}

fn read_json(root: &Path, relative_path: &str) -> RunnerResult<Value> {
    let text = read_text(root, relative_path)?;
    serde_json::from_str(&text).map_err(|error| format!("{relative_path}: invalid json: {error}"))
}

fn read_text(root: &Path, relative_path: &str) -> RunnerResult<String> {
    fs::read_to_string(root.join(relative_path))
        .map_err(|error| format!("failed to read {relative_path}: {error}"))
}

fn field<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{PLAN_STATUS, SPEC_STATUS};

    #[test]
    fn statuses_are_distinct_contract_states() {
        assert_eq!(PLAN_STATUS, "ready_for_solver_rerun");
        assert_eq!(SPEC_STATUS, "requires_solver_rerun");
    }
}
