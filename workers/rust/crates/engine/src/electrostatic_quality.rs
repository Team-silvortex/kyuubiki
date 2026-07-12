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
    let watch_count = score_terms
        .iter()
        .filter(|term| term.get("status").and_then(Value::as_str) == Some("watch"))
        .count();
    let score = score_terms
        .iter()
        .filter_map(|term| term.get("penalty").and_then(Value::as_f64))
        .sum::<f64>();
    let max_ready_score = config_number(&config, "max_ready_score", 8.0);
    let grade = quality_grade(score, missing_count, max_ready_score);
    let dominant_term = dominant_quality_term(&score_terms);
    let blocking_terms = if grade == "block" {
        score_terms
            .iter()
            .filter(|term| {
                matches!(
                    term.get("status").and_then(Value::as_str),
                    Some("missing" | "watch")
                )
            })
            .map(compact_quality_term)
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };

    Ok(serde_json::json!({
        "electrostatic_quality_contract": "kyuubiki.electrostatic_quality_score/v1",
        "electrostatic_quality_score": score,
        "electrostatic_quality_grade": grade,
        "electrostatic_quality_ready": grade != "block",
        "electrostatic_quality_missing_metric_count": missing_count,
        "electrostatic_quality_watch_count": watch_count,
        "electrostatic_quality_term_count": score_terms.len(),
        "electrostatic_quality_max_ready_score": max_ready_score,
        "electrostatic_quality_peak_field": numeric_field(object, "electrostatic_field_peak_magnitude"),
        "electrostatic_quality_peak_energy_density": numeric_field(object, "electrostatic_peak_energy_density"),
        "electrostatic_quality_potential_span": numeric_field(object, "electrostatic_potential_span"),
        "electrostatic_quality_total_energy": numeric_field(object, "electrostatic_total_stored_energy"),
        "electrostatic_quality_dominant_term": dominant_term,
        "electrostatic_quality_blocking_terms": blocking_terms,
        "electrostatic_quality_terms": score_terms,
        "electrostatic_quality_summary": format!(
            "Electrostatic quality {grade}: score={score:.4}, missing={missing_count}, watch={watch_count}, ready_limit={max_ready_score:.4}."
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

fn dominant_quality_term(terms: &[Value]) -> Value {
    terms
        .iter()
        .max_by(|left, right| {
            let left_penalty = left.get("penalty").and_then(Value::as_f64).unwrap_or(0.0);
            let right_penalty = right.get("penalty").and_then(Value::as_f64).unwrap_or(0.0);
            left_penalty
                .partial_cmp(&right_penalty)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(compact_quality_term)
        .unwrap_or(Value::Null)
}

fn compact_quality_term(term: &Value) -> Value {
    serde_json::json!({
        "field": term.get("field").cloned().unwrap_or(Value::Null),
        "label": term.get("label").cloned().unwrap_or(Value::Null),
        "status": term.get("status").cloned().unwrap_or(Value::Null),
        "penalty": term.get("penalty").cloned().unwrap_or(Value::Null),
    })
}

fn numeric_field(object: &Map<String, Value>, field: &str) -> Option<f64> {
    object
        .get(field)
        .and_then(finite_number)
        .or_else(|| electrostatic_alias_field(object, field))
}

fn electrostatic_alias_field(object: &Map<String, Value>, field: &str) -> Option<f64> {
    match field {
        "electrostatic_field_peak_magnitude" => first_alias_number(
            object,
            &[
                "max_electric_field",
                "peak_electric_field",
                "electric_field_peak",
            ],
        ),
        "electrostatic_peak_energy_density" => first_alias_number(
            object,
            &[
                "max_energy_density",
                "peak_energy_density",
                "electric_energy_density_peak",
            ],
        ),
        "electrostatic_flux_peak_magnitude" => {
            first_alias_number(object, &["max_flux_density", "peak_flux_density"])
        }
        "electrostatic_total_stored_energy" => first_alias_number(
            object,
            &[
                "total_stored_energy",
                "stored_energy_total",
                "electric_total_energy",
            ],
        ),
        "electrostatic_potential_span" => {
            first_alias_number(object, &["potential_span", "voltage_span", "max_potential"])
                .or_else(|| potential_span_from_bounds(object))
        }
        _ => None,
    }
}

fn first_alias_number(object: &Map<String, Value>, aliases: &[&str]) -> Option<f64> {
    aliases
        .iter()
        .find_map(|alias| object.get(*alias).and_then(finite_number))
}

fn potential_span_from_bounds(object: &Map<String, Value>) -> Option<f64> {
    let max = first_alias_number(object, &["potential_max", "max_voltage", "voltage_max"])?;
    let min = first_alias_number(object, &["potential_min", "min_voltage", "voltage_min"])?;
    Some((max - min).abs())
}

fn finite_number(value: &Value) -> Option<f64> {
    value.as_f64().filter(|number| number.is_finite())
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
