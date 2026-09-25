use crate::workflow_result_admission::require_converged_result;
use serde_json::Value;

pub use crate::workflow_field_extracts::{extract_field_hotspots, extract_field_statistics};
pub use crate::workflow_summary_transforms::merge_summary_pair;

pub fn extract_result_summary(payload: Value, config: Value) -> Result<Value, String> {
    let object = payload
        .as_object()
        .ok_or_else(|| "extract.result_summary expects an object payload".to_string())?;
    require_converged_result(object, "extract.result_summary", "payload")?;

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
