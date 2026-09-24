use crate::workflow_domain_quality::{QualityGoal, QualityScore, QualityTerm, score_quality_terms};
use crate::workflow_metric_resolver::display_metric_value;
use crate::workflow_result_admission::require_converged_result;
use serde_json::{Map, Value};

pub fn score_thermal_quality(payload: Value, config: Value) -> Result<Value, String> {
    let object = payload
        .as_object()
        .ok_or_else(|| "transform.score_thermal_quality expects an object payload".to_string())?;
    require_converged_result(object, "transform.score_thermal_quality", "payload")?;
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
    .map_err(|error| format!("transform.score_thermal_quality: {error}"))?;

    Ok(serde_json::json!({
        "thermal_quality_contract": "kyuubiki.thermal_quality_score/v1",
        "thermal_quality_score": score,
        "thermal_quality_grade": grade,
        "thermal_quality_ready": grade != "block",
        "thermal_quality_missing_metric_count": missing_count,
        "thermal_quality_watch_count": watch_count,
        "thermal_quality_term_count": score_terms.len(),
        "thermal_quality_max_ready_score": max_ready_score,
        "thermal_quality_max_temperature": numeric_field(object, "thermal_temperature_max"),
        "thermal_quality_temperature_delta": numeric_field(object, "thermo_temperature_delta_max"),
        "thermal_quality_peak_flux_magnitude": numeric_field(object, "thermal_flux_peak_magnitude"),
        "thermal_quality_peak_stress": numeric_field(object, "thermo_stress_peak"),
        "thermal_quality_total_energy": numeric_field(object, "thermal_total_energy"),
        "thermal_quality_dominant_term": dominant_term,
        "thermal_quality_blocking_terms": blocking_terms,
        "thermal_quality_terms": score_terms,
        "thermal_quality_summary": format!(
            "Thermal quality {grade}: score={score:.4}, missing={missing_count}, watch={watch_count}, ready_limit={max_ready_score:.4}."
        ),
    }))
}

fn default_quality_terms() -> [QualityTerm; 4] {
    [
        QualityTerm {
            field: "thermal_temperature_max",
            label: "Peak thermal temperature",
            target: 120.0,
            weight: 3.0,
            goal: QualityGoal::Min,
        },
        QualityTerm {
            field: "thermo_temperature_delta_max",
            label: "Peak thermo-mechanical temperature delta",
            target: 80.0,
            weight: 2.0,
            goal: QualityGoal::Min,
        },
        QualityTerm {
            field: "thermal_flux_peak_magnitude",
            label: "Peak heat flux magnitude",
            target: 20.0,
            weight: 2.0,
            goal: QualityGoal::Min,
        },
        QualityTerm {
            field: "thermo_stress_peak",
            label: "Peak thermo-mechanical stress",
            target: 250.0,
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
        "thermal_total_energy" => Some(QualityTerm {
            field: "thermal_total_energy",
            label: "Total thermal energy",
            target: 5000.0,
            weight: 1.0,
            goal: QualityGoal::Min,
        }),
        _ => None,
    }
}

fn numeric_field(object: &Map<String, Value>, field: &str) -> Option<f64> {
    display_metric_value(object, field)
}
