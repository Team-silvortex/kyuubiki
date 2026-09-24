use crate::workflow_domain_quality::{QualityGoal, QualityScore, QualityTerm, score_quality_terms};
use crate::workflow_metric_resolver::display_metric_value;
use crate::workflow_result_admission::require_converged_result;
use serde_json::Value;

pub fn score_dynamic_quality(payload: Value, config: Value) -> Result<Value, String> {
    let object = payload
        .as_object()
        .ok_or_else(|| "transform.score_dynamic_quality expects an object payload".to_string())?;
    require_converged_result(object, "transform.score_dynamic_quality", "payload")?;
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
    .map_err(|error| format!("transform.score_dynamic_quality: {error}"))?;

    Ok(serde_json::json!({
        "dynamic_quality_contract": "kyuubiki.dynamic_quality_score/v1",
        "dynamic_quality_score": score,
        "dynamic_quality_grade": grade,
        "dynamic_quality_ready": grade != "block",
        "dynamic_quality_missing_metric_count": missing_count,
        "dynamic_quality_watch_count": watch_count,
        "dynamic_quality_term_count": score_terms.len(),
        "dynamic_quality_max_ready_score": max_ready_score,
        "dynamic_quality_peak_frequency_hz": display_metric_value(object, "peak_frequency_hz"),
        "dynamic_quality_max_displacement": display_metric_value(object, "max_displacement"),
        "dynamic_quality_max_velocity": display_metric_value(object, "max_velocity"),
        "dynamic_quality_max_acceleration": display_metric_value(object, "max_acceleration"),
        "dynamic_quality_max_force": display_metric_value(object, "max_force"),
        "dynamic_quality_dominant_term": dominant_term,
        "dynamic_quality_blocking_terms": blocking_terms,
        "dynamic_quality_terms": score_terms,
        "dynamic_quality_summary": format!(
            "Dynamic quality {grade}: score={score:.4}, missing={missing_count}, watch={watch_count}, ready_limit={max_ready_score:.4}."
        ),
    }))
}

fn default_quality_terms() -> [QualityTerm; 4] {
    [
        QualityTerm {
            field: "peak_frequency_hz",
            label: "Peak response frequency",
            target: 20.0,
            weight: 3.0,
            goal: QualityGoal::Max,
        },
        QualityTerm {
            field: "max_displacement",
            label: "Peak displacement amplitude",
            target: 0.02,
            weight: 3.0,
            goal: QualityGoal::Min,
        },
        QualityTerm {
            field: "max_acceleration",
            label: "Peak acceleration amplitude",
            target: 250.0,
            weight: 1.5,
            goal: QualityGoal::Min,
        },
        QualityTerm {
            field: "max_force",
            label: "Peak dynamic force",
            target: 5000.0,
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
        "max_velocity" => Some(QualityTerm {
            field: "max_velocity",
            label: "Peak velocity amplitude",
            target: 2.0,
            weight: 1.0,
            goal: QualityGoal::Min,
        }),
        _ => None,
    }
}
