use crate::workflow_quality_sweep_admission::admit_plan;
use crate::workflow_result_admission::require_converged_result;
use crate::workflow_sweep_contract::{
    bool_option, count_option, declared_case_count, object_or_null, text_option,
};
use serde_json::Value;

const QUALITY_FIELDS: &[&str] = &[
    "source_plan_contract",
    "coupled_readiness",
    "optimization_hint",
    "repair_strategy",
    "repair_focus",
    "sweep_enabled",
    "sweep_action",
    "sweep_blocking_reason",
    "source_ranking_complete",
    "source_rejected_candidates",
    "selected_candidate_ready",
    "sweep_budget",
    "converged",
    "stability_result",
];

pub(crate) fn validate(payload: &Value, config: &Value) -> Result<Option<usize>, String> {
    check_payload(payload, "payload")?;
    check_config(config)?;
    let wrapped = payload.get("quality_sweep_expansion_contract").is_some();
    let (body, nested_config, enabled, blocked) = if wrapped {
        if payload["quality_sweep_expansion_contract"] != "kyuubiki.quality_sweep_expansion/v1" {
            return Err("unsupported quality_sweep_expansion_contract".into());
        }
        let enabled = bool_option(payload.get("expansion_enabled"), "expansion_enabled", true)?;
        let ready = bool_option(
            payload.get("expansion_budget_ready"),
            "expansion_budget_ready",
            true,
        )?;
        let blocked = budget_blocked(payload)? || !ready;
        let body = payload.get("payload").unwrap_or(&Value::Null);
        if !body.is_null() || (enabled && !blocked) {
            check_payload(body, "payload.payload")?;
        }
        let nested_config = payload.get("config").unwrap_or(&Value::Null);
        check_config(nested_config)?;
        (body, nested_config, enabled, blocked)
    } else {
        (payload, &Value::Null, true, false)
    };
    let aliases = [
        body.get("case_metadata"),
        nested_config.get("case_metadata"),
        config.get("case_metadata"),
    ];
    let quality = wrapped
        || aliases.iter().flatten().any(|value| {
            QUALITY_FIELDS
                .iter()
                .any(|field| value.get(field).is_some())
        });
    let axes = body
        .get("axes")
        .or_else(|| config.get("axes"))
        .or_else(|| nested_config.get("axes"));
    let actual = match axes {
        Some(axes) => Some(declared_case_count(
            axes.as_array()
                .ok_or("parameter sweep axes must be an array")?,
            blocked,
        )?),
        None if enabled && !blocked => {
            return Err("transform.expand_parameter_sweep requires axes".into());
        }
        None => None,
    };
    let active = enabled && !blocked;
    if !quality {
        validate_counts(&[payload, body], actual, active)?;
        return Ok(actual);
    }

    // Check every declared alias before configuration merging can discard evidence.
    let mut selected = None;
    for value in aliases.into_iter().flatten() {
        if selected.is_some_and(|previous| previous != value) {
            return Err("quality case_metadata aliases conflict across payload and config".into());
        }
        object_or_null(value, "quality case_metadata")?;
        selected = Some(value);
    }
    let Some(metadata) = selected.filter(|value| !value.is_null()) else {
        validate_counts(&[payload, body], actual, active)?;
        return Ok(actual);
    };
    validate_counts(&[payload, body, metadata], actual, active)?;
    let metadata_enabled = admit_plan(metadata)?;
    let metadata_blocked = budget_blocked(metadata)?;
    if enabled && !blocked && (!metadata_enabled || metadata_blocked) {
        return Err(
            "quality expansion conflicts with disabled or budget-blocked case_metadata".into(),
        );
    }
    if wrapped {
        for field in ["source_candidate_id", "source_plan_contract"] {
            if let (Some(outer), Some(inner)) = (payload.get(field), metadata.get(field)) {
                if !outer.is_null() && !inner.is_null() && outer != inner {
                    return Err(format!(
                        "quality expansion {field} conflicts with case_metadata"
                    ));
                }
            }
        }
    }
    Ok(actual)
}

fn validate_counts(sources: &[&Value], actual: Option<usize>, active: bool) -> Result<(), String> {
    let mut expected = actual;
    // Resolve all count mirrors before checking any budget, including diagnostic-only envelopes.
    for source in sources {
        let budget = source.get("sweep_budget").unwrap_or(&Value::Null);
        object_or_null(budget, "sweep_budget")?;
        for value in [
            source.get("case_count_estimate"),
            budget.get("case_count_estimate"),
        ]
        .into_iter()
        .flatten()
        {
            let count = count_option(Some(value), "case_count_estimate", 0, 0)?;
            if expected.is_some_and(|expected| expected != count) {
                return Err(
                    "case_count_estimate conflicts with axes or another count mirror".into(),
                );
            }
            expected = Some(count);
        }
    }
    for source in sources {
        let blocked = budget_blocked(source)?;
        let budget = source.get("sweep_budget").unwrap_or(&Value::Null);
        if active && blocked {
            return Err("parameter sweep execution conflicts with a supplied budget block".into());
        }
        if let Some(limit) = budget.get("max_cases") {
            let limit = count_option(Some(limit), "sweep_budget.max_cases", 0, 0)?;
            if !blocked && expected.is_some_and(|count| count > limit) {
                return Err(
                    "sweep_budget must remain blocked when its declared case limit is exceeded"
                        .into(),
                );
            }
        }
    }
    Ok(())
}

fn check_payload(value: &Value, path: &str) -> Result<(), String> {
    let object = value
        .as_object()
        .ok_or_else(|| format!("{path} must be an object"))?;
    require_converged_result(object, "transform.expand_parameter_sweep", path)
}

fn check_config(config: &Value) -> Result<(), String> {
    object_or_null(config, "parameter sweep config")?;
    count_option(config.get("max_cases"), "max_cases", 256, 0)?;
    text_option(config.get("id_prefix"), "id_prefix", "case")?;
    Ok(())
}

fn budget_blocked(value: &Value) -> Result<bool, String> {
    let budget = value.get("sweep_budget").unwrap_or(&Value::Null);
    object_or_null(budget, "sweep_budget")?;
    bool_option(
        budget.get("case_budget_exceeded"),
        "sweep_budget.case_budget_exceeded",
        false,
    )
}
