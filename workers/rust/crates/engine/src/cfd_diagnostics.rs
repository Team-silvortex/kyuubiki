use serde_json::{Map, Value};

pub fn extract_stokes_flow_result_diagnostics(
    payload: Value,
    config: Value,
) -> Result<Value, String> {
    let object = payload.as_object().ok_or_else(|| {
        "extract.stokes_flow_result_diagnostics expects an object payload".to_string()
    })?;
    let nodes = object
        .get("nodes")
        .and_then(Value::as_array)
        .ok_or_else(|| "extract.stokes_flow_result_diagnostics expects nodes array".to_string())?;
    let elements = object
        .get("elements")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            "extract.stokes_flow_result_diagnostics expects elements array".to_string()
        })?;
    if nodes.is_empty() && elements.is_empty() {
        return Err(
            "extract.stokes_flow_result_diagnostics expects non-empty nodes or elements".into(),
        );
    }

    let prefix = config
        .get("output_prefix")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("cfd");
    let velocity_values = numeric_values(nodes, "velocity_magnitude");
    let pressure_values = numeric_values(nodes, "pressure");
    let divergence_values = numeric_values(elements, "divergence_error");
    let reynolds_values = numeric_values(elements, "reynolds_number");
    let dissipation_values = numeric_values(elements, "viscous_dissipation");
    let mut summary = Map::new();

    summary.insert(
        "diagnostic_contract".into(),
        Value::String("kyuubiki.workflow_diagnostics/v1".into()),
    );
    summary.insert("diagnostic_domain".into(), Value::String("fluid".into()));
    summary.insert(
        "diagnostic_subject".into(),
        Value::String("stokes_flow_result".into()),
    );
    summary.insert("diagnostic_prefix".into(), Value::String(prefix.into()));
    summary.insert("diagnostic_node_count".into(), Value::from(nodes.len()));
    summary.insert(
        "diagnostic_element_count".into(),
        Value::from(elements.len()),
    );
    merge_min_max(
        &mut summary,
        &format!("{prefix}_velocity"),
        &velocity_values,
    );
    merge_min_max(
        &mut summary,
        &format!("{prefix}_pressure"),
        &pressure_values,
    );
    merge_peak(
        &mut summary,
        &format!("{prefix}_divergence_error"),
        elements,
        "divergence_error",
    );
    merge_peak(
        &mut summary,
        &format!("{prefix}_reynolds_number"),
        elements,
        "reynolds_number",
    );
    merge_peak(
        &mut summary,
        &format!("{prefix}_viscous_dissipation"),
        elements,
        "viscous_dissipation",
    );
    summary.insert(
        format!("{prefix}_velocity_mean"),
        Value::from(mean_or_zero(&velocity_values)),
    );
    summary.insert(
        format!("{prefix}_pressure_mean"),
        Value::from(mean_or_zero(&pressure_values)),
    );
    summary.insert(
        format!("{prefix}_divergence_error_mean"),
        Value::from(mean_or_zero(&divergence_values)),
    );
    summary.insert(
        format!("{prefix}_reynolds_number_mean"),
        Value::from(mean_or_zero(&reynolds_values)),
    );
    summary.insert(
        format!("{prefix}_viscous_dissipation_total"),
        Value::from(dissipation_values.iter().sum::<f64>()),
    );

    Ok(Value::Object(summary))
}

fn merge_min_max(summary: &mut Map<String, Value>, key: &str, values: &[f64]) {
    let min = values.iter().copied().reduce(f64::min).unwrap_or(0.0);
    let max = values.iter().copied().reduce(f64::max).unwrap_or(0.0);
    summary.insert(format!("{key}_min"), Value::from(min));
    summary.insert(format!("{key}_max"), Value::from(max));
}

fn merge_peak(summary: &mut Map<String, Value>, key: &str, elements: &[Value], field: &str) {
    let peak = elements
        .iter()
        .filter_map(Value::as_object)
        .filter_map(|element| Some((element, element.get(field)?.as_f64()?)))
        .max_by(|(_, left), (_, right)| {
            left.abs()
                .partial_cmp(&right.abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        });

    match peak {
        Some((element, value)) => {
            summary.insert(format!("{key}_peak"), Value::from(value));
            summary.insert(
                format!("{key}_peak_element_id"),
                element.get("id").cloned().unwrap_or(Value::Null),
            );
        }
        None => {
            summary.insert(format!("{key}_peak"), Value::from(0.0));
            summary.insert(format!("{key}_peak_element_id"), Value::Null);
        }
    }
}

fn numeric_values(values: &[Value], field: &str) -> Vec<f64> {
    values
        .iter()
        .filter_map(Value::as_object)
        .filter_map(|value| value.get(field).and_then(Value::as_f64))
        .collect()
}

fn mean_or_zero(values: &[f64]) -> f64 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f64>() / values.len() as f64
    }
}
