use crate::workflow_quality_checks as check;
use crate::workflow_quality_checks::finite_quality_value;
use crate::workflow_result_admission::require_converged_result;
use serde_json::{Map, Value};
use std::collections::BTreeSet;

type QualityEntry<'a> = (&'a str, &'a Map<String, Value>);

pub(crate) struct QualityTerm {
    value: Value,
    pub(crate) contribution: f64,
    pub(crate) missing_metric_count: u64,
    pub(crate) watch_count: u64,
    pub(crate) ready: bool,
}

impl QualityTerm {
    pub(crate) fn into_value(self) -> Value {
        self.value
    }
}

pub(crate) struct CandidateRank {
    pub(crate) candidate_id: String,
    pub(crate) label: Option<String>,
    pub(crate) metadata: Value,
    pub(crate) score: f64,
    pub(crate) ready: bool,
    pub(crate) objective: Value,
}

impl CandidateRank {
    pub(crate) fn into_value(self, rank: usize) -> Value {
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
        let dominant_term = self
            .objective
            .get("composite_quality_dominant_term")
            .cloned()
            .unwrap_or(Value::Null);
        let blocking_terms = self
            .objective
            .get("composite_quality_blocking_terms")
            .cloned()
            .unwrap_or_else(|| Value::Array(Vec::new()));

        serde_json::json!({
            "rank": rank,
            "candidate_id": candidate_id,
            "candidate_label": candidate_label,
            "metadata": self.metadata,
            "score": self.score,
            "grade": grade,
            "ready": self.ready,
            "term_count": term_count,
            "missing_metric_count": missing_metric_count,
            "blocked_term_count": blocked_term_count,
            "dominant_term": dominant_term,
            "blocking_terms": blocking_terms,
            "objective": self.objective,
        })
    }
}

pub(crate) fn quality_entries(
    object: &Map<String, Value>,
) -> Result<Vec<QualityEntry<'_>>, String> {
    let source = match object.get("qualities") {
        None => object,
        Some(value) => value.as_object().ok_or_else(|| {
            "transform.compose_quality_objective expects an object qualities map".to_string()
        })?,
    };
    let entries = source
        .iter()
        // Candidate envelope metadata is not a quality source in the shorthand form.
        .filter(|(id, _)| {
            object.contains_key("qualities") || !matches!(id.as_str(), "id" | "label" | "metadata")
        })
        .map(|(source_id, value)| {
            if source_id.trim().is_empty() {
                return Err("quality source id must be nonblank text".to_string());
            }
            value
                .as_object()
                .map(|summary| (source_id.as_str(), summary))
                .ok_or_else(|| {
                    format!(
                        "transform.compose_quality_objective expects object summary for {source_id}"
                    )
                })
        })
        .collect::<Result<Vec<_>, _>>()?;
    if entries.is_empty() {
        Err("transform.compose_quality_objective expects named quality summaries".to_string())
    } else {
        Ok(entries)
    }
}

pub(crate) fn candidate_entries(
    object: &Map<String, Value>,
) -> Result<Vec<(String, &Value)>, String> {
    let entries = match object.get("candidates") {
        Some(Value::Object(candidates)) => candidates
            .iter()
            .map(|(id, value)| (id.clone(), value))
            .collect::<Vec<_>>(),
        Some(Value::Array(candidates)) => candidates
            .iter()
            .enumerate()
            .map(|(index, value)| {
                let id = check::text(value.get("id"), "candidate.id")?
                    .map(ToString::to_string)
                    .unwrap_or_else(|| format!("candidate_{}", index + 1));
                Ok((id, value))
            })
            .collect::<Result<Vec<_>, String>>()?,
        Some(_) => {
            return Err(
                "transform.rank_quality_candidates candidates must be an object or array"
                    .to_string(),
            );
        }
        None => object
            .iter()
            .map(|(id, value)| (id.clone(), value))
            .collect(),
    };
    let mut seen = BTreeSet::new();
    for (id, _) in &entries {
        if id.trim().is_empty() || !seen.insert(id) {
            return Err(format!(
                "transform.rank_quality_candidates blank or duplicate candidate id {id:?}"
            ));
        }
    }
    Ok(entries)
}

pub(crate) fn next_round_action(
    ready: bool,
    score: f64,
    target_score: f64,
    config: &Value,
) -> &'static str {
    if !ready {
        if config
            .get("require_ready")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            "replan"
        } else {
            "continue"
        }
    } else if score <= target_score {
        "stop"
    } else {
        "continue"
    }
}

pub(crate) fn quality_term(
    source_id: &str,
    summary: &Map<String, Value>,
    config: &Value,
    missing_metric_penalty: f64,
    not_ready_penalty: f64,
) -> Result<QualityTerm, String> {
    require_converged_result(
        summary,
        "transform.compose_quality_objective",
        &format!("qualities.{source_id}"),
    )?;
    if summary
        .get("validation_contract")
        .and_then(Value::as_str)
        .is_some_and(|contract| contract == "kyuubiki.summary_tolerance_validation/v1")
    {
        return validation_quality_term(source_id, summary, config, not_ready_penalty);
    }

    let (score_field, score) = check::domain_score(summary, source_id)?;
    let prefix = score_field.strip_suffix("_score").unwrap();
    let field = |suffix: &str| summary.get(&format!("{prefix}_{suffix}"));
    let path = |suffix: &str| format!("source {source_id}.{prefix}_{suffix}");
    let contract = check::text(field("contract"), &path("contract"))?;
    let domain = contract
        .and_then(|contract| contract.strip_prefix("kyuubiki."))
        .and_then(|contract| contract.strip_suffix("_quality_score/v1"))
        .unwrap_or_else(|| score_field.trim_end_matches("_quality_score"));
    let declared_ready = check::boolean(field("ready"), &path("ready"), false)?;
    let missing_metric_count =
        check::count(field("missing_metric_count"), &path("missing_metric_count"))?.unwrap_or(0);
    let watch_count = check::count(field("watch_count"), &path("watch_count"))?.unwrap_or(0);
    let grade = check::text(field("grade"), &path("grade"))?.unwrap_or("unknown");
    let dominant_term = field("dominant_term").cloned().unwrap_or(Value::Null);
    let blocking_terms = check::list(field("blocking_terms"), &path("blocking_terms"))?;
    let ready = declared_ready && blocking_terms.is_empty();
    let weight = quality_weight(config, source_id, domain);
    let weighted_score = finite_quality_value(score * weight, source_id, "weighted_score")?;
    let missing_penalty = finite_quality_value(
        missing_metric_count as f64 * missing_metric_penalty,
        source_id,
        "missing_metric_penalty",
    )?;
    let readiness_penalty = if ready { 0.0 } else { not_ready_penalty };
    let contribution = finite_quality_value(
        weighted_score + missing_penalty + readiness_penalty,
        source_id,
        "contribution",
    )?;

    Ok(QualityTerm {
        contribution,
        missing_metric_count,
        watch_count,
        ready,
        value: serde_json::json!({
            "source": source_id,
            "domain": domain,
            "score_field": score_field,
            "score": score,
            "weight": weight,
            "weighted_score": weighted_score,
            "ready": ready,
            "grade": if ready { grade } else { "block" },
            "missing_metric_count": missing_metric_count,
            "watch_count": watch_count,
            "missing_metric_penalty": missing_penalty,
            "readiness_penalty": readiness_penalty,
            "contribution": contribution,
            "dominant_term": dominant_term,
            "blocking_terms": blocking_terms,
        }),
    })
}

fn validation_quality_term(
    source_id: &str,
    summary: &Map<String, Value>,
    config: &Value,
    not_ready_penalty: f64,
) -> Result<QualityTerm, String> {
    let ready = check::validation(summary, source_id)?;
    let failed_count = summary
        .get("validation_failed_field_count")
        .and_then(Value::as_u64);
    let missing_count = summary
        .get("validation_missing_field_count")
        .and_then(Value::as_u64);
    let failed_count = failed_count.unwrap_or(0);
    let missing_count = missing_count.unwrap_or(0);
    let score = validation_score(summary, source_id)?;
    let weight = quality_weight(config, source_id, "validation");
    let weighted_score = finite_quality_value(score * weight, source_id, "weighted_score")?;
    let readiness_penalty = if ready { 0.0 } else { not_ready_penalty };
    let contribution = finite_quality_value(
        weighted_score + readiness_penalty,
        source_id,
        "contribution",
    )?;
    let mut blocking_terms = if ready {
        Vec::new()
    } else {
        validation_blocking_terms(summary)
    };
    if !ready && blocking_terms.is_empty() {
        blocking_terms.push(serde_json::json!({
            "field": "validation_passed",
            "status": "invalid_validation_evidence",
            "reason": "Passing validation requires nonempty comparisons and consistent success counters.",
        }));
    }

    Ok(QualityTerm {
        contribution,
        missing_metric_count: missing_count,
        watch_count: failed_count,
        ready,
        value: serde_json::json!({
            "source": source_id,
            "domain": "validation",
            "score_field": "validation_max_relative_error",
            "score": score,
            "weight": weight,
            "weighted_score": weighted_score,
            "ready": ready,
            "grade": if ready { "pass" } else { "block" },
            "missing_metric_count": missing_count,
            "watch_count": failed_count,
            "missing_metric_penalty": 0.0,
            "readiness_penalty": readiness_penalty,
            "contribution": contribution,
            "dominant_term": {
                "field": "validation_max_relative_error",
                "status": if ready { "pass" } else { "block" },
                "absolute_error": summary.get("validation_max_absolute_error").cloned().unwrap_or(Value::Null),
                "relative_error": summary.get("validation_max_relative_error").cloned().unwrap_or(Value::Null),
            },
            "blocking_terms": blocking_terms,
        }),
    })
}

fn validation_score(summary: &Map<String, Value>, source_id: &str) -> Result<f64, String> {
    let metric = |field: &str| -> Result<f64, String> {
        let value = match summary.get(field) {
            None => 0.0,
            Some(value) => value
                .as_f64()
                .ok_or_else(|| format!("quality source {source_id} {field} must be numeric"))?,
        };
        finite_quality_value(value, source_id, field)
    };
    let relative_error = metric("validation_max_relative_error")?;
    let absolute_error = metric("validation_max_absolute_error")?;
    finite_quality_value(
        (relative_error * 1000.0).max(absolute_error),
        source_id,
        "validation_score",
    )
}

fn validation_blocking_terms(summary: &Map<String, Value>) -> Vec<Value> {
    let mut terms = Vec::new();
    if let Some(failures) = summary.get("validation_failures").and_then(Value::as_array) {
        for failure in failures {
            terms.push(serde_json::json!({
                "field": failure.get("field").cloned().unwrap_or(Value::Null),
                "status": "tolerance_failed",
                "absolute_error": failure.get("absolute_error").cloned().unwrap_or(Value::Null),
                "relative_error": failure.get("relative_error").cloned().unwrap_or(Value::Null),
            }));
        }
    }
    if let Some(missing) = summary
        .get("validation_missing_fields")
        .and_then(Value::as_array)
    {
        for field in missing {
            terms.push(serde_json::json!({
                "field": field,
                "status": "missing_validation_field",
            }));
        }
    }
    terms
}

pub(crate) fn dominant_composite_term(terms: &[QualityTerm]) -> Value {
    terms
        .iter()
        .max_by(|left, right| left.contribution.total_cmp(&right.contribution))
        .map(|term| compact_composite_term(&term.value))
        .unwrap_or(Value::Null)
}

pub(crate) fn composite_blocking_terms(terms: &[QualityTerm]) -> Vec<Value> {
    terms
        .iter()
        .filter_map(|term| {
            let source_blocking_terms = term
                .value
                .get("blocking_terms")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let ready = term
                .value
                .get("ready")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            if ready && source_blocking_terms.is_empty() {
                None
            } else {
                let mut compact = compact_composite_term(&term.value);
                if let Some(object) = compact.as_object_mut() {
                    object.insert(
                        "source_blocking_terms".to_string(),
                        Value::Array(source_blocking_terms),
                    );
                }
                Some(compact)
            }
        })
        .collect()
}

pub(crate) fn config_number(
    config: &Value,
    field: &str,
    default_value: f64,
) -> Result<f64, String> {
    match config.get(field) {
        None => Ok(default_value),
        Some(value) => {
            let number = value
                .as_f64()
                .ok_or_else(|| format!("quality config.{field} must be numeric"))?;
            finite_quality_value(number, "config", field)
        }
    }
}

pub(crate) fn validate_quality_config(config: &Value) -> Result<(), String> {
    if !config.is_null() && !config.is_object() {
        return Err("quality config must be an object".to_string());
    }
    for field in [
        "missing_metric_penalty",
        "not_ready_penalty",
        "max_ready_score",
        "target_score",
        "max_candidates",
    ] {
        config_number(config, field, 0.0)?;
    }
    if let Some(weights) = config.get("weights") {
        let weights = weights
            .as_object()
            .ok_or_else(|| "quality config.weights must be an object".to_string())?;
        for (id, value) in weights {
            let field = format!("weights.{id}");
            let number = value
                .as_f64()
                .ok_or_else(|| format!("quality config.{field} must be numeric"))?;
            finite_quality_value(number, "config", &field)?;
        }
    }
    Ok(())
}

pub(crate) fn composite_grade(
    score: f64,
    blocked_term_count: u64,
    max_ready_score: f64,
) -> &'static str {
    if !score.is_finite()
        || score < 0.0
        || !max_ready_score.is_finite()
        || max_ready_score < 0.0
        || blocked_term_count > 0
        || score > max_ready_score
    {
        "block"
    } else if score > max_ready_score * 0.7 {
        "review"
    } else if score > max_ready_score * 0.35 {
        "good"
    } else {
        "excellent"
    }
}

pub(crate) fn quality_iteration_hint(dominant_term: &Value, blocking_terms: &Value) -> Value {
    let blocking_terms = blocking_terms.as_array().cloned().unwrap_or_else(Vec::new);
    if let Some(blocking_term) = blocking_terms.first() {
        let source_blocker = blocking_term
            .get("source_blocking_terms")
            .and_then(Value::as_array)
            .and_then(|terms| terms.first())
            .cloned()
            .unwrap_or(Value::Null);
        let focus_field = source_blocker
            .get("field")
            .or_else(|| {
                blocking_term
                    .get("dominant_term")
                    .and_then(|term| term.get("field"))
            })
            .cloned()
            .unwrap_or(Value::Null);
        return serde_json::json!({
            "action": if blocking_term.get("domain").and_then(Value::as_str) == Some("validation") {
                "fix_validation_failure"
            } else {
                "fix_blocking_term"
            },
            "focus_domain": blocking_term.get("domain").cloned().unwrap_or(Value::Null),
            "focus_source": blocking_term.get("source").cloned().unwrap_or(Value::Null),
            "focus_field": focus_field,
            "blocking_count": blocking_terms.len(),
            "source_blocking_term": source_blocker,
        });
    }

    let nested_term = dominant_term
        .get("dominant_term")
        .cloned()
        .unwrap_or(Value::Null);
    serde_json::json!({
        "action": "reduce_dominant_term",
        "focus_domain": dominant_term.get("domain").cloned().unwrap_or(Value::Null),
        "focus_source": dominant_term.get("source").cloned().unwrap_or(Value::Null),
        "focus_field": nested_term.get("field").cloned().unwrap_or(Value::Null),
        "blocking_count": 0,
        "source_dominant_term": nested_term,
    })
}

fn compact_composite_term(term: &Value) -> Value {
    serde_json::json!({
        "source": term.get("source").cloned().unwrap_or(Value::Null),
        "domain": term.get("domain").cloned().unwrap_or(Value::Null),
        "grade": term.get("grade").cloned().unwrap_or(Value::Null),
        "ready": term.get("ready").cloned().unwrap_or(Value::Null),
        "contribution": term.get("contribution").cloned().unwrap_or(Value::Null),
        "dominant_term": term.get("dominant_term").cloned().unwrap_or(Value::Null),
    })
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
