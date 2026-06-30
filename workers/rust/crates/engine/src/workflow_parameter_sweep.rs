use serde_json::{Map, Value};

pub use crate::workflow_parameter_sweep_results::{
    join_parameter_sweep_results, score_parameter_sweep,
};

#[derive(Debug, Clone)]
struct SweepAxis {
    label: String,
    path: String,
    values: Vec<Value>,
}

pub fn expand_parameter_sweep(payload: Value, config: Value) -> Result<Value, String> {
    let base = payload
        .get("base")
        .or_else(|| payload.get("model"))
        .cloned()
        .ok_or_else(|| "transform.expand_parameter_sweep requires payload.base".to_string())?;
    let axes = parse_axes(payload.get("axes").or_else(|| config.get("axes")))?;
    let max_cases = config
        .get("max_cases")
        .and_then(Value::as_u64)
        .unwrap_or(256) as usize;
    let id_prefix = config
        .get("id_prefix")
        .and_then(Value::as_str)
        .unwrap_or("case");

    let case_count = axes
        .iter()
        .try_fold(1usize, |count, axis| count.checked_mul(axis.values.len()))
        .ok_or_else(|| "transform.expand_parameter_sweep case count overflowed".to_string())?;
    if case_count == 0 {
        return Err("transform.expand_parameter_sweep requires at least one case".to_string());
    }
    if case_count > max_cases {
        return Err(format!(
            "transform.expand_parameter_sweep would emit {case_count} cases, above max_cases {max_cases}"
        ));
    }

    let mut cases = Vec::with_capacity(case_count);
    expand_axis_cases(&base, &axes, 0, &mut Map::new(), &mut cases, id_prefix)?;

    Ok(serde_json::json!({
        "cases": cases,
        "case_count": case_count,
        "axis_count": axes.len(),
    }))
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

fn parse_axes(value: Option<&Value>) -> Result<Vec<SweepAxis>, String> {
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
                label: axis
                    .get("label")
                    .and_then(Value::as_str)
                    .unwrap_or(path)
                    .to_string(),
                path: path.to_string(),
                values: values.clone(),
            })
        })
        .collect()
}

fn expand_axis_cases(
    base: &Value,
    axes: &[SweepAxis],
    axis_index: usize,
    parameters: &mut Map<String, Value>,
    cases: &mut Vec<Value>,
    id_prefix: &str,
) -> Result<(), String> {
    if axis_index == axes.len() {
        let mut model = base.clone();
        for axis in axes {
            let value = parameters
                .get(&axis.label)
                .ok_or_else(|| format!("missing sweep parameter {}", axis.label))?;
            set_dotted_path(&mut model, &axis.path, value.clone())?;
        }
        let index = cases.len();
        cases.push(serde_json::json!({
            "id": format!("{id_prefix}_{index}"),
            "label": format_case_label(parameters),
            "parameters": parameters.clone(),
            "model": model,
        }));
        return Ok(());
    }

    let axis = &axes[axis_index];
    for value in &axis.values {
        parameters.insert(axis.label.clone(), value.clone());
        expand_axis_cases(base, axes, axis_index + 1, parameters, cases, id_prefix)?;
    }
    parameters.remove(&axis.label);
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
