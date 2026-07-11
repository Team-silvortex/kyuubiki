use serde_json::Value;

pub fn validate_summary_tolerance(payload: Value, config: Value) -> Result<Value, String> {
    let object = payload.as_object().ok_or_else(|| {
        "transform.validate_summary_tolerance expects an object payload".to_string()
    })?;
    let left = object
        .get("left")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            "transform.validate_summary_tolerance expects object payload.left".to_string()
        })?;
    let right = object
        .get("right")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            "transform.validate_summary_tolerance expects object payload.right".to_string()
        })?;
    let absolute_tolerance = config
        .get("absolute_tolerance")
        .and_then(Value::as_f64)
        .unwrap_or(1.0e-9);
    let relative_tolerance = config
        .get("relative_tolerance")
        .and_then(Value::as_f64)
        .unwrap_or(1.0e-6);
    let fail_on_missing = config
        .get("fail_on_missing")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let fields = resolve_validation_fields(left, right, &config)?;

    let mut checks = Vec::new();
    let mut failures = Vec::new();
    let mut missing = Vec::new();
    let mut max_absolute_error = 0.0_f64;
    let mut max_relative_error = 0.0_f64;

    for field in fields {
        let left_value = left.get(&field).and_then(Value::as_f64);
        let right_value = right.get(&field).and_then(Value::as_f64);
        let (Some(left_value), Some(right_value)) = (left_value, right_value) else {
            missing.push(Value::from(field));
            continue;
        };
        let absolute_error = (right_value - left_value).abs();
        let denominator = left_value.abs().max(right_value.abs()).max(1.0e-12);
        let relative_error = absolute_error / denominator;
        let passed = absolute_error <= absolute_tolerance || relative_error <= relative_tolerance;
        max_absolute_error = max_absolute_error.max(absolute_error);
        max_relative_error = max_relative_error.max(relative_error);

        let check = serde_json::json!({
            "field": field,
            "left": left_value,
            "right": right_value,
            "absolute_error": absolute_error,
            "relative_error": relative_error,
            "absolute_tolerance": absolute_tolerance,
            "relative_tolerance": relative_tolerance,
            "passed": passed,
        });
        if !passed {
            failures.push(check.clone());
        }
        checks.push(check);
    }

    if checks.is_empty() && missing.is_empty() {
        return Err(
            "transform.validate_summary_tolerance did not find any summary fields to validate"
                .to_string(),
        );
    }

    let missing_blocks = fail_on_missing && !missing.is_empty();
    let passed = failures.is_empty() && !missing_blocks;
    Ok(serde_json::json!({
        "validation_contract": "kyuubiki.summary_tolerance_validation/v1",
        "validation_passed": passed,
        "validation_grade": if passed { "pass" } else { "block" },
        "validation_checked_field_count": checks.len(),
        "validation_failed_field_count": failures.len(),
        "validation_missing_field_count": missing.len(),
        "validation_max_absolute_error": max_absolute_error,
        "validation_max_relative_error": max_relative_error,
        "validation_absolute_tolerance": absolute_tolerance,
        "validation_relative_tolerance": relative_tolerance,
        "validation_fail_on_missing": fail_on_missing,
        "validation_checks": checks,
        "validation_failures": failures,
        "validation_missing_fields": missing,
    }))
}

fn resolve_validation_fields(
    left: &serde_json::Map<String, Value>,
    right: &serde_json::Map<String, Value>,
    config: &Value,
) -> Result<Vec<String>, String> {
    if let Some(fields) = config.get("fields").and_then(Value::as_array) {
        let requested = fields
            .iter()
            .filter_map(Value::as_str)
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        if requested.is_empty() {
            return Err(
                "transform.validate_summary_tolerance config.fields must include at least one field"
                    .to_string(),
            );
        }
        return Ok(requested);
    }

    let mut fields = left
        .iter()
        .filter(|(key, value)| value.is_number() && right.get(*key).is_some_and(Value::is_number))
        .map(|(key, _)| key.clone())
        .collect::<Vec<_>>();
    fields.sort();
    Ok(fields)
}
