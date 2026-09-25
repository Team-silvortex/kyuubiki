use crate::workflow_bundle_focus::{
    report_focus_context, report_focus_metrics, report_focus_payloads,
};
use crate::workflow_bundle_integrity::{
    check_bundle_admission, configured_bool, diagnostic_sources, total_count,
};
use crate::workflow_guard_contract::{GuardRule, optional_text};
use serde_json::Value;

pub fn compose_diagnostics_bundle(payload: Value, config: Value) -> Result<Value, String> {
    const OPERATOR: &str = "transform.compose_diagnostics_bundle";
    let compose = || -> Result<Value, String> {
        let object = payload.as_object().ok_or("expects an object payload")?;
        let include_non_diagnostics = configured_bool(&config, "include_non_diagnostics", false)?;
        let include_payloads = configured_bool(&config, "include_payloads", true)?;
        let include_numeric_fields = configured_bool(&config, "include_numeric_fields", true)?;
        let diagnostics = diagnostic_sources(object, include_non_diagnostics, OPERATOR)?;
        let items = diagnostics.iter().map(|source| {
            let entry = source.object;
            serde_json::json!({
                "source":source.id,
                "domain":entry.get("diagnostic_domain"),
                "subject":entry.get("diagnostic_subject"),
                "prefix":entry.get("diagnostic_prefix"),
                "node_count":source.node_count,
                "element_count":source.element_count,
                "metric_groups":entry.get("diagnostic_metric_groups").cloned().unwrap_or_else(||serde_json::json!([]))
            })
        }).collect::<Vec<_>>();
        let numeric_fields = sorted_unique_strings(
            diagnostics
                .iter()
                .flat_map(|source| source.object.iter())
                .filter(|(_, value)| value.is_number())
                .map(|(field, _)| field.clone()),
        );
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
            item["metric_groups"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
        }));
        let mut domain_counts = serde_json::Map::new();
        for domain in items
            .iter()
            .filter_map(|item| item.get("domain").and_then(Value::as_str))
        {
            let count = domain_counts
                .get(domain)
                .and_then(Value::as_u64)
                .unwrap_or(0);
            domain_counts.insert(domain.to_string(), Value::from(count + 1));
        }
        let mut bundle = serde_json::json!({
            "bundle_contract":"kyuubiki.workflow_diagnostics_bundle/v1",
            "bundle_kind":"workflow_diagnostics_bundle",
            "bundle_source_count":items.len(),
            "bundle_sources":diagnostics.iter().map(|source|source.id).collect::<Vec<_>>(),
            "bundle_domains":domains,"bundle_subjects":subjects,
            "bundle_domain_counts":domain_counts,"bundle_metric_groups":metric_groups,
            "bundle_items":items,
            "bundle_total_node_count":total_count(&diagnostics, "node")?,
            "bundle_total_element_count":total_count(&diagnostics, "element")?,
            "bundle_numeric_field_count":numeric_fields.len(),
        });
        if include_payloads {
            bundle["bundle_payloads"] = Value::Object(
                diagnostics
                    .iter()
                    .map(|source| (source.id.to_string(), source.payload.clone()))
                    .collect(),
            );
        }
        if include_numeric_fields {
            bundle["bundle_numeric_fields"] = serde_json::json!(numeric_fields);
        }
        Ok(bundle)
    };
    compose().map_err(|error| format!("{OPERATOR}: {error}"))
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

    check_bundle_admission(object, "transform.evaluate_diagnostics_bundle_guard")?;
    let mut triggers = Vec::new();
    for (index, rule) in rules.iter().enumerate() {
        if let Some(trigger) = evaluate_bundle_guard_rule(object, rule, index)
            .map_err(|error| format!("transform.evaluate_diagnostics_bundle_guard: {error}"))?
        {
            triggers.push(trigger);
        }
    }
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
    let focus_context = report_focus_context(bundle);
    let focus_payloads = report_focus_payloads(bundle);
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
        report_object.insert(
            "report_focus_metrics".to_string(),
            Value::Object(focus_metrics),
        );
        report_object.insert(
            "report_focus_context".to_string(),
            Value::Object(focus_context),
        );
        report_object.insert(
            "report_focus_payloads".to_string(),
            Value::Object(focus_payloads),
        );
        report_object.insert("report_highlights".to_string(), Value::Array(highlights));
    }
    Ok(report)
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
        &[
            "electrostatic_peak_field",
            "electrostatic_field_peak_magnitude",
        ],
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
    index: usize,
) -> Result<Option<Value>, String> {
    let path = format!("config.rules[{index}]");
    let parsed = GuardRule::parse(rule, &path)?;
    let rule = rule
        .as_object()
        .ok_or_else(|| format!("{path} must be an object rule"))?;
    let source = optional_text(rule, "source", &path)?;
    let (object, source_path) = if let Some(source) = source {
        let source_path = format!("payload.bundle_payloads.{source}");
        let object = payload
            .get("bundle_payloads")
            .and_then(Value::as_object)
            .and_then(|sources| sources.get(source))
            .and_then(Value::as_object)
            .ok_or_else(|| format!("{path}: {source_path} must be an object source"))?;
        (object, source_path)
    } else {
        (payload, "payload".to_string())
    };
    // Bundle rules address exact published fields, not the domain resolver's aliases.
    let value = object
        .get(parsed.field)
        .and_then(Value::as_f64)
        .filter(|value| value.is_finite())
        .ok_or_else(|| {
            format!(
                "{path}: {source_path}.{} must be a finite numeric metric",
                parsed.field
            )
        })?;
    if !parsed.triggered(value) {
        return Ok(None);
    }
    Ok(Some(serde_json::json!({
        "field":parsed.field,"source":source.unwrap_or("bundle"),"value":value,
        "threshold":parsed.threshold,"comparison":parsed.comparison,
        "severity":parsed.severity,"label":parsed.label
    })))
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
