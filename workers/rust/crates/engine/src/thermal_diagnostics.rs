use crate::workflow_diagnostic_samples::{Field, VectorFields, extract_checked};
use serde_json::Value;

pub fn extract_thermal_result_diagnostics(payload: Value, config: Value) -> Result<Value, String> {
    extract_checked(
        payload,
        config,
        "thermal",
        &["temperature", "heat_load", "gradient", "flux"],
        |diagnostics, config| {
            diagnostics.node_distribution(
                "temperature",
                Field::configured(config, "temperature_field", &["temperature"])?,
            )?;
            diagnostics.node_distribution(
                "heat_load",
                Field::configured(config, "heat_load_field", &["heat_load"])?,
            )?;
            diagnostics.element_vector_peak(
                "gradient",
                VectorFields {
                    x: Field::configured(config, "gradient_x_field", &["temperature_gradient_x"])?,
                    y: Field::configured(config, "gradient_y_field", &["temperature_gradient_y"])?,
                    z: Field::configured(config, "gradient_z_field", &[])?,
                    magnitude: Field::configured(config, "gradient_magnitude_field", &[])?,
                },
            )?;
            diagnostics.element_vector_peak(
                "flux",
                VectorFields {
                    x: Field::configured(config, "flux_x_field", &["heat_flux_x"])?,
                    y: Field::configured(config, "flux_y_field", &["heat_flux_y"])?,
                    z: Field::configured(config, "flux_z_field", &[])?,
                    magnitude: Field::configured(
                        config,
                        "flux_magnitude_field",
                        &["heat_flux_magnitude"],
                    )?,
                },
            )
        },
    )
}
