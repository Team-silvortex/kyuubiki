use serde_json::{Map, Value};

pub fn extract_stokes_flow_result_diagnostics(
    payload: Value,
    config: Value,
) -> Result<Value, String> {
    let object = payload.as_object().ok_or_else(|| {
        "extract.stokes_flow_result_diagnostics expects an object payload".to_string()
    })?;
    let nodes = object
        .get("nodes")
        .and_then(Value::as_array)
        .ok_or_else(|| "extract.stokes_flow_result_diagnostics expects nodes array".to_string())?;
    let elements = object
        .get("elements")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            "extract.stokes_flow_result_diagnostics expects elements array".to_string()
        })?;
    if nodes.is_empty() && elements.is_empty() {
        return Err(
            "extract.stokes_flow_result_diagnostics expects non-empty nodes or elements".into(),
        );
    }

    let prefix = config
        .get("output_prefix")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("cfd");
    let velocity_values = numeric_values(nodes, "velocity_magnitude");
    let pressure_values = numeric_values(nodes, "pressure");
    let divergence_values = numeric_values(elements, "divergence_error");
    let reynolds_values = numeric_values(elements, "reynolds_number");
    let dissipation_values = numeric_values(elements, "viscous_dissipation");
    let mut summary = Map::new();

    summary.insert(
        "diagnostic_contract".into(),
        Value::String("kyuubiki.workflow_diagnostics/v1".into()),
    );
    summary.insert("diagnostic_domain".into(), Value::String("fluid".into()));
    summary.insert(
        "diagnostic_subject".into(),
        Value::String("stokes_flow_result".into()),
    );
    summary.insert("diagnostic_prefix".into(), Value::String(prefix.into()));
    summary.insert("diagnostic_node_count".into(), Value::from(nodes.len()));
    summary.insert(
        "diagnostic_element_count".into(),
        Value::from(elements.len()),
    );
    merge_min_max(
        &mut summary,
        &format!("{prefix}_velocity"),
        &velocity_values,
    );
    merge_min_max(
        &mut summary,
        &format!("{prefix}_pressure"),
        &pressure_values,
    );
    merge_peak(
        &mut summary,
        &format!("{prefix}_divergence_error"),
        elements,
        "divergence_error",
    );
    merge_peak(
        &mut summary,
        &format!("{prefix}_reynolds_number"),
        elements,
        "reynolds_number",
    );
    merge_peak(
        &mut summary,
        &format!("{prefix}_viscous_dissipation"),
        elements,
        "viscous_dissipation",
    );
    summary.insert(
        format!("{prefix}_velocity_mean"),
        Value::from(mean_or_zero(&velocity_values)),
    );
    summary.insert(
        format!("{prefix}_pressure_mean"),
        Value::from(mean_or_zero(&pressure_values)),
    );
    summary.insert(
        format!("{prefix}_divergence_error_mean"),
        Value::from(mean_or_zero(&divergence_values)),
    );
    summary.insert(
        format!("{prefix}_reynolds_number_mean"),
        Value::from(mean_or_zero(&reynolds_values)),
    );
    summary.insert(
        format!("{prefix}_viscous_dissipation_total"),
        Value::from(dissipation_values.iter().sum::<f64>()),
    );

    Ok(Value::Object(summary))
}

pub fn score_cfd_quality(payload: Value, config: Value) -> Result<Value, String> {
    let object = payload
        .as_object()
        .ok_or_else(|| "transform.score_cfd_quality expects an object payload".to_string())?;
    let terms = default_quality_terms();
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
        "cfd_quality_contract": "kyuubiki.cfd_quality_score/v1",
        "cfd_quality_score": score,
        "cfd_quality_grade": grade,
        "cfd_quality_ready": grade != "block",
        "cfd_quality_missing_metric_count": missing_count,
        "cfd_quality_watch_count": watch_count,
        "cfd_quality_term_count": score_terms.len(),
        "cfd_quality_max_ready_score": max_ready_score,
        "cfd_quality_dominant_term": dominant_term,
        "cfd_quality_blocking_terms": blocking_terms,
        "cfd_quality_terms": score_terms,
        "cfd_quality_summary": format!(
            "CFD quality {grade}: score={score:.4}, missing={missing_count}, watch={watch_count}, ready_limit={max_ready_score:.4}."
        ),
    }))
}

struct QualityTerm {
    field: &'static str,
    label: &'static str,
    target: f64,
    weight: f64,
}

fn default_quality_terms() -> [QualityTerm; 5] {
    [
        QualityTerm {
            field: "cfd_divergence_error_peak",
            label: "Divergence peak",
            target: 0.05,
            weight: 4.0,
        },
        QualityTerm {
            field: "cfd_reynolds_number_peak",
            label: "Reynolds peak",
            target: 10.0,
            weight: 2.0,
        },
        QualityTerm {
            field: "cfd_viscous_dissipation_total",
            label: "Viscous dissipation",
            target: 1.0,
            weight: 1.0,
        },
        QualityTerm {
            field: "cfd_velocity_span",
            label: "Velocity span",
            target: 2.0,
            weight: 0.5,
        },
        QualityTerm {
            field: "cfd_pressure_span",
            label: "Pressure span",
            target: 5.0,
            weight: 0.5,
        },
    ]
}

fn score_quality_term(object: &Map<String, Value>, config: &Value, term: &QualityTerm) -> Value {
    let target = configured_term_number(config, "targets", term.field, term.target).max(1e-12);
    let weight = configured_term_number(config, "weights", term.field, term.weight).max(0.0);
    let value = object
        .get(term.field)
        .and_then(Value::as_f64)
        .or_else(|| derived_span(object, term.field));

    match value {
        Some(value) if value.is_finite() => {
            let penalty = (value.abs() / target) * weight;
            serde_json::json!({
                "field": term.field,
                "label": term.label,
                "value": value,
                "target": target,
                "weight": weight,
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

fn merge_min_max(summary: &mut Map<String, Value>, key: &str, values: &[f64]) {
    let min = values.iter().copied().reduce(f64::min).unwrap_or(0.0);
    let max = values.iter().copied().reduce(f64::max).unwrap_or(0.0);
    summary.insert(format!("{key}_min"), Value::from(min));
    summary.insert(format!("{key}_max"), Value::from(max));
    summary.insert(format!("{key}_span"), Value::from(max - min));
}

fn merge_peak(summary: &mut Map<String, Value>, key: &str, elements: &[Value], field: &str) {
    let peak = elements
        .iter()
        .filter_map(Value::as_object)
        .filter_map(|element| Some((element, element.get(field)?.as_f64()?)))
        .max_by(|(_, left), (_, right)| {
            left.abs()
                .partial_cmp(&right.abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        });

    match peak {
        Some((element, value)) => {
            summary.insert(format!("{key}_peak"), Value::from(value));
            summary.insert(
                format!("{key}_peak_element_id"),
                element.get("id").cloned().unwrap_or(Value::Null),
            );
        }
        None => {
            summary.insert(format!("{key}_peak"), Value::from(0.0));
            summary.insert(format!("{key}_peak_element_id"), Value::Null);
        }
    }
}

fn numeric_values(values: &[Value], field: &str) -> Vec<f64> {
    values
        .iter()
        .filter_map(Value::as_object)
        .filter_map(|value| value.get(field).and_then(Value::as_f64))
        .collect()
}

fn mean_or_zero(values: &[f64]) -> f64 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f64>() / values.len() as f64
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

fn derived_span(object: &Map<String, Value>, field: &str) -> Option<f64> {
    let prefix = field.strip_suffix("_span")?;
    let min = object.get(&format!("{prefix}_min"))?.as_f64()?;
    let max = object.get(&format!("{prefix}_max"))?.as_f64()?;
    Some(max - min)
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
