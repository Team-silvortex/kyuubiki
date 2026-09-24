use crate::workflow_domain_quality::{QualityGoal, QualityScore, QualityTerm, score_quality_terms};
use crate::workflow_metric_resolver::display_metric_value;
use crate::workflow_result_admission::require_converged_result;
use serde_json::{Map, Value};

pub fn score_acoustic_quality(payload: Value, config: Value) -> Result<Value, String> {
    let object = payload
        .as_object()
        .ok_or_else(|| "transform.score_acoustic_quality expects an object payload".to_string())?;
    require_converged_result(object, "transform.score_acoustic_quality", "payload")?;
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
        7.0,
    )
    .map_err(|error| format!("transform.score_acoustic_quality: {error}"))?;

    Ok(serde_json::json!({
        "acoustic_quality_contract": "kyuubiki.acoustic_quality_score/v1",
        "acoustic_quality_score": score,
        "acoustic_quality_grade": grade,
        "acoustic_quality_ready": grade != "block",
        "acoustic_quality_missing_metric_count": missing_count,
        "acoustic_quality_watch_count": watch_count,
        "acoustic_quality_term_count": score_terms.len(),
        "acoustic_quality_max_ready_score": max_ready_score,
        "acoustic_quality_max_spl_db": numeric_field(object, "max_sound_pressure_level_db"),
        "acoustic_quality_max_intensity": numeric_field(object, "max_acoustic_intensity"),
        "acoustic_quality_max_pressure": numeric_field(object, "max_pressure_amplitude"),
        "acoustic_quality_total_damping_loss": numeric_field(object, "total_damping_loss"),
        "acoustic_quality_dominant_term": dominant_term,
        "acoustic_quality_blocking_terms": blocking_terms,
        "acoustic_quality_terms": score_terms,
        "acoustic_quality_summary": format!(
            "Acoustic quality {grade}: score={score:.4}, missing={missing_count}, watch={watch_count}, ready_limit={max_ready_score:.4}."
        ),
    }))
}

fn default_quality_terms() -> [QualityTerm; 4] {
    [
        QualityTerm {
            field: "max_sound_pressure_level_db",
            label: "Peak sound pressure level",
            target: 85.0,
            weight: 3.0,
            goal: QualityGoal::Min,
        },
        QualityTerm {
            field: "max_acoustic_intensity",
            label: "Peak acoustic intensity",
            target: 0.25,
            weight: 2.0,
            goal: QualityGoal::Min,
        },
        QualityTerm {
            field: "max_pressure_amplitude",
            label: "Peak pressure amplitude",
            target: 1.0,
            weight: 1.0,
            goal: QualityGoal::Min,
        },
        QualityTerm {
            field: "total_damping_loss",
            label: "Damping loss",
            target: 0.1,
            weight: 1.0,
            goal: QualityGoal::Max,
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
