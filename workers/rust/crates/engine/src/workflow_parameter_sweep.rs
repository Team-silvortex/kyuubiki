use serde_json::{Map, Value};

use crate::workflow_sweep_axes::{expand_sweep_cases, prepare_sweep_axes};
use crate::workflow_sweep_contract::{
    actual_case_count, bool_option, count_option, object_or_null, refreshed_budget,
};

pub use crate::workflow_parameter_sweep_results::{
    join_parameter_sweep_results, map_parameter_sweep_scores_to_quality_candidates,
    score_parameter_sweep,
};
pub use crate::workflow_sweep_summary::summarize_parameter_sweep;

pub fn expand_parameter_sweep(payload: Value, config: Value) -> Result<Value, String> {
    object_or_null(&config, "parameter sweep config")?;
    let quality_context = if let Some(contract) = payload.get("quality_sweep_expansion_contract") {
        if contract.as_str() != Some("kyuubiki.quality_sweep_expansion/v1") {
            return Err("unsupported quality_sweep_expansion_contract".to_string());
        }
        if !bool_option(payload.get("expansion_enabled"), "expansion_enabled", true)? {
            return Ok(disabled_quality_sweep_result(&payload));
        }
        let budget_ready = bool_option(
            payload.get("expansion_budget_ready"),
            "expansion_budget_ready",
            true,
        )?;
        let budget = payload.get("sweep_budget").unwrap_or(&Value::Null);
        object_or_null(budget, "sweep_budget")?;
        let upstream_blocked = bool_option(
            budget.get("case_budget_exceeded"),
            "sweep_budget.case_budget_exceeded",
            false,
        )?;
        if !budget_ready || upstream_blocked {
            return Ok(budget_blocked_quality_sweep_result(&payload));
        }
        Some(serde_json::json!({
            "source_candidate_id": payload.get("source_candidate_id"),
            "sweep_budget": budget,
        }))
    } else {
        None
    };

    let (payload, config) = normalize_expand_input(payload, config)?;
    let base = payload
        .get("base")
        .or_else(|| payload.get("model"))
        .ok_or_else(|| "transform.expand_parameter_sweep requires payload.base".to_string())?;
    let axes_value = payload
        .get("axes")
        .or_else(|| config.get("axes"))
        .and_then(Value::as_array)
        .ok_or_else(|| "transform.expand_parameter_sweep requires axes".to_string())?;
    let case_count = actual_case_count(axes_value)?;
    let max_cases = count_option(config.get("max_cases"), "max_cases", 256, 0)?;
    if case_count > max_cases {
        if let Some(mut context) = quality_context {
            context["case_count_estimate"] = Value::from(case_count);
            context["sweep_budget"] =
                refreshed_budget(&context["sweep_budget"], case_count, max_cases, false);
            return Ok(budget_blocked_quality_sweep_result(&context));
        }
        return Err(format!(
            "transform.expand_parameter_sweep would emit {case_count} cases, above max_cases {max_cases}"
        ));
    }
    let axes = prepare_sweep_axes(base, axes_value)?;
    let id_prefix = config
        .get("id_prefix")
        .and_then(Value::as_str)
        .unwrap_or("case");
    let case_metadata = payload
        .get("case_metadata")
        .or_else(|| config.get("case_metadata"))
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));

    let cases = expand_sweep_cases(base, &axes, case_count, id_prefix, &case_metadata)?;

    Ok(serde_json::json!({
        "cases": cases,
        "case_count": case_count,
        "axis_count": axes.len(),
    }))
}

fn disabled_quality_sweep_result(payload: &Value) -> Value {
    let reason = payload
        .get("reason")
        .and_then(Value::as_str)
        .unwrap_or("stopped");
    serde_json::json!({
        "cases": [],
        "case_count": 0,
        "axis_count": 0,
        "sweep_enabled": false,
        "expansion_enabled": false,
        "sweep_action": reason,
        "sweep_blocking_reason": payload.get("expansion_blocking_reason").cloned().unwrap_or(Value::Null),
        "source_rejected_candidates": payload.get("source_rejected_candidates").cloned().unwrap_or_else(|| serde_json::json!([])),
        "sweep_summary": format!("Quality parameter sweep was skipped: {reason}."),
    })
}

fn budget_blocked_quality_sweep_result(payload: &Value) -> Value {
    let reason = payload
        .get("expansion_blocking_reason")
        .and_then(Value::as_str)
        .unwrap_or("case_budget_exceeded");
    serde_json::json!({
        "cases": [],
        "case_count": 0,
        "axis_count": 0,
        "sweep_enabled": true,
        "expansion_enabled": true,
        "expansion_budget_ready": false,
        "expansion_blocking_reason": reason,
        "source_candidate_id": payload.get("source_candidate_id").cloned().unwrap_or(Value::Null),
        "case_count_estimate": payload.get("case_count_estimate").cloned().unwrap_or(Value::Null),
        "sweep_budget": payload.get("sweep_budget").cloned().unwrap_or(Value::Null),
        "sweep_summary": format!(
            "Quality parameter sweep expansion was blocked before case generation: {reason}."
        ),
    })
}

fn normalize_expand_input(mut payload: Value, config: Value) -> Result<(Value, Value), String> {
    if payload.get("quality_sweep_expansion_contract").is_none() {
        return Ok((payload, config));
    }

    let nested_payload = payload
        .get_mut("payload")
        .map(Value::take)
        .ok_or_else(|| "quality sweep expansion requires payload".to_string())?;
    let nested_config = payload
        .get_mut("config")
        .map(Value::take)
        .unwrap_or_else(|| serde_json::json!({}));

    object_or_null(&nested_config, "quality sweep expansion config")?;
    Ok((nested_payload, merge_config(nested_config, config)))
}

fn merge_config(base: Value, overrides: Value) -> Value {
    let mut merged = match base {
        Value::Object(object) => object,
        _ => Map::new(),
    };
    if let Value::Object(object) = overrides {
        merged.extend(object);
    }
    Value::Object(merged)
}
