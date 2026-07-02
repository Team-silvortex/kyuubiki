use serde_json::{Map, Value};

pub fn build_quality_parameter_sweep_plan(payload: Value, config: Value) -> Result<Value, String> {
    let request = payload
        .get("request_payload")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    if payload.get("action").and_then(Value::as_str) == Some("stop") {
        return Ok(serde_json::json!({
            "quality_parameter_sweep_plan_contract": "kyuubiki.quality_parameter_sweep_plan/v1",
            "sweep_enabled": false,
            "sweep_action": "stop",
            "case_count_estimate": 0,
            "axes": [],
            "base": config.get("base").cloned().unwrap_or_else(|| serde_json::json!({})),
            "plan_summary": "Quality exploration stopped; no parameter sweep was planned.",
        }));
    }

    let search_space = request
        .get("search_space")
        .or_else(|| config.get("search_space"))
        .and_then(Value::as_object)
        .ok_or_else(|| {
            "transform.build_quality_parameter_sweep_plan requires search_space".to_string()
        })?;
    let base = config
        .get("base")
        .or_else(|| payload.get("base"))
        .or_else(|| request.get("base"))
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    let samples = config_number(&config, "samples_per_axis", 3.0).round() as usize;
    let axes = search_space_axes(search_space, samples);
    if axes.is_empty() {
        return Err(
            "transform.build_quality_parameter_sweep_plan requires usable search_space axes"
                .to_string(),
        );
    }
    let case_count_estimate = axes
        .iter()
        .map(|axis| {
            axis.get("values")
                .and_then(Value::as_array)
                .map(Vec::len)
                .unwrap_or(0)
        })
        .product::<usize>();

    Ok(serde_json::json!({
        "quality_parameter_sweep_plan_contract": "kyuubiki.quality_parameter_sweep_plan/v1",
        "sweep_enabled": true,
        "sweep_action": payload.get("action").and_then(Value::as_str).unwrap_or("continue"),
        "source_candidate_id": payload.get("selected_candidate_id").cloned().unwrap_or(Value::Null),
        "target_score": payload.get("target_score").cloned().unwrap_or(Value::Null),
        "id_prefix": config.get("id_prefix").and_then(Value::as_str).unwrap_or("quality_round"),
        "max_cases": config_number(
            &config,
            "max_cases",
            request
                .get("max_candidates")
                .and_then(Value::as_f64)
                .unwrap_or(64.0),
        ),
        "case_count_estimate": case_count_estimate,
        "axes": axes,
        "base": base,
        "plan_summary": format!(
            "Quality parameter sweep planned with {} axis/axes and {case_count_estimate} estimated cases.",
            axes.len()
        ),
    }))
}

fn search_space_axes(search_space: &Map<String, Value>, samples: usize) -> Vec<Value> {
    search_space
        .iter()
        .filter_map(|(path, spec)| {
            let values = axis_values(spec, samples);
            if values.is_empty() {
                None
            } else {
                Some(serde_json::json!({
                    "label": path,
                    "path": path,
                    "values": values,
                }))
            }
        })
        .collect()
}

fn axis_values(spec: &Value, samples: usize) -> Vec<Value> {
    if let Some(values) = spec.as_array() {
        return values.clone();
    }
    if let Some(values) = spec.get("values").and_then(Value::as_array) {
        return values.clone();
    }
    let Some(min) = spec.get("min").and_then(Value::as_f64) else {
        return Vec::new();
    };
    let Some(max) = spec.get("max").and_then(Value::as_f64) else {
        return Vec::new();
    };
    if samples <= 1 {
        return Vec::new();
    }
    let step = (max - min) / (samples - 1) as f64;
    (0..samples)
        .map(|index| Value::from(min + step * index as f64))
        .collect()
}

fn config_number(config: &Value, field: &str, default_value: f64) -> f64 {
    config
        .get(field)
        .and_then(Value::as_f64)
        .filter(|value| value.is_finite() && *value >= 0.0)
        .unwrap_or(default_value)
}
