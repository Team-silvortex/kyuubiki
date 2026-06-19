use serde_json::Value;

pub fn evaluate_thermal_guard(payload: Value, config: Value) -> Result<Value, String> {
    let object = payload
        .as_object()
        .ok_or_else(|| "transform.evaluate_thermal_guard expects an object payload".to_string())?;
    let rules = config
        .get("rules")
        .and_then(Value::as_array)
        .ok_or_else(|| "transform.evaluate_thermal_guard requires config.rules".to_string())?;
    if rules.is_empty() {
        return Err("transform.evaluate_thermal_guard requires at least one rule".to_string());
    }

    let triggers = rules
        .iter()
        .filter_map(|rule| evaluate_guard_rule(object, rule))
        .collect::<Vec<_>>();
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
        "guard_summary": guard_summary(status, &triggers),
    }))
}

pub fn benchmark_coupled_heat_pair(payload: Value, config: Value) -> Result<Value, String> {
    let object = payload.as_object().ok_or_else(|| {
        "transform.benchmark_coupled_heat_pair expects an object payload".to_string()
    })?;
    let left = object
        .get("left")
        .and_then(Value::as_object)
        .ok_or_else(|| "transform.benchmark_coupled_heat_pair expects payload.left".to_string())?;
    let right = object
        .get("right")
        .and_then(Value::as_object)
        .ok_or_else(|| "transform.benchmark_coupled_heat_pair expects payload.right".to_string())?;
    let criteria = config
        .get("criteria")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            "transform.benchmark_coupled_heat_pair requires config.criteria".to_string()
        })?;
    if criteria.is_empty() {
        return Err(
            "transform.benchmark_coupled_heat_pair requires at least one criterion".to_string(),
        );
    }

    let left_label = normalize_label(config.get("left_label").and_then(Value::as_str), "left");
    let right_label = normalize_label(config.get("right_label").and_then(Value::as_str), "right");
    let breakdown = criteria
        .iter()
        .filter_map(|criterion| {
            benchmark_criterion(left, right, criterion, &left_label, &right_label)
        })
        .collect::<Vec<_>>();
    if breakdown.is_empty() {
        return Err(
            "transform.benchmark_coupled_heat_pair did not find any comparable numeric fields"
                .to_string(),
        );
    }

    let left_score = breakdown
        .iter()
        .filter_map(|item| item.get("left_score").and_then(Value::as_f64))
        .sum::<f64>();
    let right_score = breakdown
        .iter()
        .filter_map(|item| item.get("right_score").and_then(Value::as_f64))
        .sum::<f64>();
    let left_win_count = breakdown
        .iter()
        .filter(|item| {
            item.get("left_score")
                .and_then(Value::as_f64)
                .unwrap_or(0.0)
                > item
                    .get("right_score")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0)
        })
        .count();
    let right_win_count = breakdown
        .iter()
        .filter(|item| {
            item.get("right_score")
                .and_then(Value::as_f64)
                .unwrap_or(0.0)
                > item
                    .get("left_score")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0)
        })
        .count();
    let tie_count = breakdown.len() - left_win_count - right_win_count;
    let winner = benchmark_winner(left_score, right_score, &left_label, &right_label);

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
        "benchmark_recommendation": benchmark_recommendation(&winner, &left_label, &right_label),
        "benchmark_summary": benchmark_summary(&winner, left_score, right_score, &left_label, &right_label, breakdown.len()),
    }))
}

fn evaluate_guard_rule(payload: &serde_json::Map<String, Value>, rule: &Value) -> Option<Value> {
    let rule_object = rule.as_object()?;
    let field = rule_object.get("field")?.as_str()?;
    let value = payload.get(field)?.as_f64()?;
    if !guard_triggered(value, rule_object) {
        return None;
    }

    Some(serde_json::json!({
        "field": field,
        "value": value,
        "threshold": rule_threshold(rule_object),
        "comparison": rule_comparison(rule_object),
        "severity": normalize_severity(rule_object.get("severity").and_then(Value::as_str)),
        "label": rule_object.get("label").and_then(Value::as_str).unwrap_or(field),
    }))
}

fn benchmark_criterion(
    left: &serde_json::Map<String, Value>,
    right: &serde_json::Map<String, Value>,
    criterion: &Value,
    left_label: &str,
    right_label: &str,
) -> Option<Value> {
    let criterion = criterion.as_object()?;
    let left_field = criterion_field(criterion, "left_field")?;
    let right_field = criterion_field(criterion, "right_field")?;
    let label_field = criterion
        .get("field")
        .and_then(Value::as_str)
        .unwrap_or(left_field);
    let left_value = left.get(left_field)?.as_f64()?;
    let right_value = right.get(right_field)?.as_f64()?;
    let weight = normalize_weight(criterion.get("weight").and_then(Value::as_f64));
    let goal = normalize_goal(criterion.get("goal").and_then(Value::as_str));
    let (left_score, right_score) = score_benchmark_pair(left_value, right_value, goal, weight);

    Some(serde_json::json!({
        "field": label_field,
        "left_field": left_field,
        "right_field": right_field,
        "goal": goal,
        "weight": weight,
        format!("{left_label}_value"): left_value,
        format!("{right_label}_value"): right_value,
        "delta": right_value - left_value,
        "left_score": left_score,
        "right_score": right_score,
    }))
}

fn guard_triggered(value: f64, rule: &serde_json::Map<String, Value>) -> bool {
    match (rule_comparison(rule), rule_threshold(rule)) {
        ("gt", Some(threshold)) => value > threshold,
        ("gte", Some(threshold)) => value >= threshold,
        ("lt", Some(threshold)) => value < threshold,
        ("lte", Some(threshold)) => value <= threshold,
        ("eq", Some(threshold)) => value == threshold,
        _ => false,
    }
}

fn rule_comparison<'a>(rule: &'a serde_json::Map<String, Value>) -> &'a str {
    match rule
        .get("comparison")
        .and_then(Value::as_str)
        .unwrap_or("gte")
    {
        "gt" | "gte" | "lt" | "lte" | "eq" => rule
            .get("comparison")
            .and_then(Value::as_str)
            .unwrap_or("gte"),
        _ => "gte",
    }
}

fn rule_threshold(rule: &serde_json::Map<String, Value>) -> Option<f64> {
    rule.get("threshold")
        .and_then(Value::as_f64)
        .or_else(|| rule.get("value").and_then(Value::as_f64))
}

fn normalize_severity(severity: Option<&str>) -> &'static str {
    match severity.unwrap_or("warn") {
        "block" => "block",
        _ => "warn",
    }
}

fn normalize_goal(goal: Option<&str>) -> &'static str {
    match goal.unwrap_or("min") {
        "max" => "max",
        _ => "min",
    }
}

fn normalize_weight(weight: Option<f64>) -> f64 {
    match weight {
        Some(value) if value > 0.0 => value,
        _ => 1.0,
    }
}

fn criterion_field<'a>(
    criterion: &'a serde_json::Map<String, Value>,
    key: &str,
) -> Option<&'a str> {
    criterion
        .get(key)
        .and_then(Value::as_str)
        .or_else(|| criterion.get("field").and_then(Value::as_str))
}

fn score_benchmark_pair(left_value: f64, right_value: f64, goal: &str, weight: f64) -> (f64, f64) {
    match goal {
        "max" => {
            if left_value > right_value {
                (weight, 0.0)
            } else if right_value > left_value {
                (0.0, weight)
            } else {
                (weight * 0.5, weight * 0.5)
            }
        }
        _ => {
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

fn normalize_label(label: Option<&str>, default_value: &str) -> String {
    label
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(default_value)
        .to_string()
}

fn guard_recommendation(status: &str) -> &'static str {
    match status {
        "block" => "hold_and_review",
        "warn" => "review_before_continue",
        _ => "continue",
    }
}

fn guard_summary(status: &str, triggers: &[Value]) -> String {
    if status == "pass" {
        return "All thermal guard rules passed.".to_string();
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
