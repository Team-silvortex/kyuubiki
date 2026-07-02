use serde_json::{Map, Value};

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
        if !term.ready {
            blocked_term_count += 1;
        }
        terms.push(term.into_value());
    }

    if terms.is_empty() {
        return Err(
            "transform.compose_quality_objective did not find quality score summaries".to_string(),
        );
    }

    let grade = composite_grade(total, blocked_term_count, max_ready_score);
    Ok(serde_json::json!({
        "composite_quality_contract": "kyuubiki.composite_quality_objective/v1",
        "composite_quality_score": total,
        "composite_quality_grade": grade,
        "composite_quality_ready": grade != "block",
        "composite_quality_term_count": terms.len(),
        "composite_quality_missing_metric_count": missing_metric_count,
        "composite_quality_blocked_term_count": blocked_term_count,
        "composite_quality_max_ready_score": max_ready_score,
        "composite_quality_terms": terms,
        "composite_quality_summary": format!(
            "Composite quality {grade}: score={total:.4}, blocked_terms={blocked_term_count}, missing_metrics={missing_metric_count}."
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

    Ok(serde_json::json!({
        "quality_next_round_contract": "kyuubiki.quality_next_round_request/v1",
        "action": action,
        "selected_candidate_id": candidate_id,
        "selected_candidate_score": score,
        "selected_candidate_ready": ready,
        "target_score": target_score,
        "source_ranking_contract": payload.get("quality_candidate_ranking_contract").cloned().unwrap_or(Value::Null),
        "request_payload": {
            "seed_candidate_id": candidate_id,
            "seed_objective": selected.get("objective").cloned().unwrap_or(Value::Null),
            "constraints": config.get("constraints").cloned().unwrap_or_else(|| serde_json::json!({})),
            "search_space": config.get("search_space").cloned().unwrap_or_else(|| serde_json::json!({})),
            "max_candidates": config_number(&config, "max_candidates", 8.0),
        },
        "next_round_summary": format!(
            "Quality exploration {action}: selected={candidate_id}, score={score}, target={target_score}."
        ),
    }))
}

struct QualityTerm {
    value: Value,
    contribution: f64,
    missing_metric_count: u64,
    ready: bool,
}

impl QualityTerm {
    fn into_value(self) -> Value {
        self.value
    }
}

struct CandidateRank {
    candidate_id: String,
    label: Option<String>,
    score: f64,
    ready: bool,
    objective: Value,
}

impl CandidateRank {
    fn into_value(self, rank: usize) -> Value {
        let candidate_id = self.candidate_id;
        let candidate_label = self.label.unwrap_or_else(|| candidate_id.clone());
        let grade = self
            .objective
            .get("composite_quality_grade")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string();
        let term_count = self
            .objective
            .get("composite_quality_term_count")
            .cloned()
            .unwrap_or(Value::Null);
        let missing_metric_count = self
            .objective
            .get("composite_quality_missing_metric_count")
            .cloned()
            .unwrap_or(Value::Null);
        let blocked_term_count = self
            .objective
            .get("composite_quality_blocked_term_count")
            .cloned()
            .unwrap_or(Value::Null);

        serde_json::json!({
            "rank": rank,
            "candidate_id": candidate_id,
            "candidate_label": candidate_label,
            "score": self.score,
            "grade": grade,
            "ready": self.ready,
            "term_count": term_count,
            "missing_metric_count": missing_metric_count,
            "blocked_term_count": blocked_term_count,
            "objective": self.objective,
        })
    }
}

fn quality_entries(
    object: &Map<String, Value>,
) -> Result<Vec<(&str, &Map<String, Value>)>, String> {
    let source = object
        .get("qualities")
        .and_then(Value::as_object)
        .unwrap_or(object);
    let entries = source
        .iter()
        .filter_map(|(source_id, value)| {
            value
                .as_object()
                .map(|summary| (source_id.as_str(), summary))
        })
        .collect::<Vec<_>>();
    if entries.is_empty() {
        Err("transform.compose_quality_objective expects named quality summaries".to_string())
    } else {
        Ok(entries)
    }
}

fn candidate_entries(object: &Map<String, Value>) -> Vec<(String, &Value)> {
    match object.get("candidates") {
        Some(Value::Object(candidates)) => candidates
            .iter()
            .filter(|(_id, value)| value.is_object())
            .map(|(id, value)| (id.clone(), value))
            .collect(),
        Some(Value::Array(candidates)) => candidates
            .iter()
            .enumerate()
            .filter(|(_index, value)| value.is_object())
            .map(|(index, value)| {
                let id = value
                    .get("id")
                    .and_then(Value::as_str)
                    .map(ToString::to_string)
                    .unwrap_or_else(|| format!("candidate_{}", index + 1));
                (id, value)
            })
            .collect(),
        _ => object
            .iter()
            .filter(|(_id, value)| value.is_object())
            .map(|(id, value)| (id.clone(), value))
            .collect(),
    }
}

fn next_round_action(ready: bool, score: f64, target_score: f64, config: &Value) -> &'static str {
    if !ready
        && config
            .get("require_ready")
            .and_then(Value::as_bool)
            .unwrap_or(false)
    {
        "replan"
    } else if score <= target_score {
        "stop"
    } else {
        "continue"
    }
}

fn quality_term(
    source_id: &str,
    summary: &Map<String, Value>,
    config: &Value,
    missing_metric_penalty: f64,
    not_ready_penalty: f64,
) -> Result<QualityTerm, String> {
    let (score_field, score) = find_number_suffix(summary, "_quality_score").ok_or_else(|| {
        format!("transform.compose_quality_objective missing quality score for {source_id}")
    })?;
    let domain = quality_domain(summary, &score_field);
    let ready = find_bool_suffix(summary, "_quality_ready").unwrap_or(false);
    let missing_metric_count =
        find_u64_suffix(summary, "_quality_missing_metric_count").unwrap_or(0);
    let grade = find_string_suffix(summary, "_quality_grade").unwrap_or("unknown");
    let weight = quality_weight(config, source_id, &domain);
    let weighted_score = score * weight;
    let missing_penalty = missing_metric_count as f64 * missing_metric_penalty;
    let readiness_penalty = if ready { 0.0 } else { not_ready_penalty };
    let contribution = weighted_score + missing_penalty + readiness_penalty;

    Ok(QualityTerm {
        contribution,
        missing_metric_count,
        ready,
        value: serde_json::json!({
            "source": source_id,
            "domain": domain,
            "score_field": score_field,
            "score": score,
            "weight": weight,
            "weighted_score": weighted_score,
            "ready": ready,
            "grade": grade,
            "missing_metric_count": missing_metric_count,
            "missing_metric_penalty": missing_penalty,
            "readiness_penalty": readiness_penalty,
            "contribution": contribution,
        }),
    })
}

fn quality_domain(summary: &Map<String, Value>, score_field: &str) -> String {
    summary
        .iter()
        .find_map(|(key, value)| {
            if key.ends_with("_quality_contract") {
                value
                    .as_str()
                    .and_then(|contract| contract.strip_prefix("kyuubiki."))
                    .and_then(|contract| contract.strip_suffix("_quality_score/v1"))
                    .map(ToString::to_string)
            } else {
                None
            }
        })
        .unwrap_or_else(|| score_field.trim_end_matches("_quality_score").to_string())
}

fn quality_weight(config: &Value, source_id: &str, domain: &str) -> f64 {
    config
        .get("weights")
        .and_then(Value::as_object)
        .and_then(|weights| weights.get(source_id).or_else(|| weights.get(domain)))
        .and_then(Value::as_f64)
        .filter(|value| value.is_finite() && *value >= 0.0)
        .unwrap_or(1.0)
}

fn find_number_suffix(summary: &Map<String, Value>, suffix: &str) -> Option<(String, f64)> {
    for (key, value) in summary {
        if key.ends_with(suffix) {
            let Some(value) = value.as_f64().filter(|value| value.is_finite()) else {
                continue;
            };
            return Some((key.clone(), value));
        }
    }
    None
}

fn find_bool_suffix(summary: &Map<String, Value>, suffix: &str) -> Option<bool> {
    summary.iter().find_map(|(key, value)| {
        if key.ends_with(suffix) {
            value.as_bool()
        } else {
            None
        }
    })
}

fn find_u64_suffix(summary: &Map<String, Value>, suffix: &str) -> Option<u64> {
    summary.iter().find_map(|(key, value)| {
        if key.ends_with(suffix) {
            value.as_u64()
        } else {
            None
        }
    })
}

fn find_string_suffix<'a>(summary: &'a Map<String, Value>, suffix: &str) -> Option<&'a str> {
    summary.iter().find_map(|(key, value)| {
        if key.ends_with(suffix) {
            value.as_str()
        } else {
            None
        }
    })
}

fn config_number(config: &Value, field: &str, default_value: f64) -> f64 {
    config
        .get(field)
        .and_then(Value::as_f64)
        .filter(|value| value.is_finite() && *value >= 0.0)
        .unwrap_or(default_value)
}

fn composite_grade(score: f64, blocked_term_count: u64, max_ready_score: f64) -> &'static str {
    if blocked_term_count > 0 || score > max_ready_score {
        "block"
    } else if score > max_ready_score * 0.7 {
        "review"
    } else if score > max_ready_score * 0.35 {
        "good"
    } else {
        "excellent"
    }
}
