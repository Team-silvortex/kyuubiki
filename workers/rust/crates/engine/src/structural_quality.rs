use serde_json::{Map, Value};

pub fn score_structural_quality(payload: Value, config: Value) -> Result<Value, String> {
    let object = payload.as_object().ok_or_else(|| {
        "transform.score_structural_quality expects an object payload".to_string()
    })?;
    let terms = quality_terms(&config);
    let score_terms = terms
        .iter()
        .map(|term| score_quality_term(object, &config, term))
        .collect::<Vec<_>>();
    let missing_count = score_terms
        .iter()
        .filter(|term| term.get("status").and_then(Value::as_str) == Some("missing"))
        .count();
    let score = score_terms
        .iter()
        .filter_map(|term| term.get("penalty").and_then(Value::as_f64))
        .sum::<f64>();
    let max_ready_score = config_number(&config, "max_ready_score", 8.0);
    let grade = quality_grade(score, missing_count, max_ready_score);

    Ok(serde_json::json!({
        "structural_quality_contract": "kyuubiki.structural_quality_score/v1",
        "structural_quality_score": score,
        "structural_quality_grade": grade,
        "structural_quality_ready": grade != "block",
        "structural_quality_missing_metric_count": missing_count,
        "structural_quality_term_count": score_terms.len(),
        "structural_quality_max_ready_score": max_ready_score,
        "structural_quality_max_displacement": numeric_field(object, "max_displacement"),
        "structural_quality_max_stress": numeric_field(object, "max_stress"),
        "structural_quality_mass": numeric_field(object, "mass"),
        "structural_quality_stiffness_margin": numeric_field(object, "stiffness_margin"),
        "structural_quality_terms": score_terms,
        "structural_quality_summary": format!(
            "Structural quality {grade}: score={score:.4}, missing={missing_count}, ready_limit={max_ready_score:.4}."
        ),
    }))
}

#[derive(Clone, Copy)]
struct QualityTerm {
    field: &'static str,
    label: &'static str,
    target: f64,
    weight: f64,
    goal: QualityGoal,
}

#[derive(Clone, Copy)]
enum QualityGoal {
    Min,
    Max,
}

fn default_quality_terms() -> [QualityTerm; 4] {
    [
        QualityTerm {
            field: "max_displacement",
            label: "Maximum displacement",
            target: 0.02,
            weight: 3.0,
            goal: QualityGoal::Min,
        },
        QualityTerm {
            field: "max_stress",
            label: "Maximum stress",
            target: 250.0,
            weight: 3.0,
            goal: QualityGoal::Min,
        },
        QualityTerm {
            field: "mass",
            label: "Mass",
            target: 15.0,
            weight: 1.0,
            goal: QualityGoal::Min,
        },
        QualityTerm {
            field: "stiffness_margin",
            label: "Stiffness margin",
            target: 1.2,
            weight: 1.0,
            goal: QualityGoal::Max,
        },
    ]
}

fn quality_terms(config: &Value) -> Vec<QualityTerm> {
    config
        .get("enabled_terms")
        .and_then(Value::as_array)
        .map(|terms| {
            terms
                .iter()
                .filter_map(Value::as_str)
                .filter_map(quality_term_for)
                .collect::<Vec<_>>()
        })
        .filter(|terms| !terms.is_empty())
        .unwrap_or_else(|| default_quality_terms().to_vec())
}

fn quality_term_for(field: &str) -> Option<QualityTerm> {
    default_quality_terms()
        .into_iter()
        .find(|term| term.field == field)
}

fn score_quality_term(object: &Map<String, Value>, config: &Value, term: &QualityTerm) -> Value {
    let target = configured_term_number(config, "targets", term.field, term.target).max(1e-12);
    let weight = configured_term_number(config, "weights", term.field, term.weight).max(0.0);
    let value = numeric_field(object, term.field);

    match value {
        Some(value) if value.is_finite() => {
            let ratio = match term.goal {
                QualityGoal::Max => target / value.abs().max(1e-12),
                QualityGoal::Min => value.abs() / target,
            };
            let penalty = ratio * weight;
            serde_json::json!({
                "field": term.field,
                "label": term.label,
                "value": value,
                "target": target,
                "weight": weight,
                "goal": match term.goal {
                    QualityGoal::Max => "max",
                    QualityGoal::Min => "min",
                },
                "penalty": penalty,
                "status": if meets_target(value, target, term.goal) { "ok" } else { "watch" },
            })
        }
        _ => serde_json::json!({
            "field": term.field,
            "label": term.label,
            "target": target,
            "weight": weight,
            "penalty": 0.0,
            "status": "missing",
        }),
    }
}

fn numeric_field(object: &Map<String, Value>, field: &str) -> Option<f64> {
    object
        .get(field)
        .and_then(Value::as_f64)
        .filter(|value| value.is_finite())
}

fn meets_target(value: f64, target: f64, goal: QualityGoal) -> bool {
    match goal {
        QualityGoal::Max => value >= target,
        QualityGoal::Min => value.abs() <= target,
    }
}

fn configured_term_number(config: &Value, group: &str, field: &str, default_value: f64) -> f64 {
    config
        .get(group)
        .and_then(Value::as_object)
        .and_then(|values| values.get(field))
        .and_then(Value::as_f64)
        .filter(|value| value.is_finite())
        .unwrap_or(default_value)
}

fn config_number(config: &Value, field: &str, default_value: f64) -> f64 {
    config
        .get(field)
        .and_then(Value::as_f64)
        .filter(|value| value.is_finite() && *value >= 0.0)
        .unwrap_or(default_value)
}

fn quality_grade(score: f64, missing_count: usize, max_ready_score: f64) -> &'static str {
    if missing_count > 0 || score > max_ready_score {
        "block"
    } else if score > max_ready_score * 0.7 {
        "review"
    } else if score > max_ready_score * 0.35 {
        "good"
    } else {
        "excellent"
    }
}
