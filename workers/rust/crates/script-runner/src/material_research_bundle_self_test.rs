use super::{
    BUNDLE_SCHEMA_VERSION, CHAIN_SCHEMA_VERSION, EXECUTION_SCHEMA_VERSION,
    EXPLORATION_SCHEMA_VERSION, POSTURE, RunnerResult, sha256_json,
    validate_material_research_bundle_value,
};
use serde_json::{Value, json};
use std::path::Path;

pub(super) fn run_self_test() -> RunnerResult<u8> {
    let bad_bundle = json!({
        "schema_version": BUNDLE_SCHEMA_VERSION,
        "posture": POSTURE,
        "study": "unsupported-study",
        "artifact_checksums": { "initial_exploration_sha256": "bad" },
        "initial_exploration": {},
        "next_round_execution_plan": {},
        "next_exploration": {},
        "chain": {},
        "summary": {},
        "reproducibility": { "initial_command": [] },
    });
    expect_failure(Path::new("."), &bad_bundle, "bad checksum")?;

    let artifact = json!({ "schema_version": EXPLORATION_SCHEMA_VERSION, "iteration": 2 });
    let plan = json!({
        "schema_version": EXECUTION_SCHEMA_VERSION,
        "decision": "repair_validation",
        "iteration": 2,
        "runnable_step_count": 1,
    });
    let chain = json!({
        "schema_version": CHAIN_SCHEMA_VERSION,
        "stop_reason": "validation_repair_required"
    });
    let mismatch = json!({
        "schema_version": BUNDLE_SCHEMA_VERSION,
        "posture": POSTURE,
        "study": "heat-spreader",
        "artifact_checksums": {
            "initial_exploration_sha256": sha256_json(&artifact)?,
            "next_round_execution_plan_sha256": sha256_json(&plan)?,
            "next_exploration_sha256": sha256_json(&artifact)?,
            "chain_sha256": sha256_json(&chain)?,
        },
        "initial_exploration": artifact,
        "next_round_execution_plan": plan,
        "next_exploration": artifact,
        "chain": chain,
        "summary": {
            "winner_candidate_id": "candidate-a",
            "reliability_decision": "blocked_by_quality_gates",
            "next_round_decision": "mitigate_design_risk",
            "runnable_next_step_count": 1,
            "next_iteration": 2,
            "chain_stop_reason": "validation_repair_required",
        },
        "reproducibility": { "initial_command": ["kyuubiki-material-explore"] },
    });
    expect_failure(Path::new("."), &mismatch, "summary/plan decision mismatch")?;
    expect_failure(
        Path::new("."),
        &readiness_reason_mismatch_bundle()?,
        "validation readiness reason mismatch",
    )?;
    println!("material research bundle check self-test passed");
    Ok(0)
}

fn readiness_reason_mismatch_bundle() -> RunnerResult<Value> {
    let artifact = json!({
        "schema_version": EXPLORATION_SCHEMA_VERSION,
        "iteration": 1,
        "report": {}
    });
    let plan = json!({
        "schema_version": EXECUTION_SCHEMA_VERSION,
        "decision": "mitigate_design_risk",
        "iteration": 2,
        "runnable_step_count": 1,
    });
    let chain = json!({
        "schema_version": CHAIN_SCHEMA_VERSION,
        "stop_reason": "risk_mitigation_required"
    });
    Ok(json!({
        "schema_version": BUNDLE_SCHEMA_VERSION,
        "posture": POSTURE,
        "study": "heat-spreader",
        "artifact_checksums": {
            "initial_exploration_sha256": sha256_json(&artifact)?,
            "next_round_execution_plan_sha256": sha256_json(&plan)?,
            "next_exploration_sha256": sha256_json(&artifact)?,
            "chain_sha256": sha256_json(&chain)?,
        },
        "initial_exploration": artifact,
        "next_round_execution_plan": plan,
        "next_exploration": artifact,
        "chain": chain,
        "summary": {
            "winner_candidate_id": "candidate-a",
            "reliability_decision": "blocked_by_quality_gates",
            "next_round_decision": "mitigate_design_risk",
            "runnable_next_step_count": 1,
            "next_iteration": 2,
            "chain_stop_reason": "risk_mitigation_required",
            "chain_convergence_state": "blocked_by_quality_gates",
            "chain_round_count": 1,
        },
        "reproducibility": { "initial_command": ["kyuubiki-material-explore"] },
        "research_evidence": {
            "winner_candidate_id": "candidate-a",
            "quality_gate_decision": "blocked_by_quality_gates",
            "ranked_candidate_ids": ["candidate-a"],
            "candidate_count": 1,
            "primary_metric_ids": ["peak_temperature_c"],
            "metric_objective_count": 1,
            "focus_candidate_ids": ["candidate-a"],
            "violated_quality_gate_ids": ["gate.temperature"],
            "plan_decision": "mitigate_design_risk",
            "plan_step_count": 1,
            "chain_round_count": 1,
            "chain_trace_round_count": 1,
            "final_winner_candidate_id": "candidate-a",
        },
        "validation_evidence": {
            "schema_version": "kyuubiki.material-validation-evidence/v1",
            "validation_posture": "screening_validation",
            "baseline_refs": [{ "baseline_id": "baseline-a" }],
            "candidate_confidence_counts": { "low": 1, "medium": 0, "high": 0, "unknown": 0 },
            "sensitivity_summary": {
                "schema_version": "kyuubiki.material-sensitivity-summary/v1",
                "primary_metric_ids": ["peak_temperature_c"],
                "focus_candidate_ids": ["candidate-a"],
                "chain_trace_round_count": 1,
            },
            "acceptance_criteria": [{ "criterion_id": "gate.temperature" }],
            "uncertainty_summary": {
                "schema_version": "kyuubiki.material-uncertainty-summary/v1",
                "candidate_confidence_counts": { "low": 1, "medium": 0, "high": 0, "unknown": 0 },
                "external_validation_required": true,
            },
            "validation_readiness": {
                "schema_version": "kyuubiki.material-validation-readiness/v1",
                "decision": "screening_only",
                "score": 0.4,
                "blocking_reasons": ["external_validation_required"],
                "next_validation_actions": ["run_external_solver_or_analytic_baseline"],
            },
            "external_validation_plan": ["run external baseline"],
            "violated_quality_gate_ids": ["gate.temperature"],
        },
    }))
}

fn expect_failure(root: &Path, bundle: &Value, label: &str) -> RunnerResult<()> {
    if validate_material_research_bundle_value(root, bundle, None).is_ok() {
        Err(format!("self-test did not reject {label}"))
    } else {
        Ok(())
    }
}
