use crate::workflow_diagnostic_samples::{
    Field, Samples, configured_name, insert_finite, missing_sample,
};
use crate::workflow_result_admission::require_converged_result;
use serde_json::{Map, Value, json};

pub fn extract_transport_result_diagnostics(
    payload: Value,
    config: Value,
) -> Result<Value, String> {
    const OPERATOR: &str = "extract.transport_result_diagnostics";
    let extract = || {
        let object = payload.as_object().ok_or("expects an object payload")?;
        require_converged_result(object, OPERATOR, "payload")?;
        if !config.is_null() && !config.is_object() {
            return Err("config must be an object or null".to_string());
        }
        let nodes = Samples::fixed(object, "nodes")?;
        let elements = Samples::fixed(object, "elements")?;
        let prefix = configured_name(&config, "output_prefix")?.unwrap_or("transport");
        let mut summary = Map::new();
        for (name, value) in [
            (
                "diagnostic_contract",
                json!("kyuubiki.workflow_diagnostics/v1"),
            ),
            ("diagnostic_domain", json!("transport")),
            ("diagnostic_subject", json!("advection_diffusion_result")),
            ("diagnostic_prefix", json!(prefix)),
            ("diagnostic_node_count", json!(nodes.len())),
            ("diagnostic_element_count", json!(elements.len())),
            (
                "diagnostic_metric_groups",
                json!(["concentration", "source", "flux", "peclet"]),
            ),
        ] {
            summary.insert(name.into(), value);
        }
        let mut measured_groups = usize::from(concentration(&mut summary, prefix, &nodes)?);
        measured_groups += usize::from(sources(&mut summary, prefix, &nodes)?);
        for (label, names, components) in [
            (
                "total_flux",
                &["total_flux", "flux", "flux_total"],
                Some(["flux_x", "flux_y"]),
            ),
            (
                "diffusive_flux",
                &["diffusive_flux", "diffusion_flux", "diffusive_flux_total"],
                Some(["diffusive_flux_x", "diffusive_flux_y"]),
            ),
            (
                "advective_flux",
                &["advective_flux", "advection_flux", "advective_flux_total"],
                Some(["advective_flux_x", "advective_flux_y"]),
            ),
            ("peclet", &["peclet_number", "peclet", "pe"], None),
        ] {
            measured_groups += usize::from(signed_peak(
                &mut summary,
                &format!("{prefix}_{label}"),
                &elements,
                names,
                components,
            )?);
        }
        if measured_groups == 0 {
            return Err("did not find any diagnostic fields".to_string());
        }
        Ok(Value::Object(summary))
    };
    extract().map_err(|error| format!("{OPERATOR}: {error}"))
}

fn concentration(
    summary: &mut Map<String, Value>,
    prefix: &str,
    nodes: &Samples<'_>,
) -> Result<bool, String> {
    let field = Field::aliases(&["concentration", "c", "species", "scalar"]);
    let name = format!("{prefix}_concentration");
    let (mut count, mut mean) = (0_usize, 0.0_f64);
    let (mut min, mut max) = (f64::INFINITY, f64::NEG_INFINITY);
    let present = nodes.scan(
        field.label(),
        false,
        |row, source, index| field.read(row, source, index),
        |_, value| {
            count += 1;
            min = min.min(value);
            max = max.max(value);
            if !(max - min).is_finite() {
                return Err(format!("{name}_span is non-finite"));
            }
            // No concentration sum is exported; avoid overflowing an unused intermediate sum.
            mean += (value - mean) / count as f64;
            Ok(())
        },
    )?;
    if present {
        for (suffix, value) in [
            ("min", min),
            ("max", max),
            ("span", max - min),
            ("mean", mean),
        ] {
            insert_finite(summary, format!("{name}_{suffix}"), value)?;
        }
    }
    Ok(present)
}

fn sources(
    summary: &mut Map<String, Value>,
    prefix: &str,
    nodes: &Samples<'_>,
) -> Result<bool, String> {
    let field = Field::aliases(&["source", "source_density", "net_source", "source_term"]);
    let name = format!("{prefix}_source");
    let (mut count, mut sum) = (0_usize, 0.0_f64);
    let present = nodes.scan(
        field.label(),
        false,
        |row, source, index| field.read(row, source, index),
        |_, value| {
            count += 1;
            sum += value;
            if !sum.is_finite() {
                return Err(format!("{name}_sum is non-finite"));
            }
            Ok(())
        },
    )?;
    if present {
        summary.insert(format!("{name}_count"), count.into());
        insert_finite(summary, format!("{name}_sum"), sum)?;
        insert_finite(summary, format!("{name}_mean"), sum / count as f64)?;
    }
    Ok(present)
}

fn signed_peak(
    summary: &mut Map<String, Value>,
    prefix: &str,
    elements: &Samples<'_>,
    names: &[&str],
    components: Option<[&str; 2]>,
) -> Result<bool, String> {
    let scalar = Field::aliases(names);
    let vector = components.map(|[x, y]| [Field::aliases(&[x]), Field::aliases(&[y])]);
    let mut peak: Option<(&Value, f64)> = None;
    let present = elements.scan(
        scalar.label(),
        false,
        |row, source, index| {
            if let Some(value) = scalar.read(row, source, index)? {
                return Ok(Some(value));
            }
            let Some([x, y]) = &vector else {
                return Ok(None);
            };
            let (vx, vy) = (x.read(row, source, index)?, y.read(row, source, index)?);
            if vx.is_none() && vy.is_none() {
                return Ok(None);
            }
            let vx = vx.ok_or_else(|| missing_sample(source, index, x.label()))?;
            let vy = vy.ok_or_else(|| missing_sample(source, index, y.label()))?;
            let magnitude = vx.hypot(vy);
            if !magnitude.is_finite() {
                return Err(format!(
                    "payload.{source}[{index}] vector magnitude is non-finite"
                ));
            }
            Ok(Some(magnitude))
        },
        |entry, value| {
            if peak.is_none_or(|(_, best)| !value.abs().total_cmp(&best.abs()).is_lt()) {
                peak = Some((entry, value));
            }
            Ok(())
        },
    )?;
    if let Some((entry, value)) = peak {
        insert_finite(summary, format!("{prefix}_peak"), value)?;
        insert_finite(summary, format!("{prefix}_peak_magnitude"), value.abs())?;
        if let Some(id) = entry.get("id") {
            summary.insert(format!("{prefix}_peak_id"), id.clone());
        }
    }
    Ok(present)
}
