use crate::workflow_quality_checks as check;
use crate::workflow_result_admission::require_converged_result;
use serde_json::{Map, Value};
use std::collections::BTreeSet;

pub(crate) struct RankingInput<'a> {
    pub selected: &'a Map<String, Value>,
    pub candidate_id: &'a str,
    pub score: f64,
    pub ready: bool,
    pub complete: bool,
    pub rejected: Vec<Value>,
}

pub(crate) fn admit(payload: &Value) -> Result<RankingInput<'_>, String> {
    let object = payload
        .as_object()
        .ok_or("quality next round payload must be an object")?;
    converged(object, "payload")?;
    let ranking = check::list(object.get("ranking"), "ranking")?;
    if ranking.is_empty() {
        return Err("quality next round expects a nonempty quality ranking".to_string());
    }
    let mut ids = BTreeSet::new();
    let mut ready_count = 0;
    let mut selected_row = None;
    let mut previous = None;
    for (index, row) in ranking.iter().enumerate() {
        let path = format!("ranking[{index}]");
        let row = row
            .as_object()
            .ok_or_else(|| format!("quality {path} must be an object"))?;
        converged(row, &path)?;
        let id = required_text(row, "candidate_id", &path)?;
        unique(&mut ids, id)?;
        let score = check::score(row.get("score"), id, "score")?;
        let ready = check::boolean(row.get("ready"), &format!("{path}.ready"), false)?;
        if check::count(row.get("rank"), &format!("{path}.rank"))?
            .is_some_and(|rank| rank != index as u64 + 1)
        {
            return Err(format!("quality {path}.rank disagrees with its position"));
        }
        let key = (ready, score, id);
        if previous.is_some_and(|previous| check::ranking_order(previous, key).is_gt()) {
            return Err(format!(
                "quality {path} is out of order: ready, score, candidate_id"
            ));
        }
        previous = Some(key);
        checked_objective(row, score, &path)?;
        ready_count += usize::from(ready);
        selected_row.get_or_insert((row, id, score, ready));
    }
    let rejected = check::list(object.get("rejected_candidates"), "rejected_candidates")?;
    for (index, row) in rejected.iter().enumerate() {
        let path = format!("rejected_candidates[{index}]");
        let row = row
            .as_object()
            .ok_or_else(|| format!("quality {path} must be an object"))?;
        unique(&mut ids, required_text(row, "candidate_id", &path)?)?;
        required_text(row, "error", &path)?;
    }
    for (field, actual) in [
        ("candidate_count", ranking.len()),
        ("input_candidate_count", ids.len()),
        ("rejected_candidate_count", rejected.len()),
        ("ready_candidate_count", ready_count),
    ] {
        if check::count(object.get(field), field)?.is_some_and(|count| count != actual as u64) {
            return Err(format!(
                "quality ranking {field} disagrees with candidate details"
            ));
        }
    }
    let complete = check::boolean(
        object.get("ranking_complete"),
        "ranking_complete",
        rejected.is_empty(),
    )?;
    if complete && !rejected.is_empty() {
        return Err("quality ranking cannot be complete with rejected candidates".to_string());
    }
    let (selected, candidate_id, score, ready) = selected_row.unwrap();
    let id_claim = check::text(object.get("best_candidate_id"), "best_candidate_id")?;
    if id_claim.is_some_and(|id| id != candidate_id) {
        return Err(
            "quality ranking best_candidate_id disagrees with selected candidate".to_string(),
        );
    }
    if object.contains_key("best_candidate_score")
        && check::score(
            object.get("best_candidate_score"),
            "ranking",
            "best_candidate_score",
        )? != score
    {
        return Err(
            "quality ranking best_candidate_score disagrees with selected candidate".to_string(),
        );
    }
    if check::boolean(
        object.get("best_candidate_ready"),
        "best_candidate_ready",
        ready,
    )? != ready
    {
        return Err(
            "quality ranking best_candidate_ready disagrees with selected candidate".to_string(),
        );
    }
    Ok(RankingInput {
        selected,
        candidate_id,
        score,
        ready,
        complete,
        rejected: rejected.to_vec(),
    })
}

pub(crate) fn selected_evidence(selected: &Map<String, Value>) -> Result<(bool, Value), String> {
    let objective = check::optional_object(selected.get("objective"), "selected.objective")?;
    if let Some(objective) = objective {
        converged(objective, "selected.objective")?;
    }
    let objective_ready = check::boolean(
        objective.and_then(|value| value.get("composite_quality_ready")),
        "selected.objective.composite_quality_ready",
        true,
    )?;
    let mut blockers =
        check::list(selected.get("blocking_terms"), "selected.blocking_terms")?.to_vec();
    // Outer evidence can add restrictions, but cannot erase an objective's blockers.
    for blocker in check::list(
        objective.and_then(|value| value.get("composite_quality_blocking_terms")),
        "selected.objective.composite_quality_blocking_terms",
    )? {
        if !blockers.contains(blocker) {
            blockers.push(blocker.clone());
        }
    }
    Ok((objective_ready, Value::Array(blockers)))
}

pub(crate) fn coupled_readiness(value: &Value) -> Result<(), String> {
    let Some(object) = check::optional_object(Some(value), "selected.coupled_readiness")? else {
        return Ok(());
    };
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
        if check::text(object.get(field), field)?.is_some_and(|value| !choices.contains(&value)) {
            return Err(format!("quality {field} has an unsupported value"));
        }
    }
    Ok(())
}

fn required_text<'a>(
    object: &'a Map<String, Value>,
    field: &str,
    path: &str,
) -> Result<&'a str, String> {
    let path = format!("{path}.{field}");
    check::text(object.get(field), &path)?.ok_or_else(|| format!("quality {path} is required"))
}

fn checked_objective(row: &Map<String, Value>, score: f64, path: &str) -> Result<(), String> {
    let path = format!("{path}.objective");
    let Some(objective) = check::optional_object(row.get("objective"), &path)? else {
        return Ok(());
    };
    converged(objective, &path)?;
    if objective.contains_key("composite_quality_score")
        && check::score(
            objective.get("composite_quality_score"),
            &path,
            "composite_quality_score",
        )? != score
    {
        return Err(format!(
            "quality {path}.composite_quality_score disagrees with ranking score"
        ));
    }
    check::boolean(
        objective.get("composite_quality_ready"),
        &format!("{path}.composite_quality_ready"),
        true,
    )?;
    check::list(
        objective.get("composite_quality_blocking_terms"),
        &format!("{path}.composite_quality_blocking_terms"),
    )?;
    Ok(())
}

fn unique<'a>(ids: &mut BTreeSet<&'a str>, id: &'a str) -> Result<(), String> {
    if ids.insert(id) {
        Ok(())
    } else {
        Err(format!("quality ranking duplicate candidate id {id:?}"))
    }
}

fn converged(object: &Map<String, Value>, path: &str) -> Result<(), String> {
    require_converged_result(object, "transform.prepare_quality_next_round_request", path)
}
