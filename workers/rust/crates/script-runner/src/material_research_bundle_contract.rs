use serde_json::Value;
use sha2::{Digest, Sha256};
use std::ffi::OsString;
use std::fs;
use std::path::Path;

type RunnerResult<T> = Result<T, String>;

#[path = "material_research_bundle_contract_self_test.rs"]
mod material_research_bundle_contract_self_test;

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
const RESEARCH_EVIDENCE_SCHEMA_VERSION: &str = "kyuubiki.material-research-evidence/v1";
const VALIDATION_EVIDENCE_SCHEMA_VERSION: &str = "kyuubiki.material-validation-evidence/v1";

pub(crate) fn run_check_material_research_bundle_contract(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    if args.iter().any(|arg| arg == "--self-test") {
        material_research_bundle_contract_self_test::run_self_test(root)?;
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
    if !required_fields(schema)
        .iter()
        .any(|required| *required == "research_evidence")
    {
        issues.push(format!(
            "{SCHEMA_PATH}: missing required field research_evidence"
        ));
    }
    if !required_fields(schema)
        .iter()
        .any(|required| *required == "validation_evidence")
    {
        issues.push(format!(
            "{SCHEMA_PATH}: missing required field validation_evidence"
        ));
    }
    let summary_required = schema
        .pointer("/$defs/summary/required")
        .and_then(Value::as_array)
        .map(|fields| fields.iter().filter_map(Value::as_str).collect::<Vec<_>>())
        .unwrap_or_default();
    for field in ["material_card_ref_count", "material_card_refs"] {
        if !summary_required.iter().any(|required| *required == field) {
            issues.push(format!("{SCHEMA_PATH}: summary missing required {field}"));
        }
    }
    if schema.pointer("/$defs/materialCardRef").is_none() {
        issues.push(format!("{SCHEMA_PATH}: missing materialCardRef definition"));
    }
    let evidence_required = schema
        .pointer("/$defs/researchEvidence/required")
        .and_then(Value::as_array)
        .map(|fields| fields.iter().filter_map(Value::as_str).collect::<Vec<_>>())
        .unwrap_or_default();
    for field in [
        "candidate_count",
        "ranked_candidate_ids",
        "winner_candidate_id",
        "primary_metric_ids",
        "metric_objective_count",
        "focus_candidate_ids",
        "quality_gate_decision",
        "plan_decision",
        "chain_round_count",
        "chain_trace_round_count",
        "final_winner_candidate_id",
    ] {
        if !evidence_required.iter().any(|required| *required == field) {
            issues.push(format!(
                "{SCHEMA_PATH}: researchEvidence missing required {field}"
            ));
        }
    }
    let validation_required = schema
        .pointer("/$defs/validationEvidence/required")
        .and_then(Value::as_array)
        .map(|fields| fields.iter().filter_map(Value::as_str).collect::<Vec<_>>())
        .unwrap_or_default();
    for field in [
        "baseline_refs",
        "candidate_confidence_counts",
        "sensitivity_summary",
        "acceptance_criteria",
        "uncertainty_summary",
        "validation_readiness",
        "external_validation_plan",
        "violated_quality_gate_ids",
    ] {
        if !validation_required
            .iter()
            .any(|required| *required == field)
        {
            issues.push(format!(
                "{SCHEMA_PATH}: validationEvidence missing required {field}"
            ));
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
    validate_material_card_refs(example, issues);
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
    validate_research_evidence(example, issues);
    validate_validation_evidence(example, issues);
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

fn validate_validation_evidence(example: &Value, issues: &mut Vec<String>) {
    assert_schema_version(
        example.pointer("/validation_evidence/schema_version"),
        VALIDATION_EVIDENCE_SCHEMA_VERSION,
        "validation_evidence",
        issues,
    );
    if example
        .pointer("/validation_evidence/validation_posture")
        .and_then(Value::as_str)
        != Some("screening_validation")
    {
        issues.push(format!(
            "{EXAMPLE_PATH}: validation_evidence.validation_posture must be screening_validation"
        ));
    }
    if example
        .pointer("/validation_evidence/baseline_refs")
        .and_then(Value::as_array)
        .is_none_or(Vec::is_empty)
    {
        issues.push(format!(
            "{EXAMPLE_PATH}: validation_evidence.baseline_refs must be non-empty"
        ));
    }
    if string_array(example.pointer("/validation_evidence/external_validation_plan")).is_empty() {
        issues.push(format!(
            "{EXAMPLE_PATH}: validation_evidence.external_validation_plan must be non-empty"
        ));
    }
    if string_array(example.pointer("/validation_evidence/sensitivity_summary/primary_metric_ids"))
        .is_empty()
    {
        issues.push(format!(
            "{EXAMPLE_PATH}: validation_evidence.sensitivity_summary.primary_metric_ids must be non-empty"
        ));
    }
    assert_equals(
        example.pointer("/validation_evidence/sensitivity_summary/focus_candidate_ids"),
        example.pointer("/research_evidence/focus_candidate_ids"),
        "validation_evidence.sensitivity_summary.focus_candidate_ids",
        issues,
    );
    assert_equals(
        example.pointer("/validation_evidence/sensitivity_summary/chain_trace_round_count"),
        example.pointer("/research_evidence/chain_trace_round_count"),
        "validation_evidence.sensitivity_summary.chain_trace_round_count",
        issues,
    );
    assert_equals(
        example.pointer("/validation_evidence/violated_quality_gate_ids"),
        example.pointer("/research_evidence/violated_quality_gate_ids"),
        "validation_evidence.violated_quality_gate_ids",
        issues,
    );
    assert_equals(
        example.pointer("/validation_evidence/candidate_confidence_counts"),
        example.pointer("/validation_evidence/uncertainty_summary/candidate_confidence_counts"),
        "validation_evidence.candidate_confidence_counts",
        issues,
    );
    if example
        .pointer("/validation_evidence/uncertainty_summary/external_validation_required")
        .and_then(Value::as_bool)
        != Some(true)
    {
        issues.push(format!(
            "{EXAMPLE_PATH}: validation_evidence external_validation_required must be true"
        ));
    }
    validate_validation_readiness(example, issues);
}

fn validate_validation_readiness(example: &Value, issues: &mut Vec<String>) {
    assert_schema_version(
        example.pointer("/validation_evidence/validation_readiness/schema_version"),
        "kyuubiki.material-validation-readiness/v1",
        "validation_evidence.validation_readiness",
        issues,
    );
    if example
        .pointer("/validation_evidence/validation_readiness/decision")
        .and_then(Value::as_str)
        != Some("screening_only")
    {
        issues.push(format!(
            "{EXAMPLE_PATH}: validation_evidence.validation_readiness.decision must be screening_only"
        ));
    }
    let score = example
        .pointer("/validation_evidence/validation_readiness/score")
        .and_then(Value::as_f64);
    if !score.is_some_and(|score| (0.0..=1.0).contains(&score)) {
        issues.push(format!(
            "{EXAMPLE_PATH}: validation_evidence.validation_readiness.score must be between 0 and 1"
        ));
    }
    let reasons =
        string_array(example.pointer("/validation_evidence/validation_readiness/blocking_reasons"));
    if !reasons
        .iter()
        .any(|reason| *reason == "external_validation_required")
    {
        issues.push(format!(
            "{EXAMPLE_PATH}: validation_evidence.validation_readiness.blocking_reasons must include external_validation_required"
        ));
    }
    if example
        .pointer("/validation_evidence/violated_quality_gate_ids")
        .and_then(Value::as_array)
        .is_some_and(|gates| !gates.is_empty())
        && !reasons
            .iter()
            .any(|reason| *reason == "violated_quality_gates")
    {
        issues.push(format!(
            "{EXAMPLE_PATH}: validation_evidence.validation_readiness.blocking_reasons must include violated_quality_gates"
        ));
    }
    if example
        .pointer("/validation_evidence/candidate_confidence_counts/low")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        > 0
        && !reasons
            .iter()
            .any(|reason| *reason == "low_confidence_material_cards")
    {
        issues.push(format!(
            "{EXAMPLE_PATH}: validation_evidence.validation_readiness.blocking_reasons must include low_confidence_material_cards"
        ));
    }
    if string_array(
        example.pointer("/validation_evidence/validation_readiness/next_validation_actions"),
    )
    .is_empty()
    {
        issues.push(format!(
            "{EXAMPLE_PATH}: validation_evidence.validation_readiness.next_validation_actions must be non-empty"
        ));
    }
}

fn validate_research_evidence(example: &Value, issues: &mut Vec<String>) {
    assert_schema_version(
        example.pointer("/research_evidence/schema_version"),
        RESEARCH_EVIDENCE_SCHEMA_VERSION,
        "research_evidence",
        issues,
    );
    assert_equals(
        example.pointer("/research_evidence/winner_candidate_id"),
        example.pointer("/summary/winner_candidate_id"),
        "research_evidence.winner_candidate_id",
        issues,
    );
    assert_equals(
        example.pointer("/research_evidence/quality_gate_decision"),
        example.pointer("/summary/reliability_decision"),
        "research_evidence.quality_gate_decision",
        issues,
    );
    assert_equals(
        example.pointer("/research_evidence/plan_decision"),
        example.pointer("/summary/next_round_decision"),
        "research_evidence.plan_decision",
        issues,
    );
    assert_equals(
        example.pointer("/research_evidence/plan_step_count"),
        example.pointer("/summary/runnable_next_step_count"),
        "research_evidence.plan_step_count",
        issues,
    );
    assert_equals(
        example.pointer("/research_evidence/chain_round_count"),
        example.pointer("/summary/chain_round_count"),
        "research_evidence.chain_round_count",
        issues,
    );
    let ranked = string_array(example.pointer("/research_evidence/ranked_candidate_ids"));
    let focus = string_array(example.pointer("/research_evidence/focus_candidate_ids"));
    if ranked.is_empty() {
        issues.push(format!(
            "{EXAMPLE_PATH}: research_evidence.ranked_candidate_ids must be non-empty"
        ));
    }
    if example
        .pointer("/research_evidence/candidate_count")
        .and_then(Value::as_u64)
        != Some(ranked.len() as u64)
    {
        issues.push(format!(
            "{EXAMPLE_PATH}: research_evidence.candidate_count must match ranked candidates"
        ));
    }
    if !ranked.contains(&field_at(example, "/summary/winner_candidate_id")) {
        issues.push(format!(
            "{EXAMPLE_PATH}: research_evidence winner must be ranked"
        ));
    }
    if focus.is_empty() || focus.iter().any(|candidate| !ranked.contains(candidate)) {
        issues.push(format!(
            "{EXAMPLE_PATH}: research_evidence.focus_candidate_ids must be ranked candidates"
        ));
    }
    if string_array(example.pointer("/research_evidence/primary_metric_ids")).is_empty() {
        issues.push(format!(
            "{EXAMPLE_PATH}: research_evidence.primary_metric_ids must be non-empty"
        ));
    }
    if example
        .pointer("/research_evidence/metric_objective_count")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        == 0
    {
        issues.push(format!(
            "{EXAMPLE_PATH}: research_evidence.metric_objective_count must be positive"
        ));
    }
}

fn validate_material_card_refs(example: &Value, issues: &mut Vec<String>) {
    let refs = example
        .pointer("/summary/material_card_refs")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let expected_count = example
        .pointer("/summary/material_card_ref_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    if refs.is_empty() {
        issues.push(format!(
            "{EXAMPLE_PATH}: summary.material_card_refs must be non-empty"
        ));
    }
    if refs.len() as u64 != expected_count {
        issues.push(format!(
            "{EXAMPLE_PATH}: summary.material_card_ref_count must match material_card_refs length"
        ));
    }
    for (index, reference) in refs.iter().enumerate() {
        let label = format!("summary.material_card_refs/{index}");
        require_string(
            reference.get("material_card_id"),
            &format!("{label}.material_card_id"),
            EXAMPLE_PATH,
            issues,
        );
        assert_schema_version(
            reference.get("schema_version"),
            "kyuubiki.material-card/v1",
            &label,
            issues,
        );
        for field in [
            "candidate_id",
            "confidence",
            "unit_system",
            "parameter_scope",
        ] {
            require_string(
                reference.get(field),
                &format!("{label}.{field}"),
                EXAMPLE_PATH,
                issues,
            );
        }
    }
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

fn string_array(value: Option<&Value>) -> Vec<&str> {
    value
        .and_then(Value::as_array)
        .map(|items| items.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default()
}

fn field_at<'a>(value: &'a Value, pointer: &str) -> &'a str {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
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
