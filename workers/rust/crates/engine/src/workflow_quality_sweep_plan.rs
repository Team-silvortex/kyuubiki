use serde_json::Value;

pub fn materialize_quality_sweep_expansion(payload: Value, config: Value) -> Result<Value, String> {
    if payload.get("sweep_enabled").and_then(Value::as_bool) == Some(false) {
        return Ok(serde_json::json!({
            "quality_sweep_expansion_contract": "kyuubiki.quality_sweep_expansion/v1",
            "expansion_enabled": false,
            "reason": payload.get("sweep_action").and_then(Value::as_str).unwrap_or("stopped"),
            "payload": Value::Null,
            "config": Value::Null,
        }));
    }

    let axes = payload
        .get("axes")
        .and_then(Value::as_array)
        .filter(|axes| !axes.is_empty())
        .ok_or_else(|| {
            "transform.materialize_quality_sweep_expansion requires plan axes".to_string()
        })?;
    let base = payload
        .get("base")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    let id_prefix = config
        .get("id_prefix")
        .or_else(|| payload.get("id_prefix"))
        .and_then(Value::as_str)
        .unwrap_or("quality_round");
    let max_cases = config
        .get("max_cases")
        .or_else(|| payload.get("max_cases"))
        .and_then(Value::as_f64)
        .unwrap_or(64.0);

    Ok(serde_json::json!({
        "quality_sweep_expansion_contract": "kyuubiki.quality_sweep_expansion/v1",
        "expansion_enabled": true,
        "source_plan_contract": payload.get("quality_parameter_sweep_plan_contract").cloned().unwrap_or(Value::Null),
        "source_candidate_id": payload.get("source_candidate_id").cloned().unwrap_or(Value::Null),
        "case_count_estimate": payload.get("case_count_estimate").cloned().unwrap_or(Value::Null),
        "payload": {
            "base": base,
            "axes": axes,
            "case_metadata": {
                "source_candidate_id": payload.get("source_candidate_id").cloned().unwrap_or(Value::Null),
                "source_plan_contract": payload.get("quality_parameter_sweep_plan_contract").cloned().unwrap_or(Value::Null),
                "target_score": payload.get("target_score").cloned().unwrap_or(Value::Null),
            },
        },
        "config": {
            "id_prefix": id_prefix,
            "max_cases": max_cases,
        },
        "expansion_summary": format!(
            "Materialized quality sweep expansion with {} axis/axes.",
            axes.len()
        ),
    }))
}
