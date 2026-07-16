use serde_json::Value;
use std::collections::HashSet;

const RECEIPT_KEYS: &[&str] = &["failure_receipt", "operator_task_failure_receipt"];
const RECEIPT_SCHEMAS: &[&str] = &[
    "kyuubiki.headless-operator-task-failure/v1",
    "kyuubiki.agent-operator-task-failure/v1",
    "kyuubiki.control-plane-operator-task-failure/v1",
];

pub fn operator_task_failure_receipts(payload: &Value) -> Vec<Value> {
    let mut receipts = Vec::new();
    collect_failure_receipts(payload, &mut receipts);
    unique_receipts(receipts)
}

pub fn operator_task_failure_actions(payload: &Value) -> Vec<String> {
    let mut actions = Vec::new();

    for receipt in operator_task_failure_receipts(payload) {
        push_unique_action(
            &mut actions,
            receipt
                .get("recovery")
                .and_then(|recovery| recovery.get("required_action"))
                .and_then(Value::as_str),
        );
    }

    collect_recovery_actions(payload, &mut actions);
    actions
}

pub fn operator_task_recovery_summary(payload: &Value) -> Value {
    let receipts = operator_task_failure_receipts(payload);

    serde_json::json!({
        "next_action": first_string_value(payload, "next_action"),
        "target_case_ids": first_string_list_value(payload, "target_case_ids"),
        "blocked_case_ids": first_string_list_value(payload, "blocked_case_ids"),
        "recovery_actions": operator_task_failure_actions(payload),
        "failure_receipt_count": receipts.len(),
        "failure_receipts": receipts,
    })
}

fn collect_failure_receipts(value: &Value, receipts: &mut Vec<Value>) {
    match value {
        Value::Object(map) => {
            for key in RECEIPT_KEYS {
                if let Some(candidate) = map.get(*key) {
                    if is_failure_receipt(candidate) {
                        receipts.push(candidate.clone());
                    }
                }
            }

            if is_failure_receipt(value) {
                receipts.push(value.clone());
            }

            for child in map.values() {
                collect_failure_receipts(child, receipts);
            }
        }
        Value::Array(values) => {
            for child in values {
                collect_failure_receipts(child, receipts);
            }
        }
        _ => {}
    }
}

fn collect_recovery_actions(value: &Value, actions: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            if let Some(Value::Array(values)) = map.get("recovery_actions") {
                for action in values {
                    push_unique_action(actions, action.as_str());
                }
            }

            for child in map.values() {
                collect_recovery_actions(child, actions);
            }
        }
        Value::Array(values) => {
            for child in values {
                collect_recovery_actions(child, actions);
            }
        }
        _ => {}
    }
}

fn push_unique_action(actions: &mut Vec<String>, action: Option<&str>) {
    if let Some(action) = action {
        if !action.is_empty() && !actions.iter().any(|existing| existing == action) {
            actions.push(action.to_string());
        }
    }
}

fn first_string_value(value: &Value, key: &str) -> Option<String> {
    match value {
        Value::Object(map) => {
            if let Some(found) = map.get(key).and_then(Value::as_str) {
                if !found.is_empty() {
                    return Some(found.to_string());
                }
            }
            map.values()
                .find_map(|child| first_string_value(child, key))
        }
        Value::Array(values) => values
            .iter()
            .find_map(|child| first_string_value(child, key)),
        _ => None,
    }
}

fn first_string_list_value(value: &Value, key: &str) -> Vec<String> {
    match value {
        Value::Object(map) => {
            if let Some(Value::Array(values)) = map.get(key) {
                return values
                    .iter()
                    .filter_map(Value::as_str)
                    .map(ToString::to_string)
                    .collect();
            }
            map.values()
                .find_map(|child| {
                    let values = first_string_list_value(child, key);
                    (!values.is_empty()).then_some(values)
                })
                .unwrap_or_default()
        }
        Value::Array(values) => values
            .iter()
            .find_map(|child| {
                let values = first_string_list_value(child, key);
                (!values.is_empty()).then_some(values)
            })
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

fn is_failure_receipt(value: &Value) -> bool {
    value
        .get("schema_version")
        .and_then(Value::as_str)
        .is_some_and(|schema| RECEIPT_SCHEMAS.contains(&schema))
}

fn unique_receipts(receipts: Vec<Value>) -> Vec<Value> {
    let mut unique = Vec::new();
    let mut seen = HashSet::new();

    for receipt in receipts {
        let key = receipt_key(&receipt);
        if seen.insert(key) {
            unique.push(receipt);
        }
    }

    unique
}

fn receipt_key(receipt: &Value) -> String {
    [
        "schema_version",
        "failure_stage",
        "reason_code",
        "task_id",
        "operator_id",
        "task_digest",
    ]
    .iter()
    .map(|key| receipt.get(*key).and_then(Value::as_str).unwrap_or(""))
    .collect::<Vec<_>>()
    .join("\u{1f}")
}
