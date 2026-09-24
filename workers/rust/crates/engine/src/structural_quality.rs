use crate::workflow_domain_quality::{QualityGoal, QualityScore, QualityTerm, score_quality_terms};
use crate::workflow_metric_resolver::display_metric_value;
use crate::workflow_result_admission::require_converged_result;
use serde_json::{Map, Value};

pub fn score_structural_quality(payload: Value, config: Value) -> Result<Value, String> {
    let object = payload.as_object().ok_or_else(|| {
        "transform.score_structural_quality expects an object payload".to_string()
    })?;
    require_converged_result(object, "transform.score_structural_quality", "payload")?;
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
    .map_err(|error| format!("transform.score_structural_quality: {error}"))?;

    Ok(serde_json::json!({
        "structural_quality_contract": "kyuubiki.structural_quality_score/v1",
        "structural_quality_score": score,
        "structural_quality_grade": grade,
        "structural_quality_ready": grade != "block",
        "structural_quality_missing_metric_count": missing_count,
        "structural_quality_watch_count": watch_count,
        "structural_quality_term_count": score_terms.len(),
        "structural_quality_max_ready_score": max_ready_score,
        "structural_quality_max_displacement": numeric_field(object, "max_displacement"),
        "structural_quality_max_stress": numeric_field(object, "max_stress"),
        "structural_quality_mass": numeric_field(object, "mass"),
        "structural_quality_stiffness_margin": numeric_field(object, "stiffness_margin"),
        "structural_quality_dominant_term": dominant_term,
        "structural_quality_blocking_terms": blocking_terms,
        "structural_quality_terms": score_terms,
        "structural_quality_summary": format!(
            "Structural quality {grade}: score={score:.4}, missing={missing_count}, watch={watch_count}, ready_limit={max_ready_score:.4}."
        ),
    }))
}

fn default_quality_terms() -> [QualityTerm; 4] {
    [
        QualityTerm {
            field: "max_displacement",
            label: "Maximum displacement",
            target: 0.02,
            weight: 3.0,
            goal: QualityGoal::Min,
        },
        QualityTerm {
            field: "max_stress",
            label: "Maximum stress",
            target: 250.0,
            weight: 3.0,
            goal: QualityGoal::Min,
        },
        QualityTerm {
            field: "mass",
            label: "Mass",
            target: 15.0,
            weight: 1.0,
            goal: QualityGoal::Min,
        },
        QualityTerm {
            field: "stiffness_margin",
            label: "Stiffness margin",
            target: 1.2,
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
