use kyuubiki_headless_sdk::MATERIAL_EXPLORATION_CHAIN_SCHEMA_VERSION;
use serde_json::{Map, Value, json};

pub(crate) fn chain_next_rounds_from_initial(
    initial: Value,
    rounds: usize,
    mut run_next: impl FnMut(&Value) -> Result<Value, String>,
) -> Result<Value, String> {
    if rounds == 0 {
        return Err("--rounds must be at least 1".to_string());
    }
    let source_schema_version = initial
        .get("schema_version")
        .and_then(Value::as_str)
        .map(ToString::to_string);
    let mut current = initial;
    let mut runs = Vec::with_capacity(rounds);
    for _ in 0..rounds {
        let next = run_next(&current)?;
        current = next.clone();
        runs.push(next);
    }
    let final_run = runs
        .last()
        .ok_or_else(|| "material exploration chain produced no runs".to_string())?;
    let summaries = runs.iter().map(exploration_summary).collect::<Vec<_>>();
    let decision_counts = decision_counts(&summaries);
    let repair_summary = repair_summary(&runs);
    let repair_plan = repair_plan(&repair_summary);
    Ok(json!({
        "schema_version": MATERIAL_EXPLORATION_CHAIN_SCHEMA_VERSION,
        "source_schema_version": source_schema_version,
        "round_count": runs.len(),
        "stop_reason": chain_stop_reason(&summaries, rounds),
        "all_winners_stable": all_winners_stable(&summaries),
        "decision_counts": decision_counts,
        "repair_summary": repair_summary,
        "repair_plan": repair_plan,
        "final_iteration": final_run.get("iteration").and_then(Value::as_u64),
        "final_winner_candidate_id": final_run
            .get("report")
            .and_then(|report| report.get("winner_candidate_id"))
            .and_then(Value::as_str),
        "summaries": summaries,
        "runs": runs,
    }))
}

fn chain_stop_reason(summaries: &[Value], requested_rounds: usize) -> &'static str {
    if summaries.iter().any(|summary| {
        summary.get("next_round_decision").and_then(Value::as_str) == Some("repair_or_rerun")
    }) {
        "repair_required"
    } else if summaries.iter().any(|summary| {
        summary.get("next_round_decision").and_then(Value::as_str) == Some("mitigate_design_risk")
    }) {
        "risk_mitigation_required"
    } else if summaries.len() >= requested_rounds {
        "round_budget_exhausted"
    } else {
        "chain_incomplete"
    }
}

fn decision_counts(summaries: &[Value]) -> Value {
    let mut counts = Map::new();
    for summary in summaries {
        let decision = summary
            .get("next_round_decision")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let next_count = counts.get(decision).and_then(Value::as_u64).unwrap_or(0) + 1;
        counts.insert(decision.to_string(), json!(next_count));
    }
    Value::Object(counts)
}

fn all_winners_stable(summaries: &[Value]) -> bool {
    let mut winners = summaries
        .iter()
        .filter_map(|summary| summary.get("winner_candidate_id").and_then(Value::as_str));
    let Some(first) = winners.next() else {
        return false;
    };
    winners.all(|winner| winner == first)
}

fn repair_summary(runs: &[Value]) -> Value {
    let mut violated_gate_ids = Vec::new();
    let mut focus_candidate_ids = Vec::new();
    let mut warning_count = 0_u64;

    for run in runs {
        warning_count += run
            .get("report")
            .and_then(|report| report.get("warnings"))
            .and_then(Value::as_array)
            .map(|warnings| warnings.len() as u64)
            .unwrap_or(0);

        for gate in run
            .get("report")
            .and_then(|report| report.get("reliability"))
            .and_then(|reliability| reliability.get("quality_gates"))
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            if gate.get("status").and_then(Value::as_str) != Some("pass") {
                push_unique_string(
                    &mut violated_gate_ids,
                    gate.get("id").and_then(Value::as_str),
                );
            }
        }

        if run
            .get("next_round")
            .and_then(|next_round| next_round.get("decision"))
            .and_then(Value::as_str)
            .is_some_and(is_attention_decision)
        {
            for candidate_id in run
                .get("next_round")
                .and_then(|next_round| next_round.get("focus_candidate_ids"))
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
            {
                push_unique_string(&mut focus_candidate_ids, Some(candidate_id));
            }
        }
    }

    json!({
        "required": warning_count > 0 || !violated_gate_ids.is_empty(),
        "warning_count": warning_count,
        "violated_gate_ids": violated_gate_ids,
        "focus_candidate_ids": focus_candidate_ids,
    })
}

fn is_attention_decision(decision: &str) -> bool {
    matches!(decision, "repair_or_rerun" | "mitigate_design_risk")
}

fn repair_plan(summary: &Value) -> Value {
    if summary.get("required").and_then(Value::as_bool) != Some(true) {
        return json!({
            "required": false,
            "priority": "none",
            "actions": [],
        });
    }

    let violated_gate_ids = summary
        .get("violated_gate_ids")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let focus_candidate_ids = summary
        .get("focus_candidate_ids")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut actions = vec![
        json!({
            "id": "inspect_violated_quality_gates",
            "target_gate_ids": violated_gate_ids,
        }),
        json!({
            "id": "generate_lower_risk_neighbor_candidates",
            "target_candidate_ids": focus_candidate_ids.clone(),
        }),
        json!({
            "id": "rerun_focused_candidates",
            "target_candidate_ids": focus_candidate_ids,
        }),
    ];
    if summary
        .get("warning_count")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        > 0
    {
        actions.push(json!({
            "id": "resolve_report_warnings",
            "warning_count": summary.get("warning_count").and_then(Value::as_u64),
        }));
    }
    actions.push(json!({
        "id": "rebuild_report_before_expansion",
    }));

    json!({
        "required": true,
        "priority": "before_expansion",
        "actions": actions,
    })
}

fn push_unique_string(values: &mut Vec<String>, value: Option<&str>) {
    let Some(value) = value else {
        return;
    };
    if !values.iter().any(|entry| entry == value) {
        values.push(value.to_string());
    }
}

fn exploration_summary(exploration: &Value) -> Value {
    json!({
        "iteration": exploration.get("iteration").and_then(Value::as_u64),
        "mode": exploration.get("mode").and_then(Value::as_str),
        "winner_candidate_id": exploration
            .get("report")
            .and_then(|report| report.get("winner_candidate_id"))
            .and_then(Value::as_str),
        "next_round_iteration": exploration
            .get("next_round")
            .and_then(|next_round| next_round.get("iteration"))
            .and_then(Value::as_u64),
        "next_round_decision": exploration
            .get("next_round")
            .and_then(|next_round| next_round.get("decision"))
            .and_then(Value::as_str),
    })
}
