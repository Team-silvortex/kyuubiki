use crate::workflow_metric_resolver::checked_metric_value;
use serde_json::{Map, Value};

#[derive(Clone, Copy, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Comparison {
    Gt,
    Gte,
    Lt,
    Lte,
    Eq,
}

#[derive(Clone, Copy, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Goal {
    Min,
    Max,
}

pub(crate) struct GuardRule<'a> {
    pub(crate) field: &'a str,
    pub(crate) threshold: f64,
    pub(crate) comparison: Comparison,
    pub(crate) severity: &'a str,
    pub(crate) label: &'a str,
}

impl<'a> GuardRule<'a> {
    pub(crate) fn parse(value: &'a Value, path: &str) -> Result<Self, String> {
        let object = value
            .as_object()
            .ok_or_else(|| format!("{path} must be an object rule"))?;
        let field = optional_text(object, "field", path)?
            .ok_or_else(|| format!("{path}.field must be a nonblank string"))?;
        let threshold = optional_number(object, "threshold", path)?;
        let legacy_value = optional_number(object, "value", path)?;
        if let (Some(threshold), Some(value)) = (threshold, legacy_value)
            && threshold != value
        {
            return Err(format!("{path}.threshold conflicts with {path}.value"));
        }
        let threshold = threshold
            .or(legacy_value)
            .ok_or_else(|| format!("{path}.threshold (or value) must be a finite number"))?;
        let comparison = match optional_text(object, "comparison", path)? {
            None | Some("gte") => Comparison::Gte,
            Some("gt") => Comparison::Gt,
            Some("lt") => Comparison::Lt,
            Some("lte") => Comparison::Lte,
            Some("eq") => Comparison::Eq,
            Some(_) => {
                return Err(format!(
                    "{path}.comparison must be one of gt, gte, lt, lte, eq"
                ));
            }
        };
        Ok(Self {
            field,
            threshold,
            comparison,
            severity: choice(object, "severity", "warn", &["warn", "block"], path)?,
            label: optional_text(object, "label", path)?.unwrap_or(field),
        })
    }

    pub(crate) fn triggered(&self, value: f64) -> bool {
        match self.comparison {
            Comparison::Gt => value > self.threshold,
            Comparison::Gte => value >= self.threshold,
            Comparison::Lt => value < self.threshold,
            Comparison::Lte => value <= self.threshold,
            Comparison::Eq => value == self.threshold,
        }
    }
}

pub(crate) struct PairCriterion<'a> {
    pub(crate) field: &'a str,
    pub(crate) left_field: &'a str,
    pub(crate) right_field: &'a str,
    pub(crate) goal: Goal,
    pub(crate) weight: f64,
}

impl<'a> PairCriterion<'a> {
    pub(crate) fn parse(value: &'a Value, path: &str) -> Result<Self, String> {
        let object = value
            .as_object()
            .ok_or_else(|| format!("{path} must be an object criterion"))?;
        let common = optional_text(object, "field", path)?;
        let left_field = optional_text(object, "left_field", path)?
            .or(common)
            .ok_or_else(|| format!("{path}.left_field (or field) must be a nonblank string"))?;
        let right_field = optional_text(object, "right_field", path)?
            .or(common)
            .ok_or_else(|| format!("{path}.right_field (or field) must be a nonblank string"))?;
        let weight = optional_number(object, "weight", path)?.unwrap_or(1.0);
        if weight <= 0.0 {
            return Err(format!("{path}.weight must be positive"));
        }
        let goal = match optional_text(object, "goal", path)? {
            None | Some("min") => Goal::Min,
            Some("max") => Goal::Max,
            Some(_) => return Err(format!("{path}.goal must be min or max")),
        };
        Ok(Self {
            field: common.unwrap_or(left_field),
            left_field,
            right_field,
            goal,
            weight,
        })
    }
}

pub(crate) fn pair_labels(config: &Value) -> Result<(&str, &str), String> {
    let object = config
        .as_object()
        .ok_or_else(|| "config must be an object".to_string())?;
    let left = optional_text(object, "left_label", "config")?.unwrap_or("left");
    let right = optional_text(object, "right_label", "config")?.unwrap_or("right");
    if left == right {
        return Err("config.left_label and config.right_label must be distinct".to_string());
    }
    for (key, label) in [("left_label", left), ("right_label", right)] {
        if label.eq_ignore_ascii_case("tie") {
            return Err(format!("config.{key} cannot use reserved label tie"));
        }
    }
    Ok((left, right))
}

pub(crate) fn required_metric(
    object: &Map<String, Value>,
    field: &str,
    source: &str,
) -> Result<f64, String> {
    checked_metric_value(object, field)
        .map_err(|error| format!("{source}.{error} (requested metric {source}.{field})"))?
        .ok_or_else(|| format!("{source}.{field} must resolve to a finite numeric metric"))
}

pub(crate) fn finite_value(value: f64, path: &str) -> Result<f64, String> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(format!("{path} produced a non-finite value"))
    }
}

fn optional_text<'a>(
    object: &'a Map<String, Value>,
    key: &str,
    path: &str,
) -> Result<Option<&'a str>, String> {
    object
        .get(key)
        .map(|value| {
            value
                .as_str()
                .map(str::trim)
                .filter(|text| !text.is_empty())
                .ok_or_else(|| format!("{path}.{key} must be a nonblank string"))
        })
        .transpose()
}

fn optional_number(
    object: &Map<String, Value>,
    key: &str,
    path: &str,
) -> Result<Option<f64>, String> {
    object
        .get(key)
        .map(|value| {
            value
                .as_f64()
                .filter(|number| number.is_finite())
                .ok_or_else(|| format!("{path}.{key} must be a finite number"))
        })
        .transpose()
}

fn choice<'a>(
    object: &'a Map<String, Value>,
    key: &str,
    default: &'a str,
    allowed: &[&str],
    path: &str,
) -> Result<&'a str, String> {
    let selected = optional_text(object, key, path)?.unwrap_or(default);
    if allowed.contains(&selected) {
        Ok(selected)
    } else {
        Err(format!(
            "{path}.{key} must be one of {}",
            allowed.join(", ")
        ))
    }
}
