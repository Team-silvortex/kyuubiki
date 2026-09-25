use crate::workflow_diagnostic_samples::{Field, VectorFields, extract_checked};
use serde_json::Value;

pub fn extract_electrostatic_result_diagnostics(
    payload: Value,
    config: Value,
) -> Result<Value, String> {
    extract_checked(
        payload,
        config,
        "electrostatic",
        &["potential", "charge_density", "energy_density", "field"],
        |diagnostics, config| {
            diagnostics.node_distribution(
                "potential",
                Field::configured(config, "potential_field", &["potential", "phi", "voltage"])?,
            )?;
            diagnostics.node_distribution(
                "charge_density",
                Field::configured(
                    config,
                    "charge_density_field",
                    &["charge_density", "rho_e", "q_density"],
                )?,
            )?;
            diagnostics.element_scalar_peak(
                "energy_density",
                Field::configured(
                    config,
                    "energy_density_field",
                    &[
                        "energy_density",
                        "electric_energy_density",
                        "stored_energy_density",
                    ],
                )?,
            )?;
            diagnostics.element_vector_peak(
                "field",
                VectorFields {
                    x: Field::configured(config, "field_x_field", &["electric_field_x", "e_x"])?,
                    y: Field::configured(config, "field_y_field", &["electric_field_y", "e_y"])?,
                    z: Field::configured(config, "field_z_field", &[])?,
                    magnitude: Field::configured(
                        config,
                        "field_magnitude_field",
                        &["electric_field_magnitude", "e_mag", "electric_field_abs"],
                    )?,
                },
            )
        },
    )
}
