use serde_json::Value;

pub use crate::workflow_summary_transforms::{compare_summary_pair, merge_summary_pair};

pub fn extract_result_summary(payload: Value, config: Value) -> Result<Value, String> {
    let object = payload
        .as_object()
        .ok_or_else(|| "extract.result_summary expects an object payload".to_string())?;

    let requested_fields = config
        .get("fields")
        .and_then(Value::as_array)
        .map(|fields| {
            fields
                .iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        });

    let mut summary = serde_json::Map::new();
    if let Some(fields) = requested_fields {
        for field in fields {
            if let Some(value) = object.get(&field) {
                summary.insert(field, value.clone());
            }
        }
    } else {
        for (key, value) in object {
            if key.starts_with("max_") {
                summary.insert(key.clone(), value.clone());
            }
        }
    }

    if summary.is_empty() {
        return Err("extract.result_summary did not find any summary fields".to_string());
    }

    Ok(Value::Object(summary))
}

pub fn extract_field_statistics(payload: Value, config: Value) -> Result<Value, String> {
    let object = payload
        .as_object()
        .ok_or_else(|| "extract.field_statistics expects an object payload".to_string())?;
    let source = config
        .get("source")
        .and_then(Value::as_str)
        .unwrap_or("nodes");
    let field = config
        .get("field")
        .and_then(Value::as_str)
        .ok_or_else(|| "extract.field_statistics requires config.field".to_string())?;
    let items = object
        .get(source)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("extract.field_statistics expects array payload.{source}"))?;
    let values = items
        .iter()
        .filter_map(|item| item.get(field).and_then(Value::as_f64))
        .collect::<Vec<_>>();
    if values.is_empty() {
        return Err(format!(
            "extract.field_statistics did not find numeric values for {source}.{field}"
        ));
    }

    let count = values.len() as f64;
    let sum = values.iter().sum::<f64>();
    let mean = sum / count;
    let min = values
        .iter()
        .fold(f64::INFINITY, |current, value| current.min(*value));
    let max = values
        .iter()
        .fold(f64::NEG_INFINITY, |current, value| current.max(*value));
    let stddev = population_stddev(&values, mean);
    let prefix = config
        .get("output_prefix")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(|value| value.trim().to_string())
        .unwrap_or_else(|| field.to_string());
    let percentiles = config
        .get("percentiles")
        .and_then(Value::as_array)
        .map(|entries| {
            entries
                .iter()
                .filter_map(|entry| match entry {
                    Value::Number(number) => number.as_f64(),
                    _ => None,
                })
                .filter(|value| (0.0..=100.0).contains(value))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let mut summary = serde_json::Map::new();
    summary.insert(format!("{prefix}_count"), Value::from(values.len()));
    summary.insert(format!("{prefix}_min"), Value::from(min));
    summary.insert(format!("{prefix}_max"), Value::from(max));
    summary.insert(format!("{prefix}_sum"), Value::from(sum));
    summary.insert(format!("{prefix}_mean"), Value::from(mean));
    summary.insert(format!("{prefix}_stddev"), Value::from(stddev));
    for percentile in percentiles {
        let percentile_key = format_percentile_key(percentile);
        let percentile_value = interpolate_percentile(&values, percentile);
        summary.insert(
            format!("{prefix}_{percentile_key}"),
            Value::from(percentile_value),
        );
    }
    summary.insert("source_collection".to_string(), Value::from(source));
    summary.insert("source_field".to_string(), Value::from(field));

    Ok(Value::Object(summary))
}

pub fn extract_field_hotspots(payload: Value, config: Value) -> Result<Value, String> {
    let object = payload
        .as_object()
        .ok_or_else(|| "extract.field_hotspots expects an object payload".to_string())?;
    let source = config
        .get("source")
        .and_then(Value::as_str)
        .unwrap_or("elements");
    let field = config
        .get("field")
        .and_then(Value::as_str)
        .ok_or_else(|| "extract.field_hotspots requires config.field".to_string())?;
    let items = object
        .get(source)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("extract.field_hotspots expects array payload.{source}"))?;
    let threshold = resolve_hotspot_threshold(items, field, &config)?;
    let output_prefix = config
        .get("output_prefix")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(|value| value.trim().to_string())
        .unwrap_or_else(|| field.to_string());
    let sample_limit = config
        .get("sample_limit")
        .and_then(Value::as_u64)
        .map(|value| value.min(32) as usize)
        .unwrap_or(8);
    let sample_sort = config
        .get("sample_sort")
        .and_then(Value::as_str)
        .unwrap_or("value_desc");
    let mut hotspots = Vec::new();
    for item in items {
        let Some(value) = item.get(field).and_then(Value::as_f64) else {
            continue;
        };
        if value < threshold {
            continue;
        }
        hotspots.push((value, item.clone()));
    }
    if hotspots.is_empty() {
        return Err(format!(
            "extract.field_hotspots did not find any values meeting the threshold for {source}.{field}"
        ));
    }
    sort_hotspots(&mut hotspots, sample_sort);

    let hotspot_values = hotspots.iter().map(|(value, _)| *value).collect::<Vec<_>>();
    let hotspot_ids = hotspots
        .iter()
        .filter_map(|(_, item)| item.get("id").cloned())
        .collect::<Vec<_>>();
    let hotspot_samples = hotspots
        .iter()
        .take(sample_limit)
        .map(|(_, item)| item.clone())
        .collect::<Vec<_>>();

    let hotspot_count = hotspot_values.len();
    let hotspot_sum = hotspot_values.iter().sum::<f64>();
    let hotspot_mean = hotspot_sum / hotspot_count as f64;
    let hotspot_max = hotspot_values
        .iter()
        .fold(f64::NEG_INFINITY, |current, value| current.max(*value));
    let hotspot_fraction = hotspot_count as f64 / items.len() as f64;

    Ok(serde_json::json!({
        format!("{output_prefix}_threshold"): threshold,
        format!("{output_prefix}_hotspot_count"): hotspot_count,
        format!("{output_prefix}_hotspot_fraction"): hotspot_fraction,
        format!("{output_prefix}_hotspot_mean"): hotspot_mean,
        format!("{output_prefix}_hotspot_max"): hotspot_max,
        format!("{output_prefix}_sample_sort"): sample_sort,
        format!("{output_prefix}_hotspot_ids"): hotspot_ids,
        format!("{output_prefix}_hotspot_samples"): hotspot_samples,
        "source_collection": source,
        "source_field": field,
    }))
}

pub fn export_summary_json(payload: Value) -> Result<Value, String> {
    if !payload.is_object() {
        return Err("export.summary_json expects an object payload".to_string());
    }
    let content = serde_json::to_string_pretty(&payload).map_err(|err| err.to_string())?;
    Ok(serde_json::json!({
        "format": "json",
        "content_type": "application/json",
        "content": content
    }))
}

pub fn export_summary_csv(payload: Value, config: Value) -> Result<Value, String> {
    let object = payload
        .as_object()
        .ok_or_else(|| "export.summary_csv expects an object payload".to_string())?;

    let requested_fields = config
        .get("fields")
        .and_then(Value::as_array)
        .map(|fields| {
            fields
                .iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        });

    let mut rows = vec!["key,value".to_string()];
    if let Some(fields) = requested_fields {
        for field in fields {
            if let Some(value) = object.get(&field) {
                rows.push(format!("{},{}", field, csv_cell(value)));
            }
        }
    } else {
        for (key, value) in object {
            rows.push(format!("{},{}", key, csv_cell(value)));
        }
    }

    if rows.len() == 1 {
        return Err("export.summary_csv did not find any exportable fields".to_string());
    }

    Ok(serde_json::json!({
        "format": "csv",
        "content_type": "text/csv",
        "content": rows.join("\n")
    }))
}

pub fn export_alert_markdown(payload: Value, config: Value) -> Result<Value, String> {
    let object = payload
        .as_object()
        .ok_or_else(|| "export.alert_markdown expects an object payload".to_string())?;
    let title = config
        .get("title")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("Workflow Alert");
    let severity = config
        .get("severity")
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .or_else(|| {
            config
                .get("severity_path")
                .and_then(Value::as_str)
                .and_then(|path| resolve_path_value(&payload, path))
                .and_then(Value::as_str)
                .map(ToString::to_string)
        })
        .unwrap_or_else(|| "warning".to_string());
    let fields = config
        .get("fields")
        .and_then(Value::as_array)
        .map(|entries| {
            entries
                .iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        });
    let summary = config
        .get("summary")
        .and_then(Value::as_str)
        .unwrap_or("The workflow produced an alertable summary payload.");

    let mut lines = vec![
        format!("# {title}"),
        String::new(),
        format!("- Severity: {severity}"),
        format!("- Summary: {summary}"),
    ];

    if let Some(fields) = fields {
        for field in fields {
            if let Some(value) = object.get(&field) {
                lines.push(format!("- {field}: {}", markdown_value(value)));
            }
        }
    } else {
        for (key, value) in object {
            lines.push(format!("- {key}: {}", markdown_value(value)));
        }
    }

    if let Some(section) = build_alert_samples_section(object, &config) {
        lines.push(String::new());
        lines.push(section);
    }

    Ok(serde_json::json!({
        "format": "markdown",
        "content_type": "text/markdown",
        "content": lines.join("\n")
    }))
}

fn csv_cell(value: &Value) -> String {
    match value {
        Value::Null => "".to_string(),
        Value::Bool(boolean) => boolean.to_string(),
        Value::Number(number) => number.to_string(),
        Value::String(string) => {
            if string.contains([',', '"', '\n']) {
                format!("\"{}\"", string.replace('"', "\"\""))
            } else {
                string.clone()
            }
        }
        other => serde_json::to_string(other).unwrap_or_else(|_| "\"<invalid>\"".to_string()),
    }
}

fn markdown_value(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(boolean) => boolean.to_string(),
        Value::Number(number) => number.to_string(),
        Value::String(string) => string.clone(),
        other => serde_json::to_string(other).unwrap_or_else(|_| "<invalid>".to_string()),
    }
}

fn build_alert_samples_section(
    object: &serde_json::Map<String, Value>,
    config: &Value,
) -> Option<String> {
    let sample_field = config
        .get("sample_field")
        .and_then(Value::as_str)
        .unwrap_or("field_hotspot_samples");
    let sample_value_key = config
        .get("sample_value_key")
        .and_then(Value::as_str)
        .unwrap_or("electric_field_magnitude");
    let sample_id_key = config
        .get("sample_id_key")
        .and_then(Value::as_str)
        .unwrap_or("id");
    let sample_count = config
        .get("sample_count")
        .and_then(Value::as_u64)
        .map(|value| value.min(16) as usize)
        .unwrap_or(3);
    let samples = object.get(sample_field)?.as_array()?;
    if samples.is_empty() {
        return None;
    }

    let mut lines = vec!["## Sample Context".to_string()];
    for sample in samples.iter().take(sample_count) {
        let entry = sample.as_object()?;
        let label = entry
            .get(sample_id_key)
            .map(markdown_value)
            .unwrap_or_else(|| "unknown".to_string());
        let value = entry
            .get(sample_value_key)
            .map(markdown_value)
            .unwrap_or_else(|| "n/a".to_string());
        lines.push(format!("- {label}: {sample_value_key}={value}"));
    }
    Some(lines.join("\n"))
}

fn population_stddev(values: &[f64], mean: f64) -> f64 {
    let variance = values
        .iter()
        .map(|value| {
            let delta = *value - mean;
            delta * delta
        })
        .sum::<f64>()
        / values.len() as f64;
    variance.sqrt()
}

fn interpolate_percentile(values: &[f64], percentile: f64) -> f64 {
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    if sorted.len() == 1 {
        return sorted[0];
    }

    let position = (percentile / 100.0) * (sorted.len() - 1) as f64;
    let lower_index = position.floor() as usize;
    let upper_index = position.ceil() as usize;
    if lower_index == upper_index {
        return sorted[lower_index];
    }

    let weight = position - lower_index as f64;
    sorted[lower_index] * (1.0 - weight) + sorted[upper_index] * weight
}

fn format_percentile_key(percentile: f64) -> String {
    let mut normalized = percentile.to_string();
    if let Some(stripped) = normalized.strip_suffix(".0") {
        normalized = stripped.to_string();
    }
    format!("p{}", normalized.replace('.', "_"))
}

fn resolve_hotspot_threshold(
    items: &[Value],
    field: &str,
    config: &Value,
) -> Result<f64, String> {
    if let Some(threshold) = config.get("threshold").and_then(Value::as_f64) {
        return Ok(threshold);
    }

    let percentile = config
        .get("percentile")
        .and_then(Value::as_f64)
        .unwrap_or(90.0);
    if !(0.0..=100.0).contains(&percentile) {
        return Err("extract.field_hotspots config.percentile must be between 0 and 100".to_string());
    }

    let values = items
        .iter()
        .filter_map(|item| item.get(field).and_then(Value::as_f64))
        .collect::<Vec<_>>();
    if values.is_empty() {
        return Err(format!(
            "extract.field_hotspots did not find numeric values for field {field}"
        ));
    }
    Ok(interpolate_percentile(&values, percentile))
}

fn sort_hotspots(hotspots: &mut [(f64, Value)], sample_sort: &str) {
    match sample_sort {
        "value_asc" => hotspots.sort_by(|left, right| left.0.total_cmp(&right.0)),
        _ => hotspots.sort_by(|left, right| right.0.total_cmp(&left.0)),
    }
}

fn resolve_path_value<'a>(payload: &'a Value, path: &str) -> Option<&'a Value> {
    let mut current = payload;
    for segment in path.split('.').filter(|segment| !segment.is_empty()) {
        current = match current {
            Value::Object(map) => map.get(segment)?,
            Value::Array(items) => items.get(segment.parse::<usize>().ok()?)?,
            _ => return None,
        };
    }
    Some(current)
}
