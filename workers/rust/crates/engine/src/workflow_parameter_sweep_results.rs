use serde_json::{Map, Value};

pub fn join_parameter_sweep_results(payload: Value, config: Value) -> Result<Value, String> {
    let cases = payload
        .get("cases")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            "transform.join_parameter_sweep_results requires payload.cases".to_string()
        })?;
    if cases.is_empty() {
        return Err("transform.join_parameter_sweep_results cases must not be empty".to_string());
    }
    let results = payload
        .get("summaries")
        .or_else(|| payload.get("results"))
        .and_then(Value::as_array)
        .ok_or_else(|| {
            "transform.join_parameter_sweep_results requires payload.summaries or payload.results"
                .to_string()
        })?;
    let summary_field = config
        .get("summary_field")
        .and_then(Value::as_str)
        .unwrap_or("summary");
    let output_field = config
        .get("output_field")
        .and_then(Value::as_str)
        .unwrap_or("summary");
    let strict = config
        .get("strict")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let mut joined = Vec::with_capacity(cases.len());
    let mut joined_count = 0usize;
    let mut missing = Vec::new();
    for (index, case) in cases.iter().enumerate() {
        let case_object = case.as_object().ok_or_else(|| {
            format!("transform.join_parameter_sweep_results case {index} must be an object")
        })?;
        let case_id = case_object
            .get("id")
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| format!("case_{index}"));
        let mut next_case = case_object.clone();
        match find_case_result(results, &case_id, index).and_then(|result| {
            extract_join_summary(result, summary_field).map(|summary| (summary, result))
        }) {
            Some((summary, result)) => {
                joined_count += 1;
                next_case.insert(output_field.to_string(), summary);
                next_case.insert(
                    "result_status".to_string(),
                    result
                        .get("status")
                        .cloned()
                        .unwrap_or_else(|| Value::String("joined".to_string())),
                );
            }
            None => {
                missing.push(Value::String(case_id));
                next_case.insert(
                    "result_status".to_string(),
                    Value::String("missing".to_string()),
                );
            }
        }
        joined.push(Value::Object(next_case));
    }
    if strict && !missing.is_empty() {
        return Err(format!(
            "transform.join_parameter_sweep_results missing summaries for {} case(s)",
            missing.len()
        ));
    }

    Ok(serde_json::json!({
        "cases": joined,
        "case_count": cases.len(),
        "joined_summary_count": joined_count,
        "missing_summary_count": missing.len(),
        "missing_case_ids": missing,
    }))
}

pub fn score_parameter_sweep(payload: Value, config: Value) -> Result<Value, String> {
    let rows = payload
        .get("rows")
        .and_then(Value::as_array)
        .ok_or_else(|| "transform.score_parameter_sweep requires payload.rows".to_string())?;
    if rows.is_empty() {
        return Err("transform.score_parameter_sweep rows must not be empty".to_string());
    }
    let objectives = config
        .get("objectives")
        .and_then(Value::as_array)
        .ok_or_else(|| "transform.score_parameter_sweep requires config.objectives".to_string())?;
    if objectives.is_empty() {
        return Err("transform.score_parameter_sweep objectives must not be empty".to_string());
    }

    let mut scored = rows
        .iter()
        .enumerate()
        .map(|(index, row)| score_sweep_row(index, row, objectives))
        .collect::<Result<Vec<_>, String>>()?;
    scored.sort_by(|left, right| {
        right
            .get("objective_score")
            .and_then(Value::as_f64)
            .unwrap_or(f64::NEG_INFINITY)
            .total_cmp(
                &left
                    .get("objective_score")
                    .and_then(Value::as_f64)
                    .unwrap_or(f64::NEG_INFINITY),
            )
    });
    let best = scored
        .first()
        .cloned()
        .ok_or_else(|| "transform.score_parameter_sweep could not score rows".to_string())?;

    Ok(serde_json::json!({
        "best": best,
        "scored_rows": scored,
        "scored_count": rows.len(),
    }))
}

fn find_case_result<'a>(results: &'a [Value], case_id: &str, index: usize) -> Option<&'a Value> {
    results
        .iter()
        .find(|result| result_matches_case(result, case_id))
        .or_else(|| results.get(index))
}

fn result_matches_case(result: &Value, case_id: &str) -> bool {
    result.as_object().is_some_and(|object| {
        ["case_id", "id", "caseId"]
            .iter()
            .filter_map(|field| object.get(*field).and_then(Value::as_str))
            .any(|value| value == case_id)
    })
}

fn extract_join_summary(result: &Value, summary_field: &str) -> Option<Value> {
    let object = result.as_object()?;
    object
        .get(summary_field)
        .or_else(|| object.get("summary"))
        .or_else(|| object.get("result"))
        .cloned()
        .or_else(|| Some(Value::Object(object.clone())))
}

fn score_sweep_row(
    index: usize,
    row: &Value,
    objectives: &[Value],
) -> Result<Map<String, Value>, String> {
    let object = row
        .as_object()
        .ok_or_else(|| format!("transform.score_parameter_sweep row {index} must be an object"))?;
    let mut scored = object.clone();
    let mut breakdown = Vec::with_capacity(objectives.len());
    let mut total = 0.0;
    let mut feasible = true;
    for objective in objectives {
        let field = objective
            .get("field")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                "transform.score_parameter_sweep objective requires field".to_string()
            })?;
        let value = object.get(field).and_then(Value::as_f64).ok_or_else(|| {
            format!("transform.score_parameter_sweep missing numeric field {field}")
        })?;
        let goal = objective
            .get("goal")
            .and_then(Value::as_str)
            .unwrap_or("min");
        let weight = objective
            .get("weight")
            .and_then(Value::as_f64)
            .unwrap_or(1.0);
        let score = match goal {
            "min" => -value * weight,
            "max" => value * weight,
            "target" => {
                let target = objective
                    .get("target")
                    .and_then(Value::as_f64)
                    .ok_or_else(|| {
                        "transform.score_parameter_sweep target objective requires target"
                            .to_string()
                    })?;
                -(value - target).abs() * weight
            }
            other => {
                return Err(format!(
                    "transform.score_parameter_sweep unsupported objective goal: {other}"
                ));
            }
        };
        feasible &= objective_limit_allows(objective, value);
        total += score;
        breakdown.push(serde_json::json!({
            "field": field,
            "goal": goal,
            "weight": weight,
            "value": value,
            "score": score,
        }));
    }
    if !feasible {
        total -= 1.0e12;
    }
    scored.insert("objective_score".to_string(), Value::from(total));
    scored.insert("objective_feasible".to_string(), Value::from(feasible));
    scored.insert("objective_breakdown".to_string(), Value::Array(breakdown));
    Ok(scored)
}

fn objective_limit_allows(objective: &Value, value: f64) -> bool {
    let minimum_ok = objective
        .get("min_allowed")
        .and_then(Value::as_f64)
        .is_none_or(|minimum| value >= minimum);
    let maximum_ok = objective
        .get("max_allowed")
        .and_then(Value::as_f64)
        .is_none_or(|maximum| value <= maximum);
    minimum_ok && maximum_ok
}
