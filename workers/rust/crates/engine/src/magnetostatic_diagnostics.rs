use serde_json::Value;

pub fn extract_magnetostatic_result_diagnostics(
    payload: Value,
    config: Value,
) -> Result<Value, String> {
    let object = payload.as_object().ok_or_else(|| {
        "extract.magnetostatic_result_diagnostics expects an object payload".to_string()
    })?;
    let nodes = fetch_collection(object, &config, "node_source", "nodes")?;
    let elements = fetch_collection(object, &config, "element_source", "elements")?;
    let prefix = normalize_prefix(&config, "output_prefix", "magnetostatic");

    let vector_potential_nodes = collect_numeric_entries(
        nodes,
        field_name(&config, "vector_potential_field", "vector_potential"),
    );
    let current_density_nodes = collect_numeric_entries(
        nodes,
        field_name(&config, "current_density_field", "current_density"),
    );
    let energy_density_peak = peak_scalar_entry(
        elements,
        field_name(&config, "energy_density_field", "energy_area_density"),
    )
    .or_else(|| peak_scalar_entry(elements, "stored_energy"));
    let field_peak = peak_vector_entry(
        elements,
        field_name(&config, "field_x_field", "magnetic_field_strength_x"),
        field_name(&config, "field_y_field", "magnetic_field_strength_y"),
        optional_field_name(&config, "field_z_field"),
        Some(field_name(
            &config,
            "field_magnitude_field",
            "magnetic_field_strength_magnitude",
        )),
    );
    let flux_peak = peak_vector_entry(
        elements,
        field_name(&config, "flux_x_field", "magnetic_flux_density_x"),
        field_name(&config, "flux_y_field", "magnetic_flux_density_y"),
        optional_field_name(&config, "flux_z_field"),
        Some(field_name(
            &config,
            "flux_magnitude_field",
            "magnetic_flux_density_magnitude",
        )),
    );

    let diagnostics = serde_json::json!({
        format!("{prefix}_node_count"): count_object_entries(nodes),
        format!("{prefix}_element_count"): count_object_entries(elements),
        "diagnostic_contract": "kyuubiki.workflow_diagnostics/v1",
        "diagnostic_domain": "magnetostatic",
        "diagnostic_subject": "magnetostatic_result",
        "diagnostic_prefix": prefix,
        "diagnostic_node_count": count_object_entries(nodes),
        "diagnostic_element_count": count_object_entries(elements),
        "diagnostic_metric_groups": ["vector_potential", "current_density", "energy_density", "field", "flux"],
    });
    let diagnostics = merge_distribution_stats(
        diagnostics,
        &prefix,
        "vector_potential",
        &vector_potential_nodes,
    );
    let diagnostics = merge_distribution_stats(
        diagnostics,
        &prefix,
        "current_density",
        &current_density_nodes,
    );
    let diagnostics = merge_peak_scalar(
        diagnostics,
        &prefix,
        "energy_density",
        energy_density_peak.as_ref(),
    );
    let diagnostics = merge_peak_vector(diagnostics, &prefix, "field", field_peak.as_ref());
    let diagnostics = merge_peak_vector(diagnostics, &prefix, "flux", flux_peak.as_ref());

    ensure_non_empty_diagnostics(
        diagnostics,
        &prefix,
        "extract.magnetostatic_result_diagnostics did not find any diagnostic fields",
    )
}

fn fetch_collection<'a>(
    object: &'a serde_json::Map<String, Value>,
    config: &Value,
    config_key: &str,
    default_key: &str,
) -> Result<&'a [Value], String> {
    let source = field_name(config, config_key, default_key);
    object
        .get(source)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .ok_or_else(|| format!("workflow diagnostics expected array payload.{source}"))
}

fn field_name<'a>(config: &'a Value, key: &'a str, default_value: &'a str) -> &'a str {
    config
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or(default_value)
}

fn optional_field_name<'a>(config: &'a Value, key: &str) -> Option<&'a str> {
    config.get(key).and_then(Value::as_str)
}

fn normalize_prefix(config: &Value, key: &str, default_value: &str) -> String {
    config
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(default_value)
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string()
}

fn count_object_entries(entries: &[Value]) -> usize {
    entries.iter().filter(|entry| entry.is_object()).count()
}

fn collect_numeric_entries<'a>(entries: &'a [Value], field: &str) -> Vec<(&'a Value, f64)> {
    entries
        .iter()
        .filter_map(|entry| Some((entry, entry.get(field)?.as_f64()?)))
        .collect()
}

fn peak_scalar_entry<'a>(entries: &'a [Value], field: &str) -> Option<(&'a Value, f64)> {
    collect_numeric_entries(entries, field)
        .into_iter()
        .max_by(|left, right| left.1.total_cmp(&right.1))
}

fn peak_vector_entry<'a>(
    entries: &'a [Value],
    x_field: &str,
    y_field: &str,
    z_field: Option<&str>,
    magnitude_field: Option<&str>,
) -> Option<(&'a Value, f64, Vec<(&'static str, f64)>)> {
    entries
        .iter()
        .filter_map(|entry| {
            let mut components = Vec::new();
            if let Some(value) = entry.get(x_field).and_then(Value::as_f64) {
                components.push(("x", value));
            }
            if let Some(value) = entry.get(y_field).and_then(Value::as_f64) {
                components.push(("y", value));
            }
            if let Some(field) = z_field {
                if let Some(value) = entry.get(field).and_then(Value::as_f64) {
                    components.push(("z", value));
                }
            }

            let magnitude = magnitude_field
                .and_then(|field| entry.get(field).and_then(Value::as_f64))
                .or_else(|| {
                    (components.len() >= 2).then(|| {
                        components
                            .iter()
                            .map(|(_, value)| value * value)
                            .sum::<f64>()
                            .sqrt()
                    })
                })?;
            Some((entry, magnitude, components))
        })
        .max_by(|left, right| left.1.total_cmp(&right.1))
}

fn merge_distribution_stats(
    diagnostics: Value,
    prefix: &str,
    label: &str,
    entries: &[(&Value, f64)],
) -> Value {
    if entries.is_empty() {
        return diagnostics;
    }
    let values = entries.iter().map(|(_, value)| *value).collect::<Vec<_>>();
    let sum = values.iter().sum::<f64>();
    merge_json_objects(
        diagnostics,
        serde_json::json!({
            format!("{prefix}_{label}_count"): values.len(),
            format!("{prefix}_{label}_min"): values.iter().copied().fold(f64::INFINITY, f64::min),
            format!("{prefix}_{label}_max"): values.iter().copied().fold(f64::NEG_INFINITY, f64::max),
            format!("{prefix}_{label}_mean"): sum / values.len() as f64,
            format!("{prefix}_{label}_sum"): sum,
            format!("{prefix}_{label}_span"): values.iter().copied().fold(f64::NEG_INFINITY, f64::max)
                - values.iter().copied().fold(f64::INFINITY, f64::min),
        }),
    )
}

fn merge_peak_scalar(
    diagnostics: Value,
    prefix: &str,
    label: &str,
    peak: Option<&(&Value, f64)>,
) -> Value {
    let Some((entry, value)) = peak else {
        return diagnostics;
    };
    merge_json_objects(
        diagnostics,
        serde_json::json!({
            format!("{prefix}_{label}_peak"): value,
            format!("{prefix}_peak_{label}"): value,
            format!("{prefix}_peak_{label}_id"): entry.get("id").cloned().unwrap_or(Value::Null),
            format!("{prefix}_{label}_peak_element_id"): entry.get("id").cloned().unwrap_or(Value::Null),
        }),
    )
}

fn merge_peak_vector(
    diagnostics: Value,
    prefix: &str,
    label: &str,
    peak: Option<&(&Value, f64, Vec<(&'static str, f64)>)>,
) -> Value {
    let Some((entry, magnitude, components)) = peak else {
        return diagnostics;
    };
    let mut peak_json = serde_json::json!({
        format!("{prefix}_{label}_peak_magnitude"): magnitude,
        format!("{prefix}_peak_{label}"): magnitude,
        format!("{prefix}_peak_{label}_id"): entry.get("id").cloned().unwrap_or(Value::Null),
        format!("{prefix}_{label}_peak_element_id"): entry.get("id").cloned().unwrap_or(Value::Null),
    });
    if let Some(object) = peak_json.as_object_mut() {
        for (axis, value) in components {
            object.insert(format!("{prefix}_{label}_peak_{axis}"), Value::from(*value));
        }
    }
    merge_json_objects(diagnostics, peak_json)
}

fn merge_json_objects(mut left: Value, right: Value) -> Value {
    let Some(left_object) = left.as_object_mut() else {
        return right;
    };
    let Some(right_object) = right.as_object() else {
        return left;
    };
    for (key, value) in right_object {
        left_object.insert(key.clone(), value.clone());
    }
    left
}

fn ensure_non_empty_diagnostics(
    diagnostics: Value,
    prefix: &str,
    message: &str,
) -> Result<Value, String> {
    let object = diagnostics
        .as_object()
        .ok_or_else(|| "workflow diagnostics produced a non-object result".to_string())?;
    let field_count = object
        .keys()
        .filter(|key| key.starts_with(prefix) || key.starts_with("diagnostic_"))
        .count();
    if field_count <= 6 {
        return Err(message.to_string());
    }
    Ok(diagnostics)
}
