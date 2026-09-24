use crate::workflow_domain_quality::{QualityGoal, QualityScore, QualityTerm, score_quality_terms};
use crate::workflow_metric_resolver::{checked_metric_value, display_metric_value};
use crate::workflow_result_admission::require_converged_result;
use serde_json::{Map, Value};

pub fn extract_stokes_flow_result_diagnostics(
    payload: Value,
    config: Value,
) -> Result<Value, String> {
    let operator = "extract.stokes_flow_result_diagnostics";
    let object = payload
        .as_object()
        .ok_or_else(|| format!("{operator} expects an object payload"))?;
    require_converged_result(object, operator, "payload")?;
    extract_diagnostics(object, &config).map_err(|error| format!("{operator}: {error}"))
}

fn extract_diagnostics(object: &Map<String, Value>, config: &Value) -> Result<Value, String> {
    let nodes = samples(object, "nodes")?;
    let elements = samples(object, "elements")?;
    let prefix = config
        .get("output_prefix")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("cfd");
    let mut velocity = MetricStats::default();
    let mut pressure = MetricStats::default();
    for (index, entry) in nodes.iter().enumerate() {
        let row = sample_object(entry, "nodes", index)?;
        velocity.observe(
            entry,
            sample_metric(row, "velocity_magnitude", "nodes", index)?,
            index + 1,
        );
        pressure.observe(
            entry,
            sample_metric(row, "pressure", "nodes", index)?,
            index + 1,
        );
    }

    let mut divergence = MetricStats::default();
    let mut reynolds = MetricStats::default();
    let mut dissipation = MetricStats::default();
    let total_field = format!("{prefix}_viscous_dissipation_total");
    let mut total = 0.0;
    for (index, entry) in elements.iter().enumerate() {
        let row = sample_object(entry, "elements", index)?;
        divergence.observe(
            entry,
            sample_metric(row, "divergence_error", "elements", index)?,
            index + 1,
        );
        reynolds.observe(
            entry,
            sample_metric(row, "reynolds_number", "elements", index)?,
            index + 1,
        );
        let value = sample_metric(row, "viscous_dissipation", "elements", index)?;
        dissipation.observe(entry, value, index + 1);
        total += value;
        if !total.is_finite() {
            return Err(format!(
                "{total_field} became non-finite at payload.elements[{index}]"
            ));
        }
    }

    let mut summary = Map::new();
    summary.insert(
        "diagnostic_contract".into(),
        Value::String("kyuubiki.workflow_diagnostics/v1".into()),
    );
    summary.insert("diagnostic_domain".into(), Value::String("fluid".into()));
    summary.insert(
        "diagnostic_subject".into(),
        Value::String("stokes_flow_result".into()),
    );
    summary.insert("diagnostic_prefix".into(), Value::String(prefix.into()));
    summary.insert("diagnostic_node_count".into(), Value::from(nodes.len()));
    summary.insert(
        "diagnostic_element_count".into(),
        Value::from(elements.len()),
    );
    velocity.insert_bounds(&mut summary, &format!("{prefix}_velocity"))?;
    pressure.insert_bounds(&mut summary, &format!("{prefix}_pressure"))?;
    divergence.insert_peak(&mut summary, &format!("{prefix}_divergence_error"))?;
    reynolds.insert_peak(&mut summary, &format!("{prefix}_reynolds_number"))?;
    dissipation.insert_peak(&mut summary, &format!("{prefix}_viscous_dissipation"))?;
    for (field, value) in [
        ("velocity_mean", velocity.mean),
        ("pressure_mean", pressure.mean),
        ("divergence_error_mean", divergence.mean),
        ("reynolds_number_mean", reynolds.mean),
    ] {
        insert_finite(&mut summary, format!("{prefix}_{field}"), value)?;
    }
    insert_finite(&mut summary, total_field, total)?;
    Ok(Value::Object(summary))
}

pub fn score_cfd_quality(payload: Value, config: Value) -> Result<Value, String> {
    let object = payload
        .as_object()
        .ok_or_else(|| "transform.score_cfd_quality expects an object payload".to_string())?;
    require_converged_result(object, "transform.score_cfd_quality", "payload")?;
    let QualityScore {
        score_terms,
        score,
        missing_count,
        watch_count,
        max_ready_score,
        grade,
        dominant_term,
        blocking_terms,
    } = score_quality_terms(
        object,
        &config,
        &default_quality_terms(),
        quality_term_for,
        8.0,
    )
    .map_err(|error| format!("transform.score_cfd_quality: {error}"))?;

    Ok(serde_json::json!({
        "cfd_quality_contract": "kyuubiki.cfd_quality_score/v1",
        "cfd_quality_score": score,
        "cfd_quality_grade": grade,
        "cfd_quality_ready": grade != "block",
        "cfd_quality_missing_metric_count": missing_count,
        "cfd_quality_watch_count": watch_count,
        "cfd_quality_term_count": score_terms.len(),
        "cfd_quality_max_ready_score": max_ready_score,
        "cfd_quality_divergence_error_peak": numeric_field(object, "cfd_divergence_error_peak"),
        "cfd_quality_reynolds_number_peak": numeric_field(object, "cfd_reynolds_number_peak"),
        "cfd_quality_viscous_dissipation_total": numeric_field(object, "cfd_viscous_dissipation_total"),
        "cfd_quality_velocity_span": numeric_field(object, "cfd_velocity_span"),
        "cfd_quality_pressure_span": numeric_field(object, "cfd_pressure_span"),
        "cfd_quality_dominant_term": dominant_term,
        "cfd_quality_blocking_terms": blocking_terms,
        "cfd_quality_terms": score_terms,
        "cfd_quality_summary": format!(
            "CFD quality {grade}: score={score:.4}, missing={missing_count}, watch={watch_count}, ready_limit={max_ready_score:.4}."
        ),
    }))
}

fn default_quality_terms() -> [QualityTerm; 5] {
    [
        QualityTerm {
            field: "cfd_divergence_error_peak",
            label: "Divergence peak",
            target: 0.05,
            weight: 4.0,
            goal: QualityGoal::Min,
        },
        QualityTerm {
            field: "cfd_reynolds_number_peak",
            label: "Reynolds peak",
            target: 10.0,
            weight: 2.0,
            goal: QualityGoal::Min,
        },
        QualityTerm {
            field: "cfd_viscous_dissipation_total",
            label: "Viscous dissipation",
            target: 1.0,
            weight: 1.0,
            goal: QualityGoal::Min,
        },
        QualityTerm {
            field: "cfd_velocity_span",
            label: "Velocity span",
            target: 2.0,
            weight: 0.5,
            goal: QualityGoal::Min,
        },
        QualityTerm {
            field: "cfd_pressure_span",
            label: "Pressure span",
            target: 5.0,
            weight: 0.5,
            goal: QualityGoal::Min,
        },
    ]
}

fn quality_term_for(field: &str) -> Option<QualityTerm> {
    default_quality_terms()
        .into_iter()
        .find(|term| term.field == field)
}

fn numeric_field(object: &Map<String, Value>, field: &str) -> Option<f64> {
    display_metric_value(object, field)
}

fn samples<'a>(object: &'a Map<String, Value>, field: &str) -> Result<&'a [Value], String> {
    object
        .get(field)
        .and_then(Value::as_array)
        .filter(|values| !values.is_empty())
        .map(Vec::as_slice)
        .ok_or_else(|| format!("payload.{field} must be a non-empty array"))
}

fn sample_object<'a>(
    value: &'a Value,
    source: &str,
    index: usize,
) -> Result<&'a Map<String, Value>, String> {
    value
        .as_object()
        .ok_or_else(|| format!("payload.{source}[{index}] must be an object"))
}

fn sample_metric(
    object: &Map<String, Value>,
    field: &str,
    source: &str,
    index: usize,
) -> Result<f64, String> {
    checked_metric_value(object, field)
        .map_err(|error| format!("payload.{source}[{index}].{error}"))?
        .ok_or_else(|| format!("payload.{source}[{index}].{field} is missing a numeric sample"))
}

#[derive(Default)]
struct MetricStats<'a> {
    min: f64,
    max: f64,
    mean: f64,
    peak: Option<(&'a Value, f64)>,
}

impl<'a> MetricStats<'a> {
    fn observe(&mut self, entry: &'a Value, value: f64, count: usize) {
        if count == 1 {
            self.min = value;
            self.max = value;
            self.mean = value;
        } else {
            self.min = self.min.min(value);
            self.max = self.max.max(value);
            // The mean need not share the numeric range of an unused raw sum.
            let count = count as f64;
            self.mean = self.mean * ((count - 1.0) / count) + value / count;
        }
        if self
            .peak
            .is_none_or(|(_, prior)| value.abs().total_cmp(&prior.abs()).is_ge())
        {
            self.peak = Some((entry, value));
        }
    }

    fn insert_bounds(&self, summary: &mut Map<String, Value>, field: &str) -> Result<(), String> {
        insert_finite(summary, format!("{field}_min"), self.min)?;
        insert_finite(summary, format!("{field}_max"), self.max)?;
        insert_finite(summary, format!("{field}_span"), self.max - self.min)
    }

    fn insert_peak(&self, summary: &mut Map<String, Value>, field: &str) -> Result<(), String> {
        let (entry, value) = self.peak.ok_or_else(|| format!("{field} has no samples"))?;
        insert_finite(summary, format!("{field}_peak"), value)?;
        summary.insert(
            format!("{field}_peak_element_id"),
            entry.get("id").cloned().unwrap_or(Value::Null),
        );
        Ok(())
    }
}

fn insert_finite(
    summary: &mut Map<String, Value>,
    field: String,
    value: f64,
) -> Result<(), String> {
    if !value.is_finite() {
        return Err(format!("{field} is non-finite"));
    }
    summary.insert(field, Value::from(value));
    Ok(())
}
