use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaterialOptimizationConstraint {
    pub metric_id: String,
    pub operator: String,
    pub limit: f64,
    pub severity: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaterialOptimizationWeight {
    pub metric_id: String,
    pub direction: String,
    pub weight: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaterialOptimizationProfile {
    pub id: String,
    pub goal: String,
    pub score_range: String,
    pub score_formula: String,
    pub weights: Vec<MaterialOptimizationWeight>,
    pub constraints: Vec<MaterialOptimizationConstraint>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaterialOptimizationTerm {
    pub metric_id: String,
    pub direction: String,
    pub raw_value: Option<f64>,
    pub normalized_score: f64,
    pub weighted_score: f64,
    pub constraint_status: String,
}

pub fn material_optimization_profile(
    id: &str,
    goal: &str,
    score_formula: &str,
    weights: Vec<MaterialOptimizationWeight>,
    constraints: Vec<MaterialOptimizationConstraint>,
) -> MaterialOptimizationProfile {
    MaterialOptimizationProfile {
        id: id.to_string(),
        goal: goal.to_string(),
        score_range: "0.0..1.0 higher_is_better".to_string(),
        score_formula: score_formula.to_string(),
        weights,
        constraints,
    }
}

pub fn material_optimization_weight(
    metric_id: &str,
    direction: &str,
    weight: f64,
) -> MaterialOptimizationWeight {
    MaterialOptimizationWeight {
        metric_id: metric_id.to_string(),
        direction: direction.to_string(),
        weight,
    }
}

pub fn material_optimization_constraint(
    metric_id: &str,
    operator: &str,
    limit: f64,
    severity: &str,
) -> MaterialOptimizationConstraint {
    MaterialOptimizationConstraint {
        metric_id: metric_id.to_string(),
        operator: operator.to_string(),
        limit,
        severity: severity.to_string(),
    }
}

pub fn material_optimization_term(
    metric_id: &str,
    direction: &str,
    raw_value: Option<f64>,
    normalized_score: f64,
    weight: f64,
    constraint_status: &str,
) -> MaterialOptimizationTerm {
    MaterialOptimizationTerm {
        metric_id: metric_id.to_string(),
        direction: direction.to_string(),
        raw_value,
        normalized_score: round_score(normalized_score),
        weighted_score: round_score(normalized_score * weight),
        constraint_status: constraint_status.to_string(),
    }
}

pub fn less_equal_status(value: Option<f64>, limit: f64) -> String {
    match value {
        Some(actual) if actual <= limit => "pass".to_string(),
        Some(_) => "violate".to_string(),
        None => "unknown".to_string(),
    }
}

pub fn profile_weight(
    profile: &MaterialOptimizationProfile,
    metric_id: &str,
    fallback: f64,
) -> f64 {
    profile
        .weights
        .iter()
        .find(|weight| weight.metric_id == metric_id)
        .map(|weight| weight.weight)
        .unwrap_or(fallback)
}

pub fn round_score(value: f64) -> f64 {
    (value * 1000.0).round() / 1000.0
}
