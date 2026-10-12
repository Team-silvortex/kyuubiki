use crate::workflow_quality_checks as check;
use crate::workflow_quality_readiness as readiness;
use crate::workflow_result_admission::require_converged_result;
use serde_json::{Value, json};

pub(crate) struct AdmittedSweep<'a> {
    pub request: &'a Value,
    pub action: &'a str,
    pub hint: Value,
    pub coupled: Value,
    pub incomplete: bool,
}

pub(crate) fn admit(payload: &Value) -> Result<AdmittedSweep<'_>, String> {
    let object = payload
        .as_object()
        .ok_or("quality sweep payload must be an object")?;
    require_converged_result(
        object,
        "transform.build_quality_parameter_sweep_plan",
        "payload",
    )?;
    let request = payload.get("request_payload").unwrap_or(&Value::Null);
    if let Some(object) = check::optional_object(Some(request), "request_payload")? {
        require_converged_result(
            object,
            "transform.build_quality_parameter_sweep_plan",
            "request_payload",
        )?;
    }
    let action = check::text(payload.get("action"), "action")?.unwrap_or("continue");
    if !["continue", "replan", "stop"].contains(&action) {
        return Err("quality sweep action must be continue, replan or stop".into());
    }
    let complete = check::boolean(
        payload.get("source_ranking_complete"),
        "source_ranking_complete",
        true,
    )?;
    let selected_ready = check::boolean(
        payload.get("selected_candidate_ready"),
        "selected_candidate_ready",
        false,
    )?;
    let rejected = check::list(
        payload.get("source_rejected_candidates"),
        "source_rejected_candidates",
    )?;
    if rejected.iter().any(|entry| !entry.is_object()) {
        return Err("quality source_rejected_candidates entries must be objects".into());
    }
    let hint = resolve_hint(&[
        (
            request.get("optimization_hint"),
            "request_payload.optimization_hint",
        ),
        (
            payload.get("selected_iteration_hint"),
            "selected_iteration_hint",
        ),
    ])?;
    let seed = check::optional_object(
        request.get("seed_objective"),
        "request_payload.seed_objective",
    )?;
    if let Some(seed) = seed {
        require_converged_result(
            seed,
            "transform.build_quality_parameter_sweep_plan",
            "request_payload.seed_objective",
        )?;
    }
    let coupled = readiness::resolve(&[
        (
            request.get("coupled_readiness"),
            "request_payload.coupled_readiness",
        ),
        (
            payload.get("selected_coupled_readiness"),
            "selected_coupled_readiness",
        ),
        (payload.get("coupled_readiness"), "coupled_readiness"),
        (
            seed.and_then(|seed| seed.get("coupled_readiness")),
            "request_payload.seed_objective.coupled_readiness",
        ),
    ])?
    .cloned()
    .map(Value::Object)
    .unwrap_or(Value::Null);
    for field in [
        "coupled_readiness_state",
        "coupled_readiness_recommendation",
    ] {
        if let Some(value) = payload.get(field).filter(|value| !value.is_null()) {
            if Some(value) != coupled.get(field) {
                return Err(format!(
                    "quality {field} conflicts with coupled_readiness report"
                ));
            }
        }
    }
    if let Some(contract) = payload
        .get("source_coupled_readiness_contract")
        .filter(|value| !value.is_null())
    {
        if Some(contract) != coupled.get("coupled_readiness_contract") {
            return Err(
                "quality source_coupled_readiness_contract conflicts with coupled_readiness report"
                    .into(),
            );
        }
    }
    let incomplete =
        !complete || !rejected.is_empty() || hint["action"] == "repair_rejected_candidates";
    // Validate aliases even for a disabled plan; only execution intent is skipped.
    if !incomplete {
        validate_intent(action, selected_ready, &hint, &coupled)?;
    }
    Ok(AdmittedSweep {
        request,
        action,
        hint,
        coupled,
        incomplete,
    })
}

pub(crate) fn admit_plan(payload: &Value) -> Result<bool, String> {
    let object = payload
        .as_object()
        .ok_or("quality sweep plan must be an object")?;
    require_converged_result(
        object,
        "transform.materialize_quality_sweep_expansion",
        "plan",
    )?;
    let enabled = check::boolean(payload.get("sweep_enabled"), "sweep_enabled", true)?;
    let hint = resolve_hint(&[(payload.get("optimization_hint"), "optimization_hint")])?;
    let coupled = readiness::resolve(&[(payload.get("coupled_readiness"), "coupled_readiness")])?
        .cloned()
        .map(Value::Object)
        .unwrap_or(Value::Null);
    let default_action = if !enabled {
        "stop"
    } else if hint["action"] == "fix_coupled_readiness" {
        "replan"
    } else {
        "continue"
    };
    let action =
        check::text(payload.get("sweep_action"), "sweep_action")?.unwrap_or(default_action);
    if !["continue", "replan", "stop"].contains(&action) {
        return Err("quality sweep_action must be continue, replan or stop".into());
    }
    let complete = check::boolean(
        payload.get("source_ranking_complete"),
        "source_ranking_complete",
        true,
    )?;
    let ready = check::boolean(
        payload.get("selected_candidate_ready"),
        "selected_candidate_ready",
        false,
    )?;
    let rejected = check::list(
        payload.get("source_rejected_candidates"),
        "source_rejected_candidates",
    )?;
    if rejected.iter().any(|entry| !entry.is_object()) {
        return Err("quality source_rejected_candidates entries must be objects".into());
    }
    let reason = check::text(
        payload
            .get("sweep_blocking_reason")
            .filter(|v| !v.is_null()),
        "sweep_blocking_reason",
    )?;
    if enabled {
        if action == "stop"
            || !complete
            || !rejected.is_empty()
            || reason.is_some()
            || hint["action"] == "repair_rejected_candidates"
        {
            return Err("quality sweep_enabled conflicts with stopped or incomplete plan".into());
        }
        validate_intent(action, ready, &hint, &coupled)?;
    }
    if let Some(strategy) = check::text(
        payload.get("repair_strategy").filter(|v| !v.is_null()),
        "repair_strategy",
    )? {
        if strategy != repair_strategy(&hint) {
            return Err("quality repair_strategy conflicts with optimization_hint".into());
        }
    }
    if let Some(focus) = check::optional_object(payload.get("repair_focus"), "repair_focus")? {
        for (field, expected) in [
            ("field", hint.get("focus_field")),
            ("source", hint.get("focus_source")),
            ("domain", hint.get("focus_domain")),
            ("readiness_state", coupled.get("coupled_readiness_state")),
            (
                "readiness_recommendation",
                coupled.get("coupled_readiness_recommendation"),
            ),
        ] {
            if focus
                .get(field)
                .is_some_and(|value| value != expected.unwrap_or(&Value::Null))
            {
                return Err(format!(
                    "quality repair_focus.{field} conflicts with plan evidence"
                ));
            }
        }
    }
    Ok(enabled)
}

pub(crate) fn repair_strategy(hint: &Value) -> &'static str {
    match hint.get("action").and_then(Value::as_str) {
        Some("fix_validation_failure") => "rerun_validation_focused_sweep",
        Some("fix_coupled_readiness") => "repair_coupled_readiness_sweep",
        Some("review_coupled_readiness") => "review_coupled_readiness_sweep",
        Some("fix_blocking_term") => "repair_blocking_term_sweep",
        Some("reduce_dominant_term") => "reduce_dominant_term_sweep",
        _ => "general_quality_sweep",
    }
}

fn resolve_hint(sources: &[(Option<&Value>, &str)]) -> Result<Value, String> {
    let mut hint = None;
    for (value, path) in sources {
        if let Some(object) = check::optional_object(*value, path)? {
            check::text(object.get("action"), &format!("{path}.action"))?;
            for field in ["focus_field", "focus_source", "focus_domain"] {
                check::text(
                    object.get(field).filter(|value| !value.is_null()),
                    &format!("{path}.{field}"),
                )?;
            }
            if hint.is_some_and(|previous| previous != object) {
                return Err(
                    "quality optimization_hint conflicts with selected_iteration_hint".into(),
                );
            }
            hint = Some(object);
        }
    }
    Ok(hint.cloned().map(Value::Object).unwrap_or(Value::Null))
}

fn validate_intent(
    action: &str,
    selected_ready: bool,
    hint: &Value,
    coupled: &Value,
) -> Result<(), String> {
    let blocked = readiness::blocks(coupled);
    let review = readiness::requires_review(coupled);
    let expected = if blocked {
        Some("fix_coupled_readiness")
    } else if review {
        Some("review_coupled_readiness")
    } else {
        None
    };
    let hint_action = hint.get("action").and_then(Value::as_str);
    if let Some(expected) = expected {
        if action == "stop" || (blocked && action != "replan") || hint_action != Some(expected) {
            return Err(format!(
                "quality coupled_readiness requires explicit {expected} intent, not action={action}"
            ));
        }
        if blocked && selected_ready {
            return Err(
                "quality selected_candidate_ready conflicts with blocked coupled_readiness".into(),
            );
        }
        validate_hint(hint, coupled, blocked)?;
    } else if matches!(
        hint_action,
        Some("fix_coupled_readiness" | "review_coupled_readiness")
    ) {
        return Err(
            "quality optimization_hint requires matching coupled_readiness evidence".into(),
        );
    }
    Ok(())
}

fn validate_hint(hint: &Value, coupled: &Value, blocked: bool) -> Result<(), String> {
    let list = |field| coupled.get(field).cloned().unwrap_or_else(|| json!([]));
    let blocking = list("coupled_readiness_blocking_domains");
    let missing = list("coupled_readiness_required_missing");
    let warnings = list("coupled_readiness_warning_domains");
    let expected = json!({
        "focus_source": "coupled_readiness",
        "readiness_state": coupled.get("coupled_readiness_state"),
        "recommendation": if blocked { "hold_and_repair_inputs" } else { "review_before_next_round" },
        "blocking_count": blocking.as_array().unwrap().len() + missing.as_array().unwrap().len(),
        "warning_count": warnings.as_array().unwrap().len(),
        "blocking_domains": blocking, "required_missing": missing, "warning_domains": warnings,
    });
    for (field, expected) in expected.as_object().unwrap() {
        if hint.get(field).is_some_and(|value| value != expected) {
            return Err(format!(
                "quality optimization_hint.{field} conflicts with coupled_readiness evidence"
            ));
        }
    }
    Ok(())
}
