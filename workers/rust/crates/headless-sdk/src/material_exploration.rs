use crate::material_candidate_drafts::{
    material_candidate_draft_batches, material_candidate_draft_summary, material_candidate_drafts,
};
use crate::material_exploration_objectives::next_round_optimization_objectives;
use crate::{
    HeadlessWorkflowStep, build_composite_panel_steps, build_dielectric_screening_steps,
    build_heat_spreader_screening_steps, build_material_report,
    build_structural_panel_screening_steps, build_thermo_shield_screening_steps,
    describe_material_study,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const MATERIAL_EXPLORATION_SCHEMA_VERSION: &str = "kyuubiki.material-exploration-run/v1";
pub const MATERIAL_EXPLORATION_NEXT_ROUND_SCHEMA_VERSION: &str =
    "kyuubiki.material-exploration-next-round/v1";
pub const MATERIAL_EXPLORATION_NEXT_ROUND_EXECUTION_SCHEMA_VERSION: &str =
    "kyuubiki.material-exploration-next-round-execution/v1";
pub const MATERIAL_EXPLORATION_CHAIN_SCHEMA_VERSION: &str =
    "kyuubiki.material-exploration-chain/v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaterialExplorationRun {
    pub schema_version: String,
    pub mode: String,
    pub iteration: usize,
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
    pub risk_mitigation_hints: Vec<MaterialExplorationRiskMitigationHint>,
    pub optimization_objectives: Value,
    pub candidate_drafts: Vec<Value>,
    pub candidate_draft_summary: Value,
    pub draft_execution_batches: Vec<Value>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaterialExplorationRiskMitigationHint {
    pub candidate_id: String,
    pub gate_id: String,
    pub driver: String,
    pub recommendation: String,
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
    build_material_exploration_run_for_iteration(study, mode, result_payloads, 1)
}

pub fn build_material_exploration_run_for_iteration(
    study: &str,
    mode: impl Into<String>,
    result_payloads: Vec<Value>,
    iteration: usize,
) -> Result<MaterialExplorationRun, String> {
    let description = describe_material_study(study)
        .ok_or_else(|| format!("unsupported material study: {study}"))?;
    let report = build_material_report(&description.id, &result_payloads)?;
    let next_round = build_material_exploration_next_round_plan(&report, iteration);
    Ok(MaterialExplorationRun {
        schema_version: MATERIAL_EXPLORATION_SCHEMA_VERSION.to_string(),
        mode: mode.into(),
        iteration,
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

    let missing_metric_warnings = missing_metric_warning_count(report);
    let completeness_violations = violated_gates
        .iter()
        .filter(|gate| gate.contains("result_completeness"))
        .count();
    let risk_violations = violated_gates.len().saturating_sub(completeness_violations);

    let (decision, actions, rationale) =
        if missing_metric_warnings > 0 || completeness_violations > 0 {
            (
                "repair_or_rerun".to_string(),
                vec![
                    "inspect_missing_metrics".to_string(),
                    "rerun_incomplete_candidates".to_string(),
                    "rebuild_report_before_expansion".to_string(),
                ],
                repair_rationale(warnings, &violated_gates),
            )
        } else if risk_violations > 0 || warnings > 0 {
            (
                "mitigate_design_risk".to_string(),
                vec![
                    "inspect_violated_quality_gates".to_string(),
                    "generate_lower_risk_neighbor_candidates".to_string(),
                    "rerun_focused_quality_batch".to_string(),
                    "compare_against_incumbent_winner".to_string(),
                ],
                risk_mitigation_rationale(warnings, &violated_gates),
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
    let report = exploration
        .get("report")
        .cloned()
        .unwrap_or_else(|| Value::Object(Default::default()));
    let steps = match decision.as_str() {
        "expand_around_winner" => material_exploration_steps(study)?,
        "repair_or_rerun" | "mitigate_design_risk" => {
            rerun_focus_steps(study, &focus_candidate_ids)?
        }
        other => return Err(format!("unsupported next_round decision: {other}")),
    };
    let runnable_step_count = steps.len();

    let violated_gate_ids = violated_quality_gate_ids(&report);
    let risk_mitigation_hints =
        risk_mitigation_hints(&decision, &report, &focus_candidate_ids, &violated_gate_ids);
    let optimization_objectives = next_round_optimization_objectives(
        &decision,
        &report,
        &focus_candidate_ids,
        &violated_gate_ids,
    );
    let candidate_drafts =
        material_candidate_drafts(&decision, study, &report, &focus_candidate_ids);
    let candidate_draft_summary = material_candidate_draft_summary(&candidate_drafts);
    let draft_execution_batches = material_candidate_draft_batches(&candidate_drafts);

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
        risk_mitigation_hints,
        optimization_objectives,
        candidate_drafts,
        candidate_draft_summary,
        draft_execution_batches,
        notes: execution_plan_notes(&decision),
    })
}

fn material_exploration_steps_by_id(study_id: &str) -> Result<Vec<HeadlessWorkflowStep>, String> {
    match study_id {
        "material_heat_spreader_screening" => Ok(build_heat_spreader_screening_steps()),
        "material_dielectric_screening" => Ok(build_dielectric_screening_steps()),
        "material_thermo_shield_screening" => Ok(build_thermo_shield_screening_steps()),
        "material_structural_panel_screening" => Ok(build_structural_panel_screening_steps()),
        "material_composite_thermo_electric_panel" => Ok(build_composite_panel_steps()),
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
        "mitigate_design_risk" => vec![
            "current implementation reruns focused candidate solve steps while preserving the risk signal"
                .to_string(),
            "future iterations should replace this with DOE or Bayesian lower-risk neighbor generation"
                .to_string(),
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
    let summary_gate_ids = report
        .get("reliability")
        .and_then(|reliability| reliability.get("summary"))
        .and_then(|summary| summary.get("blocking_gate_ids"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(ToString::to_string)
        .collect::<Vec<_>>();

    if !summary_gate_ids.is_empty() {
        return summary_gate_ids;
    }

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

fn risk_mitigation_hints(
    decision: &str,
    report: &Value,
    focus_candidate_ids: &[String],
    violated_gates: &[String],
) -> Vec<MaterialExplorationRiskMitigationHint> {
    if decision != "mitigate_design_risk" {
        return Vec::new();
    }
    let warnings = report_warnings(report);
    report
        .get("candidates")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|candidate| candidate_is_focused(candidate, focus_candidate_ids))
        .flat_map(|candidate| {
            let candidate_id = candidate
                .get("candidate_id")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string();
            violated_gates.iter().map({
                let warnings = warnings.clone();
                move |gate_id| build_risk_hint(candidate, &candidate_id, gate_id, &warnings)
            })
        })
        .collect()
}

fn candidate_is_focused(candidate: &Value, focus_candidate_ids: &[String]) -> bool {
    let Some(candidate_id) = candidate.get("candidate_id").and_then(Value::as_str) else {
        return false;
    };
    focus_candidate_ids.iter().any(|id| id == candidate_id)
}

fn build_risk_hint(
    candidate: &Value,
    candidate_id: &str,
    gate_id: &str,
    warnings: &[String],
) -> MaterialExplorationRiskMitigationHint {
    let driver = candidate
        .get("weakest_interface")
        .and_then(|interface| interface.get("dominant_driver"))
        .and_then(Value::as_str)
        .or_else(|| risk_driver_from_gate(gate_id))
        .unwrap_or("quality_gate_violation");
    MaterialExplorationRiskMitigationHint {
        candidate_id: candidate_id.to_string(),
        gate_id: gate_id.to_string(),
        driver: driver.to_string(),
        recommendation: risk_recommendation(gate_id, driver, warnings),
    }
}

fn report_warnings(report: &Value) -> Vec<String> {
    report
        .get("warnings")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(ToString::to_string)
        .collect()
}

fn risk_driver_from_gate(gate_id: &str) -> Option<&'static str> {
    if gate_id.contains("interface") {
        Some("interface_mismatch")
    } else if gate_id.contains("temperature") {
        Some("thermal_load")
    } else if gate_id.contains("stress") {
        Some("mechanical_stress")
    } else if gate_id.contains("breakdown") {
        Some("electrical_margin")
    } else {
        None
    }
}

fn risk_recommendation(gate_id: &str, driver: &str, warnings: &[String]) -> String {
    let warning_note = if warnings.is_empty() {
        "no report warning text"
    } else {
        "see report warnings"
    };
    if gate_id.contains("interface") || driver.contains("expansion") {
        format!("try lower CTE mismatch or add compliant interlayer; {warning_note}")
    } else if gate_id.contains("temperature") {
        format!("increase thermal conductivity or reduce heat load; {warning_note}")
    } else if gate_id.contains("stress") {
        format!("reduce stiffness contrast or relax fixtures; {warning_note}")
    } else if gate_id.contains("breakdown") {
        format!("increase dielectric strength or reduce electric field; {warning_note}")
    } else {
        format!("generate conservative neighbors around focused candidates; {warning_note}")
    }
}

fn missing_metric_warning_count(report: &Value) -> usize {
    report
        .get("warnings")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .filter(|warning| warning.contains(" is missing "))
        .count()
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

fn risk_mitigation_rationale(warnings: usize, violated_gates: &[String]) -> Vec<String> {
    let mut rationale = Vec::new();
    if !violated_gates.is_empty() {
        rationale.push(format!(
            "quality gates expose design risk: {}",
            violated_gates.join(", ")
        ));
    }
    if warnings > 0 {
        rationale.push(format!(
            "{warnings} report warning(s) should guide lower-risk candidate generation"
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
