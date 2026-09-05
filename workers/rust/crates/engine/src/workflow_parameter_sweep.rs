use serde_json::{Map, Value};

use crate::workflow_sweep_contract::{
    actual_case_count, bool_option, count_option, object_or_null, refreshed_budget,
};

pub use crate::workflow_parameter_sweep_results::{
    join_parameter_sweep_results, map_parameter_sweep_scores_to_quality_candidates,
    score_parameter_sweep,
};

struct SweepAxis<'a> {
    label: &'a str,
    path: &'a str,
    values: &'a [Value],
}

pub fn expand_parameter_sweep(payload: Value, config: Value) -> Result<Value, String> {
    object_or_null(&config, "parameter sweep config")?;
    let quality_context = if let Some(contract) = payload.get("quality_sweep_expansion_contract") {
        if contract.as_str() != Some("kyuubiki.quality_sweep_expansion/v1") {
            return Err("unsupported quality_sweep_expansion_contract".to_string());
        }
        if !bool_option(payload.get("expansion_enabled"), "expansion_enabled", true)? {
            return Ok(disabled_quality_sweep_result(&payload));
        }
        let budget_ready = bool_option(
            payload.get("expansion_budget_ready"),
            "expansion_budget_ready",
            true,
        )?;
        let budget = payload.get("sweep_budget").unwrap_or(&Value::Null);
        object_or_null(budget, "sweep_budget")?;
        let upstream_blocked = bool_option(
            budget.get("case_budget_exceeded"),
            "sweep_budget.case_budget_exceeded",
            false,
        )?;
        if !budget_ready || upstream_blocked {
            return Ok(budget_blocked_quality_sweep_result(&payload));
        }
        Some(serde_json::json!({
            "source_candidate_id": payload.get("source_candidate_id"),
            "sweep_budget": budget,
        }))
    } else {
        None
    };

    let (payload, config) = normalize_expand_input(payload, config)?;
    let base = payload
        .get("base")
        .or_else(|| payload.get("model"))
        .ok_or_else(|| "transform.expand_parameter_sweep requires payload.base".to_string())?;
    let axes_value = payload.get("axes").or_else(|| config.get("axes"));
    let case_count = actual_case_count(
        axes_value
            .and_then(Value::as_array)
            .ok_or_else(|| "transform.expand_parameter_sweep requires axes".to_string())?,
    )?;
    let max_cases = count_option(config.get("max_cases"), "max_cases", 256, 0)?;
    if case_count > max_cases {
        if let Some(mut context) = quality_context {
            context["case_count_estimate"] = Value::from(case_count);
            context["sweep_budget"] =
                refreshed_budget(&context["sweep_budget"], case_count, max_cases, false);
            return Ok(budget_blocked_quality_sweep_result(&context));
        }
        return Err(format!(
            "transform.expand_parameter_sweep would emit {case_count} cases, above max_cases {max_cases}"
        ));
    }
    let axes = parse_axes(axes_value)?;
    let id_prefix = config
        .get("id_prefix")
        .and_then(Value::as_str)
        .unwrap_or("case");
    let case_metadata = payload
        .get("case_metadata")
        .or_else(|| config.get("case_metadata"))
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));

    let mut cases = Vec::new();
    cases
        .try_reserve_exact(case_count)
        .map_err(|error| format!("parameter sweep case allocation failed: {error}"))?;
    expand_axis_cases(
        base,
        &axes,
        0,
        &mut Map::new(),
        &mut cases,
        id_prefix,
        &case_metadata,
    )?;

    Ok(serde_json::json!({
        "cases": cases,
        "case_count": case_count,
        "axis_count": axes.len(),
    }))
}

fn disabled_quality_sweep_result(payload: &Value) -> Value {
    let reason = payload
        .get("reason")
        .and_then(Value::as_str)
        .unwrap_or("stopped");
    serde_json::json!({
        "cases": [],
        "case_count": 0,
        "axis_count": 0,
        "sweep_enabled": false,
        "expansion_enabled": false,
        "sweep_action": reason,
        "sweep_blocking_reason": payload.get("expansion_blocking_reason").cloned().unwrap_or(Value::Null),
        "source_rejected_candidates": payload.get("source_rejected_candidates").cloned().unwrap_or_else(|| serde_json::json!([])),
        "sweep_summary": format!("Quality parameter sweep was skipped: {reason}."),
    })
}

fn budget_blocked_quality_sweep_result(payload: &Value) -> Value {
    let reason = payload
        .get("expansion_blocking_reason")
        .and_then(Value::as_str)
        .unwrap_or("case_budget_exceeded");
    serde_json::json!({
        "cases": [],
        "case_count": 0,
        "axis_count": 0,
        "sweep_enabled": true,
        "expansion_enabled": true,
        "expansion_budget_ready": false,
        "expansion_blocking_reason": reason,
        "source_candidate_id": payload.get("source_candidate_id").cloned().unwrap_or(Value::Null),
        "case_count_estimate": payload.get("case_count_estimate").cloned().unwrap_or(Value::Null),
        "sweep_budget": payload.get("sweep_budget").cloned().unwrap_or(Value::Null),
        "sweep_summary": format!(
            "Quality parameter sweep expansion was blocked before case generation: {reason}."
        ),
    })
}

fn normalize_expand_input(mut payload: Value, config: Value) -> Result<(Value, Value), String> {
    if payload.get("quality_sweep_expansion_contract").is_none() {
        return Ok((payload, config));
    }

    let nested_payload = payload
        .get_mut("payload")
        .map(Value::take)
        .ok_or_else(|| "quality sweep expansion requires payload".to_string())?;
    let nested_config = payload
        .get_mut("config")
        .map(Value::take)
        .unwrap_or_else(|| serde_json::json!({}));

    object_or_null(&nested_config, "quality sweep expansion config")?;
    Ok((nested_payload, merge_config(nested_config, config)))
}

fn merge_config(base: Value, overrides: Value) -> Value {
    let mut merged = match base {
        Value::Object(object) => object,
        _ => Map::new(),
    };
    if let Value::Object(object) = overrides {
        merged.extend(object);
    }
    Value::Object(merged)
}

pub fn summarize_parameter_sweep(payload: Value, config: Value) -> Result<Value, String> {
    let cases = payload
        .get("cases")
        .and_then(Value::as_array)
        .ok_or_else(|| "transform.summarize_parameter_sweep requires payload.cases".to_string())?;
    if cases.is_empty() {
        return Err("transform.summarize_parameter_sweep cases must not be empty".to_string());
    }
    let fields = config
        .get("fields")
        .and_then(Value::as_array)
        .map(|entries| entries.iter().filter_map(Value::as_str).collect::<Vec<_>>())
        .unwrap_or_default();
    let include_parameters = config
        .get("include_parameters")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let include_metadata = config
        .get("include_metadata")
        .and_then(Value::as_bool)
        .unwrap_or(true);

    let mut rows = Vec::with_capacity(cases.len());
    let mut numeric_columns: Map<String, Value> = Map::new();
    for (index, case) in cases.iter().enumerate() {
        let summary = case
            .get("summary")
            .or_else(|| case.get("result"))
            .and_then(Value::as_object)
            .ok_or_else(|| {
                format!("transform.summarize_parameter_sweep case {index} requires summary")
            })?;
        let mut row = Map::new();
        row.insert(
            "case_id".to_string(),
            case.get("id")
                .cloned()
                .unwrap_or_else(|| Value::from(format!("case_{index}"))),
        );
        if include_parameters {
            row.insert(
                "parameters".to_string(),
                case.get("parameters").cloned().unwrap_or(Value::Null),
            );
        }
        if include_metadata {
            row.insert(
                "metadata".to_string(),
                case.get("metadata").cloned().unwrap_or(Value::Null),
            );
        }

        let selected_fields = if fields.is_empty() {
            summary.keys().map(String::as_str).collect::<Vec<_>>()
        } else {
            fields.clone()
        };
        for field in selected_fields {
            let Some(value) = summary.get(field) else {
                continue;
            };
            row.insert(field.to_string(), value.clone());
            if let Some(number) = value.as_f64() {
                push_numeric_column(&mut numeric_columns, field, number);
            }
        }
        rows.push(Value::Object(row));
    }

    Ok(serde_json::json!({
        "rows": rows,
        "row_count": cases.len(),
        "numeric_columns": numeric_columns,
    }))
}

fn parse_axes(value: Option<&Value>) -> Result<Vec<SweepAxis<'_>>, String> {
    let axes = value
        .and_then(Value::as_array)
        .ok_or_else(|| "transform.expand_parameter_sweep requires axes".to_string())?;
    if axes.is_empty() {
        return Err("transform.expand_parameter_sweep axes must not be empty".to_string());
    }

    axes.iter()
        .enumerate()
        .map(|(index, axis)| {
            let path = axis
                .get("path")
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    format!("transform.expand_parameter_sweep axis {index} requires path")
                })?
                .trim();
            if path.is_empty() {
                return Err(format!(
                    "transform.expand_parameter_sweep axis {index} path must not be empty"
                ));
            }
            let values = axis
                .get("values")
                .and_then(Value::as_array)
                .ok_or_else(|| {
                    format!("transform.expand_parameter_sweep axis {index} requires values")
                })?;
            if values.is_empty() {
                return Err(format!(
                    "transform.expand_parameter_sweep axis {index} values must not be empty"
                ));
            }
            Ok(SweepAxis {
                label: axis.get("label").and_then(Value::as_str).unwrap_or(path),
                path,
                values,
            })
        })
        .collect()
}

fn expand_axis_cases(
    base: &Value,
    axes: &[SweepAxis<'_>],
    axis_index: usize,
    parameters: &mut Map<String, Value>,
    cases: &mut Vec<Value>,
    id_prefix: &str,
    case_metadata: &Value,
) -> Result<(), String> {
    if axis_index == axes.len() {
        let mut model = base.clone();
        for axis in axes {
            let value = parameters
                .get(axis.label)
                .ok_or_else(|| format!("missing sweep parameter {}", axis.label))?;
            set_dotted_path(&mut model, axis.path, value.clone())?;
        }
        let index = cases.len();
        cases.push(serde_json::json!({
            "id": format!("{id_prefix}_{index}"),
            "label": format_case_label(parameters),
            "parameters": parameters.clone(),
            "metadata": case_metadata,
            "model": model,
        }));
        return Ok(());
    }

    let axis = &axes[axis_index];
    for value in axis.values {
        parameters.insert(axis.label.to_string(), value.clone());
        expand_axis_cases(
            base,
            axes,
            axis_index + 1,
            parameters,
            cases,
            id_prefix,
            case_metadata,
        )?;
    }
    parameters.remove(axis.label);
    Ok(())
}

fn set_dotted_path(target: &mut Value, path: &str, value: Value) -> Result<(), String> {
    let mut cursor = target;
    let segments = path.split('.').collect::<Vec<_>>();
    for (index, segment) in segments.iter().enumerate() {
        let is_last = index + 1 == segments.len();
        if let Ok(array_index) = segment.parse::<usize>() {
            let array = cursor
                .as_array_mut()
                .ok_or_else(|| format!("path segment {segment} expected an array"))?;
            cursor = array
                .get_mut(array_index)
                .ok_or_else(|| format!("path segment {segment} is out of range"))?;
        } else {
            let object = cursor
                .as_object_mut()
                .ok_or_else(|| format!("path segment {segment} expected an object"))?;
            if is_last {
                object.insert((*segment).to_string(), value);
                return Ok(());
            }
            cursor = object
                .get_mut(*segment)
                .ok_or_else(|| format!("path segment {segment} is missing"))?;
        }
    }
    Err("transform.expand_parameter_sweep path must target an object field".to_string())
}

fn format_case_label(parameters: &Map<String, Value>) -> String {
    parameters
        .iter()
        .map(|(key, value)| {
            let rendered = value
                .as_str()
                .map(ToString::to_string)
                .unwrap_or_else(|| value.to_string());
            format!("{key}={rendered}")
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn push_numeric_column(columns: &mut Map<String, Value>, field: &str, value: f64) {
    let entry = columns.entry(field.to_string()).or_insert_with(|| {
        serde_json::json!({
            "count": 0,
            "min": value,
            "max": value,
            "sum": 0.0,
        })
    });
    if let Some(object) = entry.as_object_mut() {
        let count = object.get("count").and_then(Value::as_u64).unwrap_or(0) + 1;
        let min = object
            .get("min")
            .and_then(Value::as_f64)
            .unwrap_or(value)
            .min(value);
        let max = object
            .get("max")
            .and_then(Value::as_f64)
            .unwrap_or(value)
            .max(value);
        let sum = object.get("sum").and_then(Value::as_f64).unwrap_or(0.0) + value;
        object.insert("count".to_string(), Value::from(count));
        object.insert("min".to_string(), Value::from(min));
        object.insert("max".to_string(), Value::from(max));
        object.insert("sum".to_string(), Value::from(sum));
        object.insert("mean".to_string(), Value::from(sum / count as f64));
    }
}
