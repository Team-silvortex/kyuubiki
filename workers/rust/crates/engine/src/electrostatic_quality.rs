use crate::workflow_domain_quality::{QualityGoal, QualityScore, QualityTerm, score_quality_terms};
use crate::workflow_metric_resolver::display_metric_value;
use crate::workflow_result_admission::require_converged_result;
use serde_json::{Map, Value};

pub fn score_electrostatic_quality(payload: Value, config: Value) -> Result<Value, String> {
    let object = payload.as_object().ok_or_else(|| {
        "transform.score_electrostatic_quality expects an object payload".to_string()
    })?;
    require_converged_result(object, "transform.score_electrostatic_quality", "payload")?;
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
    .map_err(|error| format!("transform.score_electrostatic_quality: {error}"))?;

    Ok(serde_json::json!({
        "electrostatic_quality_contract": "kyuubiki.electrostatic_quality_score/v1",
        "electrostatic_quality_score": score,
        "electrostatic_quality_grade": grade,
        "electrostatic_quality_ready": grade != "block",
        "electrostatic_quality_missing_metric_count": missing_count,
        "electrostatic_quality_watch_count": watch_count,
        "electrostatic_quality_term_count": score_terms.len(),
        "electrostatic_quality_max_ready_score": max_ready_score,
        "electrostatic_quality_peak_field": numeric_field(object, "electrostatic_field_peak_magnitude"),
        "electrostatic_quality_peak_energy_density": numeric_field(object, "electrostatic_peak_energy_density"),
        "electrostatic_quality_potential_span": numeric_field(object, "electrostatic_potential_span"),
        "electrostatic_quality_total_energy": numeric_field(object, "electrostatic_total_stored_energy"),
        "electrostatic_quality_dominant_term": dominant_term,
        "electrostatic_quality_blocking_terms": blocking_terms,
        "electrostatic_quality_terms": score_terms,
        "electrostatic_quality_summary": format!(
            "Electrostatic quality {grade}: score={score:.4}, missing={missing_count}, watch={watch_count}, ready_limit={max_ready_score:.4}."
        ),
    }))
}

fn default_quality_terms() -> [QualityTerm; 3] {
    [
        QualityTerm {
            field: "electrostatic_field_peak_magnitude",
            label: "Peak electric field magnitude",
            target: 10.0,
            weight: 4.0,
            goal: QualityGoal::Min,
        },
        QualityTerm {
            field: "electrostatic_peak_energy_density",
            label: "Peak electrostatic energy density",
            target: 0.8,
            weight: 2.0,
            goal: QualityGoal::Min,
        },
        QualityTerm {
            field: "electrostatic_potential_span",
            label: "Potential span",
            target: 4.0,
            weight: 1.0,
            goal: QualityGoal::Max,
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
        "electrostatic_total_stored_energy" => Some(QualityTerm {
            field: "electrostatic_total_stored_energy",
            label: "Total electrostatic stored energy",
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
