use crate::workflow_domain_quality::{QualityGoal, QualityScore, QualityTerm, score_quality_terms};
use crate::workflow_metric_resolver::display_metric_value;
use crate::workflow_result_admission::require_converged_result;
use serde_json::{Map, Value};

pub fn score_transport_quality(payload: Value, config: Value) -> Result<Value, String> {
    let object = payload
        .as_object()
        .ok_or_else(|| "transform.score_transport_quality expects an object payload".to_string())?;
    require_converged_result(object, "transform.score_transport_quality", "payload")?;
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
    .map_err(|error| format!("transform.score_transport_quality: {error}"))?;

    Ok(serde_json::json!({
        "transport_quality_contract": "kyuubiki.transport_quality_score/v1",
        "transport_quality_score": score,
        "transport_quality_grade": grade,
        "transport_quality_ready": grade != "block",
        "transport_quality_missing_metric_count": missing_count,
        "transport_quality_watch_count": watch_count,
        "transport_quality_term_count": score_terms.len(),
        "transport_quality_max_ready_score": max_ready_score,
        "transport_quality_peak_flux_magnitude": numeric_field(object, "transport_total_flux_peak_magnitude"),
        "transport_quality_peak_peclet": numeric_field(object, "transport_peclet_peak"),
        "transport_quality_concentration_span": numeric_field(object, "transport_concentration_span"),
        "transport_quality_source_sum": numeric_field(object, "transport_source_sum"),
        "transport_quality_dominant_term": dominant_term,
        "transport_quality_blocking_terms": blocking_terms,
        "transport_quality_terms": score_terms,
        "transport_quality_summary": format!(
            "Transport quality {grade}: score={score:.4}, missing={missing_count}, watch={watch_count}, ready_limit={max_ready_score:.4}."
        ),
    }))
}

fn default_quality_terms() -> [QualityTerm; 4] {
    [
        QualityTerm {
            field: "transport_total_flux_peak_magnitude",
            label: "Peak total transport flux magnitude",
            target: 1.5,
            weight: 3.0,
            goal: QualityGoal::Min,
        },
        QualityTerm {
            field: "transport_peclet_peak",
            label: "Peak Peclet number",
            target: 200.0,
            weight: 2.0,
            goal: QualityGoal::Min,
        },
        QualityTerm {
            field: "transport_concentration_span",
            label: "Concentration span",
            target: 1.0,
            weight: 2.0,
            goal: QualityGoal::Min,
        },
        QualityTerm {
            field: "transport_source_sum",
            label: "Net source balance",
            target: 2.0,
            weight: 1.0,
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
