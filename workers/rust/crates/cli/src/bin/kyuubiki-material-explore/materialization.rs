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
    ensure_materialization_schema(&plan)?;
    ensure_materialization_ready(&plan)?;
    ensure_materialized_candidates(&plan)?;
    let steps = build_composite_materialized_candidate_steps(&plan)?;
    let result_payloads = steps
        .iter()
        .map(crate::run_solve_step)
        .collect::<Result<Vec<_>, _>>()?;
    let report = build_composite_materialized_candidate_report(&result_payloads)?;
    let next_round = build_material_exploration_next_round_plan(&report, 1);
    Ok(serde_json::json!({
        "schema_version": "kyuubiki.materialized-candidate-rerun/v1",
        "source_materialization_schema_version": plan
            .get("schema_version")
            .and_then(Value::as_str)
            .unwrap_or("unknown"),
        "source_materialization_status": plan
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("unknown"),
        "mode": "local_solver_materialized_rerun",
        "study": "material_composite_thermo_electric_panel",
        "step_count": steps.len(),
        "materialized_candidate_ids": materialized_candidate_ids(&plan),
        "result_payloads": result_payloads,
        "report": report,
        "next_round": next_round
    }))
}

pub(crate) fn print_materialized_rerun_summary(payload: &Value) {
    for line in materialized_rerun_summary_lines(payload) {
        println!("{line}");
    }
}

pub(crate) fn materialized_rerun_summary_lines(payload: &Value) -> Vec<String> {
    let mut lines = vec![
        format!(
            "Materialized rerun: {}",
            payload["study"].as_str().unwrap_or("unknown")
        ),
        format!(
            "Source plan: {} ({})",
            payload["source_materialization_schema_version"]
                .as_str()
                .unwrap_or("unknown"),
            payload["source_materialization_status"]
                .as_str()
                .unwrap_or("unknown")
        ),
        format!("Steps: {}", payload["step_count"].as_u64().unwrap_or(0)),
        format!(
            "Candidates: {}",
            payload["materialized_candidate_ids"]
                .as_array()
                .map(Vec::len)
                .unwrap_or(0)
        ),
    ];
    if let Some(winner) = payload["report"]["winner_candidate_id"].as_str() {
        lines.push(format!("Winner: {winner}"));
    }
    if let Some(decision) = payload["next_round"]["decision"].as_str() {
        lines.push(format!("Next round: {decision}"));
    }
    lines
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

fn ensure_materialization_schema(plan: &Value) -> Result<(), String> {
    let schema = plan
        .get("schema_version")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    if schema == "kyuubiki.material-candidate-materialization-plan/v1" {
        Ok(())
    } else {
        Err(format!(
            "materialization plan schema_version must be kyuubiki.material-candidate-materialization-plan/v1, got {schema}"
        ))
    }
}

fn ensure_materialization_ready(plan: &Value) -> Result<(), String> {
    let status = plan
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    if status == "ready_for_solver_rerun" {
        Ok(())
    } else {
        Err(format!(
            "materialization plan status must be ready_for_solver_rerun, got {status}"
        ))
    }
}

fn ensure_materialized_candidates(plan: &Value) -> Result<(), String> {
    let candidates = plan
        .get("materialized_candidates")
        .and_then(Value::as_array)
        .ok_or_else(|| "materialization plan is missing materialized_candidates".to_string())?;
    let count = candidates.len();
    if count == 0 {
        return Err("materialization plan has no materialized candidates".to_string());
    }
    let declared_count = plan
        .get("materialized_candidate_count")
        .and_then(Value::as_u64)
        .ok_or_else(|| {
            "materialization plan is missing materialized_candidate_count".to_string()
        })?;
    if declared_count as usize != count {
        return Err(format!(
            "materialization plan materialized_candidate_count must match materialized_candidates length ({declared_count} != {count})"
        ));
    }
    for (index, candidate) in candidates.iter().enumerate() {
        ensure_materialized_candidate_spec(candidate, index)?;
    }
    Ok(())
}

fn ensure_materialized_candidate_spec(candidate: &Value, index: usize) -> Result<(), String> {
    let context = format!("materialized_candidates[{index}]");
    let schema = required_candidate_str(candidate, "schema_version", &context)?;
    if schema != "kyuubiki.materialized-candidate-spec/v1" {
        return Err(format!(
            "{context}.schema_version must be kyuubiki.materialized-candidate-spec/v1, got {schema}"
        ));
    }
    let status = required_candidate_str(candidate, "status", &context)?;
    if status != "requires_solver_rerun" {
        return Err(format!(
            "{context}.status must be requires_solver_rerun, got {status}"
        ));
    }
    for field in [
        "candidate_id",
        "source_draft_id",
        "source_candidate_id",
        "strategy",
        "study",
        "required_result_schema",
    ] {
        required_candidate_str(candidate, field, &context)?;
    }
    Ok(())
}

fn required_candidate_str<'a>(
    candidate: &'a Value,
    field: &str,
    context: &str,
) -> Result<&'a str, String> {
    candidate
        .get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("{context} is missing {field}"))
}

fn materialized_candidate_ids(plan: &Value) -> Vec<String> {
    plan.get("materialized_candidates")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|candidate| candidate.get("candidate_id").and_then(Value::as_str))
        .map(ToString::to_string)
        .collect()
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
