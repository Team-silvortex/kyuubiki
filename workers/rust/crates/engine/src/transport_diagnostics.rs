use serde_json::{Map, Value};

pub fn extract_transport_result_diagnostics(
    payload: Value,
    config: Value,
) -> Result<Value, String> {
    let object = payload.as_object().ok_or_else(|| {
        "extract.transport_result_diagnostics expects an object payload".to_string()
    })?;
    let nodes = collection(object, "nodes", "extract.transport_result_diagnostics")?;
    let elements = collection(object, "elements", "extract.transport_result_diagnostics")?;
    let prefix = config
        .get("output_prefix")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("transport");

    let concentrations = numeric_values(nodes, node_value, "concentration");
    let sources = numeric_values(nodes, node_value, "source");
    let mut summary = Map::new();
    summary.insert(
        "diagnostic_contract".into(),
        Value::String("kyuubiki.workflow_diagnostics/v1".into()),
    );
    summary.insert(
        "diagnostic_domain".into(),
        Value::String("transport".into()),
    );
    summary.insert(
        "diagnostic_subject".into(),
        Value::String("advection_diffusion_result".into()),
    );
    summary.insert("diagnostic_prefix".into(), Value::String(prefix.into()));
    summary.insert("diagnostic_node_count".into(), Value::from(nodes.len()));
    summary.insert(
        "diagnostic_element_count".into(),
        Value::from(elements.len()),
    );
    summary.insert(
        "diagnostic_metric_groups".into(),
        serde_json::json!(["concentration", "source", "flux", "peclet"]),
    );

    merge_min_max(
        &mut summary,
        &format!("{prefix}_concentration"),
        &concentrations,
    );
    summary.insert(
        format!("{prefix}_concentration_mean"),
        Value::from(mean_or_zero(&concentrations)),
    );
    summary.insert(format!("{prefix}_source_count"), Value::from(sources.len()));
    summary.insert(
        format!("{prefix}_source_sum"),
        Value::from(sources.iter().sum::<f64>()),
    );
    summary.insert(
        format!("{prefix}_source_mean"),
        Value::from(mean_or_zero(&sources)),
    );
    merge_peak(
        &mut summary,
        &format!("{prefix}_total_flux"),
        elements,
        "total_flux",
        element_value,
    );
    merge_peak(
        &mut summary,
        &format!("{prefix}_diffusive_flux"),
        elements,
        "diffusive_flux",
        element_value,
    );
    merge_peak(
        &mut summary,
        &format!("{prefix}_advective_flux"),
        elements,
        "advective_flux",
        element_value,
    );
    merge_peak(
        &mut summary,
        &format!("{prefix}_peclet"),
        elements,
        "peclet_number",
        element_value,
    );

    Ok(Value::Object(summary))
}

fn collection<'a>(
    object: &'a Map<String, Value>,
    key: &str,
    operator_id: &str,
) -> Result<&'a [Value], String> {
    object
        .get(key)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .ok_or_else(|| format!("{operator_id} expects {key} array"))
}

fn numeric_values(
    items: &[Value],
    value_fn: fn(&Map<String, Value>, &str) -> Option<f64>,
    field: &str,
) -> Vec<f64> {
    items
        .iter()
        .filter_map(Value::as_object)
        .filter_map(|item| value_fn(item, field))
        .collect()
}

fn merge_min_max(summary: &mut Map<String, Value>, prefix: &str, values: &[f64]) {
    if values.is_empty() {
        return;
    }
    let min = values
        .iter()
        .fold(f64::INFINITY, |current, value| current.min(*value));
    let max = values
        .iter()
        .fold(f64::NEG_INFINITY, |current, value| current.max(*value));
    summary.insert(format!("{prefix}_min"), Value::from(min));
    summary.insert(format!("{prefix}_max"), Value::from(max));
    summary.insert(format!("{prefix}_span"), Value::from(max - min));
}

fn merge_peak(
    summary: &mut Map<String, Value>,
    prefix: &str,
    items: &[Value],
    field: &str,
    value_fn: fn(&Map<String, Value>, &str) -> Option<f64>,
) {
    let Some((value, item)) = items
        .iter()
        .filter_map(|item| {
            let object = item.as_object()?;
            value_fn(object, field).map(|value| (value, item))
        })
        .max_by(|(left, _), (right, _)| left.abs().total_cmp(&right.abs()))
    else {
        return;
    };
    summary.insert(format!("{prefix}_peak"), Value::from(value));
    summary.insert(format!("{prefix}_peak_magnitude"), Value::from(value.abs()));
    if let Some(id) = item.get("id").cloned() {
        summary.insert(format!("{prefix}_peak_id"), id);
    }
}

fn node_value(object: &Map<String, Value>, field: &str) -> Option<f64> {
    object
        .get(field)
        .and_then(finite_number)
        .or_else(|| match field {
            "concentration" => first_alias_number(object, &["c", "species", "scalar"]),
            "source" => {
                first_alias_number(object, &["source_density", "net_source", "source_term"])
            }
            _ => None,
        })
}

fn element_value(object: &Map<String, Value>, field: &str) -> Option<f64> {
    object
        .get(field)
        .and_then(finite_number)
        .or_else(|| match field {
            "total_flux" => first_alias_number(object, &["flux", "flux_total"])
                .or_else(|| vector_magnitude(object, "flux_x", "flux_y")),
            "diffusive_flux" => {
                first_alias_number(object, &["diffusion_flux", "diffusive_flux_total"])
                    .or_else(|| vector_magnitude(object, "diffusive_flux_x", "diffusive_flux_y"))
            }
            "advective_flux" => {
                first_alias_number(object, &["advection_flux", "advective_flux_total"])
                    .or_else(|| vector_magnitude(object, "advective_flux_x", "advective_flux_y"))
            }
            "peclet_number" => first_alias_number(object, &["peclet", "pe"]),
            _ => None,
        })
}

fn vector_magnitude(object: &Map<String, Value>, x: &str, y: &str) -> Option<f64> {
    let x = object.get(x).and_then(finite_number)?;
    let y = object.get(y).and_then(finite_number)?;
    Some((x * x + y * y).sqrt())
}

fn first_alias_number(object: &Map<String, Value>, aliases: &[&str]) -> Option<f64> {
    aliases
        .iter()
        .find_map(|alias| object.get(*alias).and_then(finite_number))
}

fn finite_number(value: &Value) -> Option<f64> {
    value.as_f64().filter(|number| number.is_finite())
}

fn mean_or_zero(values: &[f64]) -> f64 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f64>() / values.len() as f64
    }
}
