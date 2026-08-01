use crate::MaterialResearchBundle;
use serde_json::Value;

const VALIDATION_EVIDENCE_SCHEMA_VERSION: &str = "kyuubiki.material-validation-evidence/v1";

pub(crate) fn validate_validation_evidence(
    errors: &mut Vec<String>,
    bundle: &MaterialResearchBundle,
) {
    let validation = &bundle.validation_evidence;
    require_value_str_const(
        errors,
        validation,
        "schema_version",
        VALIDATION_EVIDENCE_SCHEMA_VERSION,
        "validation_evidence",
    );
    require_value_str_const(
        errors,
        validation,
        "validation_posture",
        "screening_validation",
        "validation_evidence",
    );
    validate_object_array(
        errors,
        validation.get("baseline_refs"),
        "validation_evidence.baseline_refs",
        &["baseline_id", "kind", "status", "scope"],
    );
    validate_confidence_counts(
        errors,
        validation.get("candidate_confidence_counts"),
        "validation_evidence.candidate_confidence_counts",
    );
    validate_sensitivity_summary(errors, validation, &bundle.research_evidence);
    validate_object_array(
        errors,
        validation.get("acceptance_criteria"),
        "validation_evidence.acceptance_criteria",
        &["criterion_id", "metric_id", "operator", "status"],
    );
    validate_uncertainty_summary(errors, validation);
    validate_validation_readiness(errors, validation);
    require_string_array(
        errors,
        validation.get("external_validation_plan"),
        "validation_evidence.external_validation_plan",
        false,
    );
    let gates = require_string_array(
        errors,
        validation.get("violated_quality_gate_ids"),
        "validation_evidence.violated_quality_gate_ids",
        true,
    );
    let research_gates = require_string_array(
        errors,
        bundle.research_evidence.get("violated_quality_gate_ids"),
        "research_evidence.violated_quality_gate_ids",
        true,
    );
    if gates != research_gates {
        errors.push("validation_evidence.violated_quality_gate_ids must match research_evidence.violated_quality_gate_ids".into());
    }
}

pub(crate) fn validate_material_card_refs(
    errors: &mut Vec<String>,
    bundle: &MaterialResearchBundle,
) {
    if bundle.summary.material_card_ref_count == 0
        || bundle.summary.material_card_ref_count != bundle.summary.material_card_refs.len()
    {
        errors.push(
            "summary.material_card_ref_count must match a non-empty material_card_refs array"
                .into(),
        );
    }
    let ranked = bundle
        .research_evidence
        .get("ranked_candidate_ids")
        .and_then(Value::as_array);
    for (index, card) in bundle.summary.material_card_refs.iter().enumerate() {
        let field = format!("summary.material_card_refs[{index}]");
        for key in [
            "material_card_id",
            "candidate_id",
            "confidence",
            "unit_system",
            "parameter_scope",
        ] {
            require_value_str(errors, card, key, &field);
        }
        require_value_str_const(
            errors,
            card,
            "schema_version",
            "kyuubiki.material-card/v1",
            &field,
        );
        if let (Some(candidate), Some(ranked)) =
            (card.get("candidate_id").and_then(Value::as_str), ranked)
            && !ranked.iter().any(|value| value.as_str() == Some(candidate))
        {
            errors.push(format!(
                "{field}.candidate_id must be present in ranked candidates"
            ));
        }
    }
}

fn validate_sensitivity_summary(errors: &mut Vec<String>, validation: &Value, research: &Value) {
    let Some(summary) = validation.get("sensitivity_summary") else {
        errors.push("validation_evidence.sensitivity_summary is required".into());
        return;
    };
    require_value_str_const(
        errors,
        summary,
        "schema_version",
        "kyuubiki.material-sensitivity-summary/v1",
        "validation_evidence.sensitivity_summary",
    );
    require_value_str(
        errors,
        summary,
        "method",
        "validation_evidence.sensitivity_summary",
    );
    require_value_str(
        errors,
        summary,
        "winner_stability_state",
        "validation_evidence.sensitivity_summary",
    );
    for key in ["primary_metric_ids", "focus_candidate_ids"] {
        let actual = require_string_array(
            errors,
            summary.get(key),
            &format!("validation_evidence.sensitivity_summary.{key}"),
            false,
        );
        let expected = require_string_array(
            errors,
            research.get(key),
            &format!("research_evidence.{key}"),
            false,
        );
        if actual != expected {
            errors.push(format!(
                "validation_evidence.sensitivity_summary.{key} must match research_evidence.{key}"
            ));
        }
    }
    match (summary.get("chain_trace_round_count").and_then(Value::as_u64), research.get("chain_trace_round_count").and_then(Value::as_u64)) {
        (Some(actual), Some(expected)) if actual == expected => {}
        _ => errors.push("validation_evidence.sensitivity_summary.chain_trace_round_count must match research_evidence.chain_trace_round_count".into()),
    }
}

fn validate_uncertainty_summary(errors: &mut Vec<String>, validation: &Value) {
    let Some(summary) = validation.get("uncertainty_summary") else {
        errors.push("validation_evidence.uncertainty_summary is required".into());
        return;
    };
    require_value_str_const(
        errors,
        summary,
        "schema_version",
        "kyuubiki.material-uncertainty-summary/v1",
        "validation_evidence.uncertainty_summary",
    );
    require_string_array(
        errors,
        summary.get("known_limitations"),
        "validation_evidence.uncertainty_summary.known_limitations",
        false,
    );
    require_value_bool(
        errors,
        summary,
        "external_validation_required",
        true,
        "validation_evidence.uncertainty_summary",
    );
    validate_confidence_counts(
        errors,
        summary.get("candidate_confidence_counts"),
        "validation_evidence.uncertainty_summary.candidate_confidence_counts",
    );
    if validation.get("candidate_confidence_counts") != summary.get("candidate_confidence_counts") {
        errors.push("validation_evidence.candidate_confidence_counts must match uncertainty_summary.candidate_confidence_counts".into());
    }
}

fn validate_validation_readiness(errors: &mut Vec<String>, validation: &Value) {
    let Some(readiness) = validation.get("validation_readiness") else {
        errors.push("validation_evidence.validation_readiness is required".into());
        return;
    };
    require_value_str_const(
        errors,
        readiness,
        "schema_version",
        "kyuubiki.material-validation-readiness/v1",
        "validation_evidence.validation_readiness",
    );
    require_value_str_const(
        errors,
        readiness,
        "decision",
        "screening_only",
        "validation_evidence.validation_readiness",
    );
    if !readiness
        .get("score")
        .and_then(Value::as_f64)
        .is_some_and(|score| (0.0..=1.0).contains(&score))
    {
        errors
            .push("validation_evidence.validation_readiness.score must be between 0 and 1".into());
    }
    let reasons = require_string_array(
        errors,
        readiness.get("blocking_reasons"),
        "validation_evidence.validation_readiness.blocking_reasons",
        false,
    );
    if !reasons
        .iter()
        .any(|reason| reason == "external_validation_required")
    {
        errors.push("validation_evidence.validation_readiness.blocking_reasons must include external_validation_required".into());
    }
    if validation
        .get("violated_quality_gate_ids")
        .and_then(Value::as_array)
        .is_some_and(|items| !items.is_empty())
        && !reasons
            .iter()
            .any(|reason| reason == "violated_quality_gates")
    {
        errors.push("validation_evidence.validation_readiness.blocking_reasons must include violated_quality_gates when gates are violated".into());
    }
    if validation
        .pointer("/candidate_confidence_counts/low")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        > 0
        && !reasons
            .iter()
            .any(|reason| reason == "low_confidence_material_cards")
    {
        errors.push("validation_evidence.validation_readiness.blocking_reasons must include low_confidence_material_cards when low-confidence cards exist".into());
    }
    require_string_array(
        errors,
        readiness.get("next_validation_actions"),
        "validation_evidence.validation_readiness.next_validation_actions",
        false,
    );
}

fn require_value_str(errors: &mut Vec<String>, value: &Value, key: &str, field: &str) {
    if value
        .get(key)
        .and_then(Value::as_str)
        .is_none_or(str::is_empty)
    {
        errors.push(format!("{field}.{key} must be a non-empty string"));
    }
}

fn require_value_str_const(
    errors: &mut Vec<String>,
    value: &Value,
    key: &str,
    expected: &str,
    field: &str,
) {
    if value.get(key).and_then(Value::as_str) != Some(expected) {
        errors.push(format!("{field}.{key} must be {expected}"));
    }
}

fn require_value_bool(
    errors: &mut Vec<String>,
    value: &Value,
    key: &str,
    expected: bool,
    field: &str,
) {
    if value.get(key).and_then(Value::as_bool) != Some(expected) {
        errors.push(format!("{field}.{key} must be {expected}"));
    }
}

fn require_string_array(
    errors: &mut Vec<String>,
    value: Option<&Value>,
    field: &str,
    allow_empty: bool,
) -> Vec<String> {
    let Some(items) = value.and_then(Value::as_array) else {
        errors.push(format!("{field} must be an array"));
        return Vec::new();
    };
    if !allow_empty && items.is_empty() {
        errors.push(format!("{field} must be non-empty"));
    }
    let output = items
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect::<Vec<_>>();
    if output.len() != items.len() || output.iter().any(String::is_empty) {
        errors.push(format!("{field} must contain only non-empty strings"));
    }
    output
}

fn validate_object_array(
    errors: &mut Vec<String>,
    value: Option<&Value>,
    field: &str,
    required_strings: &[&str],
) {
    let Some(items) = value.and_then(Value::as_array) else {
        errors.push(format!("{field} must be an array"));
        return;
    };
    if items.is_empty() {
        errors.push(format!("{field} must be non-empty"));
    }
    for (index, item) in items.iter().enumerate() {
        for key in required_strings {
            require_value_str(errors, item, key, &format!("{field}[{index}]"));
        }
    }
}

fn validate_confidence_counts(errors: &mut Vec<String>, value: Option<&Value>, field: &str) {
    let Some(counts) = value else {
        errors.push(format!("{field} is required"));
        return;
    };
    for key in ["low", "medium", "high", "unknown"] {
        if counts.get(key).and_then(Value::as_u64).is_none() {
            errors.push(format!("{field}.{key} must be a non-negative integer"));
        }
    }
}
