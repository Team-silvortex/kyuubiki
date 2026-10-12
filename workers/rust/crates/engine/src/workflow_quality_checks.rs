use serde_json::{Map, Value};
use std::cmp::Ordering;

pub(crate) fn ranking_order(left: (bool, f64, &str), right: (bool, f64, &str)) -> Ordering {
    right
        .0
        .cmp(&left.0)
        // Scores are finite at both callers; signed zeros have equal quality.
        .then_with(|| {
            if left.1 == right.1 {
                Ordering::Equal
            } else {
                left.1.total_cmp(&right.1)
            }
        })
        .then_with(|| left.2.cmp(right.2))
}

pub(crate) fn finite_quality_value(value: f64, source: &str, field: &str) -> Result<f64, String> {
    if !value.is_finite() {
        Err(format!(
            "quality source {source} {field} produced a non-finite value"
        ))
    } else if value < 0.0 {
        Err(format!(
            "quality source {source} {field} must be nonnegative"
        ))
    } else {
        Ok(value)
    }
}

pub(crate) fn boolean(value: Option<&Value>, path: &str, default: bool) -> Result<bool, String> {
    value.map_or(Ok(default), |value| {
        value
            .as_bool()
            .ok_or_else(|| format!("quality {path} must be a boolean"))
    })
}

pub(crate) fn count(value: Option<&Value>, path: &str) -> Result<Option<u64>, String> {
    value
        .map(|value| {
            value
                .as_u64()
                .ok_or_else(|| format!("quality {path} must be an unsigned integer"))
        })
        .transpose()
}

pub(crate) fn text<'a>(value: Option<&'a Value>, path: &str) -> Result<Option<&'a str>, String> {
    value
        .map(|value| {
            value
                .as_str()
                .filter(|text| !text.trim().is_empty())
                .ok_or_else(|| format!("quality {path} must be nonblank text"))
        })
        .transpose()
}

pub(crate) fn list<'a>(value: Option<&'a Value>, path: &str) -> Result<&'a [Value], String> {
    value.map_or(Ok(&[]), |value| {
        value
            .as_array()
            .map(Vec::as_slice)
            .ok_or_else(|| format!("quality {path} must be an array"))
    })
}

pub(crate) fn optional_object<'a>(
    value: Option<&'a Value>,
    path: &str,
) -> Result<Option<&'a Map<String, Value>>, String> {
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(value) => value
            .as_object()
            .map(Some)
            .ok_or_else(|| format!("quality {path} must be an object")),
    }
}

pub(crate) fn score(value: Option<&Value>, source: &str, field: &str) -> Result<f64, String> {
    let value = value
        .and_then(Value::as_f64)
        .ok_or_else(|| format!("quality source {source} {field} must be numeric"))?;
    finite_quality_value(value, source, field)
}

pub(crate) fn domain_score<'a>(
    summary: &'a Map<String, Value>,
    source: &str,
) -> Result<(&'a str, f64), String> {
    let mut fields = summary.keys().filter(|key| key.ends_with("_quality_score"));
    let field = fields
        .next()
        .filter(|_| fields.next().is_none())
        .ok_or_else(|| {
            format!("quality source {source} requires exactly one quality score field")
        })?;
    let prefix = field.strip_suffix("_score").unwrap();
    for key in summary.keys() {
        for suffix in [
            "ready",
            "missing_metric_count",
            "watch_count",
            "grade",
            "contract",
            "dominant_term",
            "blocking_terms",
        ] {
            if key.ends_with(&format!("_quality_{suffix}")) && key != &format!("{prefix}_{suffix}")
            {
                return Err(format!(
                    "quality source {source} {key} conflicts with {field}"
                ));
            }
        }
    }
    Ok((field, score(summary.get(field), source, field)?))
}

pub(crate) fn validation(summary: &Map<String, Value>, source: &str) -> Result<bool, String> {
    let path = |field: &str| format!("source {source}.{field}");
    let passed = boolean(
        summary.get("validation_passed"),
        &path("validation_passed"),
        false,
    )?;
    let required = boolean(
        summary.get("validation_fail_on_missing"),
        &path("validation_fail_on_missing"),
        true,
    )?;
    let checked = count(
        summary.get("validation_checked_field_count"),
        &path("validation_checked_field_count"),
    )?;
    let failed = count(
        summary.get("validation_failed_field_count"),
        &path("validation_failed_field_count"),
    )?;
    let missing = count(
        summary.get("validation_missing_field_count"),
        &path("validation_missing_field_count"),
    )?;
    let failures = list(
        summary.get("validation_failures"),
        &path("validation_failures"),
    )?;
    let missing_fields = list(
        summary.get("validation_missing_fields"),
        &path("validation_missing_fields"),
    )?;
    if failures.iter().any(|failure| !failure.is_object()) {
        return Err(format!(
            "quality source {source} validation_failures entries must be objects"
        ));
    }
    Ok(passed
        && checked.is_some_and(|count| count > 0)
        && failed == Some(0)
        && failures.is_empty()
        && missing.is_some()
        && (!required || (missing == Some(0) && missing_fields.is_empty())))
}
