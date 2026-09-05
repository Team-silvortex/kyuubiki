use serde_json::{Value, json};

pub(crate) fn object_or_null(value: &Value, field: &str) -> Result<(), String> {
    if value.is_null() || value.is_object() {
        Ok(())
    } else {
        Err(format!("{field} must be an object"))
    }
}

pub(crate) fn count_option(
    value: Option<&Value>,
    field: &str,
    default: usize,
    minimum: usize,
) -> Result<usize, String> {
    let Some(value) = value else {
        return Ok(default);
    };
    let parsed = if let Some(integer) = value.as_u64() {
        usize::try_from(integer).ok()
    } else {
        value
            .as_f64()
            .filter(|number| {
                number.is_finite()
                && *number >= 0.0
                && number.fract() == 0.0
                // The exclusive bound also avoids saturating float-to-integer casts.
                && *number < usize::MAX as f64 + 1.0
            })
            .map(|number| number as usize)
    };
    parsed.filter(|count| *count >= minimum).ok_or_else(|| {
        format!("{field} must be a whole count >= {minimum} within the platform count range")
    })
}

pub(crate) fn bool_option(
    value: Option<&Value>,
    field: &str,
    default: bool,
) -> Result<bool, String> {
    value.map_or(Ok(default), |value| {
        value
            .as_bool()
            .ok_or_else(|| format!("{field} must be a boolean"))
    })
}

pub(crate) fn checked_case_count(mut counts: impl Iterator<Item = usize>) -> Result<usize, String> {
    counts
        .try_fold(1usize, |total, count| total.checked_mul(count))
        .ok_or_else(|| "parameter sweep case count overflowed".to_string())
}

pub(crate) fn actual_case_count(axes: &[Value]) -> Result<usize, String> {
    if axes.is_empty() {
        return Err("parameter sweep axes must not be empty".to_string());
    }
    let mut count = 1usize;
    for (index, axis) in axes.iter().enumerate() {
        let values = axis
            .get("values")
            .and_then(Value::as_array)
            .filter(|values| !values.is_empty())
            .ok_or_else(|| format!("parameter sweep axis {index} requires nonempty values"))?;
        count = count
            .checked_mul(values.len())
            .ok_or_else(|| "parameter sweep case count overflowed".to_string())?;
    }
    Ok(count)
}

pub(crate) fn refreshed_budget(
    budget: &Value,
    case_count: usize,
    max_cases: usize,
    upstream_blocked: bool,
) -> Value {
    let mut result = budget.as_object().cloned().unwrap_or_default();
    let exceeded = upstream_blocked || case_count > max_cases;
    result.insert("case_count_estimate".into(), json!(case_count));
    result.insert("max_cases".into(), json!(max_cases));
    result.insert("case_budget_exceeded".into(), json!(exceeded));
    if exceeded {
        result.insert("status".into(), json!("case_budget_exceeded"));
        if !upstream_blocked {
            result.insert("recommendation".into(), json!("replan_within_case_budget"));
        }
    }
    Value::Object(result)
}
