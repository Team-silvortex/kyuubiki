use super::*;
use crate::materialization::materialized_rerun_summary_lines;

#[test]
fn runs_materialized_candidates_from_materialization_plan_json() {
    let dir = std::env::temp_dir();
    let path = dir.join(format!(
        "kyuubiki-materialized-plan-{}.json",
        std::process::id()
    ));
    let plan = serde_json::json!({
        "schema_version": "kyuubiki.material-candidate-materialization-plan/v1",
        "status": "ready_for_solver_rerun",
        "materialized_candidate_count": 1,
        "materialized_candidates": [{
            "schema_version": "kyuubiki.materialized-candidate-spec/v1",
            "candidate_id": "copper_ptfe_glass_epoxy__add_compliant_interlayer",
            "source_draft_id": "draft-a",
            "source_candidate_id": "copper_ptfe_glass_epoxy",
            "strategy": "add_compliant_interlayer",
            "study": "material_composite_thermo_electric_panel",
            "required_result_schema": "kyuubiki.composite-thermo-electric-panel-result/v1",
            "status": "requires_solver_rerun"
        }]
    });
    fs::write(&path, serde_json::to_vec(&plan).expect("json")).expect("write");

    let rerun = run_materialized_candidates(path.to_str().expect("utf8 path")).expect("rerun");

    assert_eq!(
        rerun["schema_version"].as_str(),
        Some("kyuubiki.materialized-candidate-rerun/v1")
    );
    assert_eq!(
        rerun["source_materialization_schema_version"].as_str(),
        Some("kyuubiki.material-candidate-materialization-plan/v1")
    );
    assert_eq!(
        rerun["source_materialization_status"].as_str(),
        Some("ready_for_solver_rerun")
    );
    assert_eq!(rerun["step_count"].as_u64(), Some(1));
    assert_eq!(
        rerun["materialized_candidate_ids"][0].as_str(),
        Some("copper_ptfe_glass_epoxy__add_compliant_interlayer")
    );
    assert_eq!(
        rerun["report"]["schema_version"].as_str(),
        Some("kyuubiki.composite-materialized-candidate-report/v1")
    );
    assert_eq!(
        rerun["result_payloads"][0]["research"]["candidate_id"].as_str(),
        Some("copper_ptfe_glass_epoxy__add_compliant_interlayer")
    );
    assert!(rerun["next_round"]["decision"].is_string());
    let _ = fs::remove_file(path);
}

#[test]
fn runs_materialized_candidates_from_shared_schema_fixture() {
    let path = temp_path("kyuubiki-materialized-schema-fixture");
    fs::write(
        &path,
        include_bytes!(
            "../../../../../../../schemas/examples.material-candidate-materialization-plan.json"
        ),
    )
    .expect("write fixture");

    let rerun = run_materialized_candidates(path.to_str().expect("utf8 path")).expect("rerun");

    assert_eq!(
        rerun["source_materialization_schema_version"].as_str(),
        Some("kyuubiki.material-candidate-materialization-plan/v1")
    );
    assert_eq!(
        rerun["materialized_candidate_ids"].as_array().map(Vec::len),
        Some(2)
    );

    fs::remove_file(path).ok();
}

#[test]
fn run_materialized_rejects_missing_candidate_list() {
    let path = temp_path("kyuubiki-materialized-missing-list");
    fs::write(
        &path,
        serde_json::to_vec(&serde_json::json!({
            "schema_version": "kyuubiki.material-candidate-materialization-plan/v1",
            "status": "ready_for_solver_rerun"
        }))
        .expect("json"),
    )
    .expect("write");

    let error = run_materialized_candidates(path.to_str().expect("utf8 path")).unwrap_err();

    assert!(error.contains("missing materialized_candidates"));
    let _ = fs::remove_file(path);
}

#[test]
fn run_materialized_rejects_wrong_plan_schema() {
    let path = temp_path("kyuubiki-materialized-wrong-schema");
    fs::write(
        &path,
        serde_json::to_vec(&serde_json::json!({
            "schema_version": "kyuubiki.material-exploration-next-round-execution/v1",
            "status": "ready_for_solver_rerun",
            "materialized_candidate_count": 0,
            "materialized_candidates": []
        }))
        .expect("json"),
    )
    .expect("write");

    let error = run_materialized_candidates(path.to_str().expect("utf8 path")).unwrap_err();

    assert!(error.contains("schema_version must be"));
    let _ = fs::remove_file(path);
}

#[test]
fn run_materialized_rejects_empty_candidate_list() {
    let path = temp_path("kyuubiki-materialized-empty-list");
    fs::write(
        &path,
        serde_json::to_vec(&serde_json::json!({
            "schema_version": "kyuubiki.material-candidate-materialization-plan/v1",
            "status": "ready_for_solver_rerun",
            "materialized_candidate_count": 0,
            "materialized_candidates": []
        }))
        .expect("json"),
    )
    .expect("write");

    let error = run_materialized_candidates(path.to_str().expect("utf8 path")).unwrap_err();

    assert!(error.contains("no materialized candidates"));
    let _ = fs::remove_file(path);
}

#[test]
fn run_materialized_rejects_unready_plan_status() {
    let path = temp_path("kyuubiki-materialized-unready-status");
    fs::write(
        &path,
        serde_json::to_vec(&serde_json::json!({
            "schema_version": "kyuubiki.material-candidate-materialization-plan/v1",
            "status": "pending_agent_materialization",
            "materialized_candidate_count": 1,
            "materialized_candidates": [{
                "schema_version": "kyuubiki.materialized-candidate-spec/v1",
                "candidate_id": "copper_ptfe_glass_epoxy__add_compliant_interlayer",
                "source_draft_id": "draft-a",
                "source_candidate_id": "copper_ptfe_glass_epoxy",
                "strategy": "add_compliant_interlayer",
                "study": "material_composite_thermo_electric_panel",
                "status": "requires_solver_rerun"
            }]
        }))
        .expect("json"),
    )
    .expect("write");

    let error = run_materialized_candidates(path.to_str().expect("utf8 path")).unwrap_err();

    assert!(error.contains("status must be ready_for_solver_rerun"));
    let _ = fs::remove_file(path);
}

#[test]
fn run_materialized_rejects_candidate_count_mismatch() {
    let path = temp_path("kyuubiki-materialized-count-mismatch");
    fs::write(
        &path,
        serde_json::to_vec(&serde_json::json!({
            "schema_version": "kyuubiki.material-candidate-materialization-plan/v1",
            "status": "ready_for_solver_rerun",
            "materialized_candidate_count": 2,
            "materialized_candidates": [{
                "schema_version": "kyuubiki.materialized-candidate-spec/v1",
                "candidate_id": "copper_ptfe_glass_epoxy__add_compliant_interlayer",
                "source_draft_id": "draft-a",
                "source_candidate_id": "copper_ptfe_glass_epoxy",
                "strategy": "add_compliant_interlayer",
                "study": "material_composite_thermo_electric_panel",
                "status": "requires_solver_rerun"
            }]
        }))
        .expect("json"),
    )
    .expect("write");

    let error = run_materialized_candidates(path.to_str().expect("utf8 path")).unwrap_err();

    assert!(error.contains("materialized_candidate_count must match"));
    let _ = fs::remove_file(path);
}

#[test]
fn run_materialized_rejects_candidate_missing_required_result_schema() {
    let path = temp_path("kyuubiki-materialized-missing-result-schema");
    fs::write(
        &path,
        serde_json::to_vec(&serde_json::json!({
            "schema_version": "kyuubiki.material-candidate-materialization-plan/v1",
            "status": "ready_for_solver_rerun",
            "materialized_candidate_count": 1,
            "materialized_candidates": [{
                "schema_version": "kyuubiki.materialized-candidate-spec/v1",
                "candidate_id": "copper_ptfe_glass_epoxy__add_compliant_interlayer",
                "source_draft_id": "draft-a",
                "source_candidate_id": "copper_ptfe_glass_epoxy",
                "strategy": "add_compliant_interlayer",
                "study": "material_composite_thermo_electric_panel",
                "status": "requires_solver_rerun"
            }]
        }))
        .expect("json"),
    )
    .expect("write");

    let error = run_materialized_candidates(path.to_str().expect("utf8 path")).unwrap_err();

    assert!(error.contains("missing required_result_schema"));
    let _ = fs::remove_file(path);
}

#[test]
fn run_materialized_rejects_wrong_candidate_spec_schema() {
    let path = temp_path("kyuubiki-materialized-wrong-spec-schema");
    fs::write(
        &path,
        serde_json::to_vec(&serde_json::json!({
            "schema_version": "kyuubiki.material-candidate-materialization-plan/v1",
            "status": "ready_for_solver_rerun",
            "materialized_candidate_count": 1,
            "materialized_candidates": [{
                "schema_version": "kyuubiki.material-candidate-draft/v1",
                "candidate_id": "copper_ptfe_glass_epoxy__add_compliant_interlayer",
                "source_draft_id": "draft-a",
                "source_candidate_id": "copper_ptfe_glass_epoxy",
                "strategy": "add_compliant_interlayer",
                "study": "material_composite_thermo_electric_panel",
                "required_result_schema": "kyuubiki.composite-thermo-electric-panel-result/v1",
                "status": "requires_solver_rerun"
            }]
        }))
        .expect("json"),
    )
    .expect("write");

    let error = run_materialized_candidates(path.to_str().expect("utf8 path")).unwrap_err();

    assert!(error.contains("schema_version must be kyuubiki.materialized-candidate-spec/v1"));
    let _ = fs::remove_file(path);
}

#[test]
fn materialized_rerun_summary_shows_source_and_candidate_count() {
    let payload = serde_json::json!({
        "study": "material_composite_thermo_electric_panel",
        "source_materialization_schema_version": "schema.v1",
        "source_materialization_status": "ready_for_solver_rerun",
        "step_count": 2,
        "materialized_candidate_ids": ["a", "b"],
        "report": { "winner_candidate_id": "a" },
        "next_round": { "decision": "expand_around_winner" }
    });

    let lines = materialized_rerun_summary_lines(&payload);

    assert!(
        lines
            .iter()
            .any(|line| line == "Source plan: schema.v1 (ready_for_solver_rerun)")
    );
    assert!(lines.iter().any(|line| line == "Candidates: 2"));
}

#[test]
fn exports_review_decision_template_from_next_round_plan() {
    let plan_path = temp_path("kyuubiki-review-template-plan");
    let plan_value = composite_next_round_plan_value();
    fs::write(&plan_path, serde_json::to_vec(&plan_value).expect("json")).expect("write plan");

    let template =
        review_decision_template(plan_path.to_str().expect("utf8 path")).expect("review template");

    assert_eq!(
        template["schema_version"].as_str(),
        Some("kyuubiki.material-review-template-export/v1")
    );
    assert!(
        template["draft_ids"]
            .as_array()
            .is_some_and(|draft_ids| !draft_ids.is_empty())
    );
    assert_eq!(
        template["review_decision_template"]["schema_version"].as_str(),
        Some("kyuubiki.material-candidate-review-decision/v1")
    );
    assert!(
        template["review_checklist"]
            .as_array()
            .is_some_and(|items| !items.is_empty())
    );
    let _ = fs::remove_file(plan_path);
}

#[test]
fn approves_review_template_into_decision_json() {
    let plan_path = temp_path("kyuubiki-approval-template-plan");
    let template_path = temp_path("kyuubiki-approval-template");
    let plan_value = composite_next_round_plan_value();
    fs::write(&plan_path, serde_json::to_vec(&plan_value).expect("json")).expect("write plan");
    let template =
        review_decision_template(plan_path.to_str().expect("utf8 path")).expect("review template");
    fs::write(&template_path, serde_json::to_vec(&template).expect("json"))
        .expect("write template");

    let decision = approve_review_template(
        template_path.to_str().expect("utf8 path"),
        "reviewer-1",
        "Reviewer One",
        "explicit approval for prototype materialization",
        "2026-07-07T00:00:00Z",
    )
    .expect("decision");

    assert_eq!(
        decision["schema_version"].as_str(),
        Some("kyuubiki.material-candidate-review-decision/v1")
    );
    assert_eq!(
        decision["action"].as_str(),
        Some("approve_for_materialization")
    );
    assert_eq!(decision["reviewer"]["id"].as_str(), Some("reviewer-1"));
    assert!(
        decision["completed_item_ids"]
            .as_array()
            .is_some_and(|items| !items.is_empty())
    );
    let _ = fs::remove_file(plan_path);
    let _ = fs::remove_file(template_path);
}

#[test]
fn materializes_reviewed_next_round_batches_from_decision_json() {
    let plan_path = temp_path("kyuubiki-next-round-plan");
    let decision_path = temp_path("kyuubiki-review-decision");
    let plan_value = composite_next_round_plan_value();
    let batch = &plan_value["draft_execution_batches"][0];
    let decision = serde_json::json!({
        "schema_version": "kyuubiki.material-candidate-review-decision/v1",
        "batch_draft_ids": batch["draft_ids"],
        "action": "approve_for_materialization",
        "reviewer": { "id": "cli-reviewer", "display_name": "CLI Reviewer" },
        "reason": "explicit CLI test approval for prototype rerun",
        "completed_item_ids": batch["review_status"]["missing_item_ids"],
        "requested_changes": [],
        "decided_at": "2026-07-07T00:00:00Z"
    });
    fs::write(&plan_path, serde_json::to_vec(&plan_value).expect("json")).expect("write plan");
    fs::write(&decision_path, serde_json::to_vec(&decision).expect("json"))
        .expect("write decision");

    let materialization = materialize_reviewed_candidates(
        plan_path.to_str().expect("utf8 path"),
        decision_path.to_str().expect("utf8 path"),
    )
    .expect("materialization");

    assert_eq!(
        materialization["schema_version"].as_str(),
        Some("kyuubiki.materialization-reviewed-plan/v1")
    );
    assert_eq!(
        materialization["materialization_plan"]["status"].as_str(),
        Some("ready_for_solver_rerun")
    );
    assert!(
        materialization["materialization_plan"]["materialized_candidate_count"]
            .as_u64()
            .unwrap_or(0)
            > 0
    );
    let _ = fs::remove_file(plan_path);
    let _ = fs::remove_file(decision_path);
}

#[test]
fn required_review_identity_rejects_empty_values() {
    let error = required_flag(&Some("  ".to_string()), "--reviewer-id").unwrap_err();

    assert!(error.contains("--reviewer-id"));
}

#[test]
fn materialization_rejects_unmatched_review_decision_batch() {
    let plan_path = temp_path("kyuubiki-unmatched-review-plan");
    let decision_path = temp_path("kyuubiki-unmatched-review-decision");
    let plan_value = composite_next_round_plan_value();
    let decision = serde_json::json!({
        "schema_version": "kyuubiki.material-candidate-review-decision/v1",
        "batch_draft_ids": ["does-not-match-any-draft"],
        "action": "approve_for_materialization",
        "reviewer": { "id": "cli-reviewer", "display_name": "CLI Reviewer" },
        "reason": "should not match",
        "completed_item_ids": ["review.material_cards"],
        "requested_changes": [],
        "decided_at": "2026-07-07T00:00:00Z"
    });
    fs::write(&plan_path, serde_json::to_vec(&plan_value).expect("json")).expect("write plan");
    fs::write(&decision_path, serde_json::to_vec(&decision).expect("json"))
        .expect("write decision");

    let error = materialize_reviewed_candidates(
        plan_path.to_str().expect("utf8 path"),
        decision_path.to_str().expect("utf8 path"),
    )
    .unwrap_err();

    assert!(error.contains("no draft execution batch matches"));
    let _ = fs::remove_file(plan_path);
    let _ = fs::remove_file(decision_path);
}

#[test]
fn runs_full_reviewed_materialization_smoke_chain() {
    let plan_path = temp_path("kyuubiki-full-chain-plan");
    let template_path = temp_path("kyuubiki-full-chain-template");
    let decision_path = temp_path("kyuubiki-full-chain-decision");
    let materialization_path = temp_path("kyuubiki-full-chain-materialization");
    let plan_value = composite_next_round_plan_value();
    fs::write(&plan_path, serde_json::to_vec(&plan_value).expect("json")).expect("write plan");

    let template =
        review_decision_template(plan_path.to_str().expect("utf8 path")).expect("template");
    fs::write(&template_path, serde_json::to_vec(&template).expect("json"))
        .expect("write template");
    let decision = approve_review_template(
        template_path.to_str().expect("utf8 path"),
        "reviewer-full-chain",
        "Full Chain Reviewer",
        "explicit approval for full materialization smoke test",
        "2026-07-07T00:00:00Z",
    )
    .expect("decision");
    fs::write(&decision_path, serde_json::to_vec(&decision).expect("json"))
        .expect("write decision");

    let materialization = materialize_reviewed_candidates(
        plan_path.to_str().expect("utf8 path"),
        decision_path.to_str().expect("utf8 path"),
    )
    .expect("materialization");
    fs::write(
        &materialization_path,
        serde_json::to_vec(&materialization).expect("json"),
    )
    .expect("write materialization");
    let rerun = run_materialized_candidates(materialization_path.to_str().expect("utf8 path"))
        .expect("rerun");

    assert_eq!(
        rerun["schema_version"].as_str(),
        Some("kyuubiki.materialized-candidate-rerun/v1")
    );
    assert!(
        rerun["materialized_candidate_ids"]
            .as_array()
            .is_some_and(|candidate_ids| !candidate_ids.is_empty())
    );
    assert!(rerun["report"]["winner_candidate_id"].is_string());
    assert!(
        rerun["result_payloads"]
            .as_array()
            .is_some_and(|results| !results.is_empty())
    );
    assert!(rerun["next_round"]["decision"].is_string());
    for path in [
        plan_path,
        template_path,
        decision_path,
        materialization_path,
    ] {
        let _ = fs::remove_file(path);
    }
}

fn composite_next_round_plan_value() -> serde_json::Value {
    let exploration =
        run_material_exploration("composite-thermo-electric-panel").expect("exploration");
    let plan = build_material_exploration_next_round_execution_plan(&exploration).expect("plan");
    serde_json::to_value(&plan).expect("plan json")
}

fn temp_path(prefix: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("{prefix}-{}.json", std::process::id()))
}
