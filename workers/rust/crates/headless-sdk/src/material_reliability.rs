use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const SUMMARY_TOLERANCE_VALIDATION_CONTRACT: &str = "kyuubiki.summary_tolerance_validation/v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaterialEvidenceRef {
    pub id: String,
    pub label: String,
    pub kind: String,
    pub citation: String,
    pub confidence: String,
    pub notes: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaterialModelAssumption {
    pub id: String,
    pub label: String,
    pub value: String,
    pub impact: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaterialQualityGate {
    pub id: String,
    pub label: String,
    pub metric_id: String,
    pub operator: String,
    pub limit: f64,
    pub actual_value: Option<f64>,
    pub status: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaterialRepairHint {
    pub schema_version: String,
    pub action: String,
    pub strategy: String,
    pub domain: String,
    pub focus_field: Option<String>,
    pub focus_source: String,
    pub blocking_gate_id: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaterialReliabilitySummary {
    pub decision: String,
    pub total_gate_count: usize,
    pub pass_count: usize,
    pub violation_count: usize,
    pub unknown_count: usize,
    pub observe_count: usize,
    pub blocking_gate_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaterialReliabilityEnvelope {
    pub schema_version: String,
    pub posture: String,
    pub material_card_version: String,
    pub unit_system: String,
    pub evidence_refs: Vec<MaterialEvidenceRef>,
    pub model_assumptions: Vec<MaterialModelAssumption>,
    pub quality_gates: Vec<MaterialQualityGate>,
    pub summary: MaterialReliabilitySummary,
    pub limitations: Vec<String>,
}

pub fn material_evidence_ref(
    id: &str,
    label: &str,
    kind: &str,
    citation: &str,
    confidence: &str,
    notes: &str,
) -> MaterialEvidenceRef {
    MaterialEvidenceRef {
        id: id.to_string(),
        label: label.to_string(),
        kind: kind.to_string(),
        citation: citation.to_string(),
        confidence: confidence.to_string(),
        notes: notes.to_string(),
    }
}

pub fn material_model_assumption(
    id: &str,
    label: &str,
    value: &str,
    impact: &str,
) -> MaterialModelAssumption {
    MaterialModelAssumption {
        id: id.to_string(),
        label: label.to_string(),
        value: value.to_string(),
        impact: impact.to_string(),
    }
}

pub fn material_quality_gate(
    id: &str,
    label: &str,
    metric_id: &str,
    operator: &str,
    limit: f64,
    actual_value: Option<f64>,
    description: &str,
) -> MaterialQualityGate {
    MaterialQualityGate {
        id: id.to_string(),
        label: label.to_string(),
        metric_id: metric_id.to_string(),
        operator: operator.to_string(),
        limit,
        actual_value,
        status: gate_status(actual_value, operator, limit),
        description: description.to_string(),
    }
}

pub fn material_validation_quality_gate(validation_payload: &Value) -> Option<MaterialQualityGate> {
    if validation_payload
        .get("validation_contract")
        .and_then(Value::as_str)
        != Some(SUMMARY_TOLERANCE_VALIDATION_CONTRACT)
    {
        return None;
    }

    let failed_count = validation_payload
        .get("validation_failed_field_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let missing_count = validation_payload
        .get("validation_missing_field_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let fail_on_missing = validation_payload
        .get("validation_fail_on_missing")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let passed = validation_payload
        .get("validation_passed")
        .and_then(Value::as_bool)
        .unwrap_or_else(|| failed_count == 0 && (!fail_on_missing || missing_count == 0));
    let blocking_count = failed_count + if fail_on_missing { missing_count } else { 0 };

    Some(material_quality_gate(
        "gate.summary_tolerance_validation",
        "Summary tolerance validation",
        "summary_validation_blocking_count",
        "<=",
        0.0,
        Some(if passed {
            0.0
        } else {
            blocking_count.max(1) as f64
        }),
        "Cross-check summary fields against tolerance before trusting material research ranking.",
    ))
}

pub fn material_validation_repair_hint(validation_payload: &Value) -> Option<MaterialRepairHint> {
    if validation_payload
        .get("validation_contract")
        .and_then(Value::as_str)
        != Some(SUMMARY_TOLERANCE_VALIDATION_CONTRACT)
        || validation_payload
            .get("validation_passed")
            .and_then(Value::as_bool)
            .unwrap_or(false)
    {
        return None;
    }

    let focus_field = first_failed_validation_field(validation_payload)
        .or_else(|| first_missing_validation_field(validation_payload));
    let missing_focus = focus_field.as_deref().is_some_and(|field| {
        validation_payload
            .get("validation_missing_fields")
            .and_then(Value::as_array)
            .is_some_and(|fields| fields.iter().any(|value| value.as_str() == Some(field)))
    });
    let reason = match focus_field.as_deref() {
        Some(field) if missing_focus => format!("summary validation is missing field {field}"),
        Some(field) => format!("summary validation exceeded tolerance for field {field}"),
        None => "summary validation failed without a specific focus field".to_string(),
    };

    Some(MaterialRepairHint {
        schema_version: "kyuubiki.material-repair-hint/v1".to_string(),
        action: "fix_validation_failure".to_string(),
        strategy: if missing_focus {
            "fill_missing_summary_field".to_string()
        } else {
            "rerun_validation_focused_sweep".to_string()
        },
        domain: "validation".to_string(),
        focus_field,
        focus_source: "summary_tolerance_validation".to_string(),
        blocking_gate_id: "gate.summary_tolerance_validation".to_string(),
        reason,
    })
}

pub fn gate_status(value: Option<f64>, operator: &str, limit: f64) -> String {
    match (value, operator) {
        (Some(actual), "<=") if actual <= limit => "pass".to_string(),
        (Some(_), "<=") => "violate".to_string(),
        (Some(actual), ">=") if actual >= limit => "pass".to_string(),
        (Some(_), ">=") => "violate".to_string(),
        (Some(_), _) => "observe".to_string(),
        (None, _) => "unknown".to_string(),
    }
}

fn first_failed_validation_field(validation_payload: &Value) -> Option<String> {
    validation_payload
        .get("validation_failures")
        .and_then(Value::as_array)
        .and_then(|failures| failures.first())
        .and_then(|failure| failure.get("field"))
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

fn first_missing_validation_field(validation_payload: &Value) -> Option<String> {
    validation_payload
        .get("validation_missing_fields")
        .and_then(Value::as_array)
        .and_then(|fields| fields.first())
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

pub fn material_reliability_summary(
    quality_gates: &[MaterialQualityGate],
) -> MaterialReliabilitySummary {
    let mut pass_count = 0;
    let mut violation_count = 0;
    let mut unknown_count = 0;
    let mut observe_count = 0;
    let mut blocking_gate_ids = Vec::new();

    for gate in quality_gates {
        match gate.status.as_str() {
            "pass" => pass_count += 1,
            "violate" => {
                violation_count += 1;
                blocking_gate_ids.push(gate.id.clone());
            }
            "unknown" => unknown_count += 1,
            _ => observe_count += 1,
        }
    }

    let decision = if violation_count > 0 {
        "blocked_by_quality_gates"
    } else if unknown_count > 0 {
        "needs_more_evidence"
    } else if observe_count > 0 {
        "review_observations"
    } else {
        "ready_for_next_round"
    };

    MaterialReliabilitySummary {
        decision: decision.to_string(),
        total_gate_count: quality_gates.len(),
        pass_count,
        violation_count,
        unknown_count,
        observe_count,
        blocking_gate_ids,
    }
}
