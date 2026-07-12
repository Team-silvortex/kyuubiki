use serde_json::{Map, Value};

pub fn score_acoustic_quality(payload: Value, config: Value) -> Result<Value, String> {
    let object = payload
        .as_object()
        .ok_or_else(|| "transform.score_acoustic_quality expects an object payload".to_string())?;
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
    let max_ready_score = config_number(&config, "max_ready_score", 7.0);
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
        "acoustic_quality_contract": "kyuubiki.acoustic_quality_score/v1",
        "acoustic_quality_score": score,
        "acoustic_quality_grade": grade,
        "acoustic_quality_ready": grade != "block",
        "acoustic_quality_missing_metric_count": missing_count,
        "acoustic_quality_watch_count": watch_count,
        "acoustic_quality_term_count": score_terms.len(),
        "acoustic_quality_max_ready_score": max_ready_score,
        "acoustic_quality_max_spl_db": numeric_field(object, "max_sound_pressure_level_db"),
        "acoustic_quality_max_intensity": numeric_field(object, "max_acoustic_intensity"),
        "acoustic_quality_max_pressure": numeric_field(object, "max_pressure_amplitude"),
        "acoustic_quality_total_damping_loss": numeric_field(object, "total_damping_loss"),
        "acoustic_quality_dominant_term": dominant_term,
        "acoustic_quality_blocking_terms": blocking_terms,
        "acoustic_quality_terms": score_terms,
        "acoustic_quality_summary": format!(
            "Acoustic quality {grade}: score={score:.4}, missing={missing_count}, watch={watch_count}, ready_limit={max_ready_score:.4}."
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
            field: "max_sound_pressure_level_db",
            label: "Peak sound pressure level",
            target: 85.0,
            weight: 3.0,
            goal: QualityGoal::Min,
        },
        QualityTerm {
            field: "max_acoustic_intensity",
            label: "Peak acoustic intensity",
            target: 0.25,
            weight: 2.0,
            goal: QualityGoal::Min,
        },
        QualityTerm {
            field: "max_pressure_amplitude",
            label: "Peak pressure amplitude",
            target: 1.0,
            weight: 1.0,
            goal: QualityGoal::Min,
        },
        QualityTerm {
            field: "total_damping_loss",
            label: "Damping loss",
            target: 0.1,
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
        .and_then(finite_number)
        .or_else(|| acoustic_alias_field(object, field))
}

fn acoustic_alias_field(object: &Map<String, Value>, field: &str) -> Option<f64> {
    match field {
        "max_sound_pressure_level_db" => first_alias_number(
            object,
            &["peak_spl_db", "spl_max_db", "sound_pressure_level_max_db"],
        ),
        "max_acoustic_intensity" => first_alias_number(
            object,
            &[
                "peak_acoustic_intensity",
                "acoustic_intensity_peak",
                "intensity_max",
            ],
        ),
        "max_pressure_amplitude" => first_alias_number(
            object,
            &["max_pressure", "peak_pressure", "pressure_amplitude_peak"],
        ),
        "total_damping_loss" => first_alias_number(
            object,
            &[
                "damping_loss_total",
                "total_acoustic_damping_loss",
                "damping_energy_loss",
            ],
        ),
        _ => None,
    }
}

fn first_alias_number(object: &Map<String, Value>, aliases: &[&str]) -> Option<f64> {
    aliases
        .iter()
        .find_map(|alias| object.get(*alias).and_then(finite_number))
}

fn finite_number(value: &Value) -> Option<f64> {
    value.as_f64().filter(|number| number.is_finite())
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
