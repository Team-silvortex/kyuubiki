use crate::{
    HeadlessWorkflowStep, build_dielectric_screening_steps, build_heat_spreader_screening_steps,
    build_material_report, build_structural_panel_screening_steps,
    build_thermo_shield_screening_steps, describe_material_study,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const MATERIAL_EXPLORATION_SCHEMA_VERSION: &str = "kyuubiki.material-exploration-run/v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaterialExplorationRun {
    pub schema_version: String,
    pub mode: String,
    pub study: String,
    pub template_id: String,
    pub candidate_count: usize,
    pub result_payloads: Vec<Value>,
    pub report: Value,
}

pub fn material_exploration_steps(study: &str) -> Result<Vec<HeadlessWorkflowStep>, String> {
    let description = describe_material_study(study)
        .ok_or_else(|| format!("unsupported material study: {study}"))?;
    material_exploration_steps_by_id(&description.id)
}

pub fn build_material_exploration_run(
    study: &str,
    mode: impl Into<String>,
    result_payloads: Vec<Value>,
) -> Result<MaterialExplorationRun, String> {
    let description = describe_material_study(study)
        .ok_or_else(|| format!("unsupported material study: {study}"))?;
    let report = build_material_report(&description.id, &result_payloads)?;
    Ok(MaterialExplorationRun {
        schema_version: MATERIAL_EXPLORATION_SCHEMA_VERSION.to_string(),
        mode: mode.into(),
        study: description.id,
        template_id: description.template_id,
        candidate_count: result_payloads.len(),
        result_payloads,
        report,
    })
}

fn material_exploration_steps_by_id(study_id: &str) -> Result<Vec<HeadlessWorkflowStep>, String> {
    match study_id {
        "material_heat_spreader_screening" => Ok(build_heat_spreader_screening_steps()),
        "material_dielectric_screening" => Ok(build_dielectric_screening_steps()),
        "material_thermo_shield_screening" => Ok(build_thermo_shield_screening_steps()),
        "material_structural_panel_screening" => Ok(build_structural_panel_screening_steps()),
        other => Err(format!("unsupported material exploration study: {other}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn exposes_candidate_solve_steps_for_each_material_study() {
        for study in [
            "heat-spreader",
            "dielectric-screening",
            "thermo-shield",
            "structural-panel",
        ] {
            let steps = material_exploration_steps(study).expect("material steps");
            let solve_count = steps
                .iter()
                .filter(|step| step.action.starts_with("solve_"))
                .count();
            assert_eq!(solve_count, 3);
        }
    }

    #[test]
    fn builds_stable_exploration_run_contract() {
        let run = build_material_exploration_run(
            "dielectric-screening",
            "unit-test",
            vec![
                json!({ "max_electric_field": 42.0e6, "max_flux_density": 1.2e-3 }),
                json!({ "max_electric_field": 38.0e6, "max_flux_density": 3.3e-3 }),
                json!({ "max_electric_field": 48.0e6, "max_flux_density": 0.9e-3 }),
            ],
        )
        .expect("exploration run");

        assert_eq!(run.schema_version, MATERIAL_EXPLORATION_SCHEMA_VERSION);
        assert_eq!(run.mode, "unit-test");
        assert_eq!(run.candidate_count, 3);
        assert_eq!(
            run.report["winner_candidate_id"].as_str(),
            Some("polyimide_film")
        );
    }
}
