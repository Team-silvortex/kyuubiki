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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaterialReliabilityEnvelope {
    pub schema_version: String,
    pub posture: String,
    pub material_card_version: String,
    pub unit_system: String,
    pub evidence_refs: Vec<MaterialEvidenceRef>,
    pub model_assumptions: Vec<MaterialModelAssumption>,
    pub quality_gates: Vec<MaterialQualityGate>,
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
