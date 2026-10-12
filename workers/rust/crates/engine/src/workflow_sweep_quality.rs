use crate::workflow_sweep_contract::{
    bool_option, finite, number, object_or_null, optional_case_id, optional_number, require_count,
    require_evaluated_row, require_evaluation_complete, text_option,
};
use crate::workflow_sweep_score::{INFEASIBLE_PENALTY, Objective, ScoreIdentity};
use serde_json::{Map, Value, json};
use std::collections::HashSet;

pub fn map_parameter_sweep_scores_to_quality_candidates(
    payload: Value,
    config: Value,
) -> Result<Value, String> {
    object_or_null(&config, "quality candidate mapping config")?;
    require_evaluation_complete(&payload)?;
    let rows = payload
        .get("scored_rows")
        .and_then(Value::as_array)
        .filter(|rows| !rows.is_empty())
        .ok_or("quality candidate mapping requires nonempty payload.scored_rows")?;
    require_count(&payload, "scored_count", rows.len())?;
    let domain = text_option(
        config.get("quality_domain"),
        "config.quality_domain",
        "parameter_sweep",
    )?;
    let include_source = bool_option(
        config.get("include_source_row"),
        "config.include_source_row",
        true,
    )?;
    let reference =
        optional_number(config.get("score_reference"), "config.score_reference")?.unwrap_or(0.0);
    let mut seen = HashSet::new();
    let mut prepared = Vec::with_capacity(rows.len());
    for (index, row) in rows.iter().enumerate() {
        let path = format!("scored_rows[{index}]");
        let id = optional_case_id(row, &path)?
            .ok_or_else(|| format!("{path} requires explicit case identity"))?;
        if !seen.insert(id) {
            return Err(format!("{path} duplicate candidate id {id:?}"));
        }
        require_evaluated_row(row, &path)?;
        let score = number(
            row.get("objective_score"),
            &format!("{path}.objective_score"),
        )?;
        let feasible = row
            .get("objective_feasible")
            .and_then(Value::as_bool)
            .ok_or_else(|| format!("{path}.objective_feasible must be an explicit boolean"))?;
        let quality_score = finite(reference - score, &format!("{path} quality score"))?;
        if quality_score < 0.0 {
            return Err(format!(
                "{path} objective_score exceeds score_reference; supply a fixed calibrated reference, not a batch-relative offset"
            ));
        }
        let label = text_option(row.get("label"), &format!("{path}.label"), id)?;
        let (dominant, blocking) = breakdown(row, score, feasible, &path)?;
        let mut candidate = json!({"label":label, "parameters":row.get("parameters"), "metadata":row.get("metadata")});
        if include_source {
            candidate["source_row"] = row.clone();
        }
        candidate["qualities"] = json!({domain:{
            format!("{domain}_quality_contract"):format!("kyuubiki.{domain}_quality_score/v1"),
            format!("{domain}_quality_score"):quality_score,
            format!("{domain}_quality_ready"):feasible,
            format!("{domain}_quality_missing_metric_count"):0,
            format!("{domain}_quality_grade"):if feasible {"candidate"} else {"blocked"},
            format!("{domain}_quality_watch_count"):blocking.len(),
            format!("{domain}_quality_dominant_term"):dominant,
            format!("{domain}_quality_blocking_terms"):blocking,
            format!("{domain}_quality_summary"):format!("Parameter sweep candidate {id}: quality_score={quality_score:.4}, feasible={feasible}.")
        }});
        prepared.push((
            ScoreIdentity {
                id: id.to_string(),
                score,
                feasible,
            },
            row,
            candidate,
        ));
    }
    let feasible = prepared.iter().filter(|entry| entry.0.feasible).count();
    require_count(&payload, "feasible_count", feasible)?;
    if bool_option(
        payload.get("selection_ready"),
        "payload.selection_ready",
        feasible > 0,
    )? != (feasible > 0)
    {
        return Err("selection_ready disagrees with feasible candidates".into());
    }
    let best = prepared
        .iter()
        .filter(|entry| entry.0.feasible)
        .min_by(|left, right| left.0.compare(&right.0));
    let best_id = best.map(|entry| entry.0.id.clone());
    validate_best(&payload, best.map(|entry| (&entry.0, entry.1)))?;
    let candidates = prepared
        .into_iter()
        .map(|(identity, _, candidate)| (identity.id, candidate))
        .collect::<Map<_, _>>();
    Ok(
        json!({"quality_candidates_contract":"kyuubiki.parameter_sweep_quality_candidates/v1",
        "candidate_count":candidates.len(), "source_best_case_id":best_id,
        "quality_score_mapping":{"mode":"reference_minus_objective", "score_reference":reference},
        "candidates":candidates}),
    )
}

fn validate_best(payload: &Value, actual: Option<(&ScoreIdentity, &Value)>) -> Result<(), String> {
    let Some(best) = payload.get("best") else {
        return Ok(());
    };
    let Some((identity, row)) = actual else {
        return if best.is_null() {
            Ok(())
        } else {
            Err("best must be null when no candidate is feasible".into())
        };
    };
    if optional_case_id(best, "best")? != Some(identity.id.as_str()) {
        return Err("best identity disagrees with feasible score ordering".into());
    }
    for (key, value) in best.as_object().expect("checked identity object") {
        if !matches!(key.as_str(), "id" | "case_id" | "caseId") && row.get(key) != Some(value) {
            return Err(format!("best.{key} disagrees with its scored row"));
        }
    }
    Ok(())
}

fn breakdown(
    row: &Value,
    aggregate: f64,
    ready: bool,
    path: &str,
) -> Result<(Value, Vec<Value>), String> {
    let Some(raw) = row.get("objective_breakdown") else {
        return Ok(fallback(ready));
    };
    let terms = raw
        .as_array()
        .filter(|terms| !terms.is_empty())
        .ok_or_else(|| {
            format!("{path}.objective_breakdown must be a nonempty array when present")
        })?;
    let mut seen = HashSet::new();
    let mut total = 0.0;
    let mut records = Vec::new();
    let mut blockers = Vec::new();
    let mut all_explicit = true;
    for (index, term) in terms.iter().enumerate() {
        let context = format!("{path}.objective_breakdown[{index}]");
        let objective = Objective::parse(term, &context)?;
        if !seen.insert(objective.field.clone()) {
            return Err(format!("{context} duplicates field {}", objective.field));
        }
        let value = number(term.get("value"), &format!("{context}.value"))?;
        if let Some(source) = row.get(&objective.field) {
            if number(Some(source), &format!("{path}.{}", objective.field))? != value {
                return Err(format!("{context}.value disagrees with its source metric"));
            }
        }
        let score = number(term.get("score"), &format!("{context}.score"))?;
        if term.get("weight").is_some() && objective.score(value, &context)? != score {
            return Err(format!("{context}.score disagrees with objective formula"));
        }
        total = finite(total + score, &format!("{path} breakdown total"))?;
        let constrained = objective.minimum.is_some() || objective.maximum.is_some();
        let allowed = objective.allows(value);
        let feasible = bool_option(
            term.get("feasible"),
            &format!("{context}.feasible"),
            allowed,
        )?;
        if constrained && feasible != allowed {
            return Err(format!("{context}.feasible disagrees with limits"));
        }
        all_explicit &= constrained || term.get("feasible").is_some();
        let record = json!({"field":objective.field, "goal":objective.goal, "value":value, "score":score,
            "status":if feasible {"ok"} else {"blocked"}, "min_allowed":objective.minimum, "max_allowed":objective.maximum});
        if !feasible {
            blockers.push(record.clone());
        }
        records.push(record);
    }
    if (ready && !blockers.is_empty()) || (!ready && all_explicit && blockers.is_empty()) {
        return Err(format!(
            "{path}.objective_feasible disagrees with breakdown"
        ));
    }
    let expected = finite(
        if ready {
            total
        } else {
            total - INFEASIBLE_PENALTY
        },
        &format!("{path} breakdown penalty"),
    )?;
    if expected != aggregate {
        return Err(format!(
            "{path}.objective_score disagrees with breakdown total"
        ));
    }
    if !ready && blockers.is_empty() {
        let mut dominant = dominant(&records);
        dominant["status"] = json!("blocked");
        blockers.push(dominant);
    }
    let focus = if ready {
        dominant(&records)
    } else {
        dominant(&blockers)
    };
    Ok((focus, blockers))
}

fn dominant(terms: &[Value]) -> Value {
    terms
        .iter()
        .reduce(|best, term| {
            if term["score"].as_f64().expect("validated score").abs()
                > best["score"].as_f64().expect("validated score").abs()
            {
                term
            } else {
                best
            }
        })
        .expect("nonempty checked terms")
        .clone()
}

fn fallback(ready: bool) -> (Value, Vec<Value>) {
    let term = json!({"field":if ready {Value::Null} else {json!("objective_feasible")}, "status":if ready {"ok"} else {"blocked"}});
    (term.clone(), if ready { Vec::new() } else { vec![term] })
}
