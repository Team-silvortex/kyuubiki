use serde_json::{Value, json};

pub fn build_material_candidate_materialization_plan(
    request: &Value,
    drafts: &[Value],
) -> Result<Value, String> {
    if request.get("status").and_then(Value::as_str) != Some("ready_for_agent_materialization") {
        return Err("materialization request is not ready".to_string());
    }
    let draft_ids = request
        .get("draft_ids")
        .and_then(Value::as_array)
        .ok_or_else(|| "materialization request is missing draft_ids".to_string())?;
    let mut materialized_candidates = Vec::new();
    for draft_id in draft_ids {
        let draft_id = draft_id
            .as_str()
            .ok_or_else(|| "materialization request contains non-string draft_id".to_string())?;
        let draft = find_draft(drafts, draft_id)?;
        materialized_candidates.push(materialized_candidate_spec(draft)?);
    }
    Ok(json!({
        "schema_version": "kyuubiki.material-candidate-materialization-plan/v1",
        "source_request_schema_version": request
            .get("schema_version")
            .and_then(Value::as_str)
            .unwrap_or("unknown"),
        "required_result_schema": request
            .get("required_result_schema")
            .cloned()
            .unwrap_or_else(|| json!("unknown")),
        "materialized_candidate_count": materialized_candidates.len(),
        "materialized_candidates": materialized_candidates,
        "status": "ready_for_solver_rerun",
    }))
}

fn find_draft<'a>(drafts: &'a [Value], draft_id: &str) -> Result<&'a Value, String> {
    drafts
        .iter()
        .find(|draft| draft.get("draft_id").and_then(Value::as_str) == Some(draft_id))
        .ok_or_else(|| format!("materialization request references unknown draft_id: {draft_id}"))
}

fn materialized_candidate_spec(draft: &Value) -> Result<Value, String> {
    let draft_id = required_str(draft, "draft_id")?;
    let source_candidate_id = required_str(draft, "source_candidate_id")?;
    let strategy = required_str(draft, "strategy")?;
    Ok(json!({
        "schema_version": "kyuubiki.materialized-candidate-spec/v1",
        "candidate_id": format!("{source_candidate_id}__{strategy}"),
        "source_draft_id": draft_id,
        "source_candidate_id": source_candidate_id,
        "strategy": strategy,
        "study": draft.get("study").cloned().unwrap_or_else(|| json!("unknown")),
        "required_result_schema": draft
            .get("required_result_schema")
            .cloned()
            .unwrap_or_else(|| json!("unknown")),
        "changes": draft.get("changes").cloned().unwrap_or_else(|| json!([])),
        "expected_effects": draft
            .get("expected_effects")
            .cloned()
            .unwrap_or_else(|| json!([])),
        "status": "requires_solver_rerun",
    }))
}

fn required_str<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("draft is missing {key}"))
}
