use serde_json::Value;

pub fn compose_quality_lineage_report(payload: Value, _config: Value) -> Result<Value, String> {
    let object = payload.as_object().ok_or_else(|| {
        "transform.compose_quality_lineage_report expects an object payload".to_string()
    })?;
    let ranking = object.get("ranking").unwrap_or(&Value::Null);
    let request = object.get("request").unwrap_or(&Value::Null);
    let plan = object.get("plan").unwrap_or(&Value::Null);
    let cases = object.get("cases").unwrap_or(&Value::Null);
    let first_case_metadata = cases
        .get("cases")
        .and_then(Value::as_array)
        .and_then(|items| items.first())
        .and_then(|case| case.get("metadata"))
        .cloned()
        .unwrap_or(Value::Null);
    let seed_metadata = request
        .get("request_payload")
        .and_then(|payload| payload.get("seed_metadata"))
        .or_else(|| request.get("selected_candidate_metadata"))
        .cloned()
        .unwrap_or(Value::Null);
    let optimization_hint = request
        .get("request_payload")
        .and_then(|payload| payload.get("optimization_hint"))
        .or_else(|| request.get("selected_iteration_hint"))
        .or_else(|| plan.get("optimization_hint"))
        .cloned()
        .unwrap_or(Value::Null);
    let coupled_readiness = request
        .get("request_payload")
        .and_then(|payload| payload.get("coupled_readiness"))
        .or_else(|| request.get("selected_coupled_readiness"))
        .or_else(|| plan.get("coupled_readiness"))
        .or_else(|| first_case_metadata.get("coupled_readiness"))
        .cloned()
        .unwrap_or(Value::Null);
    let focused_axis_path = plan
        .get("focused_axis_path")
        .or_else(|| first_case_metadata.get("focused_axis_path"))
        .cloned()
        .unwrap_or(Value::Null);
    let selected_candidate_id = request
        .get("selected_candidate_id")
        .or_else(|| ranking.get("best_candidate_id"))
        .or_else(|| plan.get("source_candidate_id"))
        .cloned()
        .unwrap_or(Value::Null);
    let case_count = cases
        .get("case_count")
        .or_else(|| plan.get("case_count_estimate"))
        .cloned()
        .unwrap_or(Value::Null);
    let expansion_budget_ready = cases
        .get("expansion_budget_ready")
        .or_else(|| plan.get("expansion_budget_ready"))
        .cloned()
        .unwrap_or(Value::Null);
    let expansion_blocking_reason = cases
        .get("expansion_blocking_reason")
        .or_else(|| plan.get("expansion_blocking_reason"))
        .cloned()
        .unwrap_or(Value::Null);
    let sweep_budget = plan
        .get("sweep_budget")
        .or_else(|| cases.get("sweep_budget"))
        .or_else(|| first_case_metadata.get("sweep_budget"))
        .cloned()
        .unwrap_or(Value::Null);
    let budget_blocked = expansion_budget_ready.as_bool() == Some(false);
    let repair_plan = quality_lineage_repair_plan(request, &optimization_hint, &coupled_readiness);
    let missing_fields = quality_lineage_missing_fields(
        &selected_candidate_id,
        &seed_metadata,
        &optimization_hint,
        &first_case_metadata,
        budget_blocked,
    );
    let lineage_complete = !selected_candidate_id.is_null()
        && !seed_metadata.is_null()
        && !optimization_hint.is_null()
        && (!first_case_metadata.is_null() || budget_blocked);

    Ok(serde_json::json!({
        "quality_lineage_report_contract": "kyuubiki.quality_lineage_report/v1",
        "selected_candidate_id": selected_candidate_id,
        "selected_candidate_ready": request.get("selected_candidate_ready")
            .or_else(|| ranking.get("best_candidate_ready"))
            .cloned()
            .unwrap_or(Value::Null),
        "seed_metadata": seed_metadata,
        "optimization_hint": optimization_hint,
        "coupled_readiness": coupled_readiness,
        "focused_axis_path": focused_axis_path,
        "case_count": case_count,
        "expansion_budget_ready": expansion_budget_ready,
        "expansion_blocking_reason": expansion_blocking_reason,
        "sweep_budget": sweep_budget,
        "first_case_metadata": first_case_metadata,
        "repair_plan": repair_plan,
        "lineage_complete": lineage_complete,
        "lineage_missing_fields": missing_fields,
        "lineage_summary": format!(
            "Quality lineage {}: selected={}, focused_axis={}, missing={}.",
            if lineage_complete { "complete" } else { "partial" },
            string_or_unknown(&selected_candidate_id),
            string_or_unknown(&focused_axis_path),
            missing_fields.len()
        ),
    }))
}

fn quality_lineage_repair_plan(
    request: &Value,
    optimization_hint: &Value,
    coupled_readiness: &Value,
) -> Value {
    let action = optimization_hint
        .get("action")
        .and_then(Value::as_str)
        .unwrap_or("none");
    let blocking_terms = request
        .get("selected_blocking_terms")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let validation_blocker = blocking_terms
        .iter()
        .find(|term| term.get("domain").and_then(Value::as_str) == Some("validation"));

    if action == "fix_validation_failure" {
        let source_terms = validation_blocker
            .and_then(|term| term.get("source_blocking_terms"))
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        return serde_json::json!({
            "repair_action": "fix_validation_failure",
            "repair_domain": "validation",
            "focus_field": optimization_hint.get("focus_field").cloned().unwrap_or(Value::Null),
            "focus_source": optimization_hint.get("focus_source").cloned().unwrap_or(Value::Null),
            "blocking_count": optimization_hint.get("blocking_count").cloned().unwrap_or(Value::Null),
            "source_blocking_terms": source_terms,
            "recommended_next_step": "rerun_cross_validation_or_adjust_candidate_inputs",
        });
    }

    if action == "fix_coupled_readiness" || action == "review_coupled_readiness" {
        return serde_json::json!({
            "repair_action": action,
            "repair_domain": optimization_hint.get("focus_domain").cloned().unwrap_or(Value::Null),
            "focus_field": optimization_hint.get("focus_field").cloned().unwrap_or(Value::Null),
            "focus_source": optimization_hint.get("focus_source").cloned().unwrap_or(Value::Null),
            "blocking_count": optimization_hint.get("blocking_count").cloned().unwrap_or(Value::Null),
            "warning_count": optimization_hint.get("warning_count").cloned().unwrap_or(Value::Null),
            "source_blocking_terms": [],
            "coupled_blocking_domains": coupled_readiness
                .get("coupled_readiness_blocking_domains")
                .cloned()
                .unwrap_or(Value::Array(Vec::new())),
            "coupled_required_missing": coupled_readiness
                .get("coupled_readiness_required_missing")
                .cloned()
                .unwrap_or(Value::Array(Vec::new())),
            "coupled_warning_domains": coupled_readiness
                .get("coupled_readiness_warning_domains")
                .cloned()
                .unwrap_or(Value::Array(Vec::new())),
            "readiness_state": coupled_readiness
                .get("coupled_readiness_state")
                .cloned()
                .unwrap_or(Value::Null),
            "readiness_recommendation": coupled_readiness
                .get("coupled_readiness_recommendation")
                .cloned()
                .unwrap_or(Value::Null),
            "recommended_next_step": if action == "fix_coupled_readiness" {
                "repair_coupled_domain_inputs_before_next_sweep"
            } else {
                "review_coupled_domain_warnings_before_next_sweep"
            },
        });
    }

    serde_json::json!({
        "repair_action": action,
        "repair_domain": optimization_hint.get("focus_domain").cloned().unwrap_or(Value::Null),
        "focus_field": optimization_hint.get("focus_field").cloned().unwrap_or(Value::Null),
        "focus_source": optimization_hint.get("focus_source").cloned().unwrap_or(Value::Null),
        "blocking_count": optimization_hint.get("blocking_count").cloned().unwrap_or(Value::Null),
        "source_blocking_terms": [],
        "recommended_next_step": if action == "none" { "none" } else { "continue_quality_iteration" },
    })
}

fn quality_lineage_missing_fields(
    selected_candidate_id: &Value,
    seed_metadata: &Value,
    optimization_hint: &Value,
    first_case_metadata: &Value,
    budget_blocked: bool,
) -> Vec<&'static str> {
    let mut missing = Vec::new();
    if selected_candidate_id.is_null() {
        missing.push("selected_candidate_id");
    }
    if seed_metadata.is_null() {
        missing.push("seed_metadata");
    }
    if optimization_hint.is_null() {
        missing.push("optimization_hint");
    }
    if first_case_metadata.is_null() && !budget_blocked {
        missing.push("first_case_metadata");
    }
    missing
}

fn string_or_unknown(value: &Value) -> String {
    value
        .as_str()
        .map(ToString::to_string)
        .unwrap_or_else(|| "unknown".to_string())
}
