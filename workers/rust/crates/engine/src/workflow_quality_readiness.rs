use crate::workflow_quality_checks as check;
use crate::workflow_result_admission::require_converged_result;
use serde_json::{Map, Value};

const DOMAIN_LISTS: [&str; 3] = [
    "coupled_readiness_blocking_domains",
    "coupled_readiness_required_missing",
    "coupled_readiness_warning_domains",
];

pub(crate) fn resolve<'a>(
    sources: &[(Option<&'a Value>, &str)],
) -> Result<Option<&'a Map<String, Value>>, String> {
    let mut selected = None;
    for (value, path) in sources {
        let Some(object) = check::optional_object(*value, path)? else {
            continue;
        };
        validate(object, path)?;
        // Aliases carry one report, not competing reports to merge or prioritize.
        if selected.is_some_and(|previous| previous != object) {
            return Err(format!(
                "quality {path} conflicts with another coupled_readiness report"
            ));
        }
        selected = Some(object);
    }
    Ok(selected)
}

fn validate(object: &Map<String, Value>, path: &str) -> Result<(), String> {
    require_converged_result(object, "transform.prepare_quality_next_round_request", path)?;
    check::boolean(
        object.get("coupled_readiness_ready"),
        &format!("{path}.coupled_readiness_ready"),
        true,
    )?;
    for (field, choices) in [
        (
            "coupled_readiness_state",
            &["ready", "pass", "warn", "block"][..],
        ),
        (
            "coupled_readiness_recommendation",
            &[
                "continue_to_next_round",
                "review_before_next_round",
                "hold_and_repair_inputs",
            ][..],
        ),
    ] {
        let field_path = format!("{path}.{field}");
        if check::text(object.get(field), &field_path)?
            .is_some_and(|value| !choices.contains(&value))
        {
            return Err(format!("quality {field_path} has an unsupported value"));
        }
    }
    for field in DOMAIN_LISTS {
        let field_path = format!("{path}.{field}");
        for (index, domain) in check::list(object.get(field), &field_path)?
            .iter()
            .enumerate()
        {
            check::text(Some(domain), &format!("{field_path}[{index}]"))?;
        }
    }
    Ok(())
}

pub(crate) fn blocks(value: &Value) -> bool {
    value.get("coupled_readiness_ready") == Some(&Value::Bool(false))
        || value.get("coupled_readiness_state").and_then(Value::as_str) == Some("block")
        || value
            .get("coupled_readiness_recommendation")
            .and_then(Value::as_str)
            == Some("hold_and_repair_inputs")
        || has_domains(value, "coupled_readiness_blocking_domains")
        || has_domains(value, "coupled_readiness_required_missing")
}

pub(crate) fn requires_review(value: &Value) -> bool {
    value.get("coupled_readiness_state").and_then(Value::as_str) == Some("warn")
        || value
            .get("coupled_readiness_recommendation")
            .and_then(Value::as_str)
            == Some("review_before_next_round")
        || has_domains(value, "coupled_readiness_warning_domains")
}

fn has_domains(value: &Value, field: &str) -> bool {
    value
        .get(field)
        .and_then(Value::as_array)
        .is_some_and(|domains| !domains.is_empty())
}
