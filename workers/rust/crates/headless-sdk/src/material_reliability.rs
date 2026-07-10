use serde::{Deserialize, Serialize};

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
