use crate::workflow_diagnostic_samples::{Samples, insert_finite};
use serde_json::{Map, Value, json};

const FIELDS: [&str; 3] = [
    "artificial_diffusivity",
    "stabilization_flux",
    "numerical_flux",
];
const FLUX_IDENTITY_ROUNDOFF: f64 = 64.0 * f64::EPSILON;

pub(crate) fn extract_stabilization(
    object: &Map<String, Value>,
    elements: &Samples<'_>,
    prefix: &str,
    summary: &mut Map<String, Value>,
) -> Result<bool, String> {
    let scheme = declared_scheme(object)?;
    let mut peaks: [Option<(&Value, f64)>; 3] = [None; 3];
    let present = elements.scan(
        "stabilization",
        scheme == Some("upwind"),
        |row, source, index| {
            let Some(value) = row.get("stabilization") else {
                return Ok(None);
            };
            if scheme != Some("upwind") {
                return Err(
                    "payload.input.scheme must declare upwind for stabilization diagnostics".into(),
                );
            }
            let record = value
                .as_object()
                .ok_or_else(|| format!("payload.{source}[{index}].stabilization must be an object"))?;
            let read = |field| number(record, field, || format!("payload.{source}[{index}].stabilization.{field}"));
            let values = [
                read(FIELDS[0])?, read(FIELDS[1])?, read(FIELDS[2])?,
            ];
            if values[0] < 0.0 {
                return Err(format!("payload.{source}[{index}].stabilization.artificial_diffusivity must be nonnegative"));
            }
            let physical = number(row, "total_flux", || format!("payload.{source}[{index}].total_flux"))?;
            check_flux_identity(physical, values[1], values[2], || format!("payload.{source}[{index}].stabilization.numerical_flux"))?;
            Ok(Some(values))
        },
        |entry, values| {
            for (peak, value) in peaks.iter_mut().zip(values) {
                if peak.is_none_or(|(_, prior)| value.abs().total_cmp(&prior.abs()).is_ge()) {
                    *peak = Some((entry, value));
                }
            }
            Ok(())
        },
    )?;
    if let Some(scheme) = scheme {
        summary.insert("diagnostic_scheme".into(), json!(scheme));
    }
    if present {
        for (field, peak) in FIELDS.into_iter().zip(peaks) {
            let (entry, value) = peak.expect("complete stabilization group");
            let name = format!("{prefix}_{field}");
            insert_finite(summary, format!("{name}_peak"), value)?;
            insert_finite(summary, format!("{name}_peak_magnitude"), value.abs())?;
            if let Some(id) = entry.get("id") {
                summary.insert(format!("{name}_peak_id"), id.clone());
            }
        }
        summary
            .get_mut("diagnostic_metric_groups")
            .and_then(Value::as_array_mut)
            .expect("initialized metric groups")
            .push(json!("stabilization"));
    }
    Ok(present)
}

fn declared_scheme(object: &Map<String, Value>) -> Result<Option<&str>, String> {
    let Some(input) = object.get("input").and_then(Value::as_object) else {
        return Ok(None);
    };
    match input.get("scheme") {
        None => Ok(Some("galerkin")),
        Some(Value::String(scheme)) if matches!(scheme.as_str(), "galerkin" | "upwind") => {
            Ok(Some(scheme))
        }
        _ => Err("payload.input.scheme must be galerkin or upwind".into()),
    }
}

fn number(
    object: &Map<String, Value>,
    field: &str,
    path: impl FnOnce() -> String,
) -> Result<f64, String> {
    match object.get(field) {
        Some(value) => value
            .as_f64()
            .filter(|value| value.is_finite())
            .ok_or_else(|| format!("{} must be a finite number", path())),
        None => Err(format!("{} is missing a numeric sample", path())),
    }
}

fn check_flux_identity(
    physical: f64,
    stabilization: f64,
    numerical: f64,
    path: impl FnOnce() -> String,
) -> Result<(), String> {
    // Normalize before summing: the identity must not overflow, and tiny fluxes
    // must not inherit a unit-sized absolute tolerance.
    let scale = physical.abs().max(stabilization.abs()).max(numerical.abs());
    if scale > 0.0
        && (numerical / scale - (physical / scale + stabilization / scale)).abs()
            > FLUX_IDENTITY_ROUNDOFF
    {
        return Err(format!(
            "{} contradicts total_flux + stabilization_flux",
            path()
        ));
    }
    Ok(())
}
