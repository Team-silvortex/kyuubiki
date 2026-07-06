use serde_json::{Map, Value};

use super::{lookup_number, material_candidate_entries, number_field, numeric_config};

#[derive(Debug, Clone, PartialEq)]
struct ScoreCriterion {
    field: String,
    goal: String,
    weight: f64,
}

#[derive(Debug, Clone, PartialEq)]
struct MaterialScore {
    candidate_id: String,
    summary: Value,
    metrics: Map<String, Value>,
    feasible: bool,
    breakdown: Vec<Value>,
    weighted_score: f64,
    final_score: f64,
}

pub(super) fn score_material_candidates(payload: &Value, config: &Value) -> Result<Value, String> {
    let criteria = parse_score_criteria(config)?;
    let candidates = material_candidate_entries(payload);
    if candidates.is_empty() {
        return Err("missing_material_candidates".to_string());
    }

    let metrics_by_candidate = collect_candidate_metrics(&candidates, &criteria)?;
    let ranges = criterion_ranges(&metrics_by_candidate, &criteria);
    let total_weight = criteria
        .iter()
        .map(|criterion| criterion.weight)
        .sum::<f64>();
    let infeasible_penalty = numeric_config(config, "infeasible_penalty", 1.0);
    let mut scored = candidates
        .iter()
        .map(|(candidate_id, summary)| {
            let metrics = metrics_by_candidate
                .get(candidate_id)
                .and_then(Value::as_object)
                .expect("metrics should be collected");
            let breakdown = score_breakdown(metrics, &ranges, &criteria);
            let weighted_score = breakdown
                .iter()
                .map(|entry| number_field(entry, "weighted_score").unwrap_or(0.0))
                .sum::<f64>()
                / total_weight;
            let feasible = material_feasible(summary, config);
            let final_score = if feasible {
                weighted_score
            } else {
                weighted_score - infeasible_penalty
            };
            MaterialScore {
                candidate_id: candidate_id.clone(),
                summary: summary.clone(),
                metrics: metrics.clone(),
                feasible,
                breakdown,
                weighted_score,
                final_score,
            }
        })
        .collect::<Vec<_>>();

    scored.sort_by(|left, right| {
        right
            .final_score
            .partial_cmp(&left.final_score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                right
                    .weighted_score
                    .partial_cmp(&left.weighted_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| left.candidate_id.cmp(&right.candidate_id))
    });

    let best = scored.first();
    Ok(serde_json::json!({
        "material_score_candidate_count": scored.len(),
        "material_score_feasible_count": scored.iter().filter(|entry| entry.feasible).count(),
        "material_score_best_candidate_id": best.map(|entry| entry.candidate_id.as_str()).unwrap_or(""),
        "material_score_best_score": best.map(|entry| entry.final_score).unwrap_or(0.0),
        "material_score_criteria": criteria.iter().map(score_criterion_value).collect::<Vec<_>>(),
        "material_score_rankings": scored.iter()
            .map(|entry| score_ranking_summary(entry, config))
            .collect::<Vec<_>>()
    }))
}

fn parse_score_criteria(config: &Value) -> Result<Vec<ScoreCriterion>, String> {
    let entries = config
        .get("criteria")
        .or_else(|| config.get("objectives"))
        .and_then(Value::as_array)
        .filter(|entries| !entries.is_empty())
        .ok_or_else(|| "missing_material_score_criteria".to_string())?;

    entries
        .iter()
        .map(|entry| {
            let field = entry
                .get("field")
                .and_then(Value::as_str)
                .filter(|field| !field.is_empty())
                .ok_or_else(|| "invalid_material_score_criterion".to_string())?;
            let goal = entry.get("goal").and_then(Value::as_str).unwrap_or("min");
            let weight = entry.get("weight").and_then(Value::as_f64).unwrap_or(1.0);
            if !matches!(goal, "min" | "max") || weight <= 0.0 {
                return Err("invalid_material_score_criterion".to_string());
            }
            Ok(ScoreCriterion {
                field: field.to_string(),
                goal: goal.to_string(),
                weight,
            })
        })
        .collect()
}

fn collect_candidate_metrics(
    candidates: &[(String, Value)],
    criteria: &[ScoreCriterion],
) -> Result<Map<String, Value>, String> {
    let mut metrics_by_candidate = Map::new();
    for (candidate_id, summary) in candidates {
        let mut metrics = Map::new();
        for criterion in criteria {
            let value = lookup_number(summary, &criterion.field)
                .ok_or_else(|| "missing_material_score_metric".to_string())?;
            metrics.insert(criterion.field.clone(), Value::from(value));
        }
        metrics_by_candidate.insert(candidate_id.clone(), Value::Object(metrics));
    }
    Ok(metrics_by_candidate)
}

fn criterion_ranges(
    metrics_by_candidate: &Map<String, Value>,
    criteria: &[ScoreCriterion],
) -> Map<String, Value> {
    let mut ranges = Map::new();
    for criterion in criteria {
        let values = metrics_by_candidate
            .values()
            .filter_map(|metrics| metrics.get(&criterion.field).and_then(Value::as_f64))
            .collect::<Vec<_>>();
        let min = values
            .iter()
            .fold(f64::INFINITY, |acc, value| acc.min(*value));
        let max = values
            .iter()
            .fold(f64::NEG_INFINITY, |acc, value| acc.max(*value));
        ranges.insert(
            criterion.field.clone(),
            serde_json::json!({ "min": min, "max": max }),
        );
    }
    ranges
}

fn score_breakdown(
    metrics: &Map<String, Value>,
    ranges: &Map<String, Value>,
    criteria: &[ScoreCriterion],
) -> Vec<Value> {
    criteria
        .iter()
        .map(|criterion| {
            let actual = metrics
                .get(&criterion.field)
                .and_then(Value::as_f64)
                .unwrap_or(0.0);
            let range = ranges.get(&criterion.field).unwrap_or(&Value::Null);
            let normalized = normalize_score(actual, range, &criterion.goal);
            serde_json::json!({
                "field": criterion.field,
                "goal": criterion.goal,
                "weight": criterion.weight,
                "actual": actual,
                "normalized_score": normalized,
                "weighted_score": normalized * criterion.weight
            })
        })
        .collect()
}

fn normalize_score(value: f64, range: &Value, goal: &str) -> f64 {
    let min = range.get("min").and_then(Value::as_f64).unwrap_or(value);
    let max = range.get("max").and_then(Value::as_f64).unwrap_or(value);
    if (max - min).abs() <= f64::EPSILON {
        return 1.0;
    }
    if goal == "max" {
        (value - min) / (max - min)
    } else {
        (max - value) / (max - min)
    }
}

fn material_feasible(summary: &Value, config: &Value) -> bool {
    let status_field = config
        .get("status_field")
        .and_then(Value::as_str)
        .unwrap_or("material_status");
    let feasible_status = config
        .get("feasible_status")
        .and_then(Value::as_str)
        .unwrap_or("pass");
    summary
        .get(status_field)
        .and_then(Value::as_str)
        .map(|status| status == feasible_status)
        .unwrap_or(true)
}

fn score_criterion_value(criterion: &ScoreCriterion) -> Value {
    serde_json::json!({
        "field": criterion.field,
        "goal": criterion.goal,
        "weight": criterion.weight
    })
}

fn score_ranking_summary(scored: &MaterialScore, config: &Value) -> Value {
    let mut summary = serde_json::json!({
        "candidate_id": scored.candidate_id,
        "feasible": scored.feasible,
        "weighted_score": scored.weighted_score,
        "final_score": scored.final_score,
        "metrics": scored.metrics,
        "criteria_breakdown": scored.breakdown
    });

    if config
        .get("include_candidate_summary")
        .and_then(Value::as_bool)
        == Some(true)
    {
        summary["summary"] = scored.summary.clone();
    }

    summary
}
