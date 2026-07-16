use super::{EXAMPLE_PATH, RunnerResult, check_example, read_json};
use serde_json::Value;
use std::path::Path;

pub(super) fn run_self_test(root: &Path) -> RunnerResult<()> {
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

    let mut readiness_reason_mismatch = read_json(root, EXAMPLE_PATH)?;
    if let Some(readiness) = readiness_reason_mismatch
        .pointer_mut("/validation_evidence/validation_readiness")
        .and_then(Value::as_object_mut)
    {
        readiness.insert(
            "blocking_reasons".to_string(),
            Value::Array(vec![Value::from("external_validation_required")]),
        );
    }
    expect_check_example_failure(
        &readiness_reason_mismatch,
        "validation readiness reason mismatch",
    )
}

fn expect_check_example_failure(example: &Value, label: &str) -> RunnerResult<()> {
    let mut issues = Vec::new();
    check_example(example, &mut issues)?;
    if issues.is_empty() {
        return Err(format!("self-test did not reject {label}"));
    }
    Ok(())
}
