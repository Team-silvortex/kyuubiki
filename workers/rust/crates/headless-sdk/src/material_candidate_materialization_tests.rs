use crate::{
    HeadlessWorkflowStep, MATERIAL_EXPLORATION_SCHEMA_VERSION,
    apply_material_candidate_review_decision, build_material_candidate_materialization_plan,
    build_material_candidate_materialization_request,
    build_material_exploration_next_round_execution_plan,
};
use serde_json::{Value, json};

#[test]
fn next_round_execution_plan_filters_design_risk_steps_to_focus_candidates() {
    let exploration = json!({
        "schema_version": MATERIAL_EXPLORATION_SCHEMA_VERSION,
        "study": "material_composite_thermo_electric_panel",
        "report": {
            "warnings": ["copper_ptfe_glass_epoxy exceeds prototype interface risk threshold"],
            "candidates": [
                {
                    "candidate_id": "copper_ptfe_glass_epoxy",
                    "weakest_interface": {
                        "dominant_driver": "thermal_expansion_mismatch"
                    }
                }
            ],
            "reliability": {
                "quality_gates": [
                    { "id": "gate.interface_risk.prototype", "status": "violate" }
                ]
            }
        },
        "next_round": {
            "iteration": 2,
            "decision": "mitigate_design_risk",
            "focus_candidate_ids": ["copper_ptfe_glass_epoxy"],
            "actions": ["generate_lower_risk_neighbor_candidates"]
        }
    });

    let plan =
        build_material_exploration_next_round_execution_plan(&exploration).expect("next plan");

    assert_eq!(plan.decision, "mitigate_design_risk");
    assert_eq!(plan.runnable_step_count, 1);
    assert_eq!(plan.risk_mitigation_hints.len(), 1);
    assert_eq!(
        plan.risk_mitigation_hints[0].driver,
        "thermal_expansion_mismatch"
    );
    assert!(
        plan.risk_mitigation_hints[0]
            .recommendation
            .contains("compliant interlayer")
    );
    assert_eq!(plan.candidate_drafts.len(), 2);
    assert_eq!(
        plan.candidate_drafts[0]["schema_version"].as_str(),
        Some("kyuubiki.material-candidate-draft/v1")
    );
    assert_eq!(
        plan.candidate_drafts[0]["status"].as_str(),
        Some("draft_requires_solver_rerun")
    );
    assert_eq!(
        plan.candidate_drafts[0]["draft_id"].as_str(),
        Some("copper_ptfe_glass_epoxy.add_compliant_interlayer.draft")
    );
    assert_eq!(
        plan.candidate_drafts[0]["lineage"]["parent_decision"].as_str(),
        Some("mitigate_design_risk")
    );
    assert_eq!(
        plan.candidate_drafts[0]["required_result_schema"].as_str(),
        Some("kyuubiki.composite-thermo-electric-panel-result/v1")
    );
    assert_eq!(
        plan.candidate_draft_summary["schema_version"].as_str(),
        Some("kyuubiki.material-candidate-draft-summary/v1")
    );
    assert_eq!(
        plan.candidate_draft_summary["draft_count"].as_u64(),
        Some(2)
    );
    assert_eq!(
        plan.candidate_draft_summary["strategy_counts"]["add_compliant_interlayer"].as_u64(),
        Some(1)
    );
    assert_eq!(
        plan.candidate_draft_summary["required_result_schemas"][0].as_str(),
        Some("kyuubiki.composite-thermo-electric-panel-result/v1")
    );
    assert_eq!(plan.draft_execution_batches.len(), 1);
    assert_eq!(
        plan.draft_execution_batches[0]["draft_count"].as_u64(),
        Some(2)
    );
    assert_eq!(
        plan.draft_execution_batches[0]["dispatch_action"].as_str(),
        Some("materialize_candidate_drafts_and_rerun_solver")
    );
    assert_eq!(
        plan.draft_execution_batches[0]["required_result_schema"].as_str(),
        Some("kyuubiki.composite-thermo-electric-panel-result/v1")
    );
    assert_eq!(
        plan.draft_execution_batches[0]["execution_policy"]["requires_human_review"].as_bool(),
        Some(true)
    );
    assert_eq!(
        plan.draft_execution_batches[0]["execution_policy"]["auto_materialize_allowed"].as_bool(),
        Some(false)
    );
    assert_eq!(
        plan.draft_execution_batches[0]["execution_policy"]["qualification_claim_allowed"]
            .as_bool(),
        Some(false)
    );
    assert!(
        plan.draft_execution_batches[0]["review_checklist"]
            .as_array()
            .is_some_and(|items| items.len() >= 5)
    );
    assert_eq!(
        plan.draft_execution_batches[0]["review_checklist"][3]["id"].as_str(),
        Some("review.result_schema")
    );
    assert_eq!(
        plan.draft_execution_batches[0]["review_status"]["state"].as_str(),
        Some("pending_review")
    );
    assert_eq!(
        plan.draft_execution_batches[0]["review_status"]["blocking"].as_bool(),
        Some(true)
    );
    assert!(
        plan.draft_execution_batches[0]["review_status"]["missing_item_ids"]
            .as_array()
            .is_some_and(|items| items.len() >= 5)
    );
    assert_eq!(
        plan.draft_execution_batches[0]["allowed_review_actions"][0]["id"].as_str(),
        Some("approve_for_materialization")
    );
    assert_eq!(
        plan.draft_execution_batches[0]["allowed_review_actions"][0]["requires_reviewer_identity"]
            .as_bool(),
        Some(true)
    );
    assert_eq!(
        plan.draft_execution_batches[0]["allowed_review_actions"][2]["id"].as_str(),
        Some("reject_draft_batch")
    );
    assert_eq!(
        plan.draft_execution_batches[0]["review_decision_template"]["schema_version"].as_str(),
        Some("kyuubiki.material-candidate-review-decision/v1")
    );
    assert_eq!(
        plan.draft_execution_batches[0]["review_decision_template"]["batch_draft_ids"]
            .as_array()
            .map(Vec::len),
        Some(2)
    );
    assert_eq!(
        plan.draft_execution_batches[0]["review_decision_template"]["reviewer"]["id"].as_str(),
        Some("")
    );
    assert_eq!(
        plan.draft_execution_batches[0]["review_decision_contract"]["schema_version"].as_str(),
        Some("kyuubiki.material-candidate-review-decision-contract/v1")
    );
    assert_eq!(
        plan.draft_execution_batches[0]["review_decision_contract"]["allowed_actions"][0].as_str(),
        Some("approve_for_materialization")
    );
    assert!(
        plan.draft_execution_batches[0]["review_decision_contract"]
            ["approve_requires_completed_item_ids"]
            .as_array()
            .is_some_and(|items| items.len() >= 5)
    );
    let approved = apply_material_candidate_review_decision(
        &plan.draft_execution_batches[0],
        &json!({
            "schema_version": "kyuubiki.material-candidate-review-decision/v1",
            "batch_draft_ids": plan.draft_execution_batches[0]["draft_ids"],
            "action": "approve_for_materialization",
            "reviewer": { "id": "reviewer-1", "display_name": "Reviewer" },
            "reason": "checklist complete for prototype rerun",
            "completed_item_ids": plan.draft_execution_batches[0]["review_status"]["missing_item_ids"],
            "requested_changes": [],
            "decided_at": "2026-07-07T00:00:00Z"
        }),
    )
    .expect("approved review decision");
    assert_eq!(
        approved["status"].as_str(),
        Some("approved_for_materialization")
    );
    assert_eq!(approved["review_status"]["blocking"].as_bool(), Some(false));
    let materialization_request =
        build_material_candidate_materialization_request(&approved).expect("materialization");
    assert_eq!(
        materialization_request["schema_version"].as_str(),
        Some("kyuubiki.material-candidate-materialization-request/v1")
    );
    assert_eq!(
        materialization_request["status"].as_str(),
        Some("ready_for_agent_materialization")
    );
    assert_eq!(
        materialization_request["draft_ids"]
            .as_array()
            .map(Vec::len),
        Some(2)
    );
    let materialization_plan = build_material_candidate_materialization_plan(
        &materialization_request,
        &plan.candidate_drafts,
    )
    .expect("materialization plan");
    assert_eq!(
        materialization_plan["schema_version"].as_str(),
        Some("kyuubiki.material-candidate-materialization-plan/v1")
    );
    assert_eq!(
        materialization_plan["status"].as_str(),
        Some("ready_for_solver_rerun")
    );
    assert_eq!(
        materialization_plan["materialized_candidate_count"].as_u64(),
        Some(2)
    );
    assert!(
        materialization_plan["materialized_candidates"][0]["candidate_id"]
            .as_str()
            .is_some_and(|candidate_id| candidate_id.contains("__add_compliant_interlayer"))
    );
    let incomplete = apply_material_candidate_review_decision(
        &plan.draft_execution_batches[0],
        &json!({
            "schema_version": "kyuubiki.material-candidate-review-decision/v1",
            "batch_draft_ids": plan.draft_execution_batches[0]["draft_ids"],
            "action": "approve_for_materialization",
            "reviewer": { "id": "reviewer-1", "display_name": "Reviewer" },
            "reason": "trying to approve without all checklist items",
            "completed_item_ids": ["review.material_cards"],
            "requested_changes": [],
            "decided_at": "2026-07-07T00:00:00Z"
        }),
    );
    assert!(incomplete.is_err());
    let wrong_batch = apply_material_candidate_review_decision(
        &plan.draft_execution_batches[0],
        &json!({
            "schema_version": "kyuubiki.material-candidate-review-decision/v1",
            "batch_draft_ids": ["not-the-current-batch"],
            "action": "approve_for_materialization",
            "reviewer": { "id": "reviewer-1", "display_name": "Reviewer" },
            "reason": "attempt to approve a different batch",
            "completed_item_ids": plan.draft_execution_batches[0]["review_status"]["missing_item_ids"],
            "requested_changes": [],
            "decided_at": "2026-07-07T00:00:00Z"
        }),
    );
    assert!(wrong_batch.is_err());
    assert!(
        wrong_batch
            .unwrap_err()
            .contains("batch_draft_ids do not match")
    );
    assert!(
        plan.steps
            .iter()
            .all(|step| candidate_id_for_step(step) == Some("copper_ptfe_glass_epoxy"))
    );
}

#[test]
fn materialization_plan_rejects_empty_draft_id_list() {
    let request = json!({
        "schema_version": "kyuubiki.material-candidate-materialization-request/v1",
        "required_result_schema": "kyuubiki.material-composite-report/v1",
        "draft_ids": [],
        "status": "ready_for_agent_materialization"
    });

    let error = build_material_candidate_materialization_plan(&request, &[]).unwrap_err();

    assert!(error.contains("no draft_ids"));
}

#[test]
fn materialization_plan_rejects_missing_request_schema() {
    let request = json!({
        "required_result_schema": "kyuubiki.material-composite-report/v1",
        "draft_ids": ["draft-a"],
        "status": "ready_for_agent_materialization"
    });
    let drafts = vec![json!({
        "draft_id": "draft-a",
        "source_candidate_id": "copper_ptfe_glass_epoxy",
        "strategy": "add_compliant_interlayer",
        "study": "material_composite_thermo_electric_panel",
        "required_result_schema": "kyuubiki.material-composite-report/v1"
    })];

    let error = build_material_candidate_materialization_plan(&request, &drafts).unwrap_err();

    assert!(error.contains("missing schema_version"));
}

#[test]
fn materialization_plan_rejects_draft_missing_required_result_schema() {
    let request = json!({
        "schema_version": "kyuubiki.material-candidate-materialization-request/v1",
        "required_result_schema": "kyuubiki.material-composite-report/v1",
        "draft_ids": ["draft-a"],
        "status": "ready_for_agent_materialization"
    });
    let drafts = vec![json!({
        "draft_id": "draft-a",
        "source_candidate_id": "copper_ptfe_glass_epoxy",
        "strategy": "add_compliant_interlayer",
        "study": "material_composite_thermo_electric_panel"
    })];

    let error = build_material_candidate_materialization_plan(&request, &drafts).unwrap_err();

    assert!(error.contains("missing required_result_schema"));
}

#[test]
fn materialization_request_rejects_missing_draft_ids() {
    let approved_batch = json!({
        "schema_version": "kyuubiki.material-candidate-draft-batch/v1",
        "required_result_schema": "kyuubiki.material-composite-report/v1",
        "review_status": { "blocking": false },
        "status": "approved_for_materialization"
    });

    let error = build_material_candidate_materialization_request(&approved_batch).unwrap_err();

    assert!(error.contains("missing draft_ids"));
}

#[test]
fn materialization_request_rejects_empty_draft_ids() {
    let approved_batch = json!({
        "schema_version": "kyuubiki.material-candidate-draft-batch/v1",
        "required_result_schema": "kyuubiki.material-composite-report/v1",
        "draft_ids": [],
        "review_status": { "blocking": false },
        "status": "approved_for_materialization"
    });

    let error = build_material_candidate_materialization_request(&approved_batch).unwrap_err();

    assert!(error.contains("no draft_ids"));
}

fn candidate_id_for_step(step: &HeadlessWorkflowStep) -> Option<&str> {
    step.payload
        .get("research")
        .and_then(|research| research.get("candidate_id"))
        .and_then(Value::as_str)
}
