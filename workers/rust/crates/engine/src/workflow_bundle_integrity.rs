use crate::workflow_guard_contract::optional_text;
use crate::workflow_result_admission::require_converged_result;
use serde_json::{Map, Value};

const CONTRACT: &str = "kyuubiki.workflow_diagnostics/v1";

pub(crate) struct DiagnosticSource<'a> {
    pub(crate) id: &'a str,
    pub(crate) payload: &'a Value,
    pub(crate) object: &'a Map<String, Value>,
    pub(crate) node_count: Option<u64>,
    pub(crate) element_count: Option<u64>,
}

pub(crate) fn configured_bool(config: &Value, key: &str, default: bool) -> Result<bool, String> {
    if !config.is_null() && !config.is_object() {
        return Err("config must be an object or null".into());
    }
    config
        .get(key)
        .map(|value| {
            value
                .as_bool()
                .ok_or_else(|| format!("config.{key} must be a boolean"))
        })
        .transpose()
        .map(|value| value.unwrap_or(default))
}

pub(crate) fn diagnostic_sources<'a>(
    object: &'a Map<String, Value>,
    include_plain: bool,
    operator: &str,
) -> Result<Vec<DiagnosticSource<'a>>, String> {
    require_converged_result(object, operator, "payload")?;
    let mut sources = Vec::new();
    for (id, payload) in object {
        if ["converged", "stability_result"].contains(&id.as_str()) {
            continue;
        }
        let path = format!("payload.{id}");
        let Some(entry) = payload.as_object() else {
            if include_plain {
                return Err(format!("{path} must be an object diagnostic source"));
            }
            continue;
        };
        if let Some(contract) = entry.get("diagnostic_contract") {
            if contract.as_str() != Some(CONTRACT) {
                return Err(format!("{path}.diagnostic_contract must be {CONTRACT}"));
            }
        } else if entry.keys().any(|key| key.starts_with("diagnostic_")) {
            return Err(format!(
                "{path}.diagnostic_contract is required for a declared diagnostic source"
            ));
        } else if !include_plain {
            continue;
        }
        require_converged_result(entry, operator, &path)?;
        let prefix = optional_text(entry, "diagnostic_prefix", &path)?;
        for key in ["diagnostic_domain", "diagnostic_subject"] {
            optional_text(entry, key, &path)?;
        }
        if let Some(groups) = entry.get("diagnostic_metric_groups") {
            let groups = groups
                .as_array()
                .ok_or_else(|| format!("{path}.diagnostic_metric_groups must be an array"))?;
            for (index, group) in groups.iter().enumerate() {
                if group.as_str().is_none_or(|name| name.trim().is_empty()) {
                    return Err(format!(
                        "{path}.diagnostic_metric_groups[{index}] must be a nonblank string"
                    ));
                }
            }
        }
        let node_count = read_count(entry, prefix, "node", &path)?;
        let element_count = read_count(entry, prefix, "element", &path)?;
        let count_fields = prefix.map(|prefix| {
            [
                format!("{prefix}_node_count"),
                format!("{prefix}_element_count"),
            ]
        });
        let mut measured = false;
        for (key, value) in entry {
            if value.is_number() {
                if value.as_f64().is_none_or(|number| !number.is_finite()) {
                    return Err(format!("{path}.{key} must be a finite number"));
                }
                let count_key = count_fields
                    .as_ref()
                    .is_some_and(|fields| fields.contains(key));
                measured |= !key.starts_with("diagnostic_") && !count_key;
            }
        }
        if !measured {
            return Err(format!("{path} has no numeric diagnostic measurements"));
        }
        sources.push(DiagnosticSource {
            id,
            payload,
            object: entry,
            node_count,
            element_count,
        });
    }
    if sources.is_empty() {
        return Err("did not find any diagnostics payloads".into());
    }
    Ok(sources)
}

fn read_count(
    object: &Map<String, Value>,
    prefix: Option<&str>,
    label: &str,
    path: &str,
) -> Result<Option<u64>, String> {
    let canonical = format!("diagnostic_{label}_count");
    let fallback = prefix.map(|prefix| format!("{prefix}_{label}_count"));
    let field = if object.contains_key(&canonical) {
        Some(canonical.as_str())
    } else {
        fallback.as_deref().filter(|key| object.contains_key(*key))
    };
    field
        .map(|field| {
            object[field]
                .as_u64()
                .ok_or_else(|| format!("{path}.{field} must be a nonnegative integer"))
        })
        .transpose()
}

pub(crate) fn total_count(
    sources: &[DiagnosticSource<'_>],
    label: &str,
) -> Result<Option<u64>, String> {
    let (mut total, mut complete) = (0_u64, true);
    for source in sources {
        let count = if label == "node" {
            source.node_count
        } else {
            source.element_count
        };
        if let Some(count) = count {
            total = total
                .checked_add(count)
                .ok_or_else(|| format!("bundle_total_{label}_count overflow"))?;
        } else {
            complete = false;
        }
    }
    Ok(complete.then_some(total))
}

pub(crate) fn check_bundle_admission(
    object: &Map<String, Value>,
    operator: &str,
) -> Result<(), String> {
    require_converged_result(object, operator, "payload")?;
    if let Some(payloads) = object.get("bundle_payloads") {
        let payloads = payloads
            .as_object()
            .ok_or("payload.bundle_payloads must be an object")?;
        for (source, entry) in payloads {
            let path = format!("payload.bundle_payloads.{source}");
            let entry = entry
                .as_object()
                .ok_or_else(|| format!("{path} must be an object"))?;
            require_converged_result(entry, operator, &path)?;
        }
    }
    Ok(())
}
