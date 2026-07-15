use kyuubiki_headless_sdk::{
    MATERIAL_EXPLORATION_CHAIN_SCHEMA_VERSION, build_material_exploration_next_round_execution_plan,
};
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
    let summaries = runs
        .iter()
        .map(exploration_summary)
        .collect::<Result<Vec<_>, _>>()?;
    let decision_counts = decision_counts(&summaries);
    let repair_summary = repair_summary(&runs);
    let repair_plan = repair_plan(&repair_summary);
    let convergence_assessment = convergence_assessment(&summaries, &repair_summary);
    Ok(json!({
        "schema_version": MATERIAL_EXPLORATION_CHAIN_SCHEMA_VERSION,
        "source_schema_version": source_schema_version,
        "round_count": runs.len(),
        "stop_reason": chain_stop_reason(&summaries, rounds),
        "all_winners_stable": all_winners_stable(&summaries),
        "convergence_assessment": convergence_assessment,
        "decision_counts": decision_counts,
        "optimization_trace": optimization_trace(&summaries),
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

fn convergence_assessment(summaries: &[Value], repair_summary: &Value) -> Value {
    let score_delta = winner_score_delta(summaries);
    let winners_stable = all_winners_stable(summaries);
    let repair_required = repair_summary
        .get("required")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let score_stable = score_delta.is_some_and(|delta| delta <= 0.001);
    let state = if repair_required {
        "blocked_by_quality_gates"
    } else if winners_stable && score_stable {
        "stable_candidate"
    } else if winners_stable {
        "winner_stable_score_moving"
    } else {
        "winner_changed"
    };
    json!({
        "schema_version": "kyuubiki.material-chain-convergence-assessment/v1",
        "state": state,
        "winner_stable": winners_stable,
        "winner_score_delta": score_delta,
        "score_stable_threshold": 0.001,
        "repair_required": repair_required,
        "recommendation": convergence_recommendation(state),
    })
}

fn winner_score_delta(summaries: &[Value]) -> Option<f64> {
    let scores = summaries
        .iter()
        .filter_map(|summary| summary.get("winner_score").and_then(Value::as_f64))
        .collect::<Vec<_>>();
    let (Some(first), Some(last)) = (scores.first(), scores.last()) else {
        return None;
    };
    Some((last - first).abs())
}

fn convergence_recommendation(state: &str) -> &'static str {
    match state {
        "blocked_by_quality_gates" => {
            "repair or mitigate quality gates before declaring convergence"
        }
        "stable_candidate" => "candidate is stable enough for a higher-fidelity validation pass",
        "winner_stable_score_moving" => {
            "keep iterating or raise fidelity until winner score stabilizes"
        }
        _ => "continue exploration because the incumbent winner changed",
    }
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

        for gate_id in blocking_gate_ids(run) {
            push_unique_string(&mut violated_gate_ids, Some(&gate_id));
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

fn blocking_gate_ids(run: &Value) -> Vec<String> {
    let summary_ids = run
        .get("report")
        .and_then(|report| report.get("reliability"))
        .and_then(|reliability| reliability.get("summary"))
        .and_then(|summary| summary.get("blocking_gate_ids"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(ToString::to_string)
        .collect::<Vec<_>>();

    if !summary_ids.is_empty() {
        return summary_ids;
    }

    run.get("report")
        .and_then(|report| report.get("reliability"))
        .and_then(|reliability| reliability.get("quality_gates"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|gate| gate.get("status").and_then(Value::as_str) != Some("pass"))
        .filter_map(|gate| gate.get("id").and_then(Value::as_str))
        .map(ToString::to_string)
        .collect()
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

fn optimization_trace(summaries: &[Value]) -> Vec<Value> {
    summaries
        .iter()
        .map(|summary| {
            let objectives = summary
                .get("optimization_objectives")
                .unwrap_or(&Value::Null);
            json!({
                "iteration": summary.get("iteration").and_then(Value::as_u64),
                "decision": summary.get("next_round_decision").and_then(Value::as_str),
                "mode": objectives.get("mode").and_then(Value::as_str),
                "winner_candidate_id": summary.get("winner_candidate_id").and_then(Value::as_str),
                "primary_metric_ids": objectives
                    .get("primary_metric_ids")
                    .cloned()
                    .unwrap_or_else(|| json!([])),
                "violated_quality_gate_ids": objectives
                    .get("violated_quality_gate_ids")
                    .cloned()
                    .unwrap_or_else(|| json!([])),
            })
        })
        .collect()
}

fn push_unique_string(values: &mut Vec<String>, value: Option<&str>) {
    let Some(value) = value else {
        return;
    };
    if !values.iter().any(|entry| entry == value) {
        values.push(value.to_string());
    }
}

fn exploration_summary(exploration: &Value) -> Result<Value, String> {
    let plan = build_material_exploration_next_round_execution_plan(exploration)?;
    let winner = exploration
        .get("report")
        .and_then(|report| report.get("winner_candidate_id"))
        .and_then(Value::as_str);
    Ok(json!({
        "iteration": exploration.get("iteration").and_then(Value::as_u64),
        "mode": exploration.get("mode").and_then(Value::as_str),
        "winner_candidate_id": winner,
        "winner_score": winner_score(exploration, winner),
        "lineage_schema_version": exploration
            .get("lineage")
            .and_then(|lineage| lineage.get("schema_version"))
            .and_then(Value::as_str),
        "source_iteration": exploration
            .get("lineage")
            .and_then(|lineage| lineage.get("source_iteration"))
            .and_then(Value::as_u64),
        "material_card_refs": material_card_refs(exploration),
        "next_round_iteration": exploration
            .get("next_round")
            .and_then(|next_round| next_round.get("iteration"))
            .and_then(Value::as_u64),
        "next_round_decision": exploration
            .get("next_round")
            .and_then(|next_round| next_round.get("decision"))
            .and_then(Value::as_str),
        "optimization_objectives": plan.optimization_objectives,
    }))
}

fn material_card_refs(exploration: &Value) -> Value {
    exploration
        .get("material_card_refs")
        .cloned()
        .or_else(|| {
            exploration
                .get("lineage")
                .and_then(|lineage| lineage.get("material_card_refs"))
                .cloned()
        })
        .or_else(|| {
            exploration
                .get("report")
                .and_then(|report| report.get("material_card_refs"))
                .cloned()
        })
        .unwrap_or_else(|| json!([]))
}

fn winner_score(exploration: &Value, winner: Option<&str>) -> Option<f64> {
    let winner = winner?;
    exploration
        .get("report")
        .and_then(|report| report.get("candidates"))
        .and_then(Value::as_array)?
        .iter()
        .find(|candidate| candidate.get("candidate_id").and_then(Value::as_str) == Some(winner))?
        .get("score")
        .and_then(Value::as_f64)
}
