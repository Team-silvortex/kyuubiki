use serde_json::{Map, Value};

pub fn score_electrostatic_quality(payload: Value, config: Value) -> Result<Value, String> {
    let object = payload.as_object().ok_or_else(|| {
        "transform.score_electrostatic_quality expects an object payload".to_string()
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
        "electrostatic_quality_contract": "kyuubiki.electrostatic_quality_score/v1",
        "electrostatic_quality_score": score,
        "electrostatic_quality_grade": grade,
        "electrostatic_quality_ready": grade != "block",
        "electrostatic_quality_missing_metric_count": missing_count,
        "electrostatic_quality_term_count": score_terms.len(),
        "electrostatic_quality_max_ready_score": max_ready_score,
        "electrostatic_quality_peak_field": numeric_field(object, "electrostatic_field_peak_magnitude"),
        "electrostatic_quality_total_energy": numeric_field(object, "electrostatic_total_stored_energy"),
        "electrostatic_quality_terms": score_terms,
        "electrostatic_quality_summary": format!(
            "Electrostatic quality {grade}: score={score:.4}, missing={missing_count}, ready_limit={max_ready_score:.4}."
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

fn default_quality_terms() -> [QualityTerm; 3] {
    [
        QualityTerm {
            field: "electrostatic_field_peak_magnitude",
            label: "Peak electric field magnitude",
            target: 10.0,
            weight: 4.0,
            goal: QualityGoal::Min,
        },
        QualityTerm {
            field: "electrostatic_peak_energy_density",
            label: "Peak electrostatic energy density",
            target: 0.8,
            weight: 2.0,
            goal: QualityGoal::Min,
        },
        QualityTerm {
            field: "electrostatic_potential_span",
            label: "Potential span",
            target: 4.0,
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
    if let Some(term) = default_quality_terms()
        .into_iter()
        .find(|term| term.field == field)
    {
        return Some(term);
    }
    match field {
        "electrostatic_total_stored_energy" => Some(QualityTerm {
            field: "electrostatic_total_stored_energy",
            label: "Total electrostatic stored energy",
            target: 10.0,
            weight: 1.0,
            goal: QualityGoal::Min,
        }),
        _ => None,
    }
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
        .or_else(|| electrostatic_alias_field(object, field))
        .filter(|value| value.is_finite())
}

fn electrostatic_alias_field(object: &Map<String, Value>, field: &str) -> Option<f64> {
    let alias = match field {
        "electrostatic_field_peak_magnitude" => "max_electric_field",
        "electrostatic_flux_peak_magnitude" => "max_flux_density",
        "electrostatic_total_stored_energy" => "total_stored_energy",
        "electrostatic_potential_span" => "max_potential",
        _ => return None,
    };
    object.get(alias).and_then(Value::as_f64)
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
