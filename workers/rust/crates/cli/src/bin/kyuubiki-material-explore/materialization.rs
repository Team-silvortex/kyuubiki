use kyuubiki_headless_sdk::{
    apply_material_candidate_review_decision, build_composite_materialized_candidate_report,
    build_composite_materialized_candidate_steps, build_material_candidate_materialization_plan,
    build_material_candidate_materialization_request, build_material_exploration_next_round_plan,
};
use serde_json::Value;

pub(crate) fn approve_review_template(
    template_path: &str,
    reviewer_id: &str,
    reviewer_name: &str,
    reason: &str,
    decided_at: &str,
) -> Result<Value, String> {
    let template = read_review_template(template_path)?;
    Ok(serde_json::json!({
        "schema_version": "kyuubiki.material-candidate-review-decision/v1",
        "batch_draft_ids": template
            .get("draft_ids")
            .cloned()
            .or_else(|| template
                .get("review_decision_template")
                .and_then(|decision| decision.get("batch_draft_ids"))
                .cloned())
            .unwrap_or_else(|| serde_json::json!([])),
        "action": "approve_for_materialization",
        "reviewer": {
            "id": reviewer_id,
            "display_name": reviewer_name
        },
        "reason": reason,
        "completed_item_ids": completed_review_item_ids(&template),
        "requested_changes": [],
        "decided_at": decided_at
    }))
}

pub(crate) fn review_decision_template(plan_path: &str) -> Result<Value, String> {
    let execution_plan = read_next_round_execution_plan(plan_path)?;
    let batch = first_review_batch(&execution_plan)?;
    Ok(serde_json::json!({
        "schema_version": "kyuubiki.material-review-template-export/v1",
        "source_schema_version": execution_plan
            .get("schema_version")
            .and_then(Value::as_str)
            .unwrap_or("unknown"),
        "batch_id": batch.get("batch_id").cloned().unwrap_or(Value::Null),
        "draft_ids": batch.get("draft_ids").cloned().unwrap_or_else(|| serde_json::json!([])),
        "review_checklist": batch
            .get("review_checklist")
            .cloned()
            .unwrap_or_else(|| serde_json::json!([])),
        "review_status": batch.get("review_status").cloned().unwrap_or(Value::Null),
        "review_decision_template": batch
            .get("review_decision_template")
            .cloned()
            .ok_or_else(|| "draft batch is missing review_decision_template".to_string())?,
        "review_decision_contract": batch
            .get("review_decision_contract")
            .cloned()
            .ok_or_else(|| "draft batch is missing review_decision_contract".to_string())?,
        "notes": [
            "fill reviewer.id, reason, completed_item_ids, and decided_at before materialization",
            "this export does not approve or materialize candidates by itself"
        ]
    }))
}

pub(crate) fn materialize_reviewed_candidates(
    plan_path: &str,
    decision_path: &str,
) -> Result<Value, String> {
    let execution_plan = read_next_round_execution_plan(plan_path)?;
    let decision = crate::read_json_file(decision_path)?;
    let batch = select_review_batch(&execution_plan, &decision)?;
    let approved = apply_material_candidate_review_decision(batch, &decision)?;
    let request = build_material_candidate_materialization_request(&approved)?;
    let drafts = execution_plan
        .get("candidate_drafts")
        .and_then(Value::as_array)
        .cloned()
        .ok_or_else(|| "next-round plan is missing candidate_drafts".to_string())?;
    let materialization_plan = build_material_candidate_materialization_plan(&request, &drafts)?;
    Ok(serde_json::json!({
        "schema_version": "kyuubiki.materialization-reviewed-plan/v1",
        "source_schema_version": execution_plan
            .get("schema_version")
            .and_then(Value::as_str)
            .unwrap_or("unknown"),
        "reviewed_batch": approved,
        "materialization_request": request,
        "materialization_plan": materialization_plan
    }))
}

pub(crate) fn run_materialized_candidates(path: &str) -> Result<Value, String> {
    let plan = read_materialization_plan(path)?;
    let steps = build_composite_materialized_candidate_steps(&plan)?;
    let result_payloads = steps
        .iter()
        .map(crate::run_solve_step)
        .collect::<Result<Vec<_>, _>>()?;
    let report = build_composite_materialized_candidate_report(&result_payloads)?;
    let next_round = build_material_exploration_next_round_plan(&report, 1);
    Ok(serde_json::json!({
        "schema_version": "kyuubiki.materialized-candidate-rerun/v1",
        "mode": "local_solver_materialized_rerun",
        "study": "material_composite_thermo_electric_panel",
        "step_count": steps.len(),
        "result_payloads": result_payloads,
        "report": report,
        "next_round": next_round
    }))
}

pub(crate) fn print_materialized_rerun_summary(payload: &Value) {
    println!(
        "Materialized rerun: {}",
        payload["study"].as_str().unwrap_or("unknown")
    );
    println!("Steps: {}", payload["step_count"].as_u64().unwrap_or(0));
    if let Some(winner) = payload["report"]["winner_candidate_id"].as_str() {
        println!("Winner: {winner}");
    }
    if let Some(decision) = payload["next_round"]["decision"].as_str() {
        println!("Next round: {decision}");
    }
}

pub(crate) fn print_review_template_summary(payload: &Value) {
    println!(
        "Review template: {}",
        payload["batch_id"].as_str().unwrap_or("unknown")
    );
    println!(
        "Drafts: {}",
        payload["draft_ids"].as_array().map(Vec::len).unwrap_or(0)
    );
}

pub(crate) fn print_review_decision_summary(payload: &Value) {
    println!(
        "Review decision: {}",
        payload["action"].as_str().unwrap_or("unknown")
    );
    println!(
        "Drafts: {}",
        payload["batch_draft_ids"]
            .as_array()
            .map(Vec::len)
            .unwrap_or(0)
    );
}

pub(crate) fn print_materialization_summary(payload: &Value) {
    let plan = &payload["materialization_plan"];
    println!(
        "Materialization plan: {}",
        plan["status"].as_str().unwrap_or("unknown")
    );
    println!(
        "Candidates: {}",
        plan["materialized_candidate_count"].as_u64().unwrap_or(0)
    );
}

pub(crate) fn required_flag<'a>(value: &'a Option<String>, flag: &str) -> Result<&'a str, String> {
    value
        .as_deref()
        .filter(|text| !text.trim().is_empty())
        .ok_or_else(|| format!("--approve-review-template requires {flag}"))
}

fn read_next_round_execution_plan(path: &str) -> Result<Value, String> {
    let payload = crate::read_json_file(path)?;
    Ok(payload
        .get("next_round_execution_plan")
        .cloned()
        .or_else(|| payload.get("execution_plan").cloned())
        .or_else(|| payload.get("plan").cloned())
        .unwrap_or(payload))
}

fn read_review_template(path: &str) -> Result<Value, String> {
    let payload = crate::read_json_file(path)?;
    Ok(payload
        .get("review_template")
        .cloned()
        .or_else(|| payload.get("template").cloned())
        .unwrap_or(payload))
}

fn read_materialization_plan(path: &str) -> Result<Value, String> {
    let payload = crate::read_json_file(path)?;
    Ok(payload
        .get("materialization_plan")
        .cloned()
        .or_else(|| payload.get("plan").cloned())
        .unwrap_or(payload))
}

fn completed_review_item_ids(template: &Value) -> Value {
    template
        .get("review_status")
        .and_then(|status| status.get("missing_item_ids"))
        .cloned()
        .or_else(|| {
            template
                .get("review_decision_contract")
                .and_then(|contract| contract.get("approve_requires_completed_item_ids"))
                .cloned()
        })
        .unwrap_or_else(|| serde_json::json!([]))
}

fn select_review_batch<'a>(plan: &'a Value, decision: &Value) -> Result<&'a Value, String> {
    let decision_ids = decision
        .get("batch_draft_ids")
        .and_then(Value::as_array)
        .ok_or_else(|| "review decision is missing batch_draft_ids".to_string())?;
    plan.get("draft_execution_batches")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find(|batch| batch.get("draft_ids").and_then(Value::as_array) == Some(decision_ids))
        .ok_or_else(|| "no draft execution batch matches review decision".to_string())
}

fn first_review_batch(plan: &Value) -> Result<&Value, String> {
    plan.get("draft_execution_batches")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .next()
        .ok_or_else(|| "next-round plan has no draft execution batches".to_string())
}
