use serde_json::Value;
use std::collections::BTreeSet;

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
    // An omitted workflow node config arrives as null, not an empty object.
    if !config.is_null() && !config.is_object() {
        return Err("transform.validate_summary_tolerance config must be an object".to_string());
    }
    let absolute_tolerance = tolerance(&config, "absolute_tolerance", 1.0e-9)?;
    let relative_tolerance = tolerance(&config, "relative_tolerance", 1.0e-6)?;
    let fail_on_missing = match config.get("fail_on_missing") {
        None => true,
        Some(value) => value.as_bool().ok_or_else(|| {
            "transform.validate_summary_tolerance config.fail_on_missing must be a boolean"
                .to_string()
        })?,
    };
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
        if !absolute_error.is_finite() || !relative_error.is_finite() {
            return Err(format!(
                "transform.validate_summary_tolerance field {field:?} produced a non-finite error"
            ));
        }
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
    let passed = !checks.is_empty() && failures.is_empty() && !missing_blocks;
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

fn tolerance(config: &Value, field: &str, default: f64) -> Result<f64, String> {
    match config.get(field) {
        None => Ok(default),
        Some(value) => value
            .as_f64()
            .filter(|value| value.is_finite() && *value >= 0.0)
            .ok_or_else(|| {
                format!(
                    "transform.validate_summary_tolerance config.{field} must be a finite nonnegative number"
                )
            }),
    }
}

fn resolve_validation_fields(
    left: &serde_json::Map<String, Value>,
    right: &serde_json::Map<String, Value>,
    config: &Value,
) -> Result<Vec<String>, String> {
    if let Some(fields) = config.get("fields") {
        let fields = fields.as_array().ok_or_else(|| {
            "transform.validate_summary_tolerance config.fields must be an array".to_string()
        })?;
        if fields.is_empty() {
            return Err(
                "transform.validate_summary_tolerance config.fields must include at least one field"
                    .to_string(),
            );
        }
        let mut requested = Vec::with_capacity(fields.len());
        let mut seen = BTreeSet::new();
        for (index, value) in fields.iter().enumerate() {
            let field = value.as_str().filter(|field| !field.trim().is_empty()).ok_or_else(|| {
                format!("transform.validate_summary_tolerance config.fields[{index}] must be a nonblank string")
            })?;
            if !seen.insert(field) {
                return Err(format!(
                    "transform.validate_summary_tolerance config.fields contains duplicate field {field:?}"
                ));
            }
            requested.push(field.to_string());
        }
        return Ok(requested);
    }

    // Include one-sided numeric fields so missing evidence cannot disappear in auto mode.
    let fields = left
        .iter()
        .chain(right.iter())
        .filter(|(_, value)| value.is_number())
        .map(|(key, _)| key.clone())
        .collect::<BTreeSet<_>>();
    Ok(fields.into_iter().collect())
}
