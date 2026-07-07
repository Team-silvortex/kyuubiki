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

pub(crate) fn material_candidate_draft_summary(drafts: &[Value]) -> Value {
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
        "draft_count": drafts.len(),
        "source_candidate_ids": source_candidate_ids,
        "required_result_schemas": required_result_schemas,
        "strategy_counts": strategy_counts,
    })
}

pub(crate) fn material_candidate_draft_batches(drafts: &[Value]) -> Vec<Value> {
    let mut schemas = Vec::new();
    for draft in drafts {
        push_unique(
            &mut schemas,
            draft.get("required_result_schema").and_then(Value::as_str),
        );
    }
    schemas
        .into_iter()
        .map(|schema| {
            let draft_ids = drafts
                .iter()
                .filter(|draft| {
                    draft.get("required_result_schema").and_then(Value::as_str)
                        == Some(schema.as_str())
                })
                .filter_map(|draft| draft.get("draft_id").and_then(Value::as_str))
                .map(ToString::to_string)
                .collect::<Vec<_>>();
            let review_checklist = review_checklist(&schema);
            json!({
                "schema_version": "kyuubiki.material-candidate-draft-batch/v1",
                "required_result_schema": schema,
                "draft_ids": draft_ids,
                "draft_count": draft_ids.len(),
                "dispatch_action": "materialize_candidate_drafts_and_rerun_solver",
                "execution_policy": {
                    "requires_human_review": true,
                    "auto_materialize_allowed": false,
                    "qualification_claim_allowed": false,
                    "reasons": [
                        "candidate drafts are generated from screening heuristics",
                        "material cards and geometry edits must be reviewed before solver rerun",
                        "draft outputs are not qualification evidence until quality gates pass"
                    ]
                },
                "review_status": review_status(&review_checklist),
                "review_checklist": review_checklist,
                "allowed_review_actions": allowed_review_actions(),
                "review_decision_template": review_decision_template(&draft_ids),
                "review_decision_contract": review_decision_contract(&review_checklist),
                "status": "pending_agent_materialization",
            })
        })
        .collect()
}

fn review_decision_template(draft_ids: &[String]) -> Value {
    json!({
        "schema_version": "kyuubiki.material-candidate-review-decision/v1",
        "batch_draft_ids": draft_ids,
        "action": "approve_for_materialization | request_changes | reject_draft_batch",
        "reviewer": {
            "id": "",
            "display_name": "",
        },
        "reason": "",
        "completed_item_ids": [],
        "requested_changes": [],
        "decided_at": "RFC3339 timestamp",
    })
}

fn review_decision_contract(review_checklist: &[Value]) -> Value {
    let required_item_ids = review_checklist
        .iter()
        .filter(|item| item.get("required").and_then(Value::as_bool) == Some(true))
        .filter_map(|item| item.get("id").and_then(Value::as_str))
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    json!({
        "schema_version": "kyuubiki.material-candidate-review-decision-contract/v1",
        "required_fields": [
            "schema_version",
            "batch_draft_ids",
            "action",
            "reviewer.id",
            "reason",
            "decided_at"
        ],
        "allowed_actions": [
            "approve_for_materialization",
            "request_changes",
            "reject_draft_batch"
        ],
        "approve_requires_completed_item_ids": required_item_ids,
        "request_changes_requires": ["requested_changes"],
        "reject_requires": ["reason"],
        "timestamp_format": "RFC3339",
    })
}

fn allowed_review_actions() -> Vec<Value> {
    vec![
        review_action(
            "approve_for_materialization",
            "Approve draft batch for materialization",
            "all required review checklist items completed",
        ),
        review_action(
            "request_changes",
            "Request draft changes before solver rerun",
            "reviewer identifies missing material, geometry, unit, or gate evidence",
        ),
        review_action(
            "reject_draft_batch",
            "Reject draft batch",
            "draft strategy is unsafe, irrelevant, or outside the study scope",
        ),
    ]
}

fn review_action(id: &str, label: &str, requirement: &str) -> Value {
    json!({
        "id": id,
        "label": label,
        "requires_reviewer_identity": true,
        "requires_reason": true,
        "requirement": requirement,
    })
}

fn review_status(review_checklist: &[Value]) -> Value {
    let missing_item_ids = review_checklist
        .iter()
        .filter(|item| item.get("required").and_then(Value::as_bool) == Some(true))
        .filter_map(|item| item.get("id").and_then(Value::as_str))
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    json!({
        "schema_version": "kyuubiki.material-candidate-review-status/v1",
        "state": "pending_review",
        "blocking": !missing_item_ids.is_empty(),
        "completed_item_ids": [],
        "missing_item_ids": missing_item_ids,
        "blocked_reason": "required review checklist items are incomplete",
    })
}

fn review_checklist(required_result_schema: &str) -> Vec<Value> {
    vec![
        review_item(
            "review.material_cards",
            "Review material cards and source provenance",
            "approved material-card references for every edited material",
        ),
        review_item(
            "review.geometry_delta",
            "Review geometry and stack edits",
            "documented layer, interface, or fixture delta before materialization",
        ),
        review_item(
            "review.units",
            "Confirm SI units",
            "all draft parameters normalized to SI before solver dispatch",
        ),
        review_item(
            "review.result_schema",
            "Confirm solver result contract",
            required_result_schema,
        ),
        review_item(
            "review.quality_gates",
            "Confirm rerun quality gates",
            "electrostatic, heat, thermal, interface, and completeness gates enabled",
        ),
    ]
}

fn review_item(id: &str, label: &str, evidence: &str) -> Value {
    json!({
        "id": id,
        "label": label,
        "required": true,
        "evidence": evidence,
    })
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
