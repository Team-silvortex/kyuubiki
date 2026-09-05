use serde_json::{Map, Value, json};
use std::collections::{HashMap, HashSet};

use crate::workflow_sweep_contract::{
    bool_option, case_ids, object_or_null, optional_case_id, require_count, require_success,
    text_option,
};

pub fn join_parameter_sweep_results(payload: Value, config: Value) -> Result<Value, String> {
    object_or_null(&config, "parameter sweep join config")?;
    let cases = payload
        .get("cases")
        .and_then(Value::as_array)
        .filter(|cases| !cases.is_empty())
        .ok_or_else(|| {
            "transform.join_parameter_sweep_results requires nonempty payload.cases".to_string()
        })?;
    let results = payload
        .get("summaries")
        .or_else(|| payload.get("results"))
        .and_then(Value::as_array)
        .ok_or_else(|| {
            "transform.join_parameter_sweep_results requires payload.summaries or payload.results"
                .to_string()
        })?;
    let summary_field = text_option(config.get("summary_field"), "summary_field", "summary")?;
    let output_field = text_option(config.get("output_field"), "output_field", "summary")?;
    if matches!(
        output_field,
        "id" | "case_id"
            | "caseId"
            | "model"
            | "parameters"
            | "metadata"
            | "result_status"
            | "result_error"
    ) {
        return Err(format!(
            "output_field {output_field:?} is reserved for case identity or provenance"
        ));
    }
    let strict = bool_option(config.get("strict"), "strict", false)?;
    let ids = case_ids(cases)?;
    require_count(&payload, "case_count", cases.len())?;
    let result_ids = results
        .iter()
        .enumerate()
        .map(|(index, result)| optional_case_id(result, &format!("parameter sweep result {index}")))
        .collect::<Result<Vec<_>, _>>()?;
    let keyed_count = result_ids.iter().filter(|id| id.is_some()).count();
    if keyed_count != 0 && keyed_count != results.len() {
        return Err(
            "parameter sweep results cannot mix keyed and positional identities".to_string(),
        );
    }
    let positional = !results.is_empty() && keyed_count == 0;
    if positional && results.len() != cases.len() {
        return Err(
            "positional parameter sweep results must have exactly one result per case".to_string(),
        );
    }
    let mut by_id = HashMap::new();
    for (index, id) in result_ids.iter().enumerate() {
        if let Some(id) = id
            && by_id.insert(*id, index).is_some()
        {
            return Err(format!(
                "parameter sweep result {index} has duplicate case id {id:?}"
            ));
        }
    }
    let known = ids.iter().map(String::as_str).collect::<HashSet<_>>();
    let unmatched = result_ids
        .iter()
        .flatten()
        .filter(|id| !known.contains(**id))
        .copied()
        .collect::<Vec<_>>();
    let mut joined = Vec::with_capacity(cases.len());
    let mut missing = Vec::new();
    let mut rejected = Vec::new();
    let mut joined_count = 0usize;
    for (index, (case, case_id)) in cases.iter().zip(&ids).enumerate() {
        let mut next_case = case
            .as_object()
            .expect("case_ids validates objects")
            .clone();
        // A retry must not inherit either conventional summary alias from an old run.
        for field in ["summary", "result", "result_error", output_field] {
            next_case.remove(field);
        }
        next_case.insert("id".into(), json!(case_id));
        let result_index = if positional {
            Some(index)
        } else {
            by_id.get(case_id.as_str()).copied()
        };
        if let Some(result_index) = result_index {
            let result = &results[result_index];
            let summary = extract_summary(
                result,
                summary_field,
                config.get("summary_field").is_some(),
                positional,
            );
            match summary {
                Ok(summary) => {
                    joined_count += 1;
                    next_case.insert(output_field.into(), Value::Object(summary.clone()));
                    next_case.insert(
                        "result_status".into(),
                        result
                            .get("status")
                            .cloned()
                            .unwrap_or_else(|| json!("joined")),
                    );
                }
                Err(reason) => {
                    missing.push(case_id);
                    next_case.insert("result_status".into(), json!("rejected"));
                    next_case.insert("result_error".into(), json!(reason));
                    rejected.push(json!({"case_id": case_id, "result_index": result_index,
                        "reason": reason, "status": result.get("status"), "error": result.get("error")}));
                }
            }
        } else {
            missing.push(case_id);
            next_case.insert("result_status".into(), json!("missing"));
        }
        joined.push(Value::Object(next_case));
    }
    if strict && !missing.is_empty() {
        return Err(format!(
            "transform.join_parameter_sweep_results missing summaries for {} case(s), first: {:?}; {}",
            missing.len(),
            missing[0],
            rejected
                .iter()
                .find(|entry| entry["case_id"].as_str() == Some(missing[0].as_str()))
                .and_then(|entry| entry["reason"].as_str())
                .unwrap_or("result not received")
        ));
    }
    if strict && !unmatched.is_empty() {
        return Err(format!(
            "parameter sweep join has {} unmatched result id(s), first: {:?}",
            unmatched.len(),
            unmatched[0]
        ));
    }
    Ok(json!({
        "cases": joined, "case_count": cases.len(), "joined_summary_count": joined_count,
        "missing_summary_count": missing.len(), "missing_case_ids": missing,
        "rejected_result_count": rejected.len(), "rejected_results": rejected,
        "unmatched_result_count": unmatched.len(), "unmatched_result_ids": unmatched,
        "matching_mode": if positional { "position" } else { "case_id" },
        "joined_summary_field": output_field,
        "join_complete": missing.is_empty() && unmatched.is_empty(),
    }))
}

fn extract_summary<'a>(
    result: &'a Value,
    field: &str,
    explicit: bool,
    positional: bool,
) -> Result<&'a Map<String, Value>, String> {
    require_success(result, "status", "error", "parameter sweep result")?;
    let selected = if explicit {
        result.get(field)
    } else {
        result
            .get("summary")
            .or_else(|| result.get("result"))
            .or_else(|| {
                (positional && result.get("status").is_none() && result.get("error").is_none())
                    .then_some(result)
            })
    };
    selected
        .and_then(Value::as_object)
        .ok_or_else(|| format!("result requires an object {field:?} summary"))
}
