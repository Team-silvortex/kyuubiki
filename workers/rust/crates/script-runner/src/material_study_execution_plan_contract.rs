use serde_json::Value;
use std::ffi::OsString;
use std::fs;
use std::path::Path;

type RunnerResult<T> = Result<T, String>;

const SCHEMA_PATH: &str = "schemas/material-study-execution-plan.schema.json";
const EXAMPLE_PATH: &str = "schemas/examples.material-study-execution-plan.json";
const SDK_README_PATH: &str = "sdks/README.md";
const SDK_EXAMPLE_PATHS: &[&str] = &[
    "sdks/rust/examples/plan_material_study.rs",
    "sdks/python/examples/plan_material_study.py",
    "sdks/elixir/examples/plan_material_study.exs",
];
const SDK_README_PATHS: &[&str] = &[
    "sdks/rust/README.md",
    "sdks/python/README.md",
    "sdks/elixir/README.md",
];
const PLAN_SCHEMA_VERSION: &str = "kyuubiki.material-study-execution-plan/v1";
const MATERIAL_CARD_SCHEMA_VERSION: &str = "kyuubiki.material-card/v1";

pub(crate) fn run_check_material_study_execution_plan_contract(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    if args.iter().any(|arg| arg == "--self-test") {
        run_self_test(root)?;
        println!("material study execution plan contract check self-test passed");
        return Ok(0);
    }
    if !args.is_empty() {
        return Err(
            "check-material-study-execution-plan-contract only accepts --self-test".to_string(),
        );
    }
    let issues = check_contracts(root)?;
    if let Some(issue) = issues.first() {
        eprintln!("material study execution plan contract check failed: {issue}");
        return Ok(1);
    }
    println!("material study execution plan contract check passed");
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
    if let Some(count) = example.get("step_count").and_then(Value::as_i64) {
        example["step_count"] = Value::from(count + 1);
    }
    let mut issues = Vec::new();
    check_example(&example, &mut issues);
    if issues.is_empty() {
        return Err("self-test did not reject step count mismatch".to_string());
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
    for field in [
        "study_id",
        "material_card_contract_required",
        "material_card_schema_version",
        "material_card_ref_count",
        "step_count",
        "solve_step_count",
        "candidate_count",
        "candidate_ids",
        "actions",
        "steps",
    ] {
        if !required_fields(schema)
            .iter()
            .any(|required| *required == field)
        {
            issues.push(format!("{SCHEMA_PATH}: missing required field {field}"));
        }
    }
    if schema
        .pointer("/properties/material_card_schema_version/const")
        .and_then(Value::as_str)
        != Some(MATERIAL_CARD_SCHEMA_VERSION)
    {
        issues.push(format!(
            "{SCHEMA_PATH}: material_card_schema_version const must be {MATERIAL_CARD_SCHEMA_VERSION}"
        ));
    }
    if schema
        .pointer("/$defs/workflowStep/properties/action/type")
        .and_then(Value::as_str)
        != Some("string")
    {
        issues.push(format!(
            "{SCHEMA_PATH}: workflowStep.action must be a string"
        ));
    }
}

fn check_example(example: &Value, issues: &mut Vec<String>) {
    if field(example, "schema_version") != PLAN_SCHEMA_VERSION {
        issues.push(format!(
            "{EXAMPLE_PATH}: schema_version must be {PLAN_SCHEMA_VERSION}"
        ));
    }
    require_string(example.get("study_id"), "study_id", EXAMPLE_PATH, issues);
    if example
        .get("material_card_contract_required")
        .and_then(Value::as_bool)
        != Some(true)
    {
        issues.push(format!(
            "{EXAMPLE_PATH}: material_card_contract_required must be true"
        ));
    }
    if field(example, "material_card_schema_version") != MATERIAL_CARD_SCHEMA_VERSION {
        issues.push(format!(
            "{EXAMPLE_PATH}: material_card_schema_version must be {MATERIAL_CARD_SCHEMA_VERSION}"
        ));
    }
    let steps = array_field(example, "steps");
    if steps.is_empty() {
        issues.push(format!("{EXAMPLE_PATH}: steps must be a non-empty array"));
    }
    let actions = array_field(example, "actions");
    if actions.len() != steps.len() {
        issues.push(format!(
            "{EXAMPLE_PATH}: actions length must match steps length"
        ));
    }
    if example.get("step_count").and_then(Value::as_u64) != Some(steps.len() as u64) {
        issues.push(format!(
            "{EXAMPLE_PATH}: step_count must match steps length"
        ));
    }
    let solve_steps = solve_steps(&steps);
    if example.get("solve_step_count").and_then(Value::as_u64) != Some(solve_steps.len() as u64) {
        issues.push(format!(
            "{EXAMPLE_PATH}: solve_step_count must match solve_* steps"
        ));
    }
    let candidate_ids = array_field(example, "candidate_ids");
    if candidate_ids.is_empty() {
        issues.push(format!("{EXAMPLE_PATH}: candidate_ids must be non-empty"));
    }
    if example.get("candidate_count").and_then(Value::as_u64) != Some(candidate_ids.len() as u64) {
        issues.push(format!(
            "{EXAMPLE_PATH}: candidate_count must match candidate_ids length"
        ));
    }
    if example
        .get("material_card_ref_count")
        .and_then(Value::as_u64)
        != Some(candidate_ids.len() as u64)
    {
        issues.push(format!(
            "{EXAMPLE_PATH}: material_card_ref_count must match candidate_ids length"
        ));
    }
    for (index, step) in steps.iter().enumerate() {
        let context = format!("{EXAMPLE_PATH}#steps/{index}");
        require_string(step.get("action"), "action", &context, issues);
        if !step.get("payload").is_some_and(is_plain_object) {
            issues.push(format!("{context}: payload must be an object"));
        }
    }
    for candidate_id in candidate_ids.iter().filter_map(|value| value.as_str()) {
        if !solve_steps.iter().any(|step| {
            step.pointer("/payload/research/candidate_id")
                .and_then(Value::as_str)
                == Some(candidate_id)
        }) {
            issues.push(format!(
                "{EXAMPLE_PATH}: candidate_id {candidate_id} is not present in solve step research"
            ));
        }
    }
}

fn check_documentation(root: &Path, issues: &mut Vec<String>) -> RunnerResult<()> {
    let readme = read_text(root, SDK_README_PATH)?;
    for required_path in [SCHEMA_PATH, EXAMPLE_PATH] {
        if !readme.contains(required_path) {
            issues.push(format!(
                "{SDK_README_PATH}: missing link to {required_path}"
            ));
        }
    }
    for example in SDK_EXAMPLE_PATHS {
        let link_path = example.strip_prefix("sdks/").unwrap_or(example);
        if !readme.contains(link_path) {
            issues.push(format!(
                "{SDK_README_PATH}: missing SDK example link {link_path}"
            ));
        }
        if !root.join(example).exists() {
            issues.push(format!("{example}: SDK example file is missing"));
        }
    }
    for readme_path in SDK_README_PATHS {
        let language_readme = read_text(root, readme_path)?;
        if !language_readme.contains("material_study_execution_plan") {
            issues.push(format!(
                "{readme_path}: missing material study execution-plan helper docs"
            ));
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

fn array_field<'a>(value: &'a Value, key: &str) -> Vec<&'a Value> {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(|items| items.iter().collect())
        .unwrap_or_default()
}

fn solve_steps<'a>(steps: &'a [&'a Value]) -> Vec<&'a Value> {
    steps
        .iter()
        .copied()
        .filter(|step| field(step, "action").starts_with("solve_"))
        .collect()
}

fn require_string(value: Option<&Value>, field: &str, context: &str, issues: &mut Vec<String>) {
    if value.and_then(Value::as_str).is_none_or(str::is_empty) {
        issues.push(format!("{context}: {field} must be a non-empty string"));
    }
}

fn is_plain_object(value: &Value) -> bool {
    value.is_object() && !value.is_array()
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
    use super::{PLAN_SCHEMA_VERSION, is_plain_object};
    use serde_json::json;

    #[test]
    fn plan_schema_version_is_stable() {
        assert_eq!(
            PLAN_SCHEMA_VERSION,
            "kyuubiki.material-study-execution-plan/v1"
        );
    }

    #[test]
    fn plain_object_rejects_arrays() {
        assert!(is_plain_object(&json!({ "ok": true })));
        assert!(!is_plain_object(&json!([1, 2, 3])));
    }
}
