use serde_json::{Map, Value};

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

pub(crate) fn candidate_entries(object: &Map<String, Value>) -> Vec<(String, &Value)> {
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

pub(crate) fn next_round_action(
    ready: bool,
    score: f64,
    target_score: f64,
    config: &Value,
) -> &'static str {
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

pub(crate) fn quality_term(
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
    let watch_count = find_u64_suffix(summary, "_quality_watch_count").unwrap_or(0);
    let grade = find_string_suffix(summary, "_quality_grade").unwrap_or("unknown");
    let dominant_term = find_value_suffix(summary, "_quality_dominant_term")
        .cloned()
        .unwrap_or(Value::Null);
    let blocking_terms = find_value_suffix(summary, "_quality_blocking_terms")
        .cloned()
        .unwrap_or_else(|| Value::Array(Vec::new()));
    let weight = quality_weight(config, source_id, &domain);
    let weighted_score = score * weight;
    let missing_penalty = missing_metric_count as f64 * missing_metric_penalty;
    let readiness_penalty = if ready { 0.0 } else { not_ready_penalty };
    let contribution = weighted_score + missing_penalty + readiness_penalty;

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
            "grade": grade,
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

pub(crate) fn config_number(config: &Value, field: &str, default_value: f64) -> f64 {
    config
        .get(field)
        .and_then(Value::as_f64)
        .filter(|value| value.is_finite() && *value >= 0.0)
        .unwrap_or(default_value)
}

pub(crate) fn composite_grade(
    score: f64,
    blocked_term_count: u64,
    max_ready_score: f64,
) -> &'static str {
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
            "action": "fix_blocking_term",
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

fn find_value_suffix<'a>(summary: &'a Map<String, Value>, suffix: &str) -> Option<&'a Value> {
    summary
        .iter()
        .find_map(|(key, value)| key.ends_with(suffix).then_some(value))
}
