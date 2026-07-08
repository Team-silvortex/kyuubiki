use crate::{HeadlessWorkflowStep, describe_material_study, material_exploration_steps};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const MATERIAL_STUDY_EXECUTION_PLAN_SCHEMA_VERSION: &str =
    "kyuubiki.material-study-execution-plan/v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaterialStudyExecutionPlan {
    pub schema_version: String,
    pub study_id: String,
    pub step_count: usize,
    pub solve_step_count: usize,
    pub candidate_count: usize,
    pub candidate_ids: Vec<String>,
    pub actions: Vec<String>,
    pub steps: Vec<HeadlessWorkflowStep>,
    pub recommended_command: String,
    pub dispatch_notes: Vec<String>,
}

pub fn build_material_study_execution_plan(
    study: &str,
) -> Result<MaterialStudyExecutionPlan, String> {
    let description = describe_material_study(study)
        .ok_or_else(|| format!("unsupported material study: {study}"))?;
    let steps = material_exploration_steps(&description.id)?;
    let solve_step_count = steps
        .iter()
        .filter(|step| step.action.starts_with("solve_"))
        .count();
    let candidate_ids = steps
        .iter()
        .filter(|step| step.action.starts_with("solve_"))
        .filter_map(candidate_id_for_step)
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let actions = steps
        .iter()
        .map(|step| step.action.clone())
        .collect::<Vec<_>>();
    let recommended_command = format!(
        "kyuubiki-material-explore {}",
        description
            .aliases
            .first()
            .map(String::as_str)
            .unwrap_or(&description.id)
    );
    Ok(MaterialStudyExecutionPlan {
        schema_version: MATERIAL_STUDY_EXECUTION_PLAN_SCHEMA_VERSION.to_string(),
        study_id: description.id,
        step_count: steps.len(),
        solve_step_count,
        candidate_count: candidate_ids.len(),
        candidate_ids,
        actions,
        steps,
        recommended_command,
        dispatch_notes: vec![
            "this plan does not execute solver kernels".to_string(),
            "service/headless dispatchers may schedule solve_* steps before job_wait/result_fetch steps"
                .to_string(),
            "candidate ids are extracted from step.payload.research.candidate_id".to_string(),
        ],
    })
}

fn candidate_id_for_step(step: &HeadlessWorkflowStep) -> Option<&str> {
    step.payload
        .get("research")
        .and_then(|research| research.get("candidate_id"))
        .and_then(Value::as_str)
}
