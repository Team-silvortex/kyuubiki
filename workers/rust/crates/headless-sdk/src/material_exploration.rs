use crate::{
    HeadlessWorkflowStep, build_dielectric_screening_steps, build_heat_spreader_screening_steps,
    build_material_report, build_structural_panel_screening_steps,
    build_thermo_shield_screening_steps, describe_material_study,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const MATERIAL_EXPLORATION_SCHEMA_VERSION: &str = "kyuubiki.material-exploration-run/v1";
pub const MATERIAL_EXPLORATION_NEXT_ROUND_SCHEMA_VERSION: &str =
    "kyuubiki.material-exploration-next-round/v1";
pub const MATERIAL_EXPLORATION_NEXT_ROUND_EXECUTION_SCHEMA_VERSION: &str =
    "kyuubiki.material-exploration-next-round-execution/v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaterialExplorationRun {
    pub schema_version: String,
    pub mode: String,
    pub study: String,
    pub template_id: String,
    pub candidate_count: usize,
    pub result_payloads: Vec<Value>,
    pub report: Value,
    pub next_round: MaterialExplorationNextRoundPlan,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaterialExplorationNextRoundPlan {
    pub schema_version: String,
    pub iteration: usize,
    pub decision: String,
    pub focus_candidate_ids: Vec<String>,
    pub actions: Vec<String>,
    pub rationale: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaterialExplorationNextRoundExecutionPlan {
    pub schema_version: String,
    pub source_schema_version: String,
    pub study: String,
    pub iteration: usize,
    pub decision: String,
    pub focus_candidate_ids: Vec<String>,
    pub actions: Vec<String>,
    pub runnable_step_count: usize,
    pub steps: Vec<HeadlessWorkflowStep>,
    pub notes: Vec<String>,
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
    let next_round = build_material_exploration_next_round_plan(&report, 1);
    Ok(MaterialExplorationRun {
        schema_version: MATERIAL_EXPLORATION_SCHEMA_VERSION.to_string(),
        mode: mode.into(),
        study: description.id,
        template_id: description.template_id,
        candidate_count: result_payloads.len(),
        result_payloads,
        report,
        next_round,
    })
}

pub fn build_material_exploration_next_round_plan(
    report: &Value,
    iteration: usize,
) -> MaterialExplorationNextRoundPlan {
    let warnings = report
        .get("warnings")
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0);
    let violated_gates = violated_quality_gate_ids(report);
    let focus_candidate_ids = focus_candidate_ids(report);

    let (decision, actions, rationale) = if warnings > 0 || !violated_gates.is_empty() {
        (
            "repair_or_rerun".to_string(),
            vec![
                "inspect_missing_metrics".to_string(),
                "rerun_incomplete_candidates".to_string(),
                "rebuild_report_before_expansion".to_string(),
            ],
            repair_rationale(warnings, &violated_gates),
        )
    } else {
        (
            "expand_around_winner".to_string(),
            vec![
                "generate_neighbor_candidates".to_string(),
                "run_next_quality_batch".to_string(),
                "compare_against_incumbent_winner".to_string(),
            ],
            expansion_rationale(report, &focus_candidate_ids),
        )
    };

    MaterialExplorationNextRoundPlan {
        schema_version: MATERIAL_EXPLORATION_NEXT_ROUND_SCHEMA_VERSION.to_string(),
        iteration: iteration + 1,
        decision,
        focus_candidate_ids,
        actions,
        rationale,
    }
}

pub fn build_material_exploration_next_round_execution_plan(
    exploration: &Value,
) -> Result<MaterialExplorationNextRoundExecutionPlan, String> {
    let study = exploration
        .get("study")
        .and_then(Value::as_str)
        .ok_or_else(|| "material exploration run is missing study".to_string())?;
    let next_round = exploration
        .get("next_round")
        .ok_or_else(|| "material exploration run is missing next_round".to_string())?;
    let decision = next_round
        .get("decision")
        .and_then(Value::as_str)
        .ok_or_else(|| "next_round is missing decision".to_string())?
        .to_string();
    let iteration = next_round
        .get("iteration")
        .and_then(Value::as_u64)
        .unwrap_or(2) as usize;
    let actions = string_array(next_round, "actions");
    let focus_candidate_ids = string_array(next_round, "focus_candidate_ids");
    let steps = match decision.as_str() {
        "expand_around_winner" => material_exploration_steps(study)?,
        "repair_or_rerun" => rerun_focus_steps(study, &focus_candidate_ids)?,
        other => return Err(format!("unsupported next_round decision: {other}")),
    };
    let runnable_step_count = steps.len();

    Ok(MaterialExplorationNextRoundExecutionPlan {
        schema_version: MATERIAL_EXPLORATION_NEXT_ROUND_EXECUTION_SCHEMA_VERSION.to_string(),
        source_schema_version: exploration
            .get("schema_version")
            .and_then(Value::as_str)
            .unwrap_or(MATERIAL_EXPLORATION_SCHEMA_VERSION)
            .to_string(),
        study: study.to_string(),
        iteration,
        decision: decision.clone(),
        focus_candidate_ids,
        actions,
        runnable_step_count,
        steps,
        notes: execution_plan_notes(&decision),
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

fn rerun_focus_steps(
    study: &str,
    focus_candidate_ids: &[String],
) -> Result<Vec<HeadlessWorkflowStep>, String> {
    let focus = focus_candidate_ids
        .iter()
        .map(|id| id.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    Ok(material_exploration_steps(study)?
        .into_iter()
        .filter(|step| {
            candidate_id_for_step(step)
                .map(|candidate_id| focus.contains(candidate_id))
                .unwrap_or(false)
        })
        .collect())
}

fn candidate_id_for_step(step: &HeadlessWorkflowStep) -> Option<&str> {
    step.payload
        .get("research")
        .and_then(|research| research.get("candidate_id"))
        .and_then(Value::as_str)
}

fn string_array(value: &Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(ToString::to_string)
        .collect()
}

fn execution_plan_notes(decision: &str) -> Vec<String> {
    match decision {
        "repair_or_rerun" => vec![
            "rerun only focused candidate solve steps before expanding the design space"
                .to_string(),
            "rebuild the material report from fresh result payloads".to_string(),
        ],
        _ => vec![
            "current implementation reuses the built-in study candidate generator".to_string(),
            "future iterations should replace this with DOE or Bayesian neighbor generation"
                .to_string(),
        ],
    }
}

fn focus_candidate_ids(report: &Value) -> Vec<String> {
    let mut ids = report
        .get("candidates")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|candidate| {
            candidate
                .get("rank")
                .and_then(Value::as_u64)
                .is_some_and(|rank| rank <= 2)
        })
        .filter_map(|candidate| candidate.get("candidate_id").and_then(Value::as_str))
        .map(ToString::to_string)
        .collect::<Vec<_>>();

    if ids.is_empty()
        && let Some(winner) = report.get("winner_candidate_id").and_then(Value::as_str)
    {
        ids.push(winner.to_string());
    }
    ids
}

fn violated_quality_gate_ids(report: &Value) -> Vec<String> {
    report
        .get("reliability")
        .and_then(|reliability| reliability.get("quality_gates"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|gate| gate.get("status").and_then(Value::as_str) != Some("pass"))
        .filter_map(|gate| gate.get("id").and_then(Value::as_str))
        .map(ToString::to_string)
        .collect()
}

fn repair_rationale(warnings: usize, violated_gates: &[String]) -> Vec<String> {
    let mut rationale = Vec::new();
    if warnings > 0 {
        rationale.push(format!(
            "{warnings} report warning(s) need cleanup before expanding the search"
        ));
    }
    if !violated_gates.is_empty() {
        rationale.push(format!(
            "quality gates require attention: {}",
            violated_gates.join(", ")
        ));
    }
    rationale
}

fn expansion_rationale(report: &Value, focus_candidate_ids: &[String]) -> Vec<String> {
    let winner = report
        .get("winner_candidate_id")
        .and_then(Value::as_str)
        .unwrap_or("current winner");
    vec![
        format!("{winner} is the current incumbent with complete screening data"),
        format!(
            "focus next candidates around: {}",
            focus_candidate_ids.join(", ")
        ),
    ]
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
        assert_eq!(run.next_round.decision, "expand_around_winner");
        assert_eq!(run.next_round.iteration, 2);
        assert!(
            run.next_round
                .focus_candidate_ids
                .contains(&"polyimide_film".to_string())
        );
        assert!(
            run.next_round
                .actions
                .contains(&"run_next_quality_batch".to_string())
        );
    }

    #[test]
    fn next_round_plan_repairs_incomplete_reports_before_expansion() {
        let report = json!({
            "winner_candidate_id": "candidate-a",
            "warnings": ["candidate-b is missing max_stress_pa"],
            "candidates": [
                { "candidate_id": "candidate-a", "rank": 1, "score": 0.8 },
                { "candidate_id": "candidate-b", "rank": 2, "score": 0.6 }
            ],
            "reliability": {
                "quality_gates": [
                    { "id": "gate.result_completeness", "status": "violate" }
                ]
            }
        });

        let plan = build_material_exploration_next_round_plan(&report, 3);

        assert_eq!(
            plan.schema_version,
            MATERIAL_EXPLORATION_NEXT_ROUND_SCHEMA_VERSION
        );
        assert_eq!(plan.iteration, 4);
        assert_eq!(plan.decision, "repair_or_rerun");
        assert_eq!(plan.focus_candidate_ids, vec!["candidate-a", "candidate-b"]);
        assert!(
            plan.actions
                .contains(&"rerun_incomplete_candidates".to_string())
        );
        assert!(
            plan.rationale
                .iter()
                .any(|line| line.contains("gate.result_completeness"))
        );
    }

    #[test]
    fn next_round_execution_plan_filters_repair_reruns_to_focus_candidates() {
        let exploration = json!({
            "schema_version": MATERIAL_EXPLORATION_SCHEMA_VERSION,
            "study": "material_dielectric_screening",
            "next_round": {
                "iteration": 2,
                "decision": "repair_or_rerun",
                "focus_candidate_ids": ["polyimide_film"],
                "actions": ["rerun_incomplete_candidates"]
            }
        });

        let plan = build_material_exploration_next_round_execution_plan(&exploration)
            .expect("next round execution plan");

        assert_eq!(
            plan.schema_version,
            MATERIAL_EXPLORATION_NEXT_ROUND_EXECUTION_SCHEMA_VERSION
        );
        assert_eq!(plan.decision, "repair_or_rerun");
        assert_eq!(plan.runnable_step_count, 1);
        assert!(
            plan.steps
                .iter()
                .all(|step| candidate_id_for_step(step) == Some("polyimide_film"))
        );
    }

    #[test]
    fn next_round_execution_plan_expands_with_full_study_steps() {
        let exploration = json!({
            "schema_version": MATERIAL_EXPLORATION_SCHEMA_VERSION,
            "study": "material_structural_panel_screening",
            "next_round": {
                "iteration": 2,
                "decision": "expand_around_winner",
                "focus_candidate_ids": ["carbon_fiber_quasi_iso"],
                "actions": ["generate_neighbor_candidates", "run_next_quality_batch"]
            }
        });

        let plan = build_material_exploration_next_round_execution_plan(&exploration)
            .expect("next round execution plan");

        assert_eq!(plan.decision, "expand_around_winner");
        assert_eq!(plan.runnable_step_count, 9);
        assert_eq!(plan.steps.len(), 9);
    }
}
