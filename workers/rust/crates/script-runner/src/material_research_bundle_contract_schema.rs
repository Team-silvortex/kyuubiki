use super::{BUNDLE_SCHEMA_VERSION, POSTURE, SCHEMA_PATH};
use serde_json::Value;

pub(super) fn check_schema(schema: &Value, issues: &mut Vec<String>) {
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
    let execution_trace_required = schema
        .pointer("/$defs/executionTrace/required")
        .and_then(Value::as_array)
        .map(|fields| fields.iter().filter_map(Value::as_str).collect::<Vec<_>>())
        .unwrap_or_default();
    if !execution_trace_required.contains(&"authority") {
        issues.push(format!(
            "{SCHEMA_PATH}: executionTrace missing required authority"
        ));
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

fn required_fields(schema: &Value) -> Vec<&str> {
    schema
        .get("required")
        .and_then(Value::as_array)
        .map(|fields| fields.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default()
}
