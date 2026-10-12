use crate::workflow_quality_checks::{finite_quality_value, ranking_order};
use crate::workflow_quality_ranking_input as ranking_input;
use crate::workflow_quality_terms::{
    CandidateRank, QualityTerm, candidate_entries, composite_blocking_terms, composite_grade,
    config_number, dominant_composite_term, next_round_action, quality_entries,
    quality_iteration_hint, quality_term, validate_quality_config,
};
use crate::workflow_result_admission::require_converged_result;
use serde_json::Value;

pub fn compose_quality_objective(payload: Value, config: Value) -> Result<Value, String> {
    let object = payload.as_object().ok_or_else(|| {
        "transform.compose_quality_objective expects an object payload".to_string()
    })?;
    if object.contains_key("qualities") {
        require_converged_result(object, "transform.compose_quality_objective", "payload")?;
    }
    let entries = quality_entries(object)?;
    validate_quality_config(&config)?;
    let missing_metric_penalty = config_number(&config, "missing_metric_penalty", 5.0)?;
    let not_ready_penalty = config_number(&config, "not_ready_penalty", 25.0)?;
    let max_ready_score = config_number(&config, "max_ready_score", 12.0)?;

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
        total = finite_quality_value(
            total + term.contribution,
            source_id,
            "composite_quality_score",
        )?;
        missing_metric_count = missing_metric_count
            .checked_add(term.missing_metric_count)
            .ok_or_else(|| {
                format!(
                    "quality source {source_id} composite_quality_missing_metric_count overflow"
                )
            })?;
        watch_count = watch_count.checked_add(term.watch_count).ok_or_else(|| {
            format!("quality source {source_id} composite_quality_watch_count overflow")
        })?;
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
    if object.contains_key("candidates") {
        require_converged_result(object, "transform.rank_quality_candidates", "payload")?;
    }
    validate_quality_config(&config)?;
    let objective_config = config.get("objective").cloned().unwrap_or(config);
    validate_quality_config(&objective_config)?;
    let mut ranking = Vec::new();
    let mut rejected_candidates = Vec::new();
    let entries = candidate_entries(object)?;
    let input_candidate_count = entries.len();

    for (candidate_id, candidate_payload) in entries {
        let objective =
            match compose_quality_objective(candidate_payload.clone(), objective_config.clone()) {
                Ok(objective) => objective,
                Err(error) => {
                    rejected_candidates
                        .push(serde_json::json!({"candidate_id": candidate_id, "error": error}));
                    continue;
                }
            };
        let score = objective
            .get("composite_quality_score")
            .and_then(Value::as_f64)
            .ok_or_else(|| format!("quality candidate {candidate_id} has no finite score"))?;
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
        if let Some(first) = rejected_candidates.first() {
            return Err(format!(
                "transform.rank_quality_candidates rejected all {} candidates; {}: {}",
                rejected_candidates.len(),
                first["candidate_id"].as_str().unwrap_or("unknown"),
                first["error"]
                    .as_str()
                    .unwrap_or("invalid quality evidence"),
            ));
        }
        return Err(
            "transform.rank_quality_candidates did not find quality candidates".to_string(),
        );
    }

    ranking.sort_by(|left, right| {
        ranking_order(
            (left.ready, left.score, &left.candidate_id),
            (right.ready, right.score, &right.candidate_id),
        )
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
        "input_candidate_count": input_candidate_count,
        "rejected_candidate_count": rejected_candidates.len(),
        "ranking_complete": rejected_candidates.is_empty(),
        "rejected_candidates": rejected_candidates,
        "ready_candidate_count": ready_candidate_count,
        "best_candidate_id": best_candidate_id.clone(),
        "best_candidate_ready": best_candidate_ready,
        "best_candidate_score": best_candidate_score,
        "ranking": ranking_values,
        "ranking_summary": format!(
            "Ranked {}/{} candidates; rejected {}. Best quality candidate {}: score={}, ready={}.",
            ranking_values.len(), input_candidate_count, rejected_candidates.len(),
            best_candidate_id, best_candidate_score, best_candidate_ready,
        ),
    }))
}

pub fn prepare_quality_next_round_request(payload: Value, config: Value) -> Result<Value, String> {
    validate_quality_config(&config)?;
    if config
        .get("require_ready")
        .is_some_and(|value| !value.is_boolean())
    {
        return Err("quality config.require_ready must be a boolean".to_string());
    }
    let admitted = ranking_input::admit(&payload)?;
    let selected = admitted.selected;
    let candidate_id = admitted.candidate_id;
    let score = admitted.score;
    let selected_dominant_term = selected
        .get("dominant_term")
        .or_else(|| {
            selected
                .get("objective")
                .and_then(|objective| objective.get("composite_quality_dominant_term"))
        })
        .cloned()
        .unwrap_or(Value::Null);
    let (objective_ready, selected_blocking_terms) = ranking_input::selected_evidence(selected)?;
    let has_blockers = !selected_blocking_terms.as_array().unwrap().is_empty();
    let selected_coupled_readiness = selected_coupled_readiness(&payload, selected)?;
    let ready = admitted.ready
        && objective_ready
        && !has_blockers
        && !coupled_readiness_blocks(&selected_coupled_readiness);
    let ranking_complete = admitted.complete;
    let rejected_candidates = admitted.rejected;
    let target_score = config_number(&config, "target_score", 3.0)?;
    let action = if !ranking_complete
        || !objective_ready
        || coupled_readiness_blocks(&selected_coupled_readiness)
        || has_blockers
    {
        "replan"
    } else if coupled_readiness_requires_review(&selected_coupled_readiness) {
        "continue"
    } else {
        next_round_action(ready, score, target_score, &config)
    };
    let selected_candidate_metadata = selected.get("metadata").cloned().unwrap_or(Value::Null);
    let iteration_hint = if !ranking_complete {
        serde_json::json!({
            "action": "repair_rejected_candidates",
            "focus_source": rejected_candidates.first().and_then(|entry| entry.get("candidate_id")),
            "blocking_count": rejected_candidates.len(),
            "reason": "Complete failed candidate evaluations before accepting the research ranking.",
        })
    } else {
        coupled_readiness_iteration_hint(&selected_coupled_readiness).unwrap_or_else(|| {
            quality_iteration_hint(&selected_dominant_term, &selected_blocking_terms)
        })
    };

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
        "selected_coupled_readiness": selected_coupled_readiness.clone(),
        "source_coupled_readiness_contract": selected_coupled_readiness.get("coupled_readiness_contract").cloned().unwrap_or(Value::Null),
        "coupled_readiness_state": selected_coupled_readiness.get("coupled_readiness_state").cloned().unwrap_or(Value::Null),
        "coupled_readiness_recommendation": selected_coupled_readiness.get("coupled_readiness_recommendation").cloned().unwrap_or(Value::Null),
        "target_score": target_score,
        "source_ranking_contract": payload.get("quality_candidate_ranking_contract").cloned().unwrap_or(Value::Null),
        "source_ranking_complete": ranking_complete,
        "source_rejected_candidates": rejected_candidates,
        "request_payload": {
            "seed_candidate_id": candidate_id,
            "seed_objective": selected.get("objective").cloned().unwrap_or(Value::Null),
            "seed_metadata": selected_candidate_metadata,
            "optimization_hint": iteration_hint,
            "coupled_readiness": selected_coupled_readiness,
            "constraints": config.get("constraints").cloned().unwrap_or_else(|| serde_json::json!({})),
            "search_space": config.get("search_space").cloned().unwrap_or_else(|| serde_json::json!({})),
            "max_candidates": config_number(&config, "max_candidates", 8.0)?,
        },
        "next_round_summary": format!(
            "Quality exploration {action}: selected={candidate_id}, score={score}, target={target_score}."
        ),
    }))
}

fn selected_coupled_readiness(
    payload: &Value,
    selected: &serde_json::Map<String, Value>,
) -> Result<Value, String> {
    crate::workflow_quality_readiness::resolve(&[
        (
            selected.get("coupled_readiness"),
            "selected.coupled_readiness",
        ),
        (
            selected
                .get("objective")
                .and_then(|v| v.get("coupled_readiness")),
            "selected.objective.coupled_readiness",
        ),
        (
            payload.get("coupled_readiness"),
            "payload.coupled_readiness",
        ),
    ])
    .map(|value| value.cloned().map(Value::Object).unwrap_or(Value::Null))
}

fn coupled_readiness_blocks(readiness: &Value) -> bool {
    crate::workflow_quality_readiness::blocks(readiness)
}

fn coupled_readiness_requires_review(readiness: &Value) -> bool {
    crate::workflow_quality_readiness::requires_review(readiness)
}

fn coupled_readiness_iteration_hint(readiness: &Value) -> Option<Value> {
    let object = readiness.as_object()?;
    let recommendation = if coupled_readiness_blocks(readiness) {
        "hold_and_repair_inputs"
    } else if coupled_readiness_requires_review(readiness) {
        "review_before_next_round"
    } else {
        return None;
    };
    let blocking_domains = object
        .get("coupled_readiness_blocking_domains")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let missing_domains = object
        .get("coupled_readiness_required_missing")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let warning_domains = object
        .get("coupled_readiness_warning_domains")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let focus_domain = blocking_domains
        .first()
        .or_else(|| missing_domains.first())
        .or_else(|| warning_domains.first())
        .cloned()
        .unwrap_or(Value::Null);
    Some(serde_json::json!({
        "action": if recommendation == "hold_and_repair_inputs" {
            "fix_coupled_readiness"
        } else {
            "review_coupled_readiness"
        },
        "focus_domain": focus_domain,
        "focus_source": "coupled_readiness",
        "focus_field": Value::Null,
        "blocking_count": blocking_domains.len() + missing_domains.len(),
        "warning_count": warning_domains.len(),
        "blocking_domains": blocking_domains,
        "required_missing": missing_domains,
        "warning_domains": warning_domains,
        "recommendation": recommendation,
        "readiness_state": object.get("coupled_readiness_state").cloned().unwrap_or(Value::Null),
    }))
}
