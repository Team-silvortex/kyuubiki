use serde_json::{Value, json};
use std::fs;
use std::path::Path;

const POLICY_PATH: &str = "config/version-line-policy.json";

pub(super) fn checks(root: &Path) -> Result<Vec<Value>, String> {
    let policy: Value = serde_json::from_str(
        &fs::read_to_string(root.join(POLICY_PATH))
            .map_err(|error| format!("failed to read {POLICY_PATH}: {error}"))?,
    )
    .map_err(|error| format!("{POLICY_PATH}: invalid json: {error}"))?;
    Ok(validate(&policy))
}

fn validate(policy: &Value) -> Vec<Value> {
    let closure = policy.get("closure_window").unwrap_or(&Value::Null);
    if closure.is_null() {
        return Vec::new();
    }
    let first = closure
        .get("first_version")
        .and_then(Value::as_str)
        .and_then(super::parse_version);
    let last = closure
        .get("final_version")
        .and_then(Value::as_str)
        .and_then(super::parse_version);
    let major = policy.pointer("/active_line/major").and_then(Value::as_u64);
    let valid = first.zip(last).is_some_and(|(first, last)| {
        Some(first.0) == major
            && first.0 == last.0
            && first.1 == last.1
            && first.1 <= 20
            && first.2 <= last.2
            && last.2 <= 9
    });
    let range = first
        .zip(last)
        .map(|(first, last)| {
            json!({
                "first": first.2, "last": last.2, "count": last.2.saturating_sub(first.2).saturating_add(1)
            })
        })
        .unwrap_or(Value::Null);
    vec![
        check("active_line_range", json!(true), Some(&json!(valid))),
        check("patch_range", range, closure.get("patch_range")),
        check("mode", json!("stabilization_only"), closure.get("mode")),
        check(
            "primary_change_classes",
            json!(["bug_fix", "performance_optimization"]),
            closure.get("primary_change_classes"),
        ),
        check(
            "exceptional_change_classes",
            json!(["security_remediation", "release_blocker"]),
            closure.get("exceptional_change_classes"),
        ),
        check(
            "prohibited_change_classes",
            json!(["feature_expansion"]),
            closure.get("prohibited_change_classes"),
        ),
    ]
}

fn check(field: &str, expected: Value, actual: Option<&Value>) -> Value {
    let actual = actual.cloned().unwrap_or(Value::Null);
    let ok = actual == expected;
    json!({
        "kind": "closure_policy",
        "file": POLICY_PATH,
        "field": format!("closure_window.{field}"),
        "expected": expected,
        "actual": actual,
        "ok": ok
    })
}

#[cfg(test)]
mod tests {
    use super::validate;
    use serde_json::json;

    #[test]
    fn daji_does_not_reactivate_the_archived_moxi_closure() {
        let policy = json!({
            "active_line": {"major": 3}, "closure_window": null,
            "archived_lines": [{"major": 2, "closure_window": {"first_version": "2.20.1"}}]
        });
        assert!(validate(&policy).is_empty());
    }

    #[test]
    fn rejects_a_closure_belonging_to_another_major() {
        let checks = validate(&json!({
            "active_line": {"major": 3},
            "closure_window": {"first_version": "2.20.1", "final_version": "2.20.9"}
        }));
        assert_eq!(checks[0]["ok"], false);
    }
}
