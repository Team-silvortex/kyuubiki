use serde_json::Value;

pub fn export_diagnostics_bundle_markdown(payload: Value, config: Value) -> Result<Value, String> {
    let object = payload.as_object().ok_or_else(|| {
        "export.diagnostics_bundle_markdown expects an object payload".to_string()
    })?;
    let title = config
        .get("title")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("Workflow Diagnostics Bundle");

    let mut lines = vec![
        format!("# {title}"),
        String::new(),
        format!(
            "- Contract: {}",
            object
                .get("bundle_contract")
                .map(markdown_value)
                .unwrap_or_else(|| "unknown".to_string())
        ),
        format!(
            "- Sources: {}",
            object
                .get("bundle_source_count")
                .map(markdown_value)
                .unwrap_or_else(|| "0".to_string())
        ),
        format!(
            "- Domains: {}",
            format_list(
                object
                    .get("bundle_domains")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default()
            )
        ),
        format!(
            "- Subjects: {}",
            format_list(
                object
                    .get("bundle_subjects")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default()
            )
        ),
        format!(
            "- Total Nodes: {}",
            object
                .get("bundle_total_node_count")
                .map(markdown_value)
                .unwrap_or_else(|| "0".to_string())
        ),
        format!(
            "- Total Elements: {}",
            object
                .get("bundle_total_element_count")
                .map(markdown_value)
                .unwrap_or_else(|| "0".to_string())
        ),
    ];

    lines.extend(build_bundle_metric_groups_section(object));
    lines.extend(build_bundle_highlights_section(object));
    lines.extend(build_bundle_items_section(object, &config));
    lines.extend(build_bundle_guard_section(object, &config));

    Ok(serde_json::json!({
        "format": "markdown",
        "content_type": "text/markdown",
        "content": lines.join("\n")
    }))
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

fn format_list(values: Vec<Value>) -> String {
    let values = values.iter().map(markdown_value).collect::<Vec<_>>();
    if values.is_empty() {
        "none".to_string()
    } else {
        values.join(", ")
    }
}

fn build_bundle_metric_groups_section(object: &serde_json::Map<String, Value>) -> Vec<String> {
    let Some(groups) = object.get("bundle_metric_groups").and_then(Value::as_array) else {
        return Vec::new();
    };
    if groups.is_empty() {
        return Vec::new();
    }
    vec![
        String::new(),
        "## Metric Groups".to_string(),
        String::new(),
        format!("- {}", format_list(groups.clone())),
    ]
}

fn build_bundle_highlights_section(object: &serde_json::Map<String, Value>) -> Vec<String> {
    let Some(highlights) = object.get("report_highlights").and_then(Value::as_array) else {
        return Vec::new();
    };
    if highlights.is_empty() {
        return Vec::new();
    }

    let mut lines = vec![String::new(), "## Key Highlights".to_string()];
    for highlight in highlights {
        let Some(highlight) = highlight.as_object() else {
            continue;
        };
        let label = highlight
            .get("label")
            .map(markdown_value)
            .unwrap_or_else(|| "unknown".to_string());
        let value = highlight
            .get("value")
            .map(markdown_value)
            .unwrap_or_else(|| "n/a".to_string());
        let attention = highlight
            .get("attention")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let marker = if attention { "attention" } else { "info" };
        lines.push(format!("- [{marker}] {label}: {value}"));
    }
    lines
}

fn build_bundle_items_section(
    object: &serde_json::Map<String, Value>,
    config: &Value,
) -> Vec<String> {
    let Some(items) = object.get("bundle_items").and_then(Value::as_array) else {
        return Vec::new();
    };
    if items.is_empty() {
        return Vec::new();
    }
    let max_items = config
        .get("item_count")
        .and_then(Value::as_u64)
        .map(|value| value.min(24) as usize)
        .unwrap_or(8);

    let mut lines = vec![String::new(), "## Diagnostics Sources".to_string()];
    for item in items.iter().take(max_items) {
        let Some(item) = item.as_object() else {
            continue;
        };
        lines.push(String::new());
        lines.push(format!(
            "### {}",
            item.get("source")
                .map(markdown_value)
                .unwrap_or_else(|| "unknown".to_string())
        ));
        lines.push(format!(
            "- Domain: {}",
            item.get("domain")
                .map(markdown_value)
                .unwrap_or_else(|| "unknown".to_string())
        ));
        lines.push(format!(
            "- Subject: {}",
            item.get("subject")
                .map(markdown_value)
                .unwrap_or_else(|| "unknown".to_string())
        ));
        lines.push(format!(
            "- Prefix: {}",
            item.get("prefix")
                .map(markdown_value)
                .unwrap_or_else(|| "unknown".to_string())
        ));
        lines.push(format!(
            "- Nodes: {}",
            item.get("node_count")
                .map(markdown_value)
                .unwrap_or_else(|| "0".to_string())
        ));
        lines.push(format!(
            "- Elements: {}",
            item.get("element_count")
                .map(markdown_value)
                .unwrap_or_else(|| "0".to_string())
        ));
        lines.push(format!(
            "- Metric Groups: {}",
            format_list(
                item.get("metric_groups")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default()
            )
        ));
    }
    lines
}

fn build_bundle_guard_section(
    object: &serde_json::Map<String, Value>,
    config: &Value,
) -> Vec<String> {
    let guard_payload = object
        .get("guard_payload")
        .and_then(Value::as_object)
        .or_else(|| config.get("guard_payload").and_then(Value::as_object))
        .or(Some(object));
    let Some(guard_payload) = guard_payload else {
        return Vec::new();
    };
    if !guard_payload.contains_key("guard_status") {
        return Vec::new();
    }

    let mut lines = vec![
        String::new(),
        "## Guard Decision".to_string(),
        String::new(),
        format!(
            "- Status: {}",
            guard_payload
                .get("guard_status")
                .map(markdown_value)
                .unwrap_or_else(|| "unknown".to_string())
        ),
        format!(
            "- Passed: {}",
            guard_payload
                .get("guard_passed")
                .map(markdown_value)
                .unwrap_or_else(|| "false".to_string())
        ),
        format!(
            "- Recommendation: {}",
            guard_payload
                .get("guard_recommendation")
                .map(markdown_value)
                .unwrap_or_else(|| "continue".to_string())
        ),
        format!(
            "- Summary: {}",
            guard_payload
                .get("guard_summary")
                .map(markdown_value)
                .unwrap_or_else(|| "No guard summary.".to_string())
        ),
    ];
    lines.extend(build_bundle_guard_triggers(guard_payload));
    lines
}

fn build_bundle_guard_triggers(guard_payload: &serde_json::Map<String, Value>) -> Vec<String> {
    let Some(triggers) = guard_payload
        .get("guard_triggers")
        .and_then(Value::as_array)
    else {
        return Vec::new();
    };
    if triggers.is_empty() {
        return Vec::new();
    }

    let mut lines = vec![String::new(), "### Guard Triggers".to_string()];
    for trigger in triggers {
        let Some(trigger) = trigger.as_object() else {
            continue;
        };
        let source = trigger
            .get("source")
            .map(markdown_value)
            .unwrap_or_else(|| "bundle".to_string());
        let label = trigger
            .get("label")
            .or_else(|| trigger.get("field"))
            .map(markdown_value)
            .unwrap_or_else(|| "unknown".to_string());
        let value = trigger
            .get("value")
            .map(markdown_value)
            .unwrap_or_else(|| "n/a".to_string());
        let comparison = trigger
            .get("comparison")
            .map(markdown_value)
            .unwrap_or_else(|| "gte".to_string());
        let threshold = trigger
            .get("threshold")
            .map(markdown_value)
            .unwrap_or_else(|| "n/a".to_string());
        let severity = trigger
            .get("severity")
            .map(markdown_value)
            .unwrap_or_else(|| "warn".to_string());
        lines.push(format!(
            "- {source}.{label}: {value} {comparison} {threshold} ({severity})"
        ));
    }
    lines
}
