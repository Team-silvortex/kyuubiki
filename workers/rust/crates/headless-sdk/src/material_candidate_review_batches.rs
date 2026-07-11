use serde_json::{Value, json};

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

fn push_unique(values: &mut Vec<String>, value: Option<&str>) {
    let Some(value) = value else {
        return;
    };
    if !values.iter().any(|entry| entry == value) {
        values.push(value.to_string());
    }
}
