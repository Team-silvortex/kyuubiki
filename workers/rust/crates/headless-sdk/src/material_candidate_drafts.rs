use serde_json::{Value, json};

pub(crate) fn material_candidate_drafts(
    decision: &str,
    study: &str,
    report: &Value,
    focus_candidate_ids: &[String],
) -> Vec<Value> {
    if decision != "mitigate_design_risk" {
        return Vec::new();
    }
    match study {
        "material_composite_thermo_electric_panel" => {
            composite_candidate_drafts(report, focus_candidate_ids)
        }
        _ => generic_risk_candidate_drafts(study, focus_candidate_ids),
    }
}

pub(crate) fn material_candidate_draft_summary(decision: &str, drafts: &[Value]) -> Value {
    let mut source_candidate_ids = Vec::new();
    let mut required_result_schemas = Vec::new();
    let mut strategy_counts = serde_json::Map::new();
    for draft in drafts {
        push_unique(
            &mut source_candidate_ids,
            draft.get("source_candidate_id").and_then(Value::as_str),
        );
        push_unique(
            &mut required_result_schemas,
            draft.get("required_result_schema").and_then(Value::as_str),
        );
        let strategy = draft
            .get("strategy")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let next_count = strategy_counts
            .get(strategy)
            .and_then(Value::as_u64)
            .unwrap_or(0)
            + 1;
        strategy_counts.insert(strategy.to_string(), json!(next_count));
    }
    json!({
        "schema_version": "kyuubiki.material-candidate-draft-summary/v1",
        "source_decision": decision,
        "draft_count": drafts.len(),
        "generation_state": draft_generation_state(decision, drafts),
        "generation_reason": draft_generation_reason(decision, drafts),
        "source_candidate_ids": source_candidate_ids,
        "required_result_schemas": required_result_schemas,
        "strategy_counts": strategy_counts,
    })
}

fn draft_generation_state(decision: &str, drafts: &[Value]) -> &'static str {
    if !drafts.is_empty() {
        "generated"
    } else if decision == "repair_validation" {
        "blocked_until_validation_passes"
    } else if decision == "repair_or_rerun" {
        "blocked_until_missing_metrics_repaired"
    } else if decision == "expand_around_winner" {
        "not_required_for_builtin_expansion"
    } else {
        "not_generated"
    }
}

fn draft_generation_reason(decision: &str, drafts: &[Value]) -> &'static str {
    if !drafts.is_empty() {
        "candidate drafts are available for review before materialization"
    } else if decision == "repair_validation" {
        "summary validation must pass before generating new material candidates"
    } else if decision == "repair_or_rerun" {
        "missing result metrics must be repaired before generating new material candidates"
    } else if decision == "expand_around_winner" {
        "built-in study expansion reruns the study template directly"
    } else {
        "no candidate draft strategy is registered for this decision"
    }
}

fn composite_candidate_drafts(report: &Value, focus_candidate_ids: &[String]) -> Vec<Value> {
    report
        .get("candidates")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|candidate| focused(candidate, focus_candidate_ids))
        .flat_map(composite_drafts_for_candidate)
        .collect()
}

fn composite_drafts_for_candidate(candidate: &Value) -> Vec<Value> {
    let candidate_id = candidate
        .get("candidate_id")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let driver = candidate
        .get("weakest_interface")
        .and_then(|interface| interface.get("dominant_driver"))
        .and_then(Value::as_str)
        .unwrap_or("quality_gate_violation");
    let mut drafts = Vec::new();
    if driver.contains("thermal_expansion") {
        drafts.push(composite_draft(
            candidate_id,
            "add_compliant_interlayer",
            vec![
                "insert thin compliant adhesive between conductor and dielectric",
                "prefer intermediate CTE material card before rerun",
            ],
            vec![
                "reduce interface risk score",
                "preserve current electrical stack topology",
            ],
        ));
        drafts.push(composite_draft(
            candidate_id,
            "replace_high_cte_dielectric",
            vec![
                "screen dielectric alternatives with lower CTE mismatch",
                "keep breakdown safety factor above incumbent warning threshold",
            ],
            vec![
                "lower thermal expansion mismatch",
                "trade mass and dielectric margin explicitly",
            ],
        ));
    } else {
        drafts.push(composite_draft(
            candidate_id,
            "reduce_stiffness_contrast",
            vec![
                "screen substrate or dielectric with closer Young modulus",
                "relax local fixture assumptions before qualification",
            ],
            vec![
                "lower stiffness contrast",
                "reduce peak thermal stress sensitivity",
            ],
        ));
    }
    drafts
}

fn composite_draft(
    source_candidate_id: &str,
    strategy: &str,
    changes: Vec<&str>,
    expected_effects: Vec<&str>,
) -> Value {
    json!({
        "schema_version": "kyuubiki.material-candidate-draft/v1",
        "draft_id": format!("{source_candidate_id}.{strategy}.draft"),
        "study": "material_composite_thermo_electric_panel",
        "source_candidate_id": source_candidate_id,
        "strategy": strategy,
        "status": "draft_requires_solver_rerun",
        "lineage": {
            "source_candidate_id": source_candidate_id,
            "generation": "risk_mitigation_neighbor",
            "parent_decision": "mitigate_design_risk"
        },
        "required_result_schema": "kyuubiki.composite-thermo-electric-panel-result/v1",
        "changes": changes,
        "expected_effects": expected_effects,
        "constraints": [
            "keep all units in SI",
            "do not treat this draft as qualified material data",
            "rerun electrostatic, heat, thermal, and interface gates"
        ]
    })
}

fn generic_risk_candidate_drafts(study: &str, focus_candidate_ids: &[String]) -> Vec<Value> {
    focus_candidate_ids
        .iter()
        .map(|candidate_id| {
            json!({
                "schema_version": "kyuubiki.material-candidate-draft/v1",
                "draft_id": format!("{candidate_id}.generate_conservative_neighbor.draft"),
                "study": study,
                "source_candidate_id": candidate_id,
                "strategy": "generate_conservative_neighbor",
                "status": "draft_requires_solver_rerun",
                "lineage": {
                    "source_candidate_id": candidate_id,
                    "generation": "risk_mitigation_neighbor",
                    "parent_decision": "mitigate_design_risk"
                },
                "required_result_schema": "kyuubiki.material-result-payload/v1",
                "changes": [
                    "perturb only one risk-driving parameter at a time",
                    "preserve incumbent objective and quality gates"
                ],
                "expected_effects": [
                    "isolate quality gate sensitivity",
                    "avoid expanding around an unsafe incumbent blindly"
                ],
                "constraints": [
                    "keep all units in SI",
                    "rerun the full focused quality batch"
                ]
            })
        })
        .collect()
}

fn push_unique(values: &mut Vec<String>, value: Option<&str>) {
    let Some(value) = value else {
        return;
    };
    if !values.iter().any(|entry| entry == value) {
        values.push(value.to_string());
    }
}

fn focused(candidate: &Value, focus_candidate_ids: &[String]) -> bool {
    let Some(candidate_id) = candidate.get("candidate_id").and_then(Value::as_str) else {
        return false;
    };
    focus_candidate_ids.iter().any(|id| id == candidate_id)
}
