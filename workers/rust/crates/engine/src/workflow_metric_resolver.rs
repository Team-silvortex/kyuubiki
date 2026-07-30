use serde_json::{Map, Value};

const DYNAMIC_AMPLITUDE_FIELDS: &[&str] = &[
    "max_displacement",
    "max_velocity",
    "max_acceleration",
    "max_force",
];

pub fn metric_value(object: &Map<String, Value>, field: &str) -> Option<f64> {
    object
        .get(field)
        .and_then(finite_number)
        .or_else(|| dynamic_alias_field(object, field))
        .or_else(|| domain_alias_field(object, field))
        .or_else(|| derived_dynamic_field(object, field))
        .or_else(|| derived_domain_field(object, field))
        .or_else(|| derived_span(object, field))
}

fn dynamic_alias_field(object: &Map<String, Value>, field: &str) -> Option<f64> {
    match field {
        "peak_frequency_hz" => first_alias_number(
            object,
            &[
                "response_peak_frequency_hz",
                "dominant_frequency_hz",
                "freq_peak_hz",
            ],
        ),
        "max_displacement" => first_alias_number(
            object,
            &["peak_displacement", "displacement_amplitude_peak", "u_max"],
        ),
        "max_velocity" => first_alias_number(
            object,
            &["peak_velocity", "velocity_amplitude_peak", "v_max"],
        ),
        "max_acceleration" => first_alias_number(
            object,
            &["peak_acceleration", "acceleration_amplitude_peak", "a_max"],
        ),
        "max_force" => first_alias_number(
            object,
            &["peak_force", "force_amplitude_peak", "dynamic_force_peak"],
        ),
        _ => None,
    }
}

fn domain_alias_field(object: &Map<String, Value>, field: &str) -> Option<f64> {
    match field {
        "max_displacement" => first_alias_number(
            object,
            &[
                "peak_displacement",
                "max_translation",
                "displacement_peak",
                "u_max",
            ],
        ),
        "max_stress" => first_alias_number(
            object,
            &[
                "peak_stress",
                "von_mises_peak",
                "max_von_mises_stress",
                "stress_peak",
            ],
        ),
        "mass" => first_alias_number(object, &["total_mass", "structure_mass", "model_mass"]),
        "stiffness_margin" => first_alias_number(
            object,
            &[
                "minimum_stiffness_margin",
                "min_stiffness_margin",
                "stability_margin",
            ],
        ),
        "thermal_temperature_max" => first_alias_number(
            object,
            &["max_temperature", "temperature_max", "peak_temperature"],
        ),
        "thermal_flux_peak_magnitude" => first_alias_number(
            object,
            &["max_heat_flux", "heat_flux_peak", "peak_heat_flux"],
        ),
        "thermo_temperature_delta_max" => first_alias_number(
            object,
            &[
                "max_temperature_delta",
                "temperature_delta_max",
                "peak_temperature_delta",
            ],
        ),
        "thermo_stress_peak" => first_alias_number(
            object,
            &["max_stress", "peak_stress", "thermal_stress_peak"],
        ),
        "thermal_total_energy" => first_alias_number(
            object,
            &[
                "total_thermal_energy",
                "thermal_energy_total",
                "total_heat_energy",
            ],
        ),
        "electrostatic_field_peak_magnitude" => first_alias_number(
            object,
            &[
                "max_electric_field",
                "peak_electric_field",
                "electric_field_peak",
            ],
        ),
        "electrostatic_peak_energy_density" => first_alias_number(
            object,
            &[
                "max_energy_density",
                "peak_energy_density",
                "electric_energy_density_peak",
            ],
        ),
        "electrostatic_flux_peak_magnitude" => {
            first_alias_number(object, &["max_flux_density", "peak_flux_density"])
        }
        "electrostatic_total_stored_energy" => first_alias_number(
            object,
            &[
                "total_stored_energy",
                "stored_energy_total",
                "electric_total_energy",
            ],
        ),
        "electrostatic_potential_span" => {
            first_alias_number(object, &["potential_span", "voltage_span", "max_potential"])
        }
        "magnetostatic_field_peak_magnitude" => first_alias_number(
            object,
            &[
                "max_magnetic_field_strength",
                "peak_magnetic_field_strength",
                "h_peak",
            ],
        ),
        "magnetostatic_flux_peak_magnitude" => {
            first_alias_number(object, &["max_flux_density", "peak_flux_density", "b_peak"])
        }
        "magnetostatic_energy_density_peak" => first_alias_number(
            object,
            &[
                "max_energy_density",
                "peak_energy_density",
                "magnetic_energy_density_peak",
            ],
        ),
        "magnetostatic_current_density_sum" => first_alias_number(
            object,
            &[
                "total_current_density",
                "current_density_total",
                "sum_current_density",
            ],
        ),
        "magnetostatic_total_stored_energy" => first_alias_number(
            object,
            &[
                "total_stored_energy",
                "stored_energy_total",
                "magnetic_total_energy",
            ],
        ),
        "max_sound_pressure_level_db" => first_alias_number(
            object,
            &["peak_spl_db", "spl_max_db", "sound_pressure_level_max_db"],
        ),
        "max_acoustic_intensity" => first_alias_number(
            object,
            &[
                "peak_acoustic_intensity",
                "acoustic_intensity_peak",
                "intensity_max",
            ],
        ),
        "max_pressure_amplitude" => first_alias_number(
            object,
            &["max_pressure", "peak_pressure", "pressure_amplitude_peak"],
        ),
        "total_damping_loss" => first_alias_number(
            object,
            &[
                "damping_loss_total",
                "total_acoustic_damping_loss",
                "damping_energy_loss",
            ],
        ),
        "min_frequency_hz" => first_alias_number(
            object,
            &[
                "first_frequency_hz",
                "natural_frequency_min_hz",
                "mode_1_frequency_hz",
            ],
        ),
        "max_frequency_hz" => first_alias_number(
            object,
            &[
                "last_frequency_hz",
                "natural_frequency_max_hz",
                "modal_frequency_max_hz",
            ],
        ),
        "total_mass" => first_alias_number(
            object,
            &["modal_mass_total", "participating_mass_total", "mass_total"],
        ),
        "frequency_span_hz" => first_alias_number(
            object,
            &["modal_frequency_span_hz", "natural_frequency_span_hz"],
        ),
        "mode_1_participation_norm" => first_alias_number(
            object,
            &[
                "first_mode_participation_norm",
                "mode1_participation_norm",
                "primary_mode_participation_norm",
            ],
        ),
        "cfd_divergence_error_peak" => first_alias_number(
            object,
            &["max_divergence_error", "divergence_peak", "div_u_peak"],
        ),
        "cfd_reynolds_number_peak" => {
            first_alias_number(object, &["max_reynolds_number", "reynolds_peak", "re_peak"])
        }
        "cfd_viscous_dissipation_total" => first_alias_number(
            object,
            &[
                "total_viscous_dissipation",
                "viscous_dissipation_sum",
                "dissipation_total",
            ],
        ),
        "cfd_velocity_span" => first_alias_number(object, &["velocity_span", "speed_span"]),
        "cfd_pressure_span" => first_alias_number(object, &["pressure_span", "p_span"]),
        "velocity_magnitude" => {
            first_alias_number(object, &["speed", "u_mag"]).or_else(|| vector_magnitude(object))
        }
        "pressure" => first_alias_number(object, &["p", "static_pressure"]),
        "divergence_error" => first_alias_number(object, &["div_u", "divergence"]),
        "reynolds_number" => first_alias_number(object, &["reynolds", "re"]),
        "viscous_dissipation" => first_alias_number(object, &["dissipation", "nu_dissipation"]),
        "transport_total_flux_peak_magnitude" => first_alias_number(
            object,
            &[
                "max_transport_flux",
                "peak_transport_flux",
                "transport_flux_peak",
            ],
        ),
        "transport_peclet_peak" => {
            first_alias_number(object, &["max_peclet", "peclet_max", "peak_peclet"])
        }
        "transport_concentration_span" => first_alias_number(
            object,
            &["concentration_span", "concentration_range", "species_span"],
        ),
        "transport_source_sum" => first_alias_number(
            object,
            &["net_source", "source_balance", "total_source", "source_sum"],
        ),
        _ => None,
    }
}

fn derived_dynamic_field(object: &Map<String, Value>, field: &str) -> Option<f64> {
    match field {
        "peak_frequency_hz" => object
            .get("frequencies")
            .and_then(Value::as_array)
            .and_then(|frequencies| peak_frequency(frequencies)),
        field if DYNAMIC_AMPLITUDE_FIELDS.contains(&field) => object
            .get("frequencies")
            .and_then(Value::as_array)
            .and_then(|frequencies| max_frequency_field(frequencies, field))
            .or_else(|| max_transient_node_field(object, field)),
        _ => None,
    }
}

fn derived_domain_field(object: &Map<String, Value>, field: &str) -> Option<f64> {
    match field {
        "thermo_temperature_delta_max" => bounds_delta(
            object,
            &["temperature_max", "max_temperature"],
            &["temperature_min", "min_temperature"],
        ),
        "electrostatic_potential_span" => bounds_delta(
            object,
            &["potential_max", "max_voltage", "voltage_max"],
            &["potential_min", "min_voltage", "voltage_min"],
        ),
        "transport_concentration_span" => bounds_delta(
            object,
            &["concentration_max", "max_concentration"],
            &["concentration_min", "min_concentration"],
        ),
        "min_frequency_hz" => mode_frequency_bounds(object).map(|(min, _)| min),
        "max_frequency_hz" => mode_frequency_bounds(object).map(|(_, max)| max),
        "frequency_span_hz" => {
            let min = metric_value(object, "min_frequency_hz")?;
            let max = metric_value(object, "max_frequency_hz")?;
            Some((max - min).abs())
        }
        "mode_1_participation_norm" => object
            .get("modes")?
            .as_array()?
            .first()?
            .as_object()?
            .get("participation_norm")
            .and_then(finite_number),
        _ => None,
    }
}

fn peak_frequency(frequencies: &[Value]) -> Option<f64> {
    frequencies
        .iter()
        .filter_map(Value::as_object)
        .filter_map(|entry| {
            Some((
                frequency_entry_number(entry, "frequency_hz")?,
                frequency_entry_number(entry, "max_displacement")?,
            ))
        })
        .max_by(|left, right| left.1.total_cmp(&right.1))
        .map(|(frequency, _)| frequency)
}

fn max_frequency_field(frequencies: &[Value], field: &str) -> Option<f64> {
    frequencies
        .iter()
        .filter_map(Value::as_object)
        .filter_map(|entry| frequency_entry_number(entry, field))
        .max_by(f64::total_cmp)
}

fn frequency_entry_number(entry: &Map<String, Value>, field: &str) -> Option<f64> {
    entry
        .get(field)
        .and_then(finite_number)
        .or_else(|| match field {
            "frequency_hz" => {
                first_alias_number(entry, &["frequency", "freq_hz", "response_frequency_hz"])
            }
            "max_displacement" => first_alias_number(
                entry,
                &["peak_displacement", "displacement_amplitude", "u_peak"],
            ),
            "max_velocity" => {
                first_alias_number(entry, &["peak_velocity", "velocity_amplitude", "v_peak"])
            }
            "max_acceleration" => first_alias_number(
                entry,
                &["peak_acceleration", "acceleration_amplitude", "a_peak"],
            ),
            "max_force" => first_alias_number(
                entry,
                &["peak_force", "force_amplitude", "dynamic_force_peak"],
            ),
            _ => None,
        })
}

fn max_transient_node_field(object: &Map<String, Value>, field: &str) -> Option<f64> {
    let node_field = match field {
        "max_displacement" => "ux",
        "max_velocity" => "vx",
        "max_acceleration" => "ax",
        _ => return None,
    };
    object
        .get("nodes")?
        .as_array()?
        .iter()
        .filter_map(Value::as_object)
        .filter_map(|node| node.get(node_field).and_then(finite_number))
        .map(f64::abs)
        .max_by(f64::total_cmp)
}

fn mode_frequency_bounds(object: &Map<String, Value>) -> Option<(f64, f64)> {
    let mut frequencies = object
        .get("modes")?
        .as_array()?
        .iter()
        .filter_map(Value::as_object)
        .filter_map(|mode| mode.get("frequency_hz").and_then(finite_number));
    let first = frequencies.next()?;
    Some(frequencies.fold((first, first), |(min, max), frequency| {
        (min.min(frequency), max.max(frequency))
    }))
}

fn bounds_delta(
    object: &Map<String, Value>,
    max_aliases: &[&str],
    min_aliases: &[&str],
) -> Option<f64> {
    let max = first_alias_number(object, max_aliases)?;
    let min = first_alias_number(object, min_aliases)?;
    Some((max - min).abs())
}

fn derived_span(object: &Map<String, Value>, field: &str) -> Option<f64> {
    let prefix = field.strip_suffix("_span")?;
    let min = object
        .get(&format!("{prefix}_min"))
        .and_then(finite_number)?;
    let max = object
        .get(&format!("{prefix}_max"))
        .and_then(finite_number)?;
    Some((max - min).abs())
}

fn vector_magnitude(object: &Map<String, Value>) -> Option<f64> {
    let x = object.get("vx").and_then(finite_number)?;
    let y = object.get("vy").and_then(finite_number)?;
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

#[cfg(test)]
mod tests {
    use super::metric_value;
    use serde_json::{Value, json};

    fn resolved(payload: Value, field: &str) -> Option<f64> {
        metric_value(payload.as_object().expect("test payload object"), field)
    }

    #[test]
    fn direct_metric_value_takes_precedence_over_aliases() {
        let payload = json!({
            "max_stress": 4.0,
            "peak_stress": 8.0,
            "max_displacement": 0.25,
            "peak_displacement": 0.75
        });

        assert_eq!(resolved(payload.clone(), "max_stress"), Some(4.0));
        assert_eq!(resolved(payload, "max_displacement"), Some(0.25));
    }

    #[test]
    fn resolves_dynamic_frequency_and_transient_summaries() {
        let harmonic_payload = json!({
            "frequencies": [
                { "frequency": 10.0, "displacement_amplitude": 0.2, "velocity_amplitude": 1.0 },
                { "freq_hz": 22.0, "u_peak": 0.8, "v_peak": 1.7 },
                { "response_frequency_hz": 35.0, "peak_displacement": 0.3, "peak_velocity": 1.2 }
            ]
        });
        let transient_payload = json!({
            "nodes": [
                { "ux": -0.1, "vx": 0.5, "ax": 1.0 },
                { "ux": 0.4, "vx": -1.5, "ax": -3.0 }
            ]
        });

        assert_eq!(
            resolved(harmonic_payload.clone(), "peak_frequency_hz"),
            Some(22.0)
        );
        assert_eq!(resolved(harmonic_payload, "max_velocity"), Some(1.7));
        assert_eq!(
            resolved(transient_payload.clone(), "max_displacement"),
            Some(0.4)
        );
        assert_eq!(resolved(transient_payload, "max_acceleration"), Some(3.0));
    }

    #[test]
    fn resolves_domain_aliases_and_bounds_derived_spans() {
        let payload = json!({
            "temperature_max": 320.0,
            "temperature_min": 280.0,
            "voltage_max": 12.0,
            "voltage_min": -3.0,
            "concentration_max": 0.9,
            "concentration_min": 0.2,
            "total_thermal_energy": 42.0,
            "peak_flux_density": 6.0,
            "stored_energy_total": 1.5,
            "current_density_total": 2.5
        });

        assert_eq!(
            resolved(payload.clone(), "thermo_temperature_delta_max"),
            Some(40.0)
        );
        assert_eq!(
            resolved(payload.clone(), "electrostatic_potential_span"),
            Some(15.0)
        );
        assert_eq!(
            resolved(payload.clone(), "transport_concentration_span"),
            Some(0.7)
        );
        assert_eq!(
            resolved(payload.clone(), "thermal_total_energy"),
            Some(42.0)
        );
        assert_eq!(
            resolved(payload.clone(), "electrostatic_flux_peak_magnitude"),
            Some(6.0)
        );
        assert_eq!(
            resolved(payload.clone(), "magnetostatic_total_stored_energy"),
            Some(1.5)
        );
        assert_eq!(
            resolved(payload, "magnetostatic_current_density_sum"),
            Some(2.5)
        );
    }

    #[test]
    fn resolves_modal_modes_cfd_values_and_generic_spans() {
        let modal_payload = json!({
            "modes": [
                { "frequency_hz": 12.0, "participation_norm": 1.25 },
                { "frequency_hz": 48.0, "participation_norm": 0.5 }
            ]
        });
        let cfd_payload = json!({
            "vx": 3.0,
            "vy": 4.0,
            "p": 9.0,
            "div_u": 0.04,
            "re": 120.0,
            "nu_dissipation": 0.75,
            "cfd_pressure_min": 2.0,
            "cfd_pressure_max": 11.0
        });

        assert_eq!(
            resolved(modal_payload.clone(), "min_frequency_hz"),
            Some(12.0)
        );
        assert_eq!(
            resolved(modal_payload.clone(), "max_frequency_hz"),
            Some(48.0)
        );
        assert_eq!(
            resolved(modal_payload.clone(), "frequency_span_hz"),
            Some(36.0)
        );
        assert_eq!(
            resolved(modal_payload, "mode_1_participation_norm"),
            Some(1.25)
        );
        assert_eq!(
            resolved(cfd_payload.clone(), "velocity_magnitude"),
            Some(5.0)
        );
        assert_eq!(resolved(cfd_payload.clone(), "pressure"), Some(9.0));
        assert_eq!(
            resolved(cfd_payload.clone(), "divergence_error"),
            Some(0.04)
        );
        assert_eq!(
            resolved(cfd_payload.clone(), "reynolds_number"),
            Some(120.0)
        );
        assert_eq!(
            resolved(cfd_payload.clone(), "viscous_dissipation"),
            Some(0.75)
        );
        assert_eq!(resolved(cfd_payload, "cfd_pressure_span"), Some(9.0));
    }
}
