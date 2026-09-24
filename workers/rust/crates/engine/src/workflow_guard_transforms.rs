use crate::workflow_guard_contract::{
    Goal, GuardRule, PairCriterion, finite_value, pair_labels, required_metric,
};
use crate::workflow_result_admission::require_converged_result;
use serde_json::Value;

pub fn evaluate_thermal_guard(payload: Value, config: Value) -> Result<Value, String> {
    evaluate_threshold_guard(
        payload,
        config,
        "transform.evaluate_thermal_guard",
        "thermal",
    )
}

pub fn evaluate_structural_guard(payload: Value, config: Value) -> Result<Value, String> {
    evaluate_threshold_guard(
        payload,
        config,
        "transform.evaluate_structural_guard",
        "structural",
    )
}

pub fn evaluate_acoustic_guard(payload: Value, config: Value) -> Result<Value, String> {
    evaluate_threshold_guard(
        payload,
        config,
        "transform.evaluate_acoustic_guard",
        "acoustic",
    )
}

pub fn evaluate_modal_guard(payload: Value, config: Value) -> Result<Value, String> {
    evaluate_threshold_guard(payload, config, "transform.evaluate_modal_guard", "modal")
}

pub fn evaluate_dynamic_guard(payload: Value, config: Value) -> Result<Value, String> {
    evaluate_threshold_guard(
        payload,
        config,
        "transform.evaluate_dynamic_guard",
        "dynamic",
    )
}

pub fn evaluate_electrostatic_guard(payload: Value, config: Value) -> Result<Value, String> {
    evaluate_threshold_guard(
        payload,
        config,
        "transform.evaluate_electrostatic_guard",
        "electrostatic",
    )
}

pub fn evaluate_magnetostatic_guard(payload: Value, config: Value) -> Result<Value, String> {
    evaluate_threshold_guard(
        payload,
        config,
        "transform.evaluate_magnetostatic_guard",
        "magnetostatic",
    )
}

pub fn evaluate_cfd_guard(payload: Value, config: Value) -> Result<Value, String> {
    evaluate_threshold_guard(payload, config, "transform.evaluate_cfd_guard", "fluid")
}

pub fn evaluate_transport_guard(payload: Value, config: Value) -> Result<Value, String> {
    evaluate_threshold_guard(
        payload,
        config,
        "transform.evaluate_transport_guard",
        "transport",
    )
}

pub fn benchmark_magnetostatic_pair(payload: Value, config: Value) -> Result<Value, String> {
    benchmark_pair(
        payload,
        config,
        "transform.benchmark_magnetostatic_pair",
        "magnetostatic",
    )
}

pub fn benchmark_structural_pair(payload: Value, config: Value) -> Result<Value, String> {
    benchmark_pair(
        payload,
        config,
        "transform.benchmark_structural_pair",
        "structural",
    )
}

pub fn benchmark_acoustic_pair(payload: Value, config: Value) -> Result<Value, String> {
    benchmark_pair(
        payload,
        config,
        "transform.benchmark_acoustic_pair",
        "acoustic",
    )
}

pub fn benchmark_modal_pair(payload: Value, config: Value) -> Result<Value, String> {
    benchmark_pair(payload, config, "transform.benchmark_modal_pair", "modal")
}

pub fn benchmark_dynamic_pair(payload: Value, config: Value) -> Result<Value, String> {
    benchmark_pair(
        payload,
        config,
        "transform.benchmark_dynamic_pair",
        "dynamic",
    )
}

pub fn benchmark_electrostatic_pair(payload: Value, config: Value) -> Result<Value, String> {
    benchmark_pair(
        payload,
        config,
        "transform.benchmark_electrostatic_pair",
        "electrostatic",
    )
}

pub fn benchmark_cfd_pair(payload: Value, config: Value) -> Result<Value, String> {
    benchmark_pair(payload, config, "transform.benchmark_cfd_pair", "fluid")
}

pub fn benchmark_transport_pair(payload: Value, config: Value) -> Result<Value, String> {
    benchmark_pair(
        payload,
        config,
        "transform.benchmark_transport_pair",
        "transport",
    )
}

fn evaluate_threshold_guard(
    payload: Value,
    config: Value,
    operator_id: &str,
    domain: &str,
) -> Result<Value, String> {
    let object = payload
        .as_object()
        .ok_or_else(|| format!("{operator_id} expects an object payload"))?;
    require_converged_result(object, operator_id, "payload")?;
    let rules = config
        .get("rules")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{operator_id} requires config.rules"))?;
    if rules.is_empty() {
        return Err(format!("{operator_id} requires at least one rule"));
    }

    let mut triggers = Vec::new();
    for (index, rule) in rules.iter().enumerate() {
        let trigger = evaluate_guard_rule(object, rule, index)
            .map_err(|error| format!("{operator_id}: {error}"))?;
        if let Some(trigger) = trigger {
            triggers.push(trigger);
        }
    }
    let block_count = triggers
        .iter()
        .filter(|trigger| trigger["severity"].as_str() == Some("block"))
        .count();
    let warn_count = triggers
        .iter()
        .filter(|trigger| trigger["severity"].as_str() == Some("warn"))
        .count();
    let status = if block_count > 0 {
        "block"
    } else if warn_count > 0 {
        "warn"
    } else {
        "pass"
    };

    Ok(serde_json::json!({
        "guard_status": status,
        "guard_passed": status == "pass",
        "guard_trigger_count": triggers.len(),
        "guard_checked_rule_count": rules.len(),
        "guard_warn_count": warn_count,
        "guard_block_count": block_count,
        "guard_triggers": triggers,
        "guard_recommendation": guard_recommendation(status),
        "guard_summary": guard_summary(domain, status, &triggers),
    }))
}

pub fn benchmark_coupled_heat_pair(payload: Value, config: Value) -> Result<Value, String> {
    benchmark_pair(
        payload,
        config,
        "transform.benchmark_coupled_heat_pair",
        "thermal",
    )
}

fn benchmark_pair(
    payload: Value,
    config: Value,
    operator_id: &str,
    _domain: &str,
) -> Result<Value, String> {
    let object = payload
        .as_object()
        .ok_or_else(|| format!("{operator_id} expects an object payload"))?;
    require_converged_result(object, operator_id, "payload")?;
    let left = object
        .get("left")
        .and_then(Value::as_object)
        .ok_or_else(|| format!("{operator_id} expects payload.left"))?;
    let right = object
        .get("right")
        .and_then(Value::as_object)
        .ok_or_else(|| format!("{operator_id} expects payload.right"))?;
    require_converged_result(left, operator_id, "payload.left")?;
    require_converged_result(right, operator_id, "payload.right")?;
    let criteria = config
        .get("criteria")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{operator_id} requires config.criteria"))?;
    if criteria.is_empty() {
        return Err(format!("{operator_id} requires at least one criterion"));
    }

    let (left_label, right_label) =
        pair_labels(&config).map_err(|error| format!("{operator_id}: {error}"))?;
    let mut breakdown = Vec::with_capacity(criteria.len());
    let (mut left_score, mut right_score) = (0.0, 0.0);
    let (mut left_win_count, mut right_win_count) = (0, 0);
    for (index, criterion) in criteria.iter().enumerate() {
        let (entry, left_points, right_points) =
            benchmark_criterion(left, right, criterion, left_label, right_label, index)
                .map_err(|error| format!("{operator_id}: {error}"))?;
        left_score = finite_value(left_score + left_points, "left_score")
            .map_err(|error| format!("{operator_id} config.criteria[{index}]: {error}"))?;
        right_score = finite_value(right_score + right_points, "right_score")
            .map_err(|error| format!("{operator_id} config.criteria[{index}]: {error}"))?;
        left_win_count += usize::from(left_points > right_points);
        right_win_count += usize::from(right_points > left_points);
        breakdown.push(entry);
    }
    let tie_count = breakdown.len() - left_win_count - right_win_count;
    let winner = benchmark_winner(left_score, right_score, left_label, right_label);

    Ok(serde_json::json!({
        format!("{left_label}_score"): left_score,
        format!("{right_label}_score"): right_score,
        "benchmark_winner": winner,
        "benchmark_margin": (left_score - right_score).abs(),
        "benchmark_criteria_count": breakdown.len(),
        "benchmark_left_win_count": left_win_count,
        "benchmark_right_win_count": right_win_count,
        "benchmark_tie_count": tie_count,
        "benchmark_breakdown": breakdown,
        "benchmark_recommendation": benchmark_recommendation(&winner, left_label, right_label),
        "benchmark_summary": benchmark_summary(&winner, left_score, right_score, left_label, right_label, breakdown.len()),
    }))
}

fn evaluate_guard_rule(
    payload: &serde_json::Map<String, Value>,
    rule: &Value,
    index: usize,
) -> Result<Option<Value>, String> {
    let path = format!("config.rules[{index}]");
    let rule = GuardRule::parse(rule, &path)?;
    let value = required_metric(payload, rule.field, "payload")
        .map_err(|error| format!("{path}: {error}"))?;
    if !rule.triggered(value) {
        return Ok(None);
    }

    Ok(Some(serde_json::json!({
        "field": rule.field,
        "value": value,
        "threshold": rule.threshold,
        "comparison": rule.comparison,
        "severity": rule.severity,
        "label": rule.label,
    })))
}

fn benchmark_criterion(
    left: &serde_json::Map<String, Value>,
    right: &serde_json::Map<String, Value>,
    criterion: &Value,
    left_label: &str,
    right_label: &str,
    index: usize,
) -> Result<(Value, f64, f64), String> {
    let path = format!("config.criteria[{index}]");
    let criterion = PairCriterion::parse(criterion, &path)?;
    let left_value = required_metric(left, criterion.left_field, "payload.left")
        .map_err(|error| format!("{path}: {error}"))?;
    let right_value = required_metric(right, criterion.right_field, "payload.right")
        .map_err(|error| format!("{path}: {error}"))?;
    let delta = finite_value(right_value - left_value, &format!("{path}.delta"))?;
    let (left_score, right_score) =
        score_benchmark_pair(left_value, right_value, criterion.goal, criterion.weight);

    Ok((
        serde_json::json!({
            "field": criterion.field,
            "left_field": criterion.left_field,
            "right_field": criterion.right_field,
            "goal": criterion.goal,
            "weight": criterion.weight,
            format!("{left_label}_value"): left_value,
            format!("{right_label}_value"): right_value,
            "delta": delta,
            "left_score": left_score,
            "right_score": right_score,
        }),
        left_score,
        right_score,
    ))
}

fn score_benchmark_pair(left_value: f64, right_value: f64, goal: Goal, weight: f64) -> (f64, f64) {
    match goal {
        Goal::Max => {
            if left_value > right_value {
                (weight, 0.0)
            } else if right_value > left_value {
                (0.0, weight)
            } else {
                (weight * 0.5, weight * 0.5)
            }
        }
        Goal::Min => {
            if left_value < right_value {
                (weight, 0.0)
            } else if right_value < left_value {
                (0.0, weight)
            } else {
                (weight * 0.5, weight * 0.5)
            }
        }
    }
}

fn benchmark_winner(
    left_score: f64,
    right_score: f64,
    left_label: &str,
    right_label: &str,
) -> String {
    if left_score > right_score {
        left_label.to_string()
    } else if right_score > left_score {
        right_label.to_string()
    } else {
        "tie".to_string()
    }
}

fn benchmark_recommendation(winner: &str, left_label: &str, right_label: &str) -> String {
    if winner == "tie" {
        "keep_both_under_review".to_string()
    } else if winner == left_label {
        format!("prefer_{left_label}")
    } else if winner == right_label {
        format!("prefer_{right_label}")
    } else {
        "keep_both_under_review".to_string()
    }
}

fn benchmark_summary(
    winner: &str,
    left_score: f64,
    right_score: f64,
    left_label: &str,
    right_label: &str,
    criteria_count: usize,
) -> String {
    format!(
        "{winner} across {criteria_count} criteria ({left_label}={left_score}, {right_label}={right_score})."
    )
}

fn guard_recommendation(status: &str) -> &'static str {
    match status {
        "block" => "hold_and_review",
        "warn" => "review_before_continue",
        _ => "continue",
    }
}

fn guard_summary(domain: &str, status: &str, triggers: &[Value]) -> String {
    if status == "pass" {
        return format!("All {domain} guard rules passed.");
    }

    let lead = triggers
        .iter()
        .take(2)
        .filter_map(|trigger| {
            Some(format!(
                "{}={}",
                trigger.get("label")?.as_str()?,
                trigger.get("value")?.as_f64()?
            ))
        })
        .collect::<Vec<_>>()
        .join(", ");

    if lead.is_empty() {
        format!("{}: {} trigger(s).", status.to_uppercase(), triggers.len())
    } else {
        format!(
            "{}: {} trigger(s) ({}).",
            status.to_uppercase(),
            triggers.len(),
            lead
        )
    }
}
