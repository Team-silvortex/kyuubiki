use serde_json::Value;

pub fn compose_diagnostics_bundle(payload: Value, config: Value) -> Result<Value, String> {
    let object = payload.as_object().ok_or_else(|| {
        "transform.compose_diagnostics_bundle expects an object payload".to_string()
    })?;
    let include_non_diagnostics = config
        .get("include_non_diagnostics")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let include_payloads = config
        .get("include_payloads")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let include_numeric_fields = config
        .get("include_numeric_fields")
        .and_then(Value::as_bool)
        .unwrap_or(true);

    let diagnostics = object
        .iter()
        .filter(|(_, entry)| entry.is_object())
        .filter(|(_, entry)| diagnostic_entry(entry, include_non_diagnostics))
        .collect::<Vec<_>>();
    if diagnostics.is_empty() {
        return Err(
            "transform.compose_diagnostics_bundle did not find any diagnostics payloads"
                .to_string(),
        );
    }

    let items = diagnostics
        .iter()
        .map(|(source_id, entry)| {
            let entry = entry.as_object().expect("filtered diagnostics entry");
            serde_json::json!({
                "source": source_id,
                "domain": entry.get("diagnostic_domain").cloned().unwrap_or(Value::Null),
                "subject": entry.get("diagnostic_subject").cloned().unwrap_or(Value::Null),
                "prefix": entry.get("diagnostic_prefix").cloned().unwrap_or(Value::Null),
                "node_count": entry.get("diagnostic_node_count").cloned().unwrap_or(Value::from(0)),
                "element_count": entry.get("diagnostic_element_count").cloned().unwrap_or(Value::from(0)),
                "metric_groups": entry.get("diagnostic_metric_groups").cloned().unwrap_or(Value::Array(Vec::new())),
            })
        })
        .collect::<Vec<_>>();

    let mut numeric_fields = diagnostics
        .iter()
        .flat_map(|(_, entry)| {
            entry
                .as_object()
                .into_iter()
                .flat_map(|object| object.iter())
                .filter(|(_, value)| value.is_number())
                .map(|(field, _)| field.clone())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    numeric_fields.sort();
    numeric_fields.dedup();

    let domains = sorted_unique_strings(items.iter().filter_map(|item| {
        item.get("domain")
            .and_then(Value::as_str)
            .map(ToString::to_string)
    }));
    let subjects = sorted_unique_strings(items.iter().filter_map(|item| {
        item.get("subject")
            .and_then(Value::as_str)
            .map(ToString::to_string)
    }));
    let metric_groups = sorted_unique_strings(items.iter().flat_map(|item| {
        item.get("metric_groups")
            .and_then(Value::as_array)
            .into_iter()
            .flat_map(|groups| groups.iter())
            .filter_map(Value::as_str)
            .map(ToString::to_string)
            .collect::<Vec<_>>()
    }));
    let mut domain_counts = serde_json::Map::new();
    for domain in items
        .iter()
        .filter_map(|item| item.get("domain").and_then(Value::as_str))
    {
        let current = domain_counts
            .get(domain)
            .and_then(Value::as_u64)
            .unwrap_or(0);
        domain_counts.insert(domain.to_string(), Value::from(current + 1));
    }

    let total_node_count = items
        .iter()
        .filter_map(|item| item.get("node_count").and_then(Value::as_u64))
        .sum::<u64>();
    let total_element_count = items
        .iter()
        .filter_map(|item| item.get("element_count").and_then(Value::as_u64))
        .sum::<u64>();

    let mut bundle = serde_json::json!({
        "bundle_contract": "kyuubiki.workflow_diagnostics_bundle/v1",
        "bundle_kind": "workflow_diagnostics_bundle",
        "bundle_source_count": items.len(),
        "bundle_sources": items.iter().filter_map(|item| item.get("source").and_then(Value::as_str)).collect::<Vec<_>>(),
        "bundle_domains": domains,
        "bundle_subjects": subjects,
        "bundle_domain_counts": domain_counts,
        "bundle_metric_groups": metric_groups,
        "bundle_items": items,
        "bundle_total_node_count": total_node_count,
        "bundle_total_element_count": total_element_count,
        "bundle_numeric_field_count": numeric_fields.len(),
    });

    if let Some(object) = bundle.as_object_mut() {
        if include_payloads {
            object.insert(
                "bundle_payloads".to_string(),
                Value::Object(
                    diagnostics
                        .iter()
                        .map(|(source_id, entry)| ((*source_id).clone(), (*entry).clone()))
                        .collect(),
                ),
            );
        }
        if include_numeric_fields {
            object.insert(
                "bundle_numeric_fields".to_string(),
                Value::Array(numeric_fields.into_iter().map(Value::from).collect()),
            );
        }
    }

    Ok(bundle)
}

pub fn evaluate_diagnostics_bundle_guard(payload: Value, config: Value) -> Result<Value, String> {
    let object = payload.as_object().ok_or_else(|| {
        "transform.evaluate_diagnostics_bundle_guard expects an object payload".to_string()
    })?;
    let rules = config
        .get("rules")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            "transform.evaluate_diagnostics_bundle_guard requires config.rules".to_string()
        })?;
    if rules.is_empty() {
        return Err(
            "transform.evaluate_diagnostics_bundle_guard requires at least one rule".to_string(),
        );
    }

    let triggers = rules
        .iter()
        .filter_map(|rule| evaluate_bundle_guard_rule(object, rule))
        .collect::<Vec<_>>();
    let block_count = triggers
        .iter()
        .filter(|trigger| trigger["severity"].as_str() == Some("block"))
        .count();
    let warn_count = triggers
        .iter()
        .filter(|trigger| trigger["severity"].as_str() == Some("warn"))
        .count();
    let status = if block_count > 0 {
        "block"
    } else if warn_count > 0 {
        "warn"
    } else {
        "pass"
    };

    Ok(serde_json::json!({
        "guard_contract": "kyuubiki.workflow_guard_result/v1",
        "guard_scope": "workflow_diagnostics_bundle",
        "guard_status": status,
        "guard_passed": status == "pass",
        "guard_trigger_count": triggers.len(),
        "guard_checked_rule_count": rules.len(),
        "guard_warn_count": warn_count,
        "guard_block_count": block_count,
        "guard_triggers": triggers,
        "guard_recommendation": bundle_guard_recommendation(status),
        "guard_summary": bundle_guard_summary(status, &triggers),
    }))
}

pub fn compose_diagnostics_report_payload(payload: Value, config: Value) -> Result<Value, String> {
    let object = payload.as_object().ok_or_else(|| {
        "transform.compose_diagnostics_report_payload expects an object payload".to_string()
    })?;
    let bundle = object
        .get("bundle")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            "transform.compose_diagnostics_report_payload expects payload.bundle".to_string()
        })?;
    let guard = object
        .get("guard")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            "transform.compose_diagnostics_report_payload expects payload.guard".to_string()
        })?;
    let include_guard = config
        .get("include_guard")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let include_bundle_items = config
        .get("include_bundle_items")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let focus_metrics = report_focus_metrics(bundle);
    let highlights = report_highlights(bundle, guard);

    let mut report = Value::Object(bundle.clone());
    if let Some(report_object) = report.as_object_mut() {
        if !include_bundle_items {
            report_object.remove("bundle_items");
        }
        if include_guard {
            report_object.insert("guard_payload".to_string(), Value::Object(guard.clone()));
            report_object.insert(
                "report_guard_status".to_string(),
                guard.get("guard_status").cloned().unwrap_or(Value::Null),
            );
            report_object.insert(
                "report_guard_recommendation".to_string(),
                guard
                    .get("guard_recommendation")
                    .cloned()
                    .unwrap_or(Value::Null),
            );
        }
        report_object.insert(
            "report_contract".to_string(),
            Value::from("kyuubiki.workflow_report_payload/v1"),
        );
        report_object.insert(
            "report_kind".to_string(),
            Value::from("diagnostics_bundle_report_payload"),
        );
        report_object.insert(
            "report_sources".to_string(),
            bundle
                .get("bundle_sources")
                .cloned()
                .unwrap_or_else(|| Value::Array(Vec::new())),
        );
        report_object.insert("report_focus_metrics".to_string(), Value::Object(focus_metrics));
        report_object.insert("report_highlights".to_string(), Value::Array(highlights));
    }
    Ok(report)
}

fn report_focus_metrics(bundle: &serde_json::Map<String, Value>) -> serde_json::Map<String, Value> {
    let mut focus = serde_json::Map::new();
    let Some(payloads) = bundle.get("bundle_payloads").and_then(Value::as_object) else {
        return focus;
    };

    insert_focus_metric(
        &mut focus,
        payloads,
        "electrostatic",
        &["electrostatic_potential_max"],
        "electrostatic.potential_max",
    );
    insert_focus_metric(
        &mut focus,
        payloads,
        "electrostatic",
        &["electrostatic_peak_field", "electrostatic_field_peak_magnitude"],
        "electrostatic.field_peak",
    );
    insert_focus_metric(
        &mut focus,
        payloads,
        "thermal",
        &["thermal_temperature_max"],
        "thermal.temperature_max",
    );
    insert_focus_metric(
        &mut focus,
        payloads,
        "thermal",
        &["thermal_peak_flux", "thermal_flux_peak_magnitude"],
        "thermal.flux_peak",
    );
    insert_focus_metric(
        &mut focus,
        payloads,
        "thermo",
        &["thermo_temperature_delta_max"],
        "thermo.temperature_delta_max",
    );
    insert_focus_metric(
        &mut focus,
        payloads,
        "thermo",
        &["thermo_peak_displacement", "thermo_displacement_peak_magnitude"],
        "thermo.displacement_peak",
    );
    insert_focus_metric(
        &mut focus,
        payloads,
        "thermo",
        &["thermo_peak_stress", "thermo_stress_peak"],
        "thermo.stress_peak",
    );
    insert_focus_metric(
        &mut focus,
        payloads,
        "thermo",
        &["thermo_peak_thermal_strain", "thermo_thermal_strain_peak"],
        "thermo.thermal_strain_peak",
    );
    insert_focus_metric(
        &mut focus,
        payloads,
        "thermo",
        &["thermo_peak_mechanical_strain", "thermo_mechanical_strain_peak"],
        "thermo.mechanical_strain_peak",
    );
    insert_focus_metric(
        &mut focus,
        payloads,
        "thermo",
        &["thermo_peak_total_strain", "thermo_total_strain_peak"],
        "thermo.total_strain_peak",
    );

    focus
}

fn insert_focus_metric(
    focus: &mut serde_json::Map<String, Value>,
    payloads: &serde_json::Map<String, Value>,
    source: &str,
    fields: &[&str],
    key: &str,
) {
    let Some(payload) = payloads.get(source).and_then(Value::as_object) else {
        return;
    };
    if let Some(value) = fields.iter().find_map(|field| payload.get(*field).cloned()) {
        focus.insert(key.to_string(), value);
    }
}

fn report_highlights(
    bundle: &serde_json::Map<String, Value>,
    guard: &serde_json::Map<String, Value>,
) -> Vec<Value> {
    let focus = report_focus_metrics(bundle);
    let triggered_fields = guard
        .get("guard_triggers")
        .and_then(Value::as_array)
        .into_iter()
        .flat_map(|items| items.iter())
        .filter_map(|item| item.get("field").and_then(Value::as_str))
        .collect::<Vec<_>>();
    let mut highlights = Vec::new();

    push_highlight(
        &mut highlights,
        &focus,
        "electrostatic.potential_max",
        "Electrostatic potential peak",
        &triggered_fields,
        &["electrostatic_potential_max"],
    );
    push_highlight(
        &mut highlights,
        &focus,
        "electrostatic.field_peak",
        "Electrostatic field peak",
        &triggered_fields,
        &["electrostatic_peak_field", "electrostatic_field_peak_magnitude"],
    );
    push_highlight(
        &mut highlights,
        &focus,
        "thermal.temperature_max",
        "Thermal temperature peak",
        &triggered_fields,
        &["thermal_temperature_max"],
    );
    push_highlight(
        &mut highlights,
        &focus,
        "thermo.temperature_delta_max",
        "Thermo temperature delta peak",
        &triggered_fields,
        &["thermo_temperature_delta_max"],
    );
    push_highlight(
        &mut highlights,
        &focus,
        "thermo.stress_peak",
        "Thermo stress peak",
        &triggered_fields,
        &["thermo_peak_stress", "thermo_stress_peak"],
    );
    push_highlight(
        &mut highlights,
        &focus,
        "thermo.thermal_strain_peak",
        "Thermo thermal strain peak",
        &triggered_fields,
        &["thermo_peak_thermal_strain", "thermo_thermal_strain_peak"],
    );

    highlights
}

fn push_highlight(
    highlights: &mut Vec<Value>,
    focus: &serde_json::Map<String, Value>,
    metric_key: &str,
    label: &str,
    triggered_fields: &[&str],
    source_fields: &[&str],
) {
    let Some(value) = focus.get(metric_key).cloned() else {
        return;
    };
    let attention = source_fields
        .iter()
        .any(|field| triggered_fields.iter().any(|trigger| trigger == field));
    highlights.push(serde_json::json!({
        "id": metric_key,
        "label": label,
        "value": value,
        "attention": attention,
    }));
}

fn diagnostic_entry(entry: &Value, include_non_diagnostics: bool) -> bool {
    include_non_diagnostics
        || entry.get("diagnostic_contract").and_then(Value::as_str)
            == Some("kyuubiki.workflow_diagnostics/v1")
}

fn sorted_unique_strings<I>(values: I) -> Vec<String>
where
    I: IntoIterator<Item = String>,
{
    let mut values = values.into_iter().collect::<Vec<_>>();
    values.sort();
    values.dedup();
    values
}

fn evaluate_bundle_guard_rule(
    payload: &serde_json::Map<String, Value>,
    rule: &Value,
) -> Option<Value> {
    let rule = rule.as_object()?;
    let field = rule.get("field")?.as_str()?;
    let (value, source_ref) = fetch_bundle_guard_value(payload, rule, field)?;
    if !bundle_guard_triggered(value, rule) {
        return None;
    }

    Some(serde_json::json!({
        "field": field,
        "source": source_ref,
        "value": value,
        "threshold": bundle_guard_threshold(rule),
        "comparison": bundle_guard_comparison(rule),
        "severity": bundle_guard_severity(rule.get("severity").and_then(Value::as_str)),
        "label": rule.get("label").and_then(Value::as_str).unwrap_or(field),
    }))
}

fn fetch_bundle_guard_value(
    payload: &serde_json::Map<String, Value>,
    rule: &serde_json::Map<String, Value>,
    field: &str,
) -> Option<(f64, String)> {
    if let Some(source) = rule.get("source").and_then(Value::as_str) {
        let source_payload = payload
            .get("bundle_payloads")
            .and_then(Value::as_object)?
            .get(source)
            .and_then(Value::as_object)?;
        source_payload
            .get(field)
            .and_then(Value::as_f64)
            .map(|value| (value, source.to_string()))
    } else {
        payload
            .get(field)
            .and_then(Value::as_f64)
            .map(|value| (value, "bundle".to_string()))
    }
}

fn bundle_guard_triggered(value: f64, rule: &serde_json::Map<String, Value>) -> bool {
    match (bundle_guard_comparison(rule), bundle_guard_threshold(rule)) {
        ("gt", Some(threshold)) => value > threshold,
        ("gte", Some(threshold)) => value >= threshold,
        ("lt", Some(threshold)) => value < threshold,
        ("lte", Some(threshold)) => value <= threshold,
        ("eq", Some(threshold)) => value == threshold,
        _ => false,
    }
}

fn bundle_guard_comparison<'a>(rule: &'a serde_json::Map<String, Value>) -> &'a str {
    match rule
        .get("comparison")
        .and_then(Value::as_str)
        .unwrap_or("gte")
    {
        "gt" | "gte" | "lt" | "lte" | "eq" => rule
            .get("comparison")
            .and_then(Value::as_str)
            .unwrap_or("gte"),
        _ => "gte",
    }
}

fn bundle_guard_threshold(rule: &serde_json::Map<String, Value>) -> Option<f64> {
    rule.get("threshold")
        .and_then(Value::as_f64)
        .or_else(|| rule.get("value").and_then(Value::as_f64))
}

fn bundle_guard_severity(severity: Option<&str>) -> &'static str {
    match severity.unwrap_or("warn") {
        "block" => "block",
        _ => "warn",
    }
}

fn bundle_guard_recommendation(status: &str) -> &'static str {
    match status {
        "block" => "hold_and_review",
        "warn" => "review_before_continue",
        _ => "continue",
    }
}

fn bundle_guard_summary(status: &str, triggers: &[Value]) -> String {
    if status == "pass" {
        return "All diagnostics bundle guard rules passed.".to_string();
    }

    let lead = triggers
        .iter()
        .take(2)
        .filter_map(|trigger| {
            Some(format!(
                "{}.{}={}",
                trigger.get("source")?.as_str()?,
                trigger.get("label")?.as_str()?,
                trigger.get("value")?.as_f64()?
            ))
        })
        .collect::<Vec<_>>()
        .join(", ");

    if lead.is_empty() {
        format!("{}: {} trigger(s).", status.to_uppercase(), triggers.len())
    } else {
        format!(
            "{}: {} trigger(s) ({}).",
            status.to_uppercase(),
            triggers.len(),
            lead
        )
    }
}
