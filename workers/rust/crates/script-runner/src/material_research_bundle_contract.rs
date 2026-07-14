use serde_json::Value;
use sha2::{Digest, Sha256};
use std::ffi::OsString;
use std::fs;
use std::path::Path;

type RunnerResult<T> = Result<T, String>;

const SCHEMA_PATH: &str = "schemas/material-research-bundle.schema.json";
const EXAMPLE_PATH: &str = "schemas/examples.material-research-bundle.json";
const SCHEMAS_README_PATH: &str = "schemas/README.md";
const SCRIPTS_README_PATH: &str = "scripts/README.md";
const DOCS_PATH: &str = "docs/automated-material-research-example.md";

const BUNDLE_SCHEMA_VERSION: &str = "kyuubiki.material-research-bundle/v1";
const POSTURE: &str = "screening_research_bundle";
const EXPLORATION_SCHEMA_VERSION: &str = "kyuubiki.material-exploration-run/v1";
const EXECUTION_SCHEMA_VERSION: &str = "kyuubiki.material-exploration-next-round-execution/v1";
const CHAIN_SCHEMA_VERSION: &str = "kyuubiki.material-exploration-chain/v1";

pub(crate) fn run_check_material_research_bundle_contract(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    if args.iter().any(|arg| arg == "--self-test") {
        run_self_test(root)?;
        println!("material research bundle contract check self-test passed");
        return Ok(0);
    }
    if !args.is_empty() {
        return Err("check-material-research-bundle-contract only accepts --self-test".to_string());
    }
    let issues = check_contracts(root)?;
    if let Some(issue) = issues.first() {
        eprintln!("material research bundle contract check failed: {issue}");
        return Ok(1);
    }
    println!("material research bundle contract check passed");
    Ok(0)
}

fn check_contracts(root: &Path) -> RunnerResult<Vec<String>> {
    let mut issues = Vec::new();
    check_schema(&read_json(root, SCHEMA_PATH)?, &mut issues);
    check_example(&read_json(root, EXAMPLE_PATH)?, &mut issues)?;
    check_documentation(root, &mut issues)?;
    Ok(issues)
}

fn run_self_test(root: &Path) -> RunnerResult<()> {
    let mut checksum_mismatch = read_json(root, EXAMPLE_PATH)?;
    if let Some(checksums) = checksum_mismatch
        .get_mut("artifact_checksums")
        .and_then(Value::as_object_mut)
    {
        checksums.insert("chain_sha256".to_string(), Value::from("0".repeat(64)));
    }
    expect_check_example_failure(&checksum_mismatch, "bad retained artifact checksum")?;

    let mut decision_mismatch = read_json(root, EXAMPLE_PATH)?;
    if let Some(plan) = decision_mismatch
        .get_mut("next_round_execution_plan")
        .and_then(Value::as_object_mut)
    {
        plan.insert("decision".to_string(), Value::from("repair_validation"));
    }
    expect_check_example_failure(&decision_mismatch, "summary/plan decision mismatch")?;
    Ok(())
}

fn expect_check_example_failure(example: &Value, label: &str) -> RunnerResult<()> {
    let mut issues = Vec::new();
    check_example(example, &mut issues)?;
    if issues.is_empty() {
        return Err(format!("self-test did not reject {label}"));
    }
    Ok(())
}

fn check_schema(schema: &Value, issues: &mut Vec<String>) {
    if schema
        .pointer("/properties/schema_version/const")
        .and_then(Value::as_str)
        != Some(BUNDLE_SCHEMA_VERSION)
    {
        issues.push(format!(
            "{SCHEMA_PATH}: schema_version const must be {BUNDLE_SCHEMA_VERSION}"
        ));
    }
    if schema
        .pointer("/properties/posture/const")
        .and_then(Value::as_str)
        != Some(POSTURE)
    {
        issues.push(format!("{SCHEMA_PATH}: posture const must be {POSTURE}"));
    }
    for field in ["artifact_checksums", "reproducibility", "summary", "chain"] {
        if !required_fields(schema)
            .iter()
            .any(|required| *required == field)
        {
            issues.push(format!("{SCHEMA_PATH}: missing required field {field}"));
        }
    }
    let checksum_required = schema
        .pointer("/$defs/artifactChecksums/required")
        .and_then(Value::as_array)
        .map(|fields| fields.iter().filter_map(Value::as_str).collect::<Vec<_>>())
        .unwrap_or_default();
    for field in [
        "initial_exploration_sha256",
        "next_round_execution_plan_sha256",
        "next_exploration_sha256",
        "chain_sha256",
    ] {
        if !checksum_required.iter().any(|required| *required == field) {
            issues.push(format!("{SCHEMA_PATH}: missing checksum field {field}"));
        }
    }
}

fn check_example(example: &Value, issues: &mut Vec<String>) -> RunnerResult<()> {
    if field(example, "schema_version") != BUNDLE_SCHEMA_VERSION {
        issues.push(format!(
            "{EXAMPLE_PATH}: schema_version must be {BUNDLE_SCHEMA_VERSION}"
        ));
    }
    if field(example, "posture") != POSTURE {
        issues.push(format!("{EXAMPLE_PATH}: posture must be {POSTURE}"));
    }
    require_string(example.get("bundle_id"), "bundle_id", EXAMPLE_PATH, issues);
    require_string(example.get("study"), "study", EXAMPLE_PATH, issues);
    for field in [
        "initial_command",
        "plan_next_command_template",
        "run_next_command_template",
        "chain_next_command_template",
    ] {
        require_array(
            example.pointer(&format!("/reproducibility/{field}")),
            field,
            EXAMPLE_PATH,
            issues,
        );
    }
    require_string(
        example.pointer("/summary/winner_candidate_id"),
        "summary.winner_candidate_id",
        EXAMPLE_PATH,
        issues,
    );
    require_string(
        example.pointer("/summary/reliability_decision"),
        "summary.reliability_decision",
        EXAMPLE_PATH,
        issues,
    );
    require_string(
        example.pointer("/summary/next_round_decision"),
        "summary.next_round_decision",
        EXAMPLE_PATH,
        issues,
    );
    require_string(
        example.pointer("/summary/chain_stop_reason"),
        "summary.chain_stop_reason",
        EXAMPLE_PATH,
        issues,
    );
    assert_equals(
        example.pointer("/next_round_execution_plan/decision"),
        example.pointer("/summary/next_round_decision"),
        "next_round_execution_plan.decision",
        issues,
    );
    assert_equals(
        example.pointer("/next_round_execution_plan/runnable_step_count"),
        example.pointer("/summary/runnable_next_step_count"),
        "next_round_execution_plan.runnable_step_count",
        issues,
    );
    assert_equals(
        example.pointer("/next_round_execution_plan/iteration"),
        example.pointer("/summary/next_iteration"),
        "next_round_execution_plan.iteration",
        issues,
    );
    assert_equals(
        example.pointer("/next_exploration/iteration"),
        example.pointer("/summary/next_iteration"),
        "next_exploration.iteration",
        issues,
    );
    assert_equals(
        example.pointer("/chain/stop_reason"),
        example.pointer("/summary/chain_stop_reason"),
        "chain.stop_reason",
        issues,
    );
    assert_checksum(
        example,
        "initial_exploration_sha256",
        "initial_exploration",
        issues,
    )?;
    assert_checksum(
        example,
        "next_round_execution_plan_sha256",
        "next_round_execution_plan",
        issues,
    )?;
    assert_checksum(
        example,
        "next_exploration_sha256",
        "next_exploration",
        issues,
    )?;
    assert_checksum(example, "chain_sha256", "chain", issues)?;
    assert_schema_version(
        example.pointer("/initial_exploration/schema_version"),
        EXPLORATION_SCHEMA_VERSION,
        "initial_exploration",
        issues,
    );
    assert_schema_version(
        example.pointer("/next_round_execution_plan/schema_version"),
        EXECUTION_SCHEMA_VERSION,
        "next_round_execution_plan",
        issues,
    );
    assert_schema_version(
        example.pointer("/next_exploration/schema_version"),
        EXPLORATION_SCHEMA_VERSION,
        "next_exploration",
        issues,
    );
    assert_schema_version(
        example.pointer("/chain/schema_version"),
        CHAIN_SCHEMA_VERSION,
        "chain",
        issues,
    );
    Ok(())
}

fn assert_checksum(
    example: &Value,
    checksum_key: &str,
    artifact_key: &str,
    issues: &mut Vec<String>,
) -> RunnerResult<()> {
    let actual = example
        .pointer(&format!("/artifact_checksums/{checksum_key}"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    let expected = sha256_json(example.get(artifact_key).unwrap_or(&Value::Null))?;
    if actual != expected {
        issues.push(format!(
            "{EXAMPLE_PATH}: {checksum_key} must match {artifact_key}"
        ));
    }
    Ok(())
}

fn assert_equals(
    actual: Option<&Value>,
    expected: Option<&Value>,
    field: &str,
    issues: &mut Vec<String>,
) {
    if actual != expected {
        issues.push(format!(
            "{EXAMPLE_PATH}: {field} must be {}, got {}",
            stringify_for_message(expected),
            stringify_for_message(actual)
        ));
    }
}

fn assert_schema_version(
    actual: Option<&Value>,
    expected: &str,
    artifact: &str,
    issues: &mut Vec<String>,
) {
    if actual.and_then(Value::as_str) != Some(expected) {
        issues.push(format!(
            "{EXAMPLE_PATH}: {artifact} schema must be {expected}"
        ));
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

fn sha256_json(value: &Value) -> RunnerResult<String> {
    let mut payload = serde_json::to_string(value)
        .map_err(|error| format!("failed to serialize json: {error}"))?;
    payload.push('\n');
    let digest = Sha256::digest(payload.as_bytes());
    Ok(digest.iter().map(|byte| format!("{byte:02x}")).collect())
}

fn stringify_for_message(value: Option<&Value>) -> String {
    value
        .map(|item| serde_json::to_string(item).unwrap_or_else(|_| "null".to_string()))
        .unwrap_or_else(|| "null".to_string())
}

fn required_fields(schema: &Value) -> Vec<&str> {
    schema
        .get("required")
        .and_then(Value::as_array)
        .map(|fields| fields.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default()
}

fn require_string(value: Option<&Value>, field: &str, context: &str, issues: &mut Vec<String>) {
    if value.and_then(Value::as_str).is_none_or(str::is_empty) {
        issues.push(format!("{context}: {field} must be a non-empty string"));
    }
}

fn require_array(value: Option<&Value>, field: &str, context: &str, issues: &mut Vec<String>) {
    if !value
        .and_then(Value::as_array)
        .is_some_and(|items| !items.is_empty())
    {
        issues.push(format!("{context}: {field} must be a non-empty array"));
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
    use super::{BUNDLE_SCHEMA_VERSION, POSTURE, sha256_json};
    use serde_json::json;

    #[test]
    fn bundle_contract_tokens_are_stable() {
        assert_eq!(
            BUNDLE_SCHEMA_VERSION,
            "kyuubiki.material-research-bundle/v1"
        );
        assert_eq!(POSTURE, "screening_research_bundle");
    }

    #[test]
    fn sha256_json_matches_node_stringify_shape() {
        let digest = sha256_json(&json!({"a":1,"b":"x"})).expect("digest");
        assert_eq!(
            digest,
            "2ab52f950a71d1e5f94d20b173018ba634a1eea2e9ea6803af994a20158fc247"
        );
    }
}
