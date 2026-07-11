use crate::{
    MaterialResearchBundle, MaterialResearchBundleArtifactChecksums,
    MaterialResearchBundleReproducibility, MaterialResearchBundleSummary,
    build_material_exploration_next_round_execution_plan,
    build_material_exploration_next_round_plan, material_reliability_summary,
    material_validation_quality_gate, validate_material_research_bundle,
};
use serde_json::json;

#[test]
fn validation_gate_drives_next_round_plan_and_bundle_consistency() {
    let validation = json!({
        "validation_contract": "kyuubiki.summary_tolerance_validation/v1",
        "validation_passed": false,
        "validation_failed_field_count": 1,
        "validation_missing_field_count": 0,
        "validation_fail_on_missing": true,
        "validation_failures": [{ "field": "peak_temperature_c" }],
        "validation_missing_fields": []
    });
    let validation_gate = material_validation_quality_gate(&validation).expect("validation gate");
    let quality_gates = vec![validation_gate];
    let reliability_summary = material_reliability_summary(&quality_gates);
    let report = json!({
        "winner_candidate_id": "aluminum_6061",
        "warnings": [],
        "candidates": [
            { "candidate_id": "aluminum_6061", "rank": 1, "score": 0.8 }
        ],
        "reliability": {
            "summary": reliability_summary,
            "quality_gates": quality_gates
        }
    });

    let next_round = build_material_exploration_next_round_plan(&report, 1);
    assert_eq!(next_round.decision, "repair_validation");

    let exploration = json!({
        "schema_version": "kyuubiki.material-exploration-run/v1",
        "iteration": 1,
        "study": "material_heat_spreader_screening",
        "report": report,
        "next_round": next_round
    });
    let execution_plan =
        build_material_exploration_next_round_execution_plan(&exploration).expect("execution plan");
    assert_eq!(execution_plan.decision, "repair_validation");
    assert_eq!(
        execution_plan.optimization_objectives["mode"].as_str(),
        Some("validation_repair")
    );

    let plan_value = serde_json::to_value(&execution_plan).expect("plan value");
    let next_exploration = json!({
        "schema_version": "kyuubiki.material-exploration-run/v1",
        "iteration": execution_plan.iteration,
        "study": "material_heat_spreader_screening"
    });
    let chain = json!({
        "schema_version": "kyuubiki.material-exploration-chain/v1",
        "round_count": 1,
        "stop_reason": "validation_repair_required"
    });
    let bundle = MaterialResearchBundle {
        schema_version: "kyuubiki.material-research-bundle/v1".to_string(),
        bundle_id: "validation.repair.bundle".to_string(),
        generated_at_utc: "2026-07-11T00:00:00.000Z".to_string(),
        posture: "screening_research_bundle".to_string(),
        study: "heat-spreader".to_string(),
        artifact_checksums: valid_checksums(),
        reproducibility: reproducibility(),
        execution_trace: json!({ "duration_ms": 0 }),
        summary: MaterialResearchBundleSummary {
            winner_candidate_id: "aluminum_6061".to_string(),
            reliability_decision: "blocked_by_quality_gates".to_string(),
            next_round_decision: execution_plan.decision.clone(),
            runnable_next_step_count: Some(execution_plan.runnable_step_count),
            next_iteration: Some(execution_plan.iteration),
            chain_stop_reason: "validation_repair_required".to_string(),
            chain_convergence_state: Some("blocked_by_quality_gates".to_string()),
            chain_round_count: Some(1),
        },
        initial_exploration: exploration,
        next_round_execution_plan: plan_value,
        next_exploration,
        chain,
    };

    validate_material_research_bundle(&bundle).expect("validation repair bundle should validate");
}

fn valid_checksums() -> MaterialResearchBundleArtifactChecksums {
    MaterialResearchBundleArtifactChecksums {
        initial_exploration_sha256: "0".repeat(64),
        next_round_execution_plan_sha256: "1".repeat(64),
        next_exploration_sha256: "2".repeat(64),
        chain_sha256: "3".repeat(64),
    }
}

fn reproducibility() -> MaterialResearchBundleReproducibility {
    MaterialResearchBundleReproducibility {
        workspace: "workers/rust".to_string(),
        initial_command: vec!["kyuubiki-material-explore".to_string()],
        plan_next_command_template: vec!["kyuubiki-material-explore".to_string()],
        run_next_command_template: vec!["kyuubiki-material-explore".to_string()],
        chain_next_command_template: vec!["kyuubiki-material-explore".to_string()],
        transient_work_files: vec![],
    }
}
