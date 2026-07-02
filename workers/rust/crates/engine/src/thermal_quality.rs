use serde_json::{Map, Value};

pub fn score_thermal_quality(payload: Value, config: Value) -> Result<Value, String> {
    let object = payload
        .as_object()
        .ok_or_else(|| "transform.score_thermal_quality expects an object payload".to_string())?;
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
        "thermal_quality_contract": "kyuubiki.thermal_quality_score/v1",
        "thermal_quality_score": score,
        "thermal_quality_grade": grade,
        "thermal_quality_ready": grade != "block",
        "thermal_quality_missing_metric_count": missing_count,
        "thermal_quality_term_count": score_terms.len(),
        "thermal_quality_max_ready_score": max_ready_score,
        "thermal_quality_max_temperature": numeric_field(object, "thermal_temperature_max"),
        "thermal_quality_peak_flux_magnitude": numeric_field(object, "thermal_flux_peak_magnitude"),
        "thermal_quality_total_energy": numeric_field(object, "thermal_total_energy"),
        "thermal_quality_terms": score_terms,
        "thermal_quality_summary": format!(
            "Thermal quality {grade}: score={score:.4}, missing={missing_count}, ready_limit={max_ready_score:.4}."
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
}

fn default_quality_terms() -> [QualityTerm; 4] {
    [
        QualityTerm {
            field: "thermal_temperature_max",
            label: "Peak thermal temperature",
            target: 120.0,
            weight: 3.0,
            goal: QualityGoal::Min,
        },
        QualityTerm {
            field: "thermo_temperature_delta_max",
            label: "Peak thermo-mechanical temperature delta",
            target: 80.0,
            weight: 2.0,
            goal: QualityGoal::Min,
        },
        QualityTerm {
            field: "thermal_flux_peak_magnitude",
            label: "Peak heat flux magnitude",
            target: 20.0,
            weight: 2.0,
            goal: QualityGoal::Min,
        },
        QualityTerm {
            field: "thermo_stress_peak",
            label: "Peak thermo-mechanical stress",
            target: 250.0,
            weight: 1.0,
            goal: QualityGoal::Min,
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
        "thermal_total_energy" => Some(QualityTerm {
            field: "thermal_total_energy",
            label: "Total thermal energy",
            target: 5000.0,
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
            let ratio = value.abs() / target;
            let penalty = ratio * weight;
            serde_json::json!({
                "field": term.field,
                "label": term.label,
                "value": value,
                "target": target,
                "weight": weight,
                "goal": "min",
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
        .or_else(|| thermal_alias_field(object, field))
        .filter(|value| value.is_finite())
}

fn thermal_alias_field(object: &Map<String, Value>, field: &str) -> Option<f64> {
    let alias = match field {
        "thermal_temperature_max" => "max_temperature",
        "thermal_flux_peak_magnitude" => "max_heat_flux",
        "thermo_temperature_delta_max" => "max_temperature_delta",
        "thermo_stress_peak" => "max_stress",
        "thermal_total_energy" => "total_thermal_energy",
        _ => return None,
    };
    object.get(alias).and_then(Value::as_f64)
}

fn meets_target(value: f64, target: f64, goal: QualityGoal) -> bool {
    match goal {
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
