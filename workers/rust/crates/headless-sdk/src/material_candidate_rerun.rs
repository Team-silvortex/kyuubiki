use crate::{
    HeadlessWorkflowStep, build_composite_materialized_candidate_report,
    build_composite_materialized_candidate_steps,
    build_heat_spreader_materialized_candidate_report,
    build_heat_spreader_materialized_candidate_steps,
};
use serde_json::Value;

pub fn build_materialized_candidate_steps(
    plan: &Value,
) -> Result<Vec<HeadlessWorkflowStep>, String> {
    match materialized_candidate_study(plan)? {
        "material_composite_thermo_electric_panel" => {
            build_composite_materialized_candidate_steps(plan)
        }
        "material_heat_spreader_screening" => {
            build_heat_spreader_materialized_candidate_steps(plan)
        }
        study => Err(unsupported_study(study)),
    }
}

pub fn build_materialized_candidate_report(
    plan: &Value,
    result_payloads: &[Value],
) -> Result<Value, String> {
    match materialized_candidate_study(plan)? {
        "material_composite_thermo_electric_panel" => {
            build_composite_materialized_candidate_report(result_payloads)
        }
        "material_heat_spreader_screening" => serde_json::to_value(
            build_heat_spreader_materialized_candidate_report(plan, result_payloads)?,
        )
        .map_err(|error| error.to_string()),
        study => Err(unsupported_study(study)),
    }
}

pub fn materialized_candidate_study(plan: &Value) -> Result<&str, String> {
    let candidates = plan
        .get("materialized_candidates")
        .and_then(Value::as_array)
        .ok_or_else(|| "materialization plan is missing materialized_candidates".to_string())?;
    let first = candidates
        .first()
        .ok_or_else(|| "materialization plan has no materialized candidates".to_string())?;
    let study = required_study(first)?;
    for (index, candidate) in candidates.iter().enumerate().skip(1) {
        let candidate_study = required_study(candidate)?;
        if candidate_study != study {
            return Err(format!(
                "materialized_candidates[{index}] study {candidate_study} does not match {study}"
            ));
        }
    }
    Ok(study)
}

fn required_study(candidate: &Value) -> Result<&str, String> {
    candidate
        .get("study")
        .and_then(Value::as_str)
        .filter(|study| !study.trim().is_empty())
        .ok_or_else(|| "materialized candidate is missing study".to_string())
}

fn unsupported_study(study: &str) -> String {
    format!(
        "materialized rerun does not support study {study}; supported studies: material_heat_spreader_screening, material_composite_thermo_electric_panel"
    )
}
