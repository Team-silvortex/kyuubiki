use serde_json::Value;

pub fn merge_summary_pair(payload: Value, config: Value) -> Result<Value, String> {
    let object = payload
        .as_object()
        .ok_or_else(|| "transform.merge_summary_pair expects an object payload".to_string())?;
    let left = object
        .get("left")
        .and_then(Value::as_object)
        .ok_or_else(|| "transform.merge_summary_pair expects object payload.left".to_string())?;
    let right = object
        .get("right")
        .and_then(Value::as_object)
        .ok_or_else(|| "transform.merge_summary_pair expects object payload.right".to_string())?;

    let left_prefix = config
        .get("left_prefix")
        .and_then(Value::as_str)
        .unwrap_or("left");
    let right_prefix = config
        .get("right_prefix")
        .and_then(Value::as_str)
        .unwrap_or("right");
    let include_source_count = config
        .get("include_source_count")
        .and_then(Value::as_bool)
        .unwrap_or(true);

    let mut merged = serde_json::Map::new();
    merge_summary_fields(&mut merged, left_prefix, left);
    merge_summary_fields(&mut merged, right_prefix, right);
    if include_source_count {
        merged.insert("summary_source_count".to_string(), Value::from(2));
    }
    if merged.is_empty() {
        return Err("transform.merge_summary_pair did not find any summary fields".to_string());
    }

    Ok(Value::Object(merged))
}

pub fn compare_summary_pair(payload: Value, config: Value) -> Result<Value, String> {
    let object = payload
        .as_object()
        .ok_or_else(|| "transform.compare_summary_pair expects an object payload".to_string())?;
    let left = object
        .get("left")
        .and_then(Value::as_object)
        .ok_or_else(|| "transform.compare_summary_pair expects object payload.left".to_string())?;
    let right = object
        .get("right")
        .and_then(Value::as_object)
        .ok_or_else(|| "transform.compare_summary_pair expects object payload.right".to_string())?;

    let left_prefix = config
        .get("left_prefix")
        .and_then(Value::as_str)
        .unwrap_or("left");
    let right_prefix = config
        .get("right_prefix")
        .and_then(Value::as_str)
        .unwrap_or("right");
    let delta_prefix = config
        .get("delta_prefix")
        .and_then(Value::as_str)
        .unwrap_or("delta");
    let ratio_prefix = config
        .get("ratio_prefix")
        .and_then(Value::as_str)
        .unwrap_or("ratio");
    let percent_prefix = config
        .get("percent_prefix")
        .and_then(Value::as_str)
        .unwrap_or("percent_change");
    let include_originals = config
        .get("include_originals")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let include_delta = config
        .get("include_delta")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let include_ratio = config
        .get("include_ratio")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let include_percent_change = config
        .get("include_percent_change")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let include_shared_field_count = config
        .get("include_shared_field_count")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let fields = resolve_summary_compare_fields(left, right, &config)?;

    let mut compared = serde_json::Map::new();
    let mut compared_field_count = 0usize;
    for field in fields {
        let Some(left_value) = left.get(&field).and_then(Value::as_f64) else {
            continue;
        };
        let Some(right_value) = right.get(&field).and_then(Value::as_f64) else {
            continue;
        };
        compared_field_count += 1;

        if include_originals {
            compared.insert(
                format!("{}_{}", normalize_prefix(left_prefix), field),
                Value::from(left_value),
            );
            compared.insert(
                format!("{}_{}", normalize_prefix(right_prefix), field),
                Value::from(right_value),
            );
        }
        if include_delta {
            compared.insert(
                format!("{}_{}", normalize_prefix(delta_prefix), field),
                Value::from(right_value - left_value),
            );
        }
        if include_ratio && left_value != 0.0 {
            compared.insert(
                format!("{}_{}", normalize_prefix(ratio_prefix), field),
                Value::from(right_value / left_value),
            );
        }
        if include_percent_change && left_value != 0.0 {
            compared.insert(
                format!("{}_{}", normalize_prefix(percent_prefix), field),
                Value::from(((right_value - left_value) / left_value) * 100.0),
            );
        }
    }

    if compared.is_empty() {
        return Err(
            "transform.compare_summary_pair did not find any shared numeric summary fields"
                .to_string(),
        );
    }

    if include_shared_field_count {
        compared.insert(
            "summary_shared_numeric_field_count".to_string(),
            Value::from(compared_field_count),
        );
    }
    compared.insert("summary_left_prefix".to_string(), Value::from(left_prefix));
    compared.insert(
        "summary_right_prefix".to_string(),
        Value::from(right_prefix),
    );

    Ok(Value::Object(compared))
}

pub fn aggregate_summary_collection(payload: Value, config: Value) -> Result<Value, String> {
    let object = payload.as_object().ok_or_else(|| {
        "transform.aggregate_summary_collection expects an object payload".to_string()
    })?;
    if object.is_empty() {
        return Err(
            "transform.aggregate_summary_collection requires at least one named summary input"
                .to_string(),
        );
    }

    let requested_fields = config
        .get("fields")
        .and_then(Value::as_array)
        .map(|fields| {
            fields
                .iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let include_values = config
        .get("include_values")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let include_sources = config
        .get("include_sources")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let output_prefix = config
        .get("output_prefix")
        .and_then(Value::as_str)
        .map(normalize_prefix)
        .unwrap_or_else(|| "aggregate".to_string());

    let source_entries = object
        .iter()
        .filter_map(|(source_id, summary)| summary.as_object().map(|entry| (source_id, entry)))
        .collect::<Vec<_>>();
    if source_entries.is_empty() {
        return Err(
            "transform.aggregate_summary_collection expects named object summaries".to_string(),
        );
    }

    let fields = if requested_fields.is_empty() {
        let mut discovered = source_entries
            .iter()
            .flat_map(|(_, summary)| summary.iter())
            .filter(|(_, value)| value.is_number())
            .map(|(field, _)| field.clone())
            .collect::<Vec<_>>();
        discovered.sort();
        discovered.dedup();
        discovered
    } else {
        requested_fields
    };

    let mut aggregated = serde_json::Map::new();
    let mut aggregated_field_count = 0usize;
    for field in fields {
        let mut values = Vec::new();
        let mut sources = Vec::new();
        for (source_id, summary) in &source_entries {
            let Some(value) = summary.get(&field).and_then(Value::as_f64) else {
                continue;
            };
            values.push(value);
            sources.push(Value::from((*source_id).to_string()));
        }
        if values.is_empty() {
            continue;
        }

        aggregated_field_count += 1;
        let count = values.len();
        let sum = values.iter().sum::<f64>();
        let mean = sum / count as f64;
        let min = values
            .iter()
            .fold(f64::INFINITY, |current, value| current.min(*value));
        let max = values
            .iter()
            .fold(f64::NEG_INFINITY, |current, value| current.max(*value));
        let span = max - min;
        let field_prefix = format!("{}_{}", output_prefix, field);

        aggregated.insert(format!("{field_prefix}_count"), Value::from(count));
        aggregated.insert(format!("{field_prefix}_min"), Value::from(min));
        aggregated.insert(format!("{field_prefix}_max"), Value::from(max));
        aggregated.insert(format!("{field_prefix}_mean"), Value::from(mean));
        aggregated.insert(format!("{field_prefix}_span"), Value::from(span));
        if include_values {
            aggregated.insert(format!("{field_prefix}_values"), Value::from(values));
        }
        if include_sources {
            aggregated.insert(format!("{field_prefix}_sources"), Value::Array(sources));
        }
    }

    if aggregated.is_empty() {
        return Err(
            "transform.aggregate_summary_collection did not find any numeric summary fields"
                .to_string(),
        );
    }

    aggregated.insert(
        "summary_input_count".to_string(),
        Value::from(source_entries.len()),
    );
    aggregated.insert(
        "summary_aggregated_field_count".to_string(),
        Value::from(aggregated_field_count),
    );

    Ok(Value::Object(aggregated))
}

pub fn normalize_summary_fields(payload: Value, config: Value) -> Result<Value, String> {
    let object = payload.as_object().ok_or_else(|| {
        "transform.normalize_summary_fields expects an object payload".to_string()
    })?;
    let rules = config
        .get("rules")
        .and_then(Value::as_array)
        .ok_or_else(|| "transform.normalize_summary_fields requires config.rules".to_string())?;
    let copy_unmapped = config
        .get("copy_unmapped")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let mut normalized = if copy_unmapped {
        object.clone()
    } else {
        serde_json::Map::new()
    };

    for rule in rules {
        let source = rule
            .get("source")
            .and_then(Value::as_str)
            .ok_or_else(|| "transform.normalize_summary_fields rule requires source".to_string())?;
        let target = rule.get("target").and_then(Value::as_str).unwrap_or(source);
        let Some(source_value) = object.get(source) else {
            continue;
        };

        let next_value = if let Some(numeric) = source_value.as_f64() {
            let scale = rule.get("scale").and_then(Value::as_f64).unwrap_or(1.0);
            let offset = rule.get("offset").and_then(Value::as_f64).unwrap_or(0.0);
            let clamp_min = rule.get("clamp_min").and_then(Value::as_f64);
            let clamp_max = rule.get("clamp_max").and_then(Value::as_f64);
            let mut next_numeric = numeric * scale + offset;
            if let Some(minimum) = clamp_min {
                next_numeric = next_numeric.max(minimum);
            }
            if let Some(maximum) = clamp_max {
                next_numeric = next_numeric.min(maximum);
            }
            Value::from(next_numeric)
        } else {
            source_value.clone()
        };

        normalized.insert(target.to_string(), next_value);
        if !copy_unmapped && target != source {
            normalized.insert(format!("source_{target}"), Value::from(source));
        }
    }

    if normalized.is_empty() {
        return Err(
            "transform.normalize_summary_fields did not emit any normalized fields".to_string(),
        );
    }

    Ok(Value::Object(normalized))
}

pub fn select_best_summary(payload: Value, config: Value) -> Result<Value, String> {
    let object = payload
        .as_object()
        .ok_or_else(|| "transform.select_best_summary expects an object payload".to_string())?;
    if object.is_empty() {
        return Err(
            "transform.select_best_summary requires at least one named summary input".to_string(),
        );
    }

    let criteria = config
        .get("criteria")
        .and_then(Value::as_array)
        .ok_or_else(|| "transform.select_best_summary requires config.criteria".to_string())?;
    if criteria.is_empty() {
        return Err("transform.select_best_summary config.criteria must not be empty".to_string());
    }

    let include_breakdown = config
        .get("include_breakdown")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let include_all_scores = config
        .get("include_all_scores")
        .and_then(Value::as_bool)
        .unwrap_or(true);

    let source_entries = object
        .iter()
        .filter_map(|(source_id, summary)| summary.as_object().map(|entry| (source_id, entry)))
        .collect::<Vec<_>>();
    if source_entries.is_empty() {
        return Err("transform.select_best_summary expects named object summaries".to_string());
    }

    let mut scored = source_entries
        .iter()
        .map(|(source_id, summary)| {
            let (score, breakdown) = score_summary_entry(summary, criteria)?;
            Ok((
                (*source_id).to_string(),
                (*summary).clone(),
                score,
                breakdown,
            ))
        })
        .collect::<Result<Vec<_>, String>>()?;
    scored.sort_by(|left, right| {
        right
            .2
            .total_cmp(&left.2)
            .then_with(|| left.0.cmp(&right.0))
    });

    let (best_source, best_summary, best_score, best_breakdown) =
        scored.first().cloned().ok_or_else(|| {
            "transform.select_best_summary could not score any summary inputs".to_string()
        })?;

    let mut selected = best_summary;
    selected.insert(
        "selected_summary_source".to_string(),
        Value::from(best_source),
    );
    selected.insert(
        "selected_summary_score".to_string(),
        Value::from(best_score),
    );
    if include_breakdown {
        selected.insert(
            "selected_summary_breakdown".to_string(),
            Value::Array(best_breakdown),
        );
    }
    if include_all_scores {
        let all_scores = scored
            .iter()
            .map(|(source_id, _, score, _)| {
                serde_json::json!({
                    "source": source_id,
                    "score": score,
                })
            })
            .collect::<Vec<_>>();
        selected.insert(
            "selected_summary_candidates".to_string(),
            Value::Array(all_scores),
        );
    }

    Ok(Value::Object(selected))
}

fn merge_summary_fields(
    merged: &mut serde_json::Map<String, Value>,
    prefix: &str,
    source: &serde_json::Map<String, Value>,
) {
    let normalized_prefix = prefix.trim();
    for (key, value) in source {
        let next_key = if normalized_prefix.is_empty() {
            key.clone()
        } else {
            format!("{normalized_prefix}_{key}")
        };
        merged.insert(next_key, value.clone());
    }
}

fn resolve_summary_compare_fields(
    left: &serde_json::Map<String, Value>,
    right: &serde_json::Map<String, Value>,
    config: &Value,
) -> Result<Vec<String>, String> {
    if let Some(fields) = config.get("fields").and_then(Value::as_array) {
        let requested = fields
            .iter()
            .filter_map(Value::as_str)
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        if requested.is_empty() {
            return Err(
                "transform.compare_summary_pair config.fields must include at least one field"
                    .to_string(),
            );
        }
        return Ok(requested);
    }

    let mut fields = left
        .iter()
        .filter(|(key, value)| value.is_number() && right.get(*key).is_some_and(Value::is_number))
        .map(|(key, _)| key.clone())
        .collect::<Vec<_>>();
    fields.sort();
    Ok(fields)
}

fn normalize_prefix(prefix: &str) -> String {
    let trimmed = prefix.trim();
    if trimmed.is_empty() {
        "summary".to_string()
    } else {
        trimmed.to_string()
    }
}

fn score_summary_entry(
    summary: &serde_json::Map<String, Value>,
    criteria: &[Value],
) -> Result<(f64, Vec<Value>), String> {
    let mut total = 0.0;
    let mut breakdown = Vec::new();
    for criterion in criteria {
        let field = criterion
            .get("field")
            .and_then(Value::as_str)
            .ok_or_else(|| "transform.select_best_summary criterion requires field".to_string())?;
        let goal = criterion
            .get("goal")
            .and_then(Value::as_str)
            .unwrap_or("max");
        let weight = criterion
            .get("weight")
            .and_then(Value::as_f64)
            .unwrap_or(1.0);
        let value = summary.get(field).and_then(Value::as_f64).ok_or_else(|| {
            format!("transform.select_best_summary missing numeric field {field}")
        })?;
        let criterion_score = match goal {
            "min" => -value * weight,
            "max" => value * weight,
            other => {
                return Err(format!(
                    "transform.select_best_summary unsupported criterion goal: {other}"
                ));
            }
        };
        total += criterion_score;
        breakdown.push(serde_json::json!({
            "field": field,
            "goal": goal,
            "weight": weight,
            "value": value,
            "score": criterion_score,
        }));
    }
    Ok((total, breakdown))
}
