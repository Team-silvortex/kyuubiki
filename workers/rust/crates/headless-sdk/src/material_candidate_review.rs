use serde_json::{Value, json};

pub fn apply_material_candidate_review_decision(
    batch: &Value,
    decision: &Value,
) -> Result<Value, String> {
    validate_review_decision(batch, decision)?;
    let action = decision
        .get("action")
        .and_then(Value::as_str)
        .ok_or_else(|| "review decision is missing action".to_string())?;
    let mut reviewed = batch.clone();
    reviewed["review_status"] = review_status_for_decision(batch, decision, action);
    reviewed["last_review_decision"] = decision.clone();
    reviewed["status"] = json!(match action {
        "approve_for_materialization" => "approved_for_materialization",
        "request_changes" => "changes_requested",
        "reject_draft_batch" => "rejected",
        _ => "review_failed",
    });
    Ok(reviewed)
}

pub fn build_material_candidate_materialization_request(
    approved_batch: &Value,
) -> Result<Value, String> {
    if approved_batch.get("status").and_then(Value::as_str) != Some("approved_for_materialization")
    {
        return Err("draft batch is not approved for materialization".to_string());
    }
    if approved_batch
        .get("review_status")
        .and_then(|status| status.get("blocking"))
        .and_then(Value::as_bool)
        .unwrap_or(true)
    {
        return Err("draft batch review is still blocking".to_string());
    }
    Ok(json!({
        "schema_version": "kyuubiki.material-candidate-materialization-request/v1",
        "source_batch_schema_version": approved_batch
            .get("schema_version")
            .and_then(Value::as_str)
            .unwrap_or("unknown"),
        "required_result_schema": approved_batch
            .get("required_result_schema")
            .cloned()
            .unwrap_or_else(|| json!("unknown")),
        "draft_ids": approved_batch
            .get("draft_ids")
            .cloned()
            .unwrap_or_else(|| json!([])),
        "dispatch_action": approved_batch
            .get("dispatch_action")
            .cloned()
            .unwrap_or_else(|| json!("materialize_candidate_drafts_and_rerun_solver")),
        "review_decision": approved_batch
            .get("last_review_decision")
            .cloned()
            .unwrap_or_else(|| json!(null)),
        "status": "ready_for_agent_materialization",
    }))
}

fn validate_review_decision(batch: &Value, decision: &Value) -> Result<(), String> {
    let action = required_str(decision, "action")?;
    if !matches!(
        action,
        "approve_for_materialization" | "request_changes" | "reject_draft_batch"
    ) {
        return Err(format!("unsupported review action: {action}"));
    }
    if decision
        .get("reviewer")
        .and_then(|reviewer| reviewer.get("id"))
        .and_then(Value::as_str)
        .filter(|id| !id.trim().is_empty())
        .is_none()
    {
        return Err("review decision is missing reviewer.id".to_string());
    }
    if required_str(decision, "reason")?.trim().is_empty() {
        return Err("review decision is missing reason".to_string());
    }
    if action == "approve_for_materialization" {
        validate_completed_review_items(batch, decision)?;
    }
    Ok(())
}

fn validate_completed_review_items(batch: &Value, decision: &Value) -> Result<(), String> {
    let required = batch
        .get("review_status")
        .and_then(|status| status.get("missing_item_ids"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let completed = decision
        .get("completed_item_ids")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for item in required {
        if !completed.iter().any(|done| done == &item) {
            return Err(format!(
                "review decision is missing completed checklist item: {}",
                item.as_str().unwrap_or("unknown")
            ));
        }
    }
    Ok(())
}

fn review_status_for_decision(batch: &Value, decision: &Value, action: &str) -> Value {
    let completed_item_ids = decision
        .get("completed_item_ids")
        .cloned()
        .unwrap_or_else(|| json!([]));
    match action {
        "approve_for_materialization" => json!({
            "schema_version": "kyuubiki.material-candidate-review-status/v1",
            "state": "approved_for_materialization",
            "blocking": false,
            "completed_item_ids": completed_item_ids,
            "missing_item_ids": [],
            "blocked_reason": null,
        }),
        "request_changes" => blocked_review_status(batch, decision, "changes_requested"),
        "reject_draft_batch" => blocked_review_status(batch, decision, "rejected"),
        _ => blocked_review_status(batch, decision, "review_failed"),
    }
}

fn blocked_review_status(batch: &Value, decision: &Value, state: &str) -> Value {
    json!({
        "schema_version": "kyuubiki.material-candidate-review-status/v1",
        "state": state,
        "blocking": true,
        "completed_item_ids": decision
            .get("completed_item_ids")
            .cloned()
            .unwrap_or_else(|| json!([])),
        "missing_item_ids": batch
            .get("review_status")
            .and_then(|status| status.get("missing_item_ids"))
            .cloned()
            .unwrap_or_else(|| json!([])),
        "blocked_reason": decision
            .get("reason")
            .cloned()
            .unwrap_or_else(|| json!("review decision did not approve materialization")),
    })
}

fn required_str<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("review decision is missing {key}"))
}
