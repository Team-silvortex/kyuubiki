use serde_json::{Map, Value};
use std::collections::HashMap;

pub(crate) struct SweepAxis<'a> {
    index: usize,
    label: &'a str,
    path: &'a str,
    target: Vec<PathStep<'a>>,
    values: &'a [Value],
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum PathStep<'a> {
    Key(&'a str),
    Index(usize),
}

pub(crate) fn prepare_sweep_axes<'a>(
    base: &Value,
    axes: &'a [Value],
) -> Result<Vec<SweepAxis<'a>>, String> {
    if axes.is_empty() {
        return Err("parameter sweep axes must not be empty".to_string());
    }
    let mut labels = HashMap::new();
    let prepared = axes
        .iter()
        .enumerate()
        .map(|(index, axis)| {
            let path = axis
                .get("path")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|path| !path.is_empty())
                .ok_or_else(|| format!("parameter sweep axis {index} requires a nonempty path"))?;
            let label = match axis.get("label") {
                None => path,
                Some(value) => value
                    .as_str()
                    .filter(|label| !label.trim().is_empty())
                    .ok_or_else(|| {
                        format!("parameter sweep axis {index} label must be a nonempty string")
                    })?,
            };
            if let Some(previous) = labels.insert(label, index) {
                return Err(format!(
                    "parameter sweep axis {index} duplicate label {label:?} with axis {previous}"
                ));
            }
            let values = axis
                .get("values")
                .and_then(Value::as_array)
                .filter(|values| !values.is_empty())
                .ok_or_else(|| format!("parameter sweep axis {index} requires nonempty values"))?;
            let target = resolve_path(base, path)
                .map_err(|error| format!("parameter sweep axis {index} path {path:?}: {error}"))?;
            Ok(SweepAxis {
                index,
                label,
                path,
                target,
                values,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    // Compare resolved steps, not string prefixes: array aliases share a target,
    // but numeric object keys and sibling names such as a/ab remain distinct.
    let mut by_target = prepared.iter().collect::<Vec<_>>();
    by_target.sort_by(|left, right| left.target.cmp(&right.target));
    for pair in by_target.windows(2) {
        let (left, right) = (pair[0], pair[1]);
        if right.target.starts_with(&left.target) {
            return Err(format!(
                "parameter sweep axis {} path {:?} overlaps axis {} path {:?}",
                right.index, right.path, left.index, left.path,
            ));
        }
    }
    Ok(prepared)
}

fn resolve_path<'a>(base: &Value, path: &'a str) -> Result<Vec<PathStep<'a>>, String> {
    let mut cursor = base;
    let mut target = Vec::new();
    let mut segments = path.split('.').peekable();
    while let Some(segment) = segments.next() {
        if segment.trim().is_empty() {
            return Err("path segments must not be empty".to_string());
        }
        let last = segments.peek().is_none();
        match cursor {
            Value::Object(object) => {
                target.push(PathStep::Key(segment));
                if !last {
                    cursor = object
                        .get(segment)
                        .ok_or_else(|| format!("parent field {segment:?} is missing"))?;
                }
            }
            Value::Array(array) => {
                let index = segment
                    .bytes()
                    .all(|byte| byte.is_ascii_digit())
                    .then(|| segment.parse::<usize>().ok())
                    .flatten()
                    .ok_or_else(|| {
                        format!("array index {segment:?} must be a nonnegative decimal integer")
                    })?;
                cursor = array.get(index).ok_or_else(|| {
                    format!(
                        "array index {segment:?} is out of range (length {})",
                        array.len()
                    )
                })?;
                target.push(PathStep::Index(index));
            }
            _ => {
                return Err(format!(
                    "path segment {segment:?} requires an object or array parent"
                ));
            }
        }
    }
    Ok(target)
}

impl SweepAxis<'_> {
    fn assign(&self, model: &mut Value, value: Value) -> Result<(), String> {
        let mut cursor = model;
        for (position, step) in self.target.iter().enumerate() {
            let last = position + 1 == self.target.len();
            if last && let PathStep::Key(key) = step {
                let object = cursor
                    .as_object_mut()
                    .ok_or_else(|| self.unresolved_error())?;
                object.insert((*key).to_string(), value);
                return Ok(());
            }
            cursor = match step {
                PathStep::Key(key) => cursor
                    .as_object_mut()
                    .and_then(|object| object.get_mut(*key)),
                PathStep::Index(index) => cursor
                    .as_array_mut()
                    .and_then(|array| array.get_mut(*index)),
            }
            .ok_or_else(|| self.unresolved_error())?;
            if last {
                *cursor = value;
                return Ok(());
            }
        }
        Err(self.unresolved_error())
    }

    fn unresolved_error(&self) -> String {
        format!(
            "parameter sweep axis {} path {:?} no longer resolves",
            self.index, self.path
        )
    }
}

pub(crate) fn expand_sweep_cases(
    base: &Value,
    axes: &[SweepAxis<'_>],
    case_count: usize,
    id_prefix: &str,
    case_metadata: &Value,
) -> Result<Vec<Value>, String> {
    let mut cases = Vec::new();
    cases
        .try_reserve_exact(case_count)
        .map_err(|error| format!("parameter sweep case allocation failed: {error}"))?;
    let mut selection = vec![0usize; axes.len()];
    for case_index in 0..case_count {
        let mut model = base.clone();
        let mut parameters = Map::new();
        for (axis, selected) in axes.iter().zip(&selection) {
            let value = &axis.values[*selected];
            axis.assign(&mut model, value.clone())?;
            parameters.insert(axis.label.to_string(), value.clone());
        }
        cases.push(serde_json::json!({
            "id": format!("{id_prefix}_{case_index}"),
            "label": format_case_label(&parameters),
            "parameters": parameters,
            "metadata": case_metadata,
            "model": model,
        }));
        // Mixed-radix counting preserves the prior last-axis-fastest order
        // without a call-stack frame per axis.
        for (selected, axis) in selection.iter_mut().zip(axes).rev() {
            *selected += 1;
            if *selected < axis.values.len() {
                break;
            }
            *selected = 0;
        }
    }
    Ok(cases)
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
