use crate::workflow_domain_quality::{QualityGoal, QualityScore, QualityTerm, score_quality_terms};
use crate::workflow_metric_resolver::display_metric_value;
use crate::workflow_result_admission::require_converged_result;
use serde_json::{Map, Value};

pub fn score_magnetostatic_quality(payload: Value, config: Value) -> Result<Value, String> {
    let object = payload.as_object().ok_or_else(|| {
        "transform.score_magnetostatic_quality expects an object payload".to_string()
    })?;
    require_converged_result(object, "transform.score_magnetostatic_quality", "payload")?;
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
    .map_err(|error| format!("transform.score_magnetostatic_quality: {error}"))?;

    Ok(serde_json::json!({
        "magnetostatic_quality_contract": "kyuubiki.magnetostatic_quality_score/v1",
        "magnetostatic_quality_score": score,
        "magnetostatic_quality_grade": grade,
        "magnetostatic_quality_ready": grade != "block",
        "magnetostatic_quality_missing_metric_count": missing_count,
        "magnetostatic_quality_watch_count": watch_count,
        "magnetostatic_quality_term_count": score_terms.len(),
        "magnetostatic_quality_max_ready_score": max_ready_score,
        "magnetostatic_quality_peak_field": numeric_field(object, "magnetostatic_field_peak_magnitude"),
        "magnetostatic_quality_peak_flux": numeric_field(object, "magnetostatic_flux_peak_magnitude"),
        "magnetostatic_quality_peak_energy_density": numeric_field(object, "magnetostatic_energy_density_peak"),
        "magnetostatic_quality_current_density_sum": numeric_field(object, "magnetostatic_current_density_sum"),
        "magnetostatic_quality_total_energy": numeric_field(object, "magnetostatic_total_stored_energy"),
        "magnetostatic_quality_dominant_term": dominant_term,
        "magnetostatic_quality_blocking_terms": blocking_terms,
        "magnetostatic_quality_terms": score_terms,
        "magnetostatic_quality_summary": format!(
            "Magnetostatic quality {grade}: score={score:.4}, missing={missing_count}, watch={watch_count}, ready_limit={max_ready_score:.4}."
        ),
    }))
}

fn default_quality_terms() -> [QualityTerm; 4] {
    [
        QualityTerm {
            field: "magnetostatic_field_peak_magnitude",
            label: "Peak magnetic field strength",
            target: 12.0,
            weight: 3.0,
            goal: QualityGoal::Min,
        },
        QualityTerm {
            field: "magnetostatic_flux_peak_magnitude",
            label: "Peak magnetic flux density",
            target: 16.0,
            weight: 2.0,
            goal: QualityGoal::Min,
        },
        QualityTerm {
            field: "magnetostatic_energy_density_peak",
            label: "Peak magnetic energy density",
            target: 8.0,
            weight: 2.0,
            goal: QualityGoal::Min,
        },
        QualityTerm {
            field: "magnetostatic_current_density_sum",
            label: "Current density sum",
            target: 10.0,
            weight: 1.0,
            goal: QualityGoal::Min,
        },
    ]
}

fn quality_term_for(field: &str) -> Option<QualityTerm> {
    if let Some(term) = default_quality_terms()
        .into_iter()
        .find(|term| term.field == field)
    {
        return Some(term);
    }
    match field {
        "magnetostatic_total_stored_energy" => Some(QualityTerm {
            field: "magnetostatic_total_stored_energy",
            label: "Total magnetostatic stored energy",
            target: 10.0,
            weight: 1.0,
            goal: QualityGoal::Min,
        }),
        _ => None,
    }
}

fn numeric_field(object: &Map<String, Value>, field: &str) -> Option<f64> {
    display_metric_value(object, field)
}
