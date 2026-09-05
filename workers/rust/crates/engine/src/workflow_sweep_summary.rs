use serde_json::{Map, Value, json};
use std::collections::{BTreeMap, BTreeSet};

use crate::workflow_sweep_contract::{
    bool_option, case_ids, object_or_null, require_complete, require_count, require_success,
    text_option,
};

pub fn summarize_parameter_sweep(payload: Value, config: Value) -> Result<Value, String> {
    object_or_null(&config, "parameter sweep summary config")?;
    require_complete(
        &payload,
        "join_complete",
        &[
            ("missing_summary_count", "missing_case_ids"),
            ("unmatched_result_count", "unmatched_result_ids"),
            ("rejected_result_count", "rejected_results"),
        ],
    )?;
    let cases = payload
        .get("cases")
        .and_then(Value::as_array)
        .filter(|cases| !cases.is_empty())
        .ok_or_else(|| {
            "transform.summarize_parameter_sweep requires nonempty payload.cases".to_string()
        })?;
    let ids = case_ids(cases)?;
    require_count(&payload, "case_count", cases.len())?;
    require_count(&payload, "joined_summary_count", cases.len())?;
    let joined_field = text_option(
        payload.get("joined_summary_field"),
        "joined_summary_field",
        "summary",
    )?;
    let summary_field = text_option(config.get("summary_field"), "summary_field", joined_field)?;
    if payload.get("joined_summary_field").is_some() && summary_field != joined_field {
        return Err("summary_field disagrees with joined_summary_field".to_string());
    }
    let include_parameters =
        bool_option(config.get("include_parameters"), "include_parameters", true)?;
    let include_metadata = bool_option(config.get("include_metadata"), "include_metadata", true)?;
    let fail_on_missing = bool_option(config.get("fail_on_missing"), "fail_on_missing", true)?;
    let summaries = cases
        .iter()
        .zip(&ids)
        .map(|(case, id)| {
            require_success(
                case,
                "result_status",
                "result_error",
                &format!("parameter sweep case {id:?}"),
            )?;
            let summary = case.get(summary_field).or_else(|| {
                if config.get("summary_field").is_none()
                    && payload.get("joined_summary_field").is_none()
                {
                    case.get("result")
                } else {
                    None
                }
            });
            summary.and_then(Value::as_object).ok_or_else(|| {
                format!(
                    "parameter sweep case {id:?} requires object summary field {summary_field:?}"
                )
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let fields = summary_fields(&summaries, &config)?;
    let numeric_fields = summaries
        .iter()
        .flat_map(|summary| summary.iter())
        .filter(|(_, value)| value.is_number())
        .map(|(field, _)| field.as_str())
        .collect::<BTreeSet<_>>();
    let mut rows = Vec::with_capacity(cases.len());
    let mut columns: BTreeMap<&str, NumericColumn> = BTreeMap::new();
    let mut missing = Vec::new();
    for ((case, id), summary) in cases.iter().zip(&ids).zip(&summaries) {
        let mut row = Map::new();
        row.insert("case_id".into(), json!(id));
        if include_parameters {
            row.insert(
                "parameters".into(),
                case.get("parameters").cloned().unwrap_or(Value::Null),
            );
        }
        if include_metadata {
            row.insert(
                "metadata".into(),
                case.get("metadata").cloned().unwrap_or(Value::Null),
            );
        }
        for field in &fields {
            let value = summary.get(field);
            let reason = if value.is_none() {
                Some("missing")
            } else if value.is_some_and(Value::is_null) {
                Some("null")
            } else if numeric_fields.contains(field.as_str())
                && value.and_then(Value::as_f64).is_none()
            {
                Some("non_numeric")
            } else {
                None
            };
            if let Some(reason) = reason {
                if fail_on_missing {
                    return Err(format!(
                        "parameter sweep case {id:?} field {field:?} is {reason}"
                    ));
                }
                missing.push(json!({"case_id": id, "field": field, "reason": reason}));
            }
            if let Some(value) = value {
                row.insert(field.clone(), value.clone());
                if let Some(number) = value.as_f64() {
                    columns
                        .entry(field)
                        .or_default()
                        .push(number)
                        .map_err(|error| {
                            format!("parameter sweep case {id:?} field {field:?}: {error}")
                        })?;
                }
            }
        }
        rows.push(Value::Object(row));
    }
    let columns = columns
        .into_iter()
        .map(|(field, column)| (field.to_string(), column.into_value()))
        .collect::<Map<_, _>>();
    Ok(
        json!({"rows": rows, "row_count": cases.len(), "numeric_columns": columns,
        "summary_complete": missing.is_empty(), "missing_field_count": missing.len(), "missing_fields": missing}),
    )
}

fn summary_fields(
    summaries: &[&Map<String, Value>],
    config: &Value,
) -> Result<Vec<String>, String> {
    let fields = if let Some(fields) = config.get("fields") {
        let fields = fields
            .as_array()
            .filter(|fields| !fields.is_empty())
            .ok_or_else(|| "summary fields must be a nonempty array".to_string())?;
        let mut seen = BTreeSet::new();
        fields
            .iter()
            .enumerate()
            .map(|(index, value)| {
                let field = text_option(Some(value), &format!("fields[{index}]"), "")?;
                if !seen.insert(field) {
                    return Err(format!("summary fields contains duplicate {field:?}"));
                }
                Ok(field.to_string())
            })
            .collect::<Result<Vec<_>, _>>()?
    } else {
        summaries
            .iter()
            .flat_map(|summary| summary.keys())
            .cloned()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    };
    if fields.is_empty() {
        return Err("parameter sweep summaries contain no selected fields".to_string());
    }
    for field in &fields {
        if field.trim().is_empty() {
            return Err("summary fields must be nonblank strings".to_string());
        }
        if matches!(field.as_str(), "case_id" | "parameters" | "metadata") {
            return Err(format!(
                "summary field {field:?} is reserved for case identity or provenance"
            ));
        }
    }
    Ok(fields)
}

#[derive(Default)]
struct NumericColumn {
    count: usize,
    min: f64,
    max: f64,
    sum: f64,
}

impl NumericColumn {
    fn push(&mut self, value: f64) -> Result<(), String> {
        let sum = self.sum + value;
        if !value.is_finite() || !sum.is_finite() {
            return Err("numeric column has a non-finite value or sum".to_string());
        }
        let count = self
            .count
            .checked_add(1)
            .ok_or_else(|| "numeric column count overflowed".to_string())?;
        self.min = if self.count == 0 {
            value
        } else {
            self.min.min(value)
        };
        self.max = if self.count == 0 {
            value
        } else {
            self.max.max(value)
        };
        self.sum = sum;
        self.count = count;
        Ok(())
    }

    fn into_value(self) -> Value {
        json!({"count": self.count, "min": self.min, "max": self.max, "sum": self.sum, "mean": self.sum / self.count as f64})
    }
}
