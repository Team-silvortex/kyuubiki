use serde_json::{Value, json};
use std::collections::HashSet;

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

pub(crate) fn text_option<'a>(
    value: Option<&'a Value>,
    field: &str,
    default: &'a str,
) -> Result<&'a str, String> {
    value.map_or(Ok(default), |value| {
        value
            .as_str()
            .filter(|text| !text.trim().is_empty())
            .ok_or_else(|| format!("{field} must be a nonblank string"))
    })
}

pub(crate) fn optional_case_id<'a>(
    value: &'a Value,
    context: &str,
) -> Result<Option<&'a str>, String> {
    if !value.is_object() {
        return Err(format!("{context} must be an object"));
    }
    let mut identity = None;
    for field in ["id", "case_id", "caseId"] {
        if let Some(value) = value.get(field) {
            let id = text_option(Some(value), &format!("{context}.{field}"), "")?;
            if identity.is_some_and(|previous| previous != id) {
                return Err(format!("{context} has conflicting case identity aliases"));
            }
            identity = Some(id);
        }
    }
    Ok(identity)
}

pub(crate) fn case_ids(cases: &[Value]) -> Result<Vec<String>, String> {
    let mut seen = HashSet::new();
    cases
        .iter()
        .enumerate()
        .map(|(index, case)| {
            let id = optional_case_id(case, &format!("parameter sweep case {index}"))?
                .map(str::to_string)
                .unwrap_or_else(|| format!("case_{index}"));
            if !seen.insert(id.clone()) {
                return Err(format!(
                    "parameter sweep case {index} has duplicate case id {id:?}"
                ));
            }
            Ok(id)
        })
        .collect()
}

pub(crate) fn require_success(
    value: &Value,
    status_field: &str,
    error_field: &str,
    context: &str,
) -> Result<(), String> {
    if let Some(status) = value.get(status_field) {
        let status = text_option(Some(status), &format!("{context}.{status_field}"), "")?;
        if !matches!(
            status,
            "ok" | "success" | "succeeded" | "completed" | "complete" | "done" | "joined"
        ) {
            return Err(format!("{context} status {status:?} is not successful"));
        }
    }
    if value.get(error_field).is_some_and(|error| !error.is_null()) {
        return Err(format!("{context} contains {error_field}"));
    }
    Ok(())
}

pub(crate) fn require_complete(
    value: &Value,
    flag: &str,
    counters: &[(&str, &str)],
) -> Result<(), String> {
    if !bool_option(value.get(flag), flag, true)? {
        return Err(format!(
            "incomplete parameter sweep evidence: {flag} is false"
        ));
    }
    for (count_field, list_field) in counters {
        let list_len = value
            .get(*list_field)
            .map(|list| {
                list.as_array()
                    .map(Vec::len)
                    .ok_or_else(|| format!("{list_field} must be an array"))
            })
            .transpose()?;
        let count = count_option(
            value.get(*count_field),
            count_field,
            list_len.unwrap_or(0),
            0,
        )?;
        if list_len.is_some_and(|len| len != count) {
            return Err(format!("{count_field} disagrees with {list_field}"));
        }
        if count > 0 {
            return Err(format!(
                "incomplete parameter sweep evidence: {count_field}={count}"
            ));
        }
    }
    Ok(())
}

pub(crate) fn require_count(value: &Value, field: &str, expected: usize) -> Result<(), String> {
    let declared = count_option(value.get(field), field, expected, 0)?;
    if declared != expected {
        return Err(format!(
            "{field}={declared} disagrees with actual count {expected}"
        ));
    }
    Ok(())
}
