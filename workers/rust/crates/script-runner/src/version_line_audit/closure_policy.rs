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
    let closure = policy.get("closure_window").unwrap_or(&Value::Null);
    Ok(vec![
        check(
            "first_version",
            json!("2.20.1"),
            closure.get("first_version"),
        ),
        check(
            "final_version",
            json!("2.20.9"),
            closure.get("final_version"),
        ),
        check(
            "patch_range",
            json!({"first": 1, "last": 9, "count": 9}),
            closure.get("patch_range"),
        ),
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
    ])
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
