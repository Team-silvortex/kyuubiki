pub(super) fn metric_aliases(field: &str) -> &'static [&'static str] {
    match field {
        "peak_frequency_hz" => &[
            "response_peak_frequency_hz",
            "dominant_frequency_hz",
            "freq_peak_hz",
        ],
        "max_displacement" => &[
            "peak_displacement",
            "displacement_amplitude_peak",
            "u_max",
            "max_translation",
            "displacement_peak",
        ],
        "max_velocity" => &["peak_velocity", "velocity_amplitude_peak", "v_max"],
        "max_acceleration" => &["peak_acceleration", "acceleration_amplitude_peak", "a_max"],
        "max_force" => &["peak_force", "force_amplitude_peak", "dynamic_force_peak"],
        "max_stress" => &[
            "peak_stress",
            "von_mises_peak",
            "max_von_mises_stress",
            "stress_peak",
        ],
        "mass" => &["total_mass", "structure_mass", "model_mass"],
        "stiffness_margin" => &[
            "minimum_stiffness_margin",
            "min_stiffness_margin",
            "stability_margin",
        ],
        "thermal_temperature_max" => &["max_temperature", "temperature_max", "peak_temperature"],
        "thermal_flux_peak_magnitude" => &["max_heat_flux", "heat_flux_peak", "peak_heat_flux"],
        "thermo_temperature_delta_max" => &[
            "max_temperature_delta",
            "temperature_delta_max",
            "peak_temperature_delta",
        ],
        "thermo_stress_peak" => &["max_stress", "peak_stress", "thermal_stress_peak"],
        "thermal_total_energy" => &[
            "total_thermal_energy",
            "thermal_energy_total",
            "total_heat_energy",
        ],
        "electrostatic_field_peak_magnitude" => &[
            "max_electric_field",
            "peak_electric_field",
            "electric_field_peak",
        ],
        "electrostatic_peak_energy_density" => &[
            "max_energy_density",
            "peak_energy_density",
            "electric_energy_density_peak",
        ],
        "electrostatic_flux_peak_magnitude" => &["max_flux_density", "peak_flux_density"],
        "electrostatic_total_stored_energy" => &[
            "total_stored_energy",
            "stored_energy_total",
            "electric_total_energy",
        ],
        "electrostatic_potential_span" => &["potential_span", "voltage_span", "max_potential"],
        "magnetostatic_field_peak_magnitude" => &[
            "max_magnetic_field_strength",
            "peak_magnetic_field_strength",
            "h_peak",
        ],
        "magnetostatic_flux_peak_magnitude" => &["max_flux_density", "peak_flux_density", "b_peak"],
        "magnetostatic_energy_density_peak" => &[
            "max_energy_density",
            "peak_energy_density",
            "magnetic_energy_density_peak",
        ],
        "magnetostatic_current_density_sum" => &[
            "total_current_density",
            "current_density_total",
            "sum_current_density",
        ],
        "magnetostatic_total_stored_energy" => &[
            "total_stored_energy",
            "stored_energy_total",
            "magnetic_total_energy",
        ],
        "max_sound_pressure_level_db" => {
            &["peak_spl_db", "spl_max_db", "sound_pressure_level_max_db"]
        }
        "max_acoustic_intensity" => &[
            "peak_acoustic_intensity",
            "acoustic_intensity_peak",
            "intensity_max",
        ],
        "max_pressure_amplitude" => &["max_pressure", "peak_pressure", "pressure_amplitude_peak"],
        "total_damping_loss" => &[
            "damping_loss_total",
            "total_acoustic_damping_loss",
            "damping_energy_loss",
        ],
        "min_frequency_hz" => &[
            "first_frequency_hz",
            "natural_frequency_min_hz",
            "mode_1_frequency_hz",
        ],
        "max_frequency_hz" => &[
            "last_frequency_hz",
            "natural_frequency_max_hz",
            "modal_frequency_max_hz",
        ],
        "total_mass" => &["modal_mass_total", "participating_mass_total", "mass_total"],
        "frequency_span_hz" => &["modal_frequency_span_hz", "natural_frequency_span_hz"],
        "mode_1_participation_norm" => &[
            "first_mode_participation_norm",
            "mode1_participation_norm",
            "primary_mode_participation_norm",
        ],
        "cfd_divergence_error_peak" => &["max_divergence_error", "divergence_peak", "div_u_peak"],
        "cfd_reynolds_number_peak" => &["max_reynolds_number", "reynolds_peak", "re_peak"],
        "cfd_viscous_dissipation_total" => &[
            "total_viscous_dissipation",
            "viscous_dissipation_sum",
            "dissipation_total",
        ],
        "cfd_velocity_span" => &["velocity_span", "speed_span"],
        "cfd_pressure_span" => &["pressure_span", "p_span"],
        "velocity_magnitude" => &["speed", "u_mag"],
        "pressure" => &["p", "static_pressure"],
        "divergence_error" => &["div_u", "divergence"],
        "reynolds_number" => &["reynolds", "re"],
        "viscous_dissipation" => &["dissipation", "nu_dissipation"],
        "transport_total_flux_peak_magnitude" => &[
            "max_transport_flux",
            "peak_transport_flux",
            "transport_flux_peak",
        ],
        "transport_peclet_peak" => &["max_peclet", "peclet_max", "peak_peclet"],
        "transport_concentration_span" => {
            &["concentration_span", "concentration_range", "species_span"]
        }
        "transport_source_sum" => &["net_source", "source_balance", "total_source", "source_sum"],
        _ => &[],
    }
}

pub(super) fn frequency_aliases(field: &str) -> &'static [&'static str] {
    match field {
        "frequency_hz" => &["frequency", "freq_hz", "response_frequency_hz"],
        "max_displacement" => &["peak_displacement", "displacement_amplitude", "u_peak"],
        "max_velocity" => &["peak_velocity", "velocity_amplitude", "v_peak"],
        "max_acceleration" => &["peak_acceleration", "acceleration_amplitude", "a_peak"],
        "max_force" => &["peak_force", "force_amplitude", "dynamic_force_peak"],
        _ => &[],
    }
}
