use crate::workflow_diagnostic_samples::{Field, VectorFields, extract_checked};
use serde_json::Value;

pub fn extract_magnetostatic_result_diagnostics(
    payload: Value,
    config: Value,
) -> Result<Value, String> {
    extract_checked(
        payload,
        config,
        "magnetostatic",
        &[
            "vector_potential",
            "current_density",
            "energy_density",
            "field",
            "flux",
        ],
        |diagnostics, config| {
            diagnostics.node_distribution(
                "vector_potential",
                Field::configured(
                    config,
                    "vector_potential_field",
                    &["vector_potential", "a", "magnetic_vector_potential"],
                )?,
            )?;
            diagnostics.node_distribution(
                "current_density",
                Field::configured(
                    config,
                    "current_density_field",
                    &["current_density", "j", "current_density_sum"],
                )?,
            )?;
            if !diagnostics.element_scalar_peak(
                "energy_density",
                Field::configured(
                    config,
                    "energy_density_field",
                    &[
                        "energy_area_density",
                        "energy_density",
                        "magnetic_energy_density",
                    ],
                )?,
            )? {
                // Keep the legacy whole-group fallback without mixing total energy into density samples.
                diagnostics.element_scalar_peak(
                    "energy_density",
                    Field::configured(config, "energy_density_field", &["stored_energy"])?,
                )?;
            }
            diagnostics.element_vector_peak(
                "field",
                VectorFields {
                    x: Field::configured(
                        config,
                        "field_x_field",
                        &["magnetic_field_strength_x", "h_x", "field_x"],
                    )?,
                    y: Field::configured(
                        config,
                        "field_y_field",
                        &["magnetic_field_strength_y", "h_y", "field_y"],
                    )?,
                    z: Field::configured(config, "field_z_field", &[])?,
                    magnitude: Field::configured(
                        config,
                        "field_magnitude_field",
                        &["magnetic_field_strength_magnitude", "h_mag", "field_mag"],
                    )?,
                },
            )?;
            diagnostics.element_vector_peak(
                "flux",
                VectorFields {
                    x: Field::configured(
                        config,
                        "flux_x_field",
                        &["magnetic_flux_density_x", "b_x", "flux_x"],
                    )?,
                    y: Field::configured(
                        config,
                        "flux_y_field",
                        &["magnetic_flux_density_y", "b_y", "flux_y"],
                    )?,
                    z: Field::configured(config, "flux_z_field", &[])?,
                    magnitude: Field::configured(
                        config,
                        "flux_magnitude_field",
                        &["magnetic_flux_density_magnitude", "b_mag", "flux_mag"],
                    )?,
                },
            )
        },
    )
}
