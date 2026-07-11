use crate::workflow_quality_terms::{
    CandidateRank, QualityTerm, candidate_entries, composite_blocking_terms, composite_grade,
    config_number, dominant_composite_term, next_round_action, quality_entries,
    quality_iteration_hint, quality_term,
};
use serde_json::Value;

pub fn compose_quality_objective(payload: Value, config: Value) -> Result<Value, String> {
    let object = payload.as_object().ok_or_else(|| {
        "transform.compose_quality_objective expects an object payload".to_string()
    })?;
    let entries = quality_entries(object)?;
    let missing_metric_penalty = config_number(&config, "missing_metric_penalty", 5.0);
    let not_ready_penalty = config_number(&config, "not_ready_penalty", 25.0);
    let max_ready_score = config_number(&config, "max_ready_score", 12.0);

    let mut terms = Vec::new();
    let mut total = 0.0;
    let mut missing_metric_count = 0u64;
    let mut watch_count = 0u64;
    let mut blocked_term_count = 0u64;
    for (source_id, summary) in entries {
        let term = quality_term(
            source_id,
            summary,
            &config,
            missing_metric_penalty,
            not_ready_penalty,
        )?;
        total += term.contribution;
        missing_metric_count += term.missing_metric_count;
        watch_count += term.watch_count;
        if !term.ready {
            blocked_term_count += 1;
        }
        terms.push(term);
    }

    if terms.is_empty() {
        return Err(
            "transform.compose_quality_objective did not find quality score summaries".to_string(),
        );
    }

    let grade = composite_grade(total, blocked_term_count, max_ready_score);
    let dominant_term = dominant_composite_term(&terms);
    let blocking_terms = composite_blocking_terms(&terms);
    let term_values = terms
        .into_iter()
        .map(QualityTerm::into_value)
        .collect::<Vec<_>>();
    Ok(serde_json::json!({
        "composite_quality_contract": "kyuubiki.composite_quality_objective/v1",
        "composite_quality_score": total,
        "composite_quality_grade": grade,
        "composite_quality_ready": grade != "block",
        "composite_quality_term_count": term_values.len(),
        "composite_quality_missing_metric_count": missing_metric_count,
        "composite_quality_watch_count": watch_count,
        "composite_quality_blocked_term_count": blocked_term_count,
        "composite_quality_max_ready_score": max_ready_score,
        "composite_quality_dominant_term": dominant_term,
        "composite_quality_blocking_terms": blocking_terms,
        "composite_quality_terms": term_values,
        "composite_quality_summary": format!(
            "Composite quality {grade}: score={total:.4}, blocked_terms={blocked_term_count}, missing_metrics={missing_metric_count}, watch={watch_count}."
        ),
    }))
}

pub fn rank_quality_candidates(payload: Value, config: Value) -> Result<Value, String> {
    let object = payload
        .as_object()
        .ok_or_else(|| "transform.rank_quality_candidates expects an object payload".to_string())?;
    let objective_config = config.get("objective").cloned().unwrap_or(config);
    let mut ranking = Vec::new();

    for (candidate_id, candidate_payload) in candidate_entries(object) {
        let Ok(objective) =
            compose_quality_objective(candidate_payload.clone(), objective_config.clone())
        else {
            continue;
        };
        let score = objective
            .get("composite_quality_score")
            .and_then(Value::as_f64)
            .unwrap_or(f64::INFINITY);
        let ready = objective
            .get("composite_quality_ready")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        ranking.push(CandidateRank {
            candidate_id,
            label: candidate_payload
                .get("label")
                .and_then(Value::as_str)
                .map(ToString::to_string),
            metadata: candidate_payload
                .get("metadata")
                .cloned()
                .unwrap_or(Value::Null),
            score,
            ready,
            objective,
        });
    }

    if ranking.is_empty() {
        return Err(
            "transform.rank_quality_candidates did not find quality candidates".to_string(),
        );
    }

    ranking.sort_by(|left, right| {
        right
            .ready
            .cmp(&left.ready)
            .then_with(|| left.score.total_cmp(&right.score))
            .then_with(|| left.candidate_id.cmp(&right.candidate_id))
    });

    let ready_candidate_count = ranking.iter().filter(|entry| entry.ready).count();
    let best_candidate_id = ranking[0].candidate_id.clone();
    let best_candidate_ready = ranking[0].ready;
    let best_candidate_score = ranking[0].score;
    let ranking_values = ranking
        .into_iter()
        .enumerate()
        .map(|(index, entry)| entry.into_value(index + 1))
        .collect::<Vec<_>>();

    Ok(serde_json::json!({
        "quality_candidate_ranking_contract": "kyuubiki.quality_candidate_ranking/v1",
        "candidate_count": ranking_values.len(),
        "ready_candidate_count": ready_candidate_count,
        "best_candidate_id": best_candidate_id.clone(),
        "best_candidate_ready": best_candidate_ready,
        "best_candidate_score": best_candidate_score,
        "ranking": ranking_values,
        "ranking_summary": format!(
            "Best quality candidate {}: score={}, ready={}.",
            best_candidate_id, best_candidate_score, best_candidate_ready
        ),
    }))
}

pub fn prepare_quality_next_round_request(payload: Value, config: Value) -> Result<Value, String> {
    let selected = payload
        .get("ranking")
        .and_then(Value::as_array)
        .and_then(|ranking| ranking.first())
        .and_then(Value::as_object)
        .ok_or_else(|| {
            "transform.prepare_quality_next_round_request expects a quality ranking".to_string()
        })?;
    let candidate_id = selected
        .get("candidate_id")
        .or_else(|| payload.get("best_candidate_id"))
        .and_then(Value::as_str)
        .unwrap_or("candidate");
    let score = selected
        .get("score")
        .or_else(|| payload.get("best_candidate_score"))
        .and_then(Value::as_f64)
        .filter(|value| value.is_finite())
        .unwrap_or(0.0);
    let ready = selected
        .get("ready")
        .or_else(|| payload.get("best_candidate_ready"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let target_score = config_number(&config, "target_score", 3.0);
    let action = next_round_action(ready, score, target_score, &config);
    let selected_dominant_term = selected
        .get("dominant_term")
        .or_else(|| {
            selected
                .get("objective")
                .and_then(|objective| objective.get("composite_quality_dominant_term"))
        })
        .cloned()
        .unwrap_or(Value::Null);
    let selected_blocking_terms = selected
        .get("blocking_terms")
        .or_else(|| {
            selected
                .get("objective")
                .and_then(|objective| objective.get("composite_quality_blocking_terms"))
        })
        .cloned()
        .unwrap_or_else(|| Value::Array(Vec::new()));
    let selected_candidate_metadata = selected.get("metadata").cloned().unwrap_or(Value::Null);
    let iteration_hint = quality_iteration_hint(&selected_dominant_term, &selected_blocking_terms);

    Ok(serde_json::json!({
        "quality_next_round_contract": "kyuubiki.quality_next_round_request/v1",
        "action": action,
        "selected_candidate_id": candidate_id,
        "selected_candidate_score": score,
        "selected_candidate_ready": ready,
        "selected_candidate_metadata": selected_candidate_metadata.clone(),
        "selected_dominant_term": selected_dominant_term,
        "selected_blocking_terms": selected_blocking_terms,
        "selected_iteration_hint": iteration_hint.clone(),
        "target_score": target_score,
        "source_ranking_contract": payload.get("quality_candidate_ranking_contract").cloned().unwrap_or(Value::Null),
        "request_payload": {
            "seed_candidate_id": candidate_id,
            "seed_objective": selected.get("objective").cloned().unwrap_or(Value::Null),
            "seed_metadata": selected_candidate_metadata,
            "optimization_hint": iteration_hint,
            "constraints": config.get("constraints").cloned().unwrap_or_else(|| serde_json::json!({})),
            "search_space": config.get("search_space").cloned().unwrap_or_else(|| serde_json::json!({})),
            "max_candidates": config_number(&config, "max_candidates", 8.0),
        },
        "next_round_summary": format!(
            "Quality exploration {action}: selected={candidate_id}, score={score}, target={target_score}."
        ),
    }))
}

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
        "focused_axis_path": focused_axis_path,
        "case_count": case_count,
        "expansion_budget_ready": expansion_budget_ready,
        "expansion_blocking_reason": expansion_blocking_reason,
        "sweep_budget": sweep_budget,
        "first_case_metadata": first_case_metadata,
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
