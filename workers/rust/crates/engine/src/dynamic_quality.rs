use serde_json::{Map, Value};

pub fn score_dynamic_quality(payload: Value, config: Value) -> Result<Value, String> {
    let object = payload
        .as_object()
        .ok_or_else(|| "transform.score_dynamic_quality expects an object payload".to_string())?;
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
        "dynamic_quality_contract": "kyuubiki.dynamic_quality_score/v1",
        "dynamic_quality_score": score,
        "dynamic_quality_grade": grade,
        "dynamic_quality_ready": grade != "block",
        "dynamic_quality_missing_metric_count": missing_count,
        "dynamic_quality_term_count": score_terms.len(),
        "dynamic_quality_max_ready_score": max_ready_score,
        "dynamic_quality_peak_frequency_hz": numeric_field(object, "peak_frequency_hz"),
        "dynamic_quality_max_velocity": numeric_field(object, "max_velocity"),
        "dynamic_quality_max_acceleration": numeric_field(object, "max_acceleration"),
        "dynamic_quality_terms": score_terms,
        "dynamic_quality_summary": format!(
            "Dynamic quality {grade}: score={score:.4}, missing={missing_count}, ready_limit={max_ready_score:.4}."
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
            field: "peak_frequency_hz",
            label: "Peak response frequency",
            target: 20.0,
            weight: 3.0,
            goal: QualityGoal::Max,
        },
        QualityTerm {
            field: "max_displacement",
            label: "Peak displacement amplitude",
            target: 0.02,
            weight: 3.0,
            goal: QualityGoal::Min,
        },
        QualityTerm {
            field: "max_acceleration",
            label: "Peak acceleration amplitude",
            target: 250.0,
            weight: 1.5,
            goal: QualityGoal::Min,
        },
        QualityTerm {
            field: "max_force",
            label: "Peak dynamic force",
            target: 5000.0,
            weight: 1.0,
            goal: QualityGoal::Min,
        },
    ]
}

fn quality_terms(config: &Value) -> Vec<QualityTerm> {
    let requested = config
        .get("enabled_terms")
        .and_then(Value::as_array)
        .map(|terms| {
            terms
                .iter()
                .filter_map(Value::as_str)
                .filter_map(quality_term_for)
                .collect::<Vec<_>>()
        })
        .filter(|terms| !terms.is_empty());

    requested.unwrap_or_else(|| default_quality_terms().to_vec())
}

fn quality_term_for(field: &str) -> Option<QualityTerm> {
    if let Some(term) = default_quality_terms()
        .into_iter()
        .find(|term| term.field == field)
    {
        return Some(term);
    }
    match field {
        "max_velocity" => Some(QualityTerm {
            field: "max_velocity",
            label: "Peak velocity amplitude",
            target: 2.0,
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
        .or_else(|| derived_dynamic_field(object, field))
        .filter(|value| value.is_finite())
}

fn derived_dynamic_field(object: &Map<String, Value>, field: &str) -> Option<f64> {
    match field {
        "peak_frequency_hz" => object
            .get("frequencies")
            .and_then(Value::as_array)
            .and_then(|frequencies| peak_frequency(frequencies)),
        "max_displacement" | "max_velocity" | "max_acceleration" | "max_force" => object
            .get("frequencies")
            .and_then(Value::as_array)
            .and_then(|frequencies| max_frequency_field(frequencies, field))
            .or_else(|| max_transient_node_field(object, field)),
        _ => None,
    }
}

fn peak_frequency(frequencies: &[Value]) -> Option<f64> {
    frequencies
        .iter()
        .filter_map(Value::as_object)
        .filter_map(|entry| {
            Some((
                entry.get("frequency_hz")?.as_f64()?,
                entry.get("max_displacement")?.as_f64()?,
            ))
        })
        .max_by(|left, right| left.1.total_cmp(&right.1))
        .map(|(frequency, _)| frequency)
}

fn max_frequency_field(frequencies: &[Value], field: &str) -> Option<f64> {
    frequencies
        .iter()
        .filter_map(Value::as_object)
        .filter_map(|entry| entry.get(field).and_then(Value::as_f64))
        .filter(|value| value.is_finite())
        .max_by(f64::total_cmp)
}

fn max_transient_node_field(object: &Map<String, Value>, field: &str) -> Option<f64> {
    let node_field = match field {
        "max_displacement" => "ux",
        "max_velocity" => "vx",
        "max_acceleration" => "ax",
        _ => return None,
    };
    object
        .get("nodes")?
        .as_array()?
        .iter()
        .filter_map(Value::as_object)
        .filter_map(|node| node.get(node_field).and_then(Value::as_f64))
        .map(f64::abs)
        .filter(|value| value.is_finite())
        .max_by(f64::total_cmp)
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
