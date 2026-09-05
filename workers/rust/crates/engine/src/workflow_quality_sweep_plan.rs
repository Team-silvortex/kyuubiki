use serde_json::Value;

use crate::workflow_sweep_axes::prepare_sweep_axes;
use crate::workflow_sweep_contract::{
    actual_case_count, bool_option, count_option, object_or_null, refreshed_budget,
};

pub fn materialize_quality_sweep_expansion(payload: Value, config: Value) -> Result<Value, String> {
    object_or_null(&config, "quality sweep expansion config")?;
    if !bool_option(payload.get("sweep_enabled"), "sweep_enabled", true)? {
        return Ok(serde_json::json!({
            "quality_sweep_expansion_contract": "kyuubiki.quality_sweep_expansion/v1",
            "expansion_enabled": false,
            "reason": payload.get("sweep_action").and_then(Value::as_str).unwrap_or("stopped"),
            "expansion_blocking_reason": payload.get("sweep_blocking_reason").cloned().unwrap_or(Value::Null),
            "source_rejected_candidates": payload.get("source_rejected_candidates").cloned().unwrap_or_else(|| serde_json::json!([])),
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
    let max_cases = count_option(
        config.get("max_cases").or_else(|| payload.get("max_cases")),
        "max_cases",
        64,
        0,
    )?;
    let upstream_budget = payload.get("sweep_budget").unwrap_or(&Value::Null);
    object_or_null(upstream_budget, "sweep_budget")?;
    let upstream_blocked = bool_option(
        upstream_budget.get("case_budget_exceeded"),
        "sweep_budget.case_budget_exceeded",
        false,
    )?;
    // Deferred plans cannot be made executable by increasing a downstream limit.
    let case_count = if upstream_blocked && payload.get("case_count_estimate").is_some() {
        count_option(
            payload.get("case_count_estimate"),
            "case_count_estimate",
            0,
            0,
        )?
    } else {
        actual_case_count(axes)?
    };
    let sweep_budget = refreshed_budget(upstream_budget, case_count, max_cases, upstream_blocked);
    let expansion_budget_ready = !upstream_blocked && case_count <= max_cases;
    if expansion_budget_ready {
        prepare_sweep_axes(&base, axes)?;
    }
    let expansion_blocking_reason = if expansion_budget_ready {
        Value::Null
    } else {
        Value::from("case_budget_exceeded")
    };

    Ok(serde_json::json!({
        "quality_sweep_expansion_contract": "kyuubiki.quality_sweep_expansion/v1",
        "expansion_enabled": true,
        "expansion_budget_ready": expansion_budget_ready,
        "expansion_blocking_reason": expansion_blocking_reason,
        "source_plan_contract": payload.get("quality_parameter_sweep_plan_contract").cloned().unwrap_or(Value::Null),
        "source_candidate_id": payload.get("source_candidate_id").cloned().unwrap_or(Value::Null),
        "case_count_estimate": case_count,
        "sweep_budget": sweep_budget,
        "payload": {
            "base": base,
            "axes": axes,
            "case_metadata": {
                "source_candidate_id": payload.get("source_candidate_id").cloned().unwrap_or(Value::Null),
                "seed_metadata": payload.get("seed_metadata").cloned().unwrap_or(Value::Null),
                "source_plan_contract": payload.get("quality_parameter_sweep_plan_contract").cloned().unwrap_or(Value::Null),
                "target_score": payload.get("target_score").cloned().unwrap_or(Value::Null),
                "optimization_hint": payload.get("optimization_hint").cloned().unwrap_or(Value::Null),
                "coupled_readiness": payload.get("coupled_readiness").cloned().unwrap_or(Value::Null),
                "focused_axis_path": payload.get("focused_axis_path").cloned().unwrap_or(Value::Null),
                "repair_strategy": payload.get("repair_strategy").cloned().unwrap_or(Value::Null),
                "repair_focus": payload.get("repair_focus").cloned().unwrap_or(Value::Null),
                "sweep_budget": sweep_budget,
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
