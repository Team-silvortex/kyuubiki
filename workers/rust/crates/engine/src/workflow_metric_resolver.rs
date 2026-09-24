mod aliases;

use aliases::{frequency_aliases, metric_aliases};
use serde_json::{Map, Value};

// Best-effort metadata only. Decisions and reductions must use the checked API.
pub fn display_metric_value(object: &Map<String, Value>, field: &str) -> Option<f64> {
    checked_metric_value(object, field).ok().flatten()
}

pub(crate) fn checked_metric_value(
    object: &Map<String, Value>,
    field: &str,
) -> Result<Option<f64>, String> {
    if let Some(value) = object.get(field) {
        return number(value, field).map(Some);
    }
    if let Some(value) = first_alias_number(object, metric_aliases(field))? {
        return Ok(Some(value));
    }
    match field {
        "peak_frequency_hz" => match collection(object, "frequencies")? {
            Some(samples) => peak_frequency(samples),
            None => Ok(None),
        },
        "max_displacement" | "max_velocity" | "max_acceleration" | "max_force" => {
            dynamic_extremum(object, field)
        }
        "thermo_temperature_delta_max" => bounds_delta(
            object,
            &["temperature_max", "max_temperature"],
            &["temperature_min", "min_temperature"],
            field,
        ),
        "electrostatic_potential_span" => bounds_delta(
            object,
            &["potential_max", "max_voltage", "voltage_max"],
            &["potential_min", "min_voltage", "voltage_min"],
            field,
        ),
        "transport_concentration_span" => bounds_delta(
            object,
            &["concentration_max", "max_concentration"],
            &["concentration_min", "min_concentration"],
            field,
        ),
        "min_frequency_hz" | "max_frequency_hz" => {
            let Some(modes) = collection(object, "modes")? else {
                return Ok(None);
            };
            let bounds =
                sample_bounds(modes, "modes", |mode| required_number(mode, "frequency_hz"))?;
            Ok(bounds.map(|(min, max)| {
                if field == "min_frequency_hz" {
                    min
                } else {
                    max
                }
            }))
        }
        "frequency_span_hz" => {
            let min = checked_metric_value(object, "min_frequency_hz")?;
            let max = checked_metric_value(object, "max_frequency_hz")?;
            difference(min, max, field)
        }
        "mode_1_participation_norm" => {
            let Some(modes) = collection(object, "modes")? else {
                return Ok(None);
            };
            let Some(first) = modes.first() else {
                return Ok(None);
            };
            let first = row_object(first, "modes", 0)?;
            required_number(first, "participation_norm")
                .map(Some)
                .map_err(|error| format!("modes[0].{error}"))
        }
        "velocity_magnitude" => {
            let x = first_alias_number(object, &["vx"])?;
            let y = first_alias_number(object, &["vy"])?;
            x.zip(y).map(|(x, y)| finite(x.hypot(y), field)).transpose()
        }
        _ => derived_span(object, field),
    }
}

fn dynamic_extremum(object: &Map<String, Value>, field: &str) -> Result<Option<f64>, String> {
    if let Some(samples) = collection(object, "frequencies")? {
        return sample_bounds(samples, "frequencies", |entry| {
            frequency_number(entry, field)
        })
        .map(|bounds| bounds.map(|(_, max)| max));
    }
    let component = match field {
        "max_displacement" => "ux",
        "max_velocity" => "vx",
        "max_acceleration" => "ax",
        _ => return Ok(None),
    };
    let Some(nodes) = collection(object, "nodes")? else {
        return Ok(None);
    };
    sample_bounds(nodes, "nodes", |node| {
        required_number(node, component).map(f64::abs)
    })
    .map(|bounds| bounds.map(|(_, max)| max))
}

fn peak_frequency(samples: &[Value]) -> Result<Option<f64>, String> {
    let mut peak: Option<(f64, f64)> = None;
    for (index, entry) in samples.iter().enumerate() {
        let entry = row_object(entry, "frequencies", index)?;
        let frequency = frequency_number(entry, "frequency_hz")
            .map_err(|error| format!("frequencies[{index}].{error}"))?;
        let amplitude = frequency_number(entry, "max_displacement")
            .map_err(|error| format!("frequencies[{index}].{error}"))?;
        if peak.is_none_or(|(_, prior)| amplitude.total_cmp(&prior).is_ge()) {
            peak = Some((frequency, amplitude));
        }
    }
    Ok(peak.map(|(frequency, _)| frequency))
}

fn frequency_number(entry: &Map<String, Value>, field: &str) -> Result<f64, String> {
    if let Some(value) = entry.get(field) {
        return number(value, field);
    }
    first_alias_number(entry, frequency_aliases(field))?
        .ok_or_else(|| format!("{field} is missing a numeric sample"))
}

fn sample_bounds(
    samples: &[Value],
    source: &str,
    read: impl Fn(&Map<String, Value>) -> Result<f64, String>,
) -> Result<Option<(f64, f64)>, String> {
    let mut bounds: Option<(f64, f64)> = None;
    for (index, row) in samples.iter().enumerate() {
        let row = row_object(row, source, index)?;
        let value = read(row).map_err(|error| format!("{source}[{index}].{error}"))?;
        bounds = Some(match bounds {
            Some((min, max)) => (min.min(value), max.max(value)),
            None => (value, value),
        });
    }
    Ok(bounds)
}

fn collection<'a>(
    object: &'a Map<String, Value>,
    field: &str,
) -> Result<Option<&'a [Value]>, String> {
    object
        .get(field)
        .map(|value| {
            value
                .as_array()
                .map(Vec::as_slice)
                .ok_or_else(|| format!("{field} must be an array"))
        })
        .transpose()
}

fn row_object<'a>(
    value: &'a Value,
    source: &str,
    index: usize,
) -> Result<&'a Map<String, Value>, String> {
    value
        .as_object()
        .ok_or_else(|| format!("{source}[{index}] must be an object"))
}

fn required_number(object: &Map<String, Value>, field: &str) -> Result<f64, String> {
    let value = object
        .get(field)
        .ok_or_else(|| format!("{field} is missing a numeric sample"))?;
    number(value, field)
}

fn first_alias_number(
    object: &Map<String, Value>,
    aliases: &[&str],
) -> Result<Option<f64>, String> {
    for alias in aliases {
        if let Some(value) = object.get(*alias) {
            return number(value, alias).map(Some);
        }
    }
    Ok(None)
}

fn number(value: &Value, field: &str) -> Result<f64, String> {
    let value = value
        .as_f64()
        .ok_or_else(|| format!("{field} must be a finite number"))?;
    finite(value, field)
}

fn finite(value: f64, field: &str) -> Result<f64, String> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(format!(
            "{field} must be finite; derived value is non-finite"
        ))
    }
}

fn bounds_delta(
    object: &Map<String, Value>,
    max_aliases: &[&str],
    min_aliases: &[&str],
    field: &str,
) -> Result<Option<f64>, String> {
    let max = first_alias_number(object, max_aliases)?;
    let min = first_alias_number(object, min_aliases)?;
    difference(min, max, field)
}

fn derived_span(object: &Map<String, Value>, field: &str) -> Result<Option<f64>, String> {
    let Some(prefix) = field.strip_suffix("_span") else {
        return Ok(None);
    };
    let min = first_alias_number(object, &[&format!("{prefix}_min")])?;
    let max = first_alias_number(object, &[&format!("{prefix}_max")])?;
    difference(min, max, field)
}

fn difference(min: Option<f64>, max: Option<f64>, field: &str) -> Result<Option<f64>, String> {
    min.zip(max)
        .map(|(min, max)| finite((max - min).abs(), field))
        .transpose()
}

#[cfg(test)]
mod tests {
    use super::display_metric_value;
    use serde_json::{Value, json};

    fn resolved(payload: Value, field: &str) -> Option<f64> {
        display_metric_value(payload.as_object().expect("test payload object"), field)
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
