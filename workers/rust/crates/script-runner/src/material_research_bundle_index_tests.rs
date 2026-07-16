use super::{BundleProfile, IndexEntry, build_index};
use serde_json::json;

#[test]
fn self_test_fixture_builds_counts() {
    let index = build_index(vec![IndexEntry {
        profile: BundleProfile {
            study: "heat-spreader",
            file: "heat-spreader.json",
        },
        path: "tmp/a.json".to_string(),
        bundle: json!({
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
        }),
    }]);
    assert_eq!(
        index.pointer("/reliability_decision_counts/blocked_by_quality_gates"),
        Some(&json!(1))
    );
    assert_eq!(index.pointer("/bundles/0/next_iteration"), Some(&json!(2)));
    assert_eq!(
        index.pointer("/bundles/0/final_winner_candidate_id"),
        Some(&json!("candidate-b"))
    );
    assert_eq!(
        index.pointer("/winner_changed_in_chain_count"),
        Some(&json!(1))
    );
}
