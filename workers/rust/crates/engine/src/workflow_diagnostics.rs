use serde_json::Value;

pub fn extract_thermal_result_diagnostics(payload: Value, config: Value) -> Result<Value, String> {
    let object = payload.as_object().ok_or_else(|| {
        "extract.thermal_result_diagnostics expects an object payload".to_string()
    })?;
    let nodes = fetch_collection(object, &config, "node_source", "nodes")?;
    let elements = fetch_collection(object, &config, "element_source", "elements")?;
    let prefix = normalize_prefix(&config, "output_prefix", "thermal");

    let temperature_nodes = collect_numeric_entries(
        nodes,
        field_name(&config, "temperature_field", "temperature"),
    );
    let heat_load_nodes =
        collect_numeric_entries(nodes, field_name(&config, "heat_load_field", "heat_load"));
    let gradient_peak = peak_vector_entry(
        elements,
        field_name(&config, "gradient_x_field", "temperature_gradient_x"),
        field_name(&config, "gradient_y_field", "temperature_gradient_y"),
        optional_field_name(&config, "gradient_z_field"),
        optional_field_name(&config, "gradient_magnitude_field"),
    );
    let flux_peak = peak_vector_entry(
        elements,
        field_name(&config, "flux_x_field", "heat_flux_x"),
        field_name(&config, "flux_y_field", "heat_flux_y"),
        optional_field_name(&config, "flux_z_field"),
        Some(field_name(
            &config,
            "flux_magnitude_field",
            "heat_flux_magnitude",
        )),
    );

    let diagnostics = serde_json::json!({
        format!("{prefix}_node_count"): count_object_entries(nodes),
        format!("{prefix}_element_count"): count_object_entries(elements),
    });
    let diagnostics = merge_diagnostic_contract(
        diagnostics,
        "thermal",
        "thermal_result",
        &prefix,
        &["temperature", "heat_load", "gradient", "flux"],
    );
    let diagnostics =
        merge_distribution_stats(diagnostics, &prefix, "temperature", &temperature_nodes);
    let diagnostics = merge_distribution_stats(diagnostics, &prefix, "heat_load", &heat_load_nodes);
    let diagnostics = merge_peak_vector(diagnostics, &prefix, "gradient", gradient_peak.as_ref());
    let diagnostics = merge_peak_vector(diagnostics, &prefix, "flux", flux_peak.as_ref());

    ensure_non_empty_diagnostics(
        diagnostics,
        &prefix,
        "extract.thermal_result_diagnostics did not find any diagnostic fields",
    )
}

pub fn extract_thermo_result_diagnostics(payload: Value, config: Value) -> Result<Value, String> {
    let object = payload
        .as_object()
        .ok_or_else(|| "extract.thermo_result_diagnostics expects an object payload".to_string())?;
    let nodes = fetch_collection(object, &config, "node_source", "nodes")?;
    let elements = fetch_collection(object, &config, "element_source", "elements")?;
    let prefix = normalize_prefix(&config, "output_prefix", "thermo");

    let delta_nodes = collect_numeric_entries(
        nodes,
        first_available_field(
            &config,
            &["temperature_delta_field", "temperature_field"],
            &["temperature_delta"],
        ),
    );
    let displacement_peak = peak_vector_entry(
        nodes,
        field_name(&config, "displacement_x_field", "ux"),
        field_name(&config, "displacement_y_field", "uy"),
        optional_field_name(&config, "displacement_z_field"),
        Some(first_available_field(
            &config,
            &["displacement_magnitude_field"],
            &["displacement_magnitude"],
        )),
    );
    let explicit_stress_peak = if let Some(field) = config.get("stress_field").and_then(Value::as_str) {
        peak_scalar_entry(elements, field)
    } else {
        peak_scalar_entry_any(elements, &["von_mises_stress", "von_mises"])
    };
    let payload_stress_peak = object.get("max_stress").and_then(Value::as_f64);
    let component_stress_peak =
        peak_scalar_component_entry_any(elements, &["stress_x", "stress_y", "stress_z", "stress_xy"]);
    let thermal_strain_peak = resolve_thermo_strain_peak(elements, &config, "thermal_strain");
    let mechanical_strain_peak =
        resolve_thermo_strain_peak(elements, &config, "mechanical_strain");
    let total_strain_peak = resolve_thermo_strain_peak(elements, &config, "total_strain");

    let diagnostics = serde_json::json!({
        format!("{prefix}_node_count"): count_object_entries(nodes),
        format!("{prefix}_element_count"): count_object_entries(elements),
    });
    let diagnostics = merge_diagnostic_contract(
        diagnostics,
        "thermo_mechanical",
        "thermo_result",
        &prefix,
        &["temperature_delta", "displacement", "stress"],
    );
    let diagnostics =
        merge_distribution_stats(diagnostics, &prefix, "temperature_delta", &delta_nodes);
    let diagnostics = merge_peak_vector(
        diagnostics,
        &prefix,
        "displacement",
        displacement_peak.as_ref(),
    );
    let diagnostics = if let Some(peak) = explicit_stress_peak.as_ref() {
        merge_peak_scalar(diagnostics, &prefix, "stress", Some(peak))
    } else if let Some(value) = payload_stress_peak {
        merge_json_objects(
            diagnostics,
            serde_json::json!({
                format!("{prefix}_stress_peak"): value,
                format!("{prefix}_peak_stress"): value,
                format!("{prefix}_peak_stress_id"): "max_stress",
                format!("{prefix}_stress_peak_element_id"): "max_stress",
            }),
        )
    } else {
        merge_peak_scalar(diagnostics, &prefix, "stress", component_stress_peak.as_ref())
    };
    let diagnostics = merge_peak_scalar(
        diagnostics,
        &prefix,
        "thermal_strain",
        thermal_strain_peak.as_ref(),
    );
    let diagnostics = merge_peak_scalar(
        diagnostics,
        &prefix,
        "mechanical_strain",
        mechanical_strain_peak.as_ref(),
    );
    let diagnostics =
        merge_peak_scalar(diagnostics, &prefix, "total_strain", total_strain_peak.as_ref());

    ensure_non_empty_diagnostics(
        diagnostics,
        &prefix,
        "extract.thermo_result_diagnostics did not find any diagnostic fields",
    )
}

pub fn extract_electrostatic_result_diagnostics(
    payload: Value,
    config: Value,
) -> Result<Value, String> {
    let object = payload.as_object().ok_or_else(|| {
        "extract.electrostatic_result_diagnostics expects an object payload".to_string()
    })?;
    let nodes = fetch_collection(object, &config, "node_source", "nodes")?;
    let elements = fetch_collection(object, &config, "element_source", "elements")?;
    let prefix = normalize_prefix(&config, "output_prefix", "electrostatic");

    let potential_nodes =
        collect_numeric_entries(nodes, field_name(&config, "potential_field", "potential"));
    let charge_density_nodes = collect_numeric_entries(
        nodes,
        field_name(&config, "charge_density_field", "charge_density"),
    );
    let energy_density_peak = peak_scalar_entry(
        elements,
        field_name(&config, "energy_density_field", "energy_density"),
    );
    let field_peak = peak_vector_entry(
        elements,
        field_name(&config, "field_x_field", "electric_field_x"),
        field_name(&config, "field_y_field", "electric_field_y"),
        optional_field_name(&config, "field_z_field"),
        Some(field_name(
            &config,
            "field_magnitude_field",
            "electric_field_magnitude",
        )),
    );

    let diagnostics = serde_json::json!({
        format!("{prefix}_node_count"): count_object_entries(nodes),
        format!("{prefix}_element_count"): count_object_entries(elements),
    });
    let diagnostics = merge_diagnostic_contract(
        diagnostics,
        "electrostatic",
        "electrostatic_result",
        &prefix,
        &["potential", "charge_density", "energy_density", "field"],
    );
    let diagnostics = merge_distribution_stats(diagnostics, &prefix, "potential", &potential_nodes);
    let diagnostics = merge_distribution_stats(
        diagnostics,
        &prefix,
        "charge_density",
        &charge_density_nodes,
    );
    let diagnostics = merge_peak_scalar(
        diagnostics,
        &prefix,
        "energy_density",
        energy_density_peak.as_ref(),
    );
    let diagnostics = merge_peak_vector(diagnostics, &prefix, "field", field_peak.as_ref());

    ensure_non_empty_diagnostics(
        diagnostics,
        &prefix,
        "extract.electrostatic_result_diagnostics did not find any diagnostic fields",
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

fn first_available_field<'a>(
    config: &'a Value,
    config_keys: &[&str],
    defaults: &[&'a str],
) -> &'a str {
    config_keys
        .iter()
        .find_map(|key| config.get(*key).and_then(Value::as_str))
        .or_else(|| defaults.iter().copied().find(|value| !value.is_empty()))
        .expect("first_available_field requires at least one fallback")
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

fn peak_scalar_entry_any<'a>(entries: &'a [Value], fields: &[&str]) -> Option<(&'a Value, f64)> {
    fields
        .iter()
        .find_map(|field| peak_scalar_entry(entries, field))
}

fn peak_scalar_component_entry_any<'a>(
    entries: &'a [Value],
    fields: &[&str],
) -> Option<(&'a Value, f64)> {
    fields
        .iter()
        .filter_map(|field| peak_scalar_component_entry(entries, field))
        .max_by(|left, right| left.1.abs().total_cmp(&right.1.abs()))
}

fn peak_scalar_component_entry<'a>(entries: &'a [Value], field: &str) -> Option<(&'a Value, f64)> {
    collect_numeric_entries(entries, field)
        .into_iter()
        .max_by(|left, right| left.1.abs().total_cmp(&right.1.abs()))
}

fn resolve_thermo_strain_peak<'a>(
    entries: &'a [Value],
    config: &Value,
    base_field: &str,
) -> Option<(&'a Value, f64)> {
    let config_key = format!("{base_field}_field");
    if let Some(field) = config.get(&config_key).and_then(Value::as_str) {
        return peak_scalar_entry(entries, field);
    }

    peak_scalar_entry(entries, base_field).or_else(|| {
        let owned = ["x", "y", "z", "xy"]
            .into_iter()
            .map(|suffix| format!("{base_field}_{suffix}"))
            .collect::<Vec<_>>();
        let fields = owned.iter().map(String::as_str).collect::<Vec<_>>();
        peak_scalar_component_entry_any(entries, &fields)
    })
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

fn merge_diagnostic_contract(
    diagnostics: Value,
    domain: &str,
    subject: &str,
    prefix: &str,
    metric_groups: &[&str],
) -> Value {
    merge_json_objects(
        diagnostics,
        serde_json::json!({
            "diagnostic_contract": "kyuubiki.workflow_diagnostics/v1",
            "diagnostic_domain": domain,
            "diagnostic_subject": subject,
            "diagnostic_prefix": prefix,
            "diagnostic_metric_groups": metric_groups,
        }),
    )
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
    let mean = sum / values.len() as f64;
    merge_json_objects(
        diagnostics,
        serde_json::json!({
            format!("{prefix}_{label}_count"): values.len(),
            format!("{prefix}_{label}_min"): values.iter().copied().fold(f64::INFINITY, f64::min),
            format!("{prefix}_{label}_max"): values.iter().copied().fold(f64::NEG_INFINITY, f64::max),
            format!("{prefix}_{label}_mean"): mean,
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
            format!("{prefix}_peak_{label}_id"): entry_as_identifier(entry),
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

fn entry_as_identifier(entry: &Value) -> Value {
    entry.get("id").cloned().unwrap_or_else(|| {
        entry.as_str()
            .map(Value::from)
            .unwrap_or_else(|| Value::String("unknown".to_string()))
    })
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
