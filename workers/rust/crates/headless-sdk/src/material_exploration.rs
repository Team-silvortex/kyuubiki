use crate::material_candidate_drafts::{
    material_candidate_draft_summary, material_candidate_drafts,
};
use crate::material_candidate_review_batches::material_candidate_draft_batches;
use crate::material_exploration_objectives::next_round_optimization_objectives;
use crate::material_exploration_risk::risk_mitigation_hints;
use crate::{
    ExecutionAuthority, HeadlessWorkflowStep, build_composite_panel_steps,
    build_dielectric_screening_steps, build_heat_spreader_screening_steps, build_material_report,
    build_structural_panel_screening_steps, build_thermo_shield_screening_steps,
    describe_material_study,
};
use kyuubiki_protocol::canonical_json_sha256;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

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
    pub execution_authority: ExecutionAuthority,
    pub iteration: usize,
    pub study: String,
    pub template_id: String,
    pub candidate_count: usize,
    #[serde(default)]
    pub candidate_input_fingerprint: String,
    #[serde(default)]
    pub candidate_input_manifest: Value,
    pub material_card_refs: Vec<Value>,
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
    pub material_card_refs: Vec<Value>,
    pub actions: Vec<String>,
    pub runnable_step_count: usize,
    pub steps: Vec<HeadlessWorkflowStep>,
    pub risk_mitigation_hints: Vec<MaterialExplorationRiskMitigationHint>,
    pub optimization_objectives: Value,
    pub candidate_drafts: Vec<Value>,
    pub candidate_draft_summary: Value,
    pub draft_execution_batches: Vec<Value>,
    pub review_policy: MaterialExplorationReviewPolicy,
    pub search_space_progress: MaterialExplorationSearchSpaceProgress,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaterialExplorationReviewPolicy {
    pub schema_version: String,
    pub required: bool,
    pub state: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaterialExplorationSearchSpaceProgress {
    pub schema_version: String,
    pub state: String,
    pub source_candidate_input_fingerprint: String,
    pub planned_candidate_input_fingerprint: String,
    pub candidate_inputs_changed: bool,
    pub convergence_eligible: bool,
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
    let mode = mode.into();
    let description = describe_material_study(study)
        .ok_or_else(|| format!("unsupported material study: {study}"))?;
    let report = build_material_report(&description.id, &result_payloads)?;
    let candidate_input_manifest =
        candidate_input_manifest_from_results(&description.id, &result_payloads);
    let candidate_input_fingerprint = candidate_input_fingerprint(&candidate_input_manifest);
    let material_card_refs = material_card_refs_from_report(&report);
    let next_round = build_material_exploration_next_round_plan(&report, iteration);
    Ok(MaterialExplorationRun {
        schema_version: MATERIAL_EXPLORATION_SCHEMA_VERSION.to_string(),
        execution_authority: ExecutionAuthority::from_material_mode(&mode),
        mode,
        iteration,
        study: description.id,
        template_id: description.template_id,
        candidate_count: result_payloads.len(),
        candidate_input_fingerprint,
        candidate_input_manifest,
        material_card_refs,
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
    let validation_violations = violated_gates
        .iter()
        .filter(|gate| gate.contains("summary_tolerance_validation"))
        .count();
    let risk_violations = violated_gates
        .len()
        .saturating_sub(completeness_violations + validation_violations);

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
        } else if validation_violations > 0 {
            (
                "repair_validation".to_string(),
                vec![
                    "inspect_summary_validation_failures".to_string(),
                    "rerun_validation_focused_candidates".to_string(),
                    "rebuild_report_before_expansion".to_string(),
                ],
                validation_repair_rationale(&violated_gates),
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
    let material_card_refs = material_card_refs_from_report(&report);
    let steps = match decision.as_str() {
        "expand_around_winner" => material_exploration_steps(study)?,
        "repair_or_rerun" | "repair_validation" | "mitigate_design_risk" => {
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
    let candidate_draft_summary = material_candidate_draft_summary(&decision, &candidate_drafts);
    let draft_execution_batches = material_candidate_draft_batches(&candidate_drafts);
    let review_policy = review_policy(&draft_execution_batches);
    let search_space_progress = search_space_progress(
        exploration,
        study,
        &decision,
        &steps,
        &draft_execution_batches,
    );

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
        material_card_refs,
        actions,
        runnable_step_count,
        steps,
        risk_mitigation_hints,
        optimization_objectives,
        candidate_drafts,
        candidate_draft_summary,
        draft_execution_batches,
        review_policy,
        search_space_progress,
        notes: execution_plan_notes(&decision),
    })
}

fn review_policy(draft_execution_batches: &[Value]) -> MaterialExplorationReviewPolicy {
    let required = !draft_execution_batches.is_empty();
    MaterialExplorationReviewPolicy {
        schema_version: "kyuubiki.material-review-policy/v1".to_string(),
        required,
        state: if required {
            "required_before_materialization"
        } else {
            "not_applicable"
        }
        .to_string(),
        reason: if required {
            "candidate drafts must be reviewed before materialization"
        } else {
            "next-round plan contains no candidate draft execution batches"
        }
        .to_string(),
    }
}

fn search_space_progress(
    exploration: &Value,
    study: &str,
    decision: &str,
    steps: &[HeadlessWorkflowStep],
    draft_execution_batches: &[Value],
) -> MaterialExplorationSearchSpaceProgress {
    let source = candidate_input_fingerprint_from_exploration(exploration, study);
    let planned = planned_candidate_input_fingerprint(exploration, study, steps);
    let input_comparison = planned_candidate_inputs_changed(exploration, steps);
    let changed = input_comparison == Some(true);
    let state = if input_comparison.is_none() {
        "source_fingerprint_unavailable"
    } else if changed {
        "candidate_inputs_changed"
    } else if !draft_execution_batches.is_empty() {
        "candidate_drafts_pending_review"
    } else if decision == "expand_around_winner" {
        "builtin_candidate_replay"
    } else {
        "focused_candidate_rerun"
    };
    MaterialExplorationSearchSpaceProgress {
        schema_version: "kyuubiki.material-search-space-progress/v1".to_string(),
        state: state.to_string(),
        source_candidate_input_fingerprint: source,
        planned_candidate_input_fingerprint: planned,
        candidate_inputs_changed: changed,
        convergence_eligible: changed,
    }
}

fn planned_candidate_inputs_changed(
    exploration: &Value,
    steps: &[HeadlessWorkflowStep],
) -> Option<bool> {
    let source_models = candidate_model_fingerprints_from_exploration(exploration)?;
    let mut compared = false;
    for step in steps
        .iter()
        .filter(|step| step.action.starts_with("solve_"))
    {
        let candidate_id = candidate_id_for_step(step)?;
        let planned_model = candidate_model_for_step(step);
        compared = true;
        let Some(source_model_fingerprint) = source_models.get(candidate_id) else {
            return Some(true);
        };
        if source_model_fingerprint != &canonical_json_sha256(planned_model) {
            return Some(true);
        }
    }
    compared.then_some(false)
}

fn candidate_model_fingerprints_from_exploration(
    exploration: &Value,
) -> Option<std::collections::BTreeMap<String, String>> {
    let study = exploration.get("study")?.as_str()?;
    let manifest = candidate_input_manifest_from_exploration(exploration, study)?;
    let mut models = std::collections::BTreeMap::new();
    for entry in manifest.get("entries")?.as_array()? {
        models.insert(
            entry.get("candidate_id")?.as_str()?.to_string(),
            entry.get("model_fingerprint")?.as_str()?.to_string(),
        );
    }
    (!models.is_empty()).then_some(models)
}

fn candidate_input_fingerprint_from_exploration(exploration: &Value, study: &str) -> String {
    if let Some(fingerprint) = exploration
        .get("candidate_input_fingerprint")
        .and_then(Value::as_str)
        .filter(|fingerprint| !fingerprint.is_empty())
    {
        return fingerprint.to_string();
    }
    let manifest =
        candidate_input_manifest_from_exploration(exploration, study).unwrap_or(Value::Null);
    candidate_input_fingerprint(&manifest)
}

fn candidate_input_manifest_from_exploration(exploration: &Value, study: &str) -> Option<Value> {
    exploration
        .get("candidate_input_manifest")
        .filter(|manifest| manifest.get("entries").and_then(Value::as_array).is_some())
        .cloned()
        .or_else(|| {
            exploration
                .get("result_payloads")
                .and_then(Value::as_array)
                .map(|results| candidate_input_manifest_from_results(study, results))
        })
        .filter(|manifest| manifest != &Value::Null)
}

fn candidate_input_manifest_from_results(study: &str, results: &[Value]) -> Value {
    if results.is_empty()
        || results.iter().any(|result| {
            result
                .get("input")
                .or_else(|| result.get("model"))
                .is_none()
        })
    {
        return Value::Null;
    }
    let Ok(steps) = material_exploration_steps_by_id(study) else {
        return Value::Null;
    };
    let candidate_ids = steps
        .iter()
        .filter(|step| step.action.starts_with("solve_"))
        .filter_map(candidate_id_for_step)
        .collect::<Vec<_>>();
    if candidate_ids.len() != results.len() {
        return Value::Null;
    }
    let entries = candidate_ids
        .into_iter()
        .zip(results)
        .map(|(candidate_id, result)| {
            let model = result
                .get("input")
                .or_else(|| result.get("model"))
                .expect("validated model input");
            json!({
                "candidate_id": candidate_id,
                "model_fingerprint": canonical_json_sha256(model),
            })
        })
        .collect::<Vec<_>>();
    json!({
        "schema_version": "kyuubiki.material-candidate-input-manifest/v1",
        "study": study,
        "entries": entries,
    })
}

fn planned_candidate_input_fingerprint(
    exploration: &Value,
    study: &str,
    steps: &[HeadlessWorkflowStep],
) -> String {
    let planned_entries = steps
        .iter()
        .filter(|step| step.action.starts_with("solve_"))
        .map(|step| {
            (
                candidate_id_for_step(step).unwrap_or("unknown").to_string(),
                canonical_json_sha256(candidate_model_for_step(step)),
            )
        })
        .collect::<Vec<_>>();
    let mut entries = candidate_input_manifest_from_exploration(exploration, study)
        .and_then(|manifest| manifest.get("entries").and_then(Value::as_array).cloned())
        .unwrap_or_default();
    for entry in &mut entries {
        let Some(candidate_id) = entry.get("candidate_id").and_then(Value::as_str) else {
            continue;
        };
        if let Some((_, fingerprint)) = planned_entries
            .iter()
            .find(|(planned_id, _)| planned_id == candidate_id)
        {
            entry["model_fingerprint"] = Value::from(fingerprint.clone());
        }
    }
    for (candidate_id, fingerprint) in planned_entries {
        if !entries.iter().any(|entry| {
            entry.get("candidate_id").and_then(Value::as_str) == Some(candidate_id.as_str())
        }) {
            entries.push(json!({
                "candidate_id": candidate_id,
                "model_fingerprint": fingerprint,
            }));
        }
    }
    candidate_input_fingerprint(&json!({
        "schema_version": "kyuubiki.material-candidate-input-manifest/v1",
        "study": study,
        "entries": entries,
    }))
}

fn candidate_model_for_step(step: &HeadlessWorkflowStep) -> &Value {
    step.payload.get("model").unwrap_or(&step.payload)
}

fn candidate_input_fingerprint(manifest: &Value) -> String {
    if manifest
        .get("entries")
        .and_then(Value::as_array)
        .is_none_or(Vec::is_empty)
    {
        String::new()
    } else {
        canonical_json_sha256(manifest)
    }
}

fn material_card_refs_from_report(report: &Value) -> Vec<Value> {
    report
        .get("material_card_refs")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
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
        "repair_validation" => vec![
            "rerun focused candidates and rebuild summary validation before changing the search space"
                .to_string(),
            "only expand candidates after cross-check validation returns to pass".to_string(),
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

fn validation_repair_rationale(violated_gates: &[String]) -> Vec<String> {
    vec![
        "summary validation blocked this material research round".to_string(),
        format!(
            "validation gates require focused rerun before expansion: {}",
            violated_gates.join(", ")
        ),
    ]
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
