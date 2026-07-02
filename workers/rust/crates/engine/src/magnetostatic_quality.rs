use serde_json::{Map, Value};

pub fn score_magnetostatic_quality(payload: Value, config: Value) -> Result<Value, String> {
    let object = payload.as_object().ok_or_else(|| {
        "transform.score_magnetostatic_quality expects an object payload".to_string()
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
        "magnetostatic_quality_contract": "kyuubiki.magnetostatic_quality_score/v1",
        "magnetostatic_quality_score": score,
        "magnetostatic_quality_grade": grade,
        "magnetostatic_quality_ready": grade != "block",
        "magnetostatic_quality_missing_metric_count": missing_count,
        "magnetostatic_quality_term_count": score_terms.len(),
        "magnetostatic_quality_max_ready_score": max_ready_score,
        "magnetostatic_quality_peak_field": numeric_field(object, "magnetostatic_field_peak_magnitude"),
        "magnetostatic_quality_peak_flux": numeric_field(object, "magnetostatic_flux_peak_magnitude"),
        "magnetostatic_quality_total_energy": numeric_field(object, "magnetostatic_total_stored_energy"),
        "magnetostatic_quality_terms": score_terms,
        "magnetostatic_quality_summary": format!(
            "Magnetostatic quality {grade}: score={score:.4}, missing={missing_count}, ready_limit={max_ready_score:.4}."
        ),
    }))
}

#[derive(Clone, Copy)]
struct QualityTerm {
    field: &'static str,
    label: &'static str,
    target: f64,
    weight: f64,
}

fn default_quality_terms() -> [QualityTerm; 4] {
    [
        QualityTerm {
            field: "magnetostatic_field_peak_magnitude",
            label: "Peak magnetic field strength",
            target: 12.0,
            weight: 3.0,
        },
        QualityTerm {
            field: "magnetostatic_flux_peak_magnitude",
            label: "Peak magnetic flux density",
            target: 16.0,
            weight: 2.0,
        },
        QualityTerm {
            field: "magnetostatic_energy_density_peak",
            label: "Peak magnetic energy density",
            target: 8.0,
            weight: 2.0,
        },
        QualityTerm {
            field: "magnetostatic_current_density_sum",
            label: "Current density sum",
            target: 10.0,
            weight: 1.0,
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
        "magnetostatic_total_stored_energy" => Some(QualityTerm {
            field: "magnetostatic_total_stored_energy",
            label: "Total magnetostatic stored energy",
            target: 10.0,
            weight: 1.0,
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
            let penalty = (value.abs() / target) * weight;
            serde_json::json!({
                "field": term.field,
                "label": term.label,
                "value": value,
                "target": target,
                "weight": weight,
                "goal": "min",
                "penalty": penalty,
                "status": if value.abs() <= target { "ok" } else { "watch" },
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
        .or_else(|| magnetostatic_alias_field(object, field))
        .filter(|value| value.is_finite())
}

fn magnetostatic_alias_field(object: &Map<String, Value>, field: &str) -> Option<f64> {
    let alias = match field {
        "magnetostatic_field_peak_magnitude" => "max_magnetic_field_strength",
        "magnetostatic_flux_peak_magnitude" => "max_flux_density",
        "magnetostatic_total_stored_energy" => "total_stored_energy",
        _ => return None,
    };
    object.get(alias).and_then(Value::as_f64)
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
