use crate::workflow_domain_quality::{QualityGoal, QualityScore, QualityTerm, score_quality_terms};
use crate::workflow_metric_resolver::display_metric_value;
use crate::workflow_result_admission::require_converged_result;
use serde_json::{Map, Value};

pub fn score_modal_quality(payload: Value, config: Value) -> Result<Value, String> {
    let object = payload
        .as_object()
        .ok_or_else(|| "transform.score_modal_quality expects an object payload".to_string())?;
    require_converged_result(object, "transform.score_modal_quality", "payload")?;
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
    .map_err(|error| format!("transform.score_modal_quality: {error}"))?;

    Ok(serde_json::json!({
        "modal_quality_contract": "kyuubiki.modal_quality_score/v1",
        "modal_quality_score": score,
        "modal_quality_grade": grade,
        "modal_quality_ready": grade != "block",
        "modal_quality_missing_metric_count": missing_count,
        "modal_quality_watch_count": watch_count,
        "modal_quality_term_count": score_terms.len(),
        "modal_quality_max_ready_score": max_ready_score,
        "modal_quality_min_frequency_hz": numeric_field(object, "min_frequency_hz"),
        "modal_quality_total_mass": numeric_field(object, "total_mass"),
        "modal_quality_frequency_span_hz": numeric_field(object, "frequency_span_hz"),
        "modal_quality_mode_1_participation_norm": numeric_field(object, "mode_1_participation_norm"),
        "modal_quality_dominant_term": dominant_term,
        "modal_quality_blocking_terms": blocking_terms,
        "modal_quality_terms": score_terms,
        "modal_quality_summary": format!(
            "Modal quality {grade}: score={score:.4}, missing={missing_count}, watch={watch_count}, ready_limit={max_ready_score:.4}."
        ),
    }))
}

fn default_quality_terms() -> [QualityTerm; 4] {
    [
        QualityTerm {
            field: "min_frequency_hz",
            label: "First natural frequency",
            target: 20.0,
            weight: 4.0,
            goal: QualityGoal::Max,
        },
        QualityTerm {
            field: "total_mass",
            label: "Total modal mass",
            target: 25.0,
            weight: 1.0,
            goal: QualityGoal::Min,
        },
        QualityTerm {
            field: "mode_1_participation_norm",
            label: "Mode 1 participation",
            target: 2.0,
            weight: 1.0,
            goal: QualityGoal::Min,
        },
        QualityTerm {
            field: "frequency_span_hz",
            label: "Modal frequency spread",
            target: 250.0,
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
