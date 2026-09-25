use crate::workflow_diagnostic_samples::{Field, VectorFields, extract_checked};
use serde_json::{Value, json};

pub fn extract_thermo_result_diagnostics(payload: Value, config: Value) -> Result<Value, String> {
    let mut result = extract_checked(
        payload,
        config,
        "thermo",
        &["temperature_delta", "displacement", "stress"],
        |diagnostics, config| {
            // Preserve the thermo contract's missing scalar identity, distinct from vector IDs.
            diagnostics.scalar_id_fallback(json!("unknown"));
            let temperature_key = if config.get("temperature_delta_field").is_some() {
                "temperature_delta_field"
            } else {
                "temperature_field"
            };
            diagnostics.node_distribution(
                "temperature_delta",
                Field::configured(config, temperature_key, &["temperature_delta"])?,
            )?;
            diagnostics.node_vector_peak(
                "displacement",
                VectorFields {
                    x: Field::configured(config, "displacement_x_field", &["ux"])?,
                    y: Field::configured(config, "displacement_y_field", &["uy"])?,
                    z: Field::configured(config, "displacement_z_field", &[])?,
                    magnitude: Field::configured(
                        config,
                        "displacement_magnitude_field",
                        &["displacement_magnitude"],
                    )?,
                },
            )?;
            if !diagnostics.element_scalar_peak(
                "stress",
                Field::configured(config, "stress_field", &["von_mises_stress", "von_mises"])?,
            )? && !diagnostics.payload_scalar_peak("stress", "max_stress")?
            {
                diagnostics.element_component_peak(
                    "stress",
                    &["stress_x", "stress_y", "stress_z", "stress_xy"],
                )?;
            }
            for label in ["thermal_strain", "mechanical_strain", "total_strain"] {
                if !diagnostics.element_scalar_peak(
                    label,
                    Field::configured(config, &format!("{label}_field"), &[label])?,
                )? {
                    let names = ["x", "y", "z", "xy"].map(|axis| format!("{label}_{axis}"));
                    diagnostics
                        .element_component_peak(label, &names.each_ref().map(String::as_str))?;
                }
            }
            Ok(())
        },
    )?;
    result["diagnostic_domain"] = json!("thermo_mechanical");
    Ok(result)
}
