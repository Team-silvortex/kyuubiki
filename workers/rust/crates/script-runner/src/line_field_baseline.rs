use serde_json::Value;
use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs;
use std::path::Path;

const ARTIFACT_PATH: &str = "evidence/operator-qualification/line-field-closed-form-baseline.json";
const REQUIRED_OPERATORS: &[&str] = &[
    "solve.bar_1d",
    "solve.thermal_bar_1d",
    "solve.heat_bar_1d",
    "solve.electrostatic_bar_1d",
];

type RunnerResult<T> = Result<T, String>;

pub(crate) fn run_check_line_field_closed_form_baseline(
    root: &Path,
    args: Vec<OsString>,
) -> RunnerResult<u8> {
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        println!("usage: kyuubiki-script-runner check-line-field-closed-form-baseline");
        return Ok(0);
    }
    if !args.is_empty() {
        return Err("check-line-field-closed-form-baseline does not accept arguments".to_string());
    }
    let artifact = read_json(root, ARTIFACT_PATH)?;
    if let Err(issue) = validate(root, &artifact) {
        eprintln!("line-field closed-form baseline check failed: {issue}");
        return Ok(1);
    }
    println!(
        "line-field closed-form baseline ok: {} operators",
        array(&artifact, "baselines").len()
    );
    Ok(0)
}

fn validate(root: &Path, artifact: &Value) -> RunnerResult<()> {
    require_eq(
        field(artifact, "schema_version"),
        "kyuubiki.operator-qualification-baseline/v1",
        "unexpected schema_version",
    )?;
    require_eq(
        field(artifact, "version_line"),
        "moxi 2.0.x",
        "version_line must match moxi 2.0.x",
    )?;
    require_eq(
        field(artifact, "candidate_id"),
        "line-field-closed-form",
        "candidate_id must be line-field-closed-form",
    )?;
    require_eq(
        field(artifact, "status"),
        "collecting",
        "status must be collecting until qualification is granted",
    )?;
    let source_test = field(artifact, "source_test");
    if source_test.is_empty() {
        return Err("source_test is required".to_string());
    }
    let source = read_text(root, source_test)
        .map_err(|_| format!("source_test does not exist: {source_test}"))?;
    let policy = load_tolerance_policy(root, artifact)?;
    validate_derivation_note(root, artifact, &source)?;
    let baselines = array(artifact, "baselines");
    if baselines.len() != REQUIRED_OPERATORS.len() {
        return Err(format!(
            "baselines must contain exactly {} entries",
            REQUIRED_OPERATORS.len()
        ));
    }
    let mut seen = BTreeSet::new();
    for baseline in baselines {
        validate_baseline(baseline, &source, &policy, &mut seen)?;
    }
    for operator_id in REQUIRED_OPERATORS {
        if !seen.contains(*operator_id) {
            return Err(format!("missing baseline for {operator_id}"));
        }
    }
    Ok(())
}

fn validate_tolerance_policy(policy: &Value) -> RunnerResult<()> {
    require_eq(
        field(policy, "schema_version"),
        "kyuubiki.operator-qualification-tolerance-policy/v1",
        "tolerance_policy: unexpected schema_version",
    )?;
    require_eq(
        field(policy, "candidate_id"),
        "line-field-closed-form",
        "tolerance_policy: candidate_id must be line-field-closed-form",
    )?;
    for kind in ["absolute", "relative", "sign"] {
        if policy.pointer(&format!("/policy/{kind}")).is_none() {
            return Err(format!("tolerance_policy: missing {kind} policy"));
        }
    }
    if array_at(policy, "/scope/not_allowed").is_empty() {
        return Err(
            "tolerance_policy: scope.not_allowed must document what this policy cannot claim"
                .to_string(),
        );
    }
    Ok(())
}

fn validate_baseline(
    baseline: &Value,
    source: &str,
    policy: &Value,
    seen: &mut BTreeSet<String>,
) -> RunnerResult<()> {
    let operator_id = field(baseline, "operator_id");
    let context = if operator_id.is_empty() {
        "unknown operator"
    } else {
        operator_id
    };
    if !REQUIRED_OPERATORS.contains(&context) {
        return Err(format!(
            "{context}: not part of line-field-closed-form candidate"
        ));
    }
    if !seen.insert(context.to_string()) {
        return Err(format!("{context}: duplicate baseline entry"));
    }
    let test_name = field(baseline, "test_name");
    if test_name.is_empty() || !source.contains(test_name) {
        return Err(format!(
            "{context}: source test does not contain {test_name}"
        ));
    }
    if field(baseline, "case_id").is_empty() {
        return Err(format!("{context}: case_id is required"));
    }
    if baseline
        .pointer("/closed_form/summary")
        .and_then(Value::as_str)
        .is_none()
    {
        return Err(format!("{context}: closed_form.summary is required"));
    }
    if array_at(baseline, "/closed_form/formulae").is_empty() {
        return Err(format!("{context}: closed_form.formulae must be non-empty"));
    }
    if !baseline.get("inputs").is_some_and(Value::is_object) {
        return Err(format!("{context}: inputs must be present"));
    }
    let expectations = array(baseline, "expectations");
    if expectations.is_empty() {
        return Err(format!("{context}: expectations must be non-empty"));
    }
    for expectation in expectations {
        validate_expectation(expectation, policy, context, context)?;
    }
    Ok(())
}

fn validate_derivation_note(root: &Path, artifact: &Value, source: &str) -> RunnerResult<()> {
    let note_path = field(artifact, "derivation_note");
    if note_path.is_empty() {
        return Err("derivation_note is required".to_string());
    }
    let note = read_text(root, note_path)
        .map_err(|_| format!("derivation_note does not exist: {note_path}"))?;
    for operator_id in REQUIRED_OPERATORS {
        if !note.contains(operator_id) {
            return Err(format!("derivation_note does not mention {operator_id}"));
        }
    }
    for baseline in array(artifact, "baselines") {
        let test_name = field(baseline, "test_name");
        if !test_name.is_empty() && !source.contains(test_name) {
            return Err(format!(
                "{}: missing source test {test_name}",
                field(baseline, "operator_id")
            ));
        }
        let case_id = field(baseline, "case_id");
        if !case_id.is_empty() && !note.contains(case_id) {
            return Err(format!(
                "{}: derivation_note does not mention {case_id}",
                field(baseline, "operator_id")
            ));
        }
    }
    Ok(())
}

fn load_tolerance_policy(root: &Path, artifact: &Value) -> RunnerResult<Value> {
    let policy_path = field(artifact, "tolerance_policy");
    if policy_path.is_empty() {
        return Err("tolerance_policy is required".to_string());
    }
    if !root.join(policy_path).exists() {
        return Err(format!("tolerance_policy does not exist: {policy_path}"));
    }
    let policy = read_json(root, policy_path)?;
    validate_tolerance_policy(&policy)?;
    Ok(policy)
}

fn validate_expectation(
    expectation: &Value,
    policy: &Value,
    operator_id: &str,
    context: &str,
) -> RunnerResult<()> {
    let field_name = field(expectation, "field");
    if field_name.is_empty() {
        return Err(format!("{context}: expectation.field is required"));
    }
    let value = expectation.get("value").unwrap_or(&Value::Null);
    if !value.as_str().is_some_and(|text| !text.is_empty()) && finite_number(value).is_none() {
        return Err(format!(
            "{context}: expectation.value must be a finite number or sign label"
        ));
    }
    validate_tolerance(
        expectation,
        policy,
        operator_id,
        &format!("{context}:{field_name}"),
    )
}

fn validate_tolerance(
    expectation: &Value,
    policy: &Value,
    operator_id: &str,
    context: &str,
) -> RunnerResult<()> {
    let Some(tolerance) = expectation.get("tolerance") else {
        return Err(format!("{context}: tolerance.kind is required"));
    };
    let kind = field(tolerance, "kind");
    if kind.is_empty() {
        return Err(format!("{context}: tolerance.kind is required"));
    }
    match kind {
        "absolute" | "relative" => {
            let Some(value) = finite_number(tolerance.get("value").unwrap_or(&Value::Null)) else {
                return Err(format!(
                    "{context}: numeric tolerance must be finite and positive"
                ));
            };
            if value <= 0.0 {
                return Err(format!(
                    "{context}: numeric tolerance must be finite and positive"
                ));
            }
            validate_numeric_tolerance(expectation, policy, operator_id, context, value)
        }
        "sign" => {
            let value = field(tolerance, "value");
            if value.is_empty() {
                return Err(format!(
                    "{context}: sign tolerance must carry a non-empty label"
                ));
            }
            if field(expectation, "value").is_empty() {
                return Err(format!(
                    "{context}: sign expectation must carry a non-empty value"
                ));
            }
            if !string_array_at(policy, "/policy/sign/allowed_values").contains(&value.to_string())
            {
                return Err(format!(
                    "{context}: sign tolerance value {value} is not policy-approved"
                ));
            }
            Ok(())
        }
        _ => Err(format!("{context}: unsupported tolerance kind {kind}")),
    }
}

fn validate_numeric_tolerance(
    expectation: &Value,
    policy: &Value,
    operator_id: &str,
    context: &str,
    tolerance_value: f64,
) -> RunnerResult<()> {
    let kind = field(expectation.get("tolerance").unwrap_or(&Value::Null), "kind");
    let field_name = field(expectation, "field");
    let override_entry = find_field_override(policy, operator_id, field_name);
    if let Some(override_entry) = override_entry {
        let allowed = field(override_entry, "allowed_kind");
        if allowed != kind {
            return Err(format!(
                "{context}: tolerance kind must match field override {allowed}"
            ));
        }
    }
    let max_value = override_entry
        .and_then(|entry| entry.get("max_value"))
        .and_then(finite_number)
        .or_else(|| {
            policy
                .pointer(&format!("/policy/{kind}/max_value"))
                .and_then(finite_number)
        });
    let Some(max_value) = max_value else {
        return Err(format!(
            "{context}: tolerance {tolerance_value} exceeds policy max "
        ));
    };
    if tolerance_value > max_value {
        return Err(format!(
            "{context}: tolerance {tolerance_value} exceeds policy max {max_value}"
        ));
    }
    Ok(())
}

fn find_field_override<'a>(
    policy: &'a Value,
    operator_id: &str,
    field_name: &str,
) -> Option<&'a Value> {
    array(policy, "field_overrides").into_iter().find(|entry| {
        field(entry, "field") == field_name
            && (field(entry, "operator_id").is_empty()
                || field(entry, "operator_id") == operator_id)
    })
}

fn read_json(root: &Path, relative_path: &str) -> RunnerResult<Value> {
    let text = read_text(root, relative_path)?;
    serde_json::from_str(&text).map_err(|error| format!("{relative_path}: invalid json: {error}"))
}

fn read_text(root: &Path, relative_path: &str) -> RunnerResult<String> {
    fs::read_to_string(root.join(relative_path))
        .map_err(|error| format!("failed to read {relative_path}: {error}"))
}

fn finite_number(value: &Value) -> Option<f64> {
    value.as_f64().filter(|number| number.is_finite())
}

fn require_eq(actual: &str, expected: &str, message: &str) -> RunnerResult<()> {
    if actual == expected {
        Ok(())
    } else {
        Err(message.to_string())
    }
}

fn array<'a>(value: &'a Value, key: &str) -> Vec<&'a Value> {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(|items| items.iter().collect())
        .unwrap_or_default()
}

fn array_at<'a>(value: &'a Value, pointer: &str) -> Vec<&'a Value> {
    value
        .pointer(pointer)
        .and_then(Value::as_array)
        .map(|items| items.iter().collect())
        .unwrap_or_default()
}

fn string_array_at(value: &Value, pointer: &str) -> Vec<String> {
    array_at(value, pointer)
        .into_iter()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect()
}

fn field<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{find_field_override, finite_number};
    use serde_json::json;

    #[test]
    fn finite_number_rejects_non_numbers() {
        assert_eq!(finite_number(&json!(1.0)), Some(1.0));
        assert_eq!(finite_number(&json!("1.0")), None);
    }

    #[test]
    fn field_override_can_be_global_or_operator_specific() {
        let policy = json!({
            "field_overrides": [
                {"field": "a", "allowed_kind": "absolute"},
                {"field": "b", "operator_id": "solve.x", "allowed_kind": "relative"}
            ]
        });
        assert!(find_field_override(&policy, "solve.y", "a").is_some());
        assert!(find_field_override(&policy, "solve.x", "b").is_some());
        assert!(find_field_override(&policy, "solve.y", "b").is_none());
    }
}
