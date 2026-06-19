use serde_json::Value;

type FocusSpec = (
    &'static str,
    &'static str,
    &'static [&'static str],
    &'static [&'static str],
);

const FOCUS_SPECS: &[FocusSpec] = &[
    (
        "electrostatic.potential_max",
        "electrostatic",
        &["electrostatic_potential_max"],
        &[],
    ),
    (
        "electrostatic.field_peak",
        "electrostatic",
        &[
            "electrostatic_peak_field",
            "electrostatic_field_peak_magnitude",
        ],
        &[
            "peak_element_id",
            "peak_average_potential",
            "peak_electric_field_x",
            "peak_electric_field_y",
            "peak_flux_density_x",
            "peak_flux_density_y",
            "peak_potential_gradient_magnitude",
        ],
    ),
    (
        "thermal.temperature_max",
        "thermal",
        &["thermal_temperature_max"],
        &[],
    ),
    (
        "thermal.flux_peak",
        "thermal",
        &["thermal_peak_flux", "thermal_flux_peak_magnitude"],
        &[
            "peak_element_id",
            "peak_average_temperature",
            "peak_heat_flux_x",
            "peak_heat_flux_y",
            "peak_temperature_gradient_x",
            "peak_temperature_gradient_y",
            "peak_temperature_gradient_magnitude",
        ],
    ),
    (
        "thermo.temperature_delta_max",
        "thermo",
        &["thermo_temperature_delta_max"],
        &[],
    ),
    (
        "thermo.displacement_peak",
        "thermo",
        &[
            "thermo_peak_displacement",
            "thermo_displacement_peak_magnitude",
        ],
        &[
            "peak_node_id",
            "peak_displacement_x",
            "peak_displacement_y",
            "peak_node_temperature_delta",
        ],
    ),
    (
        "thermo.stress_peak",
        "thermo",
        &["thermo_peak_stress", "thermo_stress_peak"],
        &[
            "peak_element_id",
            "peak_stress_x",
            "peak_stress_y",
            "peak_tau_xy",
            "peak_principal_stress_1",
            "peak_principal_stress_2",
            "peak_max_in_plane_shear",
            "peak_element_temperature_delta",
        ],
    ),
    (
        "thermo.thermal_strain_peak",
        "thermo",
        &["thermo_peak_thermal_strain", "thermo_thermal_strain_peak"],
        &[
            "thermo_peak_thermal_strain_id",
            "peak_element_id",
            "peak_thermal_strain",
            "peak_mechanical_strain_x",
            "peak_mechanical_strain_y",
            "peak_total_strain_x",
            "peak_total_strain_y",
            "peak_gamma_xy",
            "peak_element_temperature_delta",
        ],
    ),
];

pub(crate) fn report_focus_metrics(
    bundle: &serde_json::Map<String, Value>,
) -> serde_json::Map<String, Value> {
    let Some(payloads) = bundle.get("bundle_payloads").and_then(Value::as_object) else {
        return serde_json::Map::new();
    };
    FOCUS_SPECS
        .iter()
        .filter_map(|(metric_id, source, value_fields, _)| {
            let payload = payloads.get(*source)?.as_object()?;
            let value = value_fields
                .iter()
                .find_map(|field| payload.get(*field).cloned())?;
            Some(((*metric_id).to_string(), value))
        })
        .collect()
}

pub(crate) fn report_focus_context(
    bundle: &serde_json::Map<String, Value>,
) -> serde_json::Map<String, Value> {
    let Some(payloads) = bundle.get("bundle_payloads").and_then(Value::as_object) else {
        return serde_json::Map::new();
    };
    FOCUS_SPECS
        .iter()
        .filter_map(|(metric_id, source, value_fields, context_fields)| {
            let payload = payloads.get(*source)?.as_object()?;
            let value_field = value_fields
                .iter()
                .find(|field| payload.contains_key(**field))?;
            let mut context = serde_json::Map::from_iter([
                ("source".to_string(), Value::from(*source)),
                ("value_field".to_string(), Value::from(*value_field)),
            ]);
            for field in *context_fields {
                if let Some(value) = payload.get(*field) {
                    context.insert((*field).to_string(), value.clone());
                }
            }
            Some(((*metric_id).to_string(), Value::Object(context)))
        })
        .collect()
}

pub(crate) fn report_focus_payloads(
    bundle: &serde_json::Map<String, Value>,
) -> serde_json::Map<String, Value> {
    let metrics = report_focus_metrics(bundle);
    let contexts = report_focus_context(bundle);
    metrics
        .into_iter()
        .filter_map(|(metric_id, value)| {
            let context = contexts.get(&metric_id)?.as_object()?;
            let source = context.get("source").cloned().unwrap_or(Value::Null);
            let value_field = context.get("value_field").cloned().unwrap_or(Value::Null);
            Some((
                metric_id.clone(),
                serde_json::json!({
                    "focus_contract": "kyuubiki.workflow_focus_payload/v1",
                    "metric_id": metric_id,
                    "source": source,
                    "value": value,
                    "value_field": value_field,
                    "context": context,
                }),
            ))
        })
        .collect()
}
