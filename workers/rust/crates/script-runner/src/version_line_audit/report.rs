use serde_json::Value;

pub(crate) fn print_human_report(report: &Value) {
    println!(
        "Version line audit for {} {}",
        field(report, "codename"),
        field(report, "expected")
    );
    println!();
    let exact = array(report, "exact_checks");
    let failed = exact
        .iter()
        .filter(|check| check.get("ok").and_then(Value::as_bool) != Some(true))
        .collect::<Vec<_>>();
    println!(
        "Exact contract checks: {} total, {} mismatched",
        exact.len(),
        failed.len()
    );
    for check in failed {
        println!(
            "- mismatch: {} :: {} expected {} but found {}",
            field(check, "file"),
            field(check, "field"),
            value_display(check.get("expected").unwrap_or(&Value::Null)),
            value_display(check.get("actual").unwrap_or(&Value::Null))
        );
    }
    if exact
        .iter()
        .all(|check| check.get("ok").and_then(Value::as_bool) == Some(true))
    {
        println!("- all exact version contracts match the expected development version");
    }
    println!();
    let inventory = array(report, "reference_inventory");
    println!("Textual version references: {} files", inventory.len());
    for entry in inventory.iter().take(20) {
        let summary = array(entry, "hits")
            .iter()
            .map(|hit| {
                format!(
                    "{}={}",
                    field(hit, "label"),
                    hit.get("count").and_then(Value::as_u64).unwrap_or(0)
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        println!("- {} :: {summary}", field(entry, "file"));
    }
    if inventory.len() > 20 {
        println!("- ... {} more files", inventory.len() - 20);
    }
    if !report.get("next_version").unwrap_or(&Value::Null).is_null() {
        let candidates = array(report, "next_candidates");
        println!();
        println!(
            "1.7 prep candidates for {}: {} files",
            field(report, "next_version"),
            candidates.len()
        );
        for entry in candidates.iter().take(20) {
            println!("- {}", field(entry, "file"));
        }
        if candidates.len() > 20 {
            println!("- ... {} more files", candidates.len() - 20);
        }
    }
}

fn value_display(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Null => "null".to_string(),
        other => other.to_string(),
    }
}

fn array<'a>(value: &'a Value, key: &str) -> Vec<&'a Value> {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(|items| items.iter().collect())
        .unwrap_or_default()
}

fn field<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or_default()
}
