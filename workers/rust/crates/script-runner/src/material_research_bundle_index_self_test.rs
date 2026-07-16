use super::{BundleProfile, IndexEntry, RunnerResult, build_index, validate_index};
use serde_json::{Value, json};

pub(super) fn run_self_test() -> RunnerResult<String> {
    let index = build_index(vec![IndexEntry {
        profile: profile(),
        path: "tmp/a.json".to_string(),
        bundle: self_test_bundle(),
    }]);
    if index.get("bundle_count").and_then(Value::as_u64) != Some(1)
        || index.pointer("/reliability_decision_counts/blocked_by_quality_gates")
            != Some(&Value::from(1))
    {
        return Err("self-test did not build expected index counts".to_string());
    }
    if index.pointer("/bundles/0/runnable_next_step_count") != Some(&Value::from(3))
        || index.pointer("/bundles/0/next_iteration") != Some(&Value::from(2))
    {
        return Err("self-test did not retain next-round execution summary".to_string());
    }
    if index.pointer("/bundles/0/final_winner_candidate_id") != Some(&Value::from("candidate-b"))
        || index.pointer("/bundles/0/winner_changed_in_chain") != Some(&Value::from(true))
        || index.pointer("/winner_changed_in_chain_count") != Some(&Value::from(1))
    {
        return Err("self-test did not retain compact research evidence".to_string());
    }
    if index.pointer("/bundles/0/validation_priority") != Some(&Value::from("p0_validation_repair"))
        || index.pointer("/bundles/0/validation_priority_rank") != Some(&Value::from(0))
        || index.pointer("/bundles/0/validation_priority_reasons/0")
            != Some(&Value::from("winner_changed_in_chain"))
        || index.pointer("/validation_priority_counts/p0_validation_repair")
            != Some(&Value::from(1))
    {
        return Err("self-test did not retain validation priority".to_string());
    }
    Ok("material research bundle index self-test passed".to_string())
}

pub(super) fn run_check_self_test() -> RunnerResult<String> {
    let mut index = build_index(vec![IndexEntry {
        profile: profile(),
        path: "tmp/a.json".to_string(),
        bundle: self_test_bundle(),
    }]);
    validate_index(&index)?;
    index["winner_changed_in_chain_count"] = Value::from(0);
    if validate_index(&index).is_ok() {
        return Err("self-test did not reject winner drift count mismatch".to_string());
    }
    let mut readiness_mismatch = build_index(vec![IndexEntry {
        profile: profile(),
        path: "tmp/a.json".to_string(),
        bundle: self_test_bundle(),
    }]);
    readiness_mismatch["bundles"][0]["validation_blocking_reasons"] =
        Value::Array(vec![Value::from("external_validation_required")]);
    if validate_index(&readiness_mismatch).is_ok() {
        return Err("self-test did not reject validation readiness reason mismatch".to_string());
    }
    let mut priority_mismatch = build_index(vec![IndexEntry {
        profile: profile(),
        path: "tmp/a.json".to_string(),
        bundle: self_test_bundle(),
    }]);
    priority_mismatch["bundles"][0]["validation_priority_rank"] = Value::from(1);
    if validate_index(&priority_mismatch).is_ok() {
        return Err("self-test did not reject validation priority mismatch".to_string());
    }
    let mut priority_reason_mismatch = build_index(vec![IndexEntry {
        profile: profile(),
        path: "tmp/a.json".to_string(),
        bundle: self_test_bundle(),
    }]);
    priority_reason_mismatch["bundles"][0]["validation_priority_reasons"] =
        Value::Array(vec![Value::from("screening_followup")]);
    if validate_index(&priority_reason_mismatch).is_ok() {
        return Err("self-test did not reject validation priority reason mismatch".to_string());
    }
    let mut priority_count_mismatch = build_index(vec![IndexEntry {
        profile: profile(),
        path: "tmp/a.json".to_string(),
        bundle: self_test_bundle(),
    }]);
    priority_count_mismatch["validation_priority_counts"]["p0_validation_repair"] = Value::from(0);
    if validate_index(&priority_count_mismatch).is_ok() {
        return Err("self-test did not reject validation priority count mismatch".to_string());
    }
    Ok("material research bundle index check self-test passed".to_string())
}

fn profile() -> BundleProfile {
    BundleProfile {
        study: "heat-spreader",
        file: "a.json",
    }
}

fn self_test_bundle() -> Value {
    json!({
        "study": "heat-spreader",
        "bundle_id": "bundle.a",
        "posture": "screening_research_bundle",
        "summary": {
            "winner_candidate_id": "candidate-a",
            "reliability_decision": "blocked_by_quality_gates",
            "next_round_decision": "mitigate_design_risk",
            "runnable_next_step_count": 3,
            "next_iteration": 2,
            "chain_stop_reason": "risk_mitigation_required",
            "chain_convergence_state": "blocked_by_quality_gates",
            "chain_round_count": 2,
        },
        "research_evidence": {
            "candidate_count": 2,
            "ranked_candidate_ids": ["candidate-a", "candidate-b"],
            "winner_candidate_id": "candidate-a",
            "primary_metric_ids": ["peak_temperature_c"],
            "metric_objective_count": 1,
            "violated_quality_gate_ids": ["gate.temperature"],
            "focus_candidate_ids": ["candidate-a"],
            "quality_gate_decision": "blocked_by_quality_gates",
            "plan_decision": "mitigate_design_risk",
            "plan_step_count": 3,
            "chain_round_count": 2,
            "chain_trace_round_count": 2,
            "final_winner_candidate_id": "candidate-b",
        },
        "validation_evidence": {
            "validation_posture": "screening_validation",
            "baseline_refs": [{ "baseline_id": "baseline-a" }],
            "candidate_confidence_counts": { "low": 1, "medium": 1, "high": 0, "unknown": 0 },
            "acceptance_criteria": [{ "criterion_id": "gate.temperature" }],
            "uncertainty_summary": { "external_validation_required": true },
            "validation_readiness": {
                "decision": "screening_only",
                "score": 0.4,
                "blocking_reasons": [
                    "external_validation_required",
                    "violated_quality_gates",
                    "low_confidence_material_cards",
                ],
                "next_validation_actions": ["run_external_solver_or_analytic_baseline"],
            },
        },
    })
}
