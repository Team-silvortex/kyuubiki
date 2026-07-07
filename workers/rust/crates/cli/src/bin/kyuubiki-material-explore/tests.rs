use super::*;

#[test]
fn explores_heat_spreader_with_real_solver_results() {
    let exploration = run_material_exploration("heat-spreader").expect("exploration");
    assert_eq!(
        exploration["schema_version"].as_str(),
        Some("kyuubiki.material-exploration-run/v1")
    );
    assert_eq!(exploration["candidate_count"].as_u64(), Some(3));
    assert_eq!(exploration["iteration"].as_u64(), Some(1));
    assert!(exploration["report"]["winner_candidate_id"].is_string());
    assert!(
        matches!(
            exploration["next_round"]["decision"].as_str(),
            Some("expand_around_winner" | "repair_or_rerun" | "mitigate_design_risk")
        ),
        "real solver runs should produce an actionable next-round decision"
    );
    assert_eq!(
        exploration["result_payloads"].as_array().map(Vec::len),
        Some(3)
    );
}

#[test]
fn explores_all_material_studies_with_real_solver_results() {
    for study in [
        "heat-spreader",
        "dielectric-screening",
        "thermo-shield",
        "structural-panel",
        "composite-thermo-electric-panel",
    ] {
        let exploration = run_material_exploration(study).expect("exploration");
        assert_eq!(exploration["candidate_count"].as_u64(), Some(3));
        assert!(exploration["report"]["winner_candidate_id"].is_string());
        assert!(exploration["next_round"]["actions"].is_array());
    }
}

#[test]
fn explores_composite_panel_with_coupled_local_solver_results() {
    let exploration =
        run_material_exploration("composite-thermo-electric-panel").expect("exploration");

    assert_eq!(exploration["candidate_count"].as_u64(), Some(3));
    assert_eq!(
        exploration["report"]["schema_version"].as_str(),
        Some("kyuubiki.composite-panel-report/v1")
    );
    assert_eq!(
        exploration["report"]["material_regions"]
            .as_array()
            .map(Vec::len),
        Some(3)
    );
    assert_eq!(
        exploration["report"]["reliability"]["posture"].as_str(),
        Some("prototype_screening_only")
    );
    assert!(
        exploration["report"]["reliability"]["quality_gates"]
            .as_array()
            .is_some_and(|gates| gates.len() >= 5)
    );
    assert!(
        exploration["report"]["candidates"]
            .as_array()
            .is_some_and(|rows| rows.iter().all(|row| {
                row["interface_risk_score"].is_number() && row["weakest_interface"].is_object()
            }))
    );
    assert_eq!(
        exploration["next_round"]["decision"].as_str(),
        Some("mitigate_design_risk")
    );
    assert_eq!(
        exploration["result_payloads"][0]["schema_version"].as_str(),
        Some("kyuubiki.composite-thermo-electric-panel-result/v1")
    );
    assert!(exploration["result_payloads"][0]["electrostatic"].is_object());
    assert!(exploration["result_payloads"][0]["heat"].is_object());
    assert!(exploration["result_payloads"][0]["thermal"].is_object());
}

#[test]
fn plans_next_round_from_previous_exploration_json() {
    let dir = std::env::temp_dir();
    let path = dir.join(format!(
        "kyuubiki-material-exploration-{}.json",
        std::process::id()
    ));
    let exploration = run_material_exploration("dielectric-screening").expect("exploration");
    fs::write(&path, serde_json::to_vec(&exploration).expect("json")).expect("write");

    let plan = plan_next_round(path.to_str().expect("utf8 path")).expect("plan");

    assert_eq!(
        plan["schema_version"].as_str(),
        Some("kyuubiki.material-exploration-next-round-execution/v1")
    );
    assert!(plan["runnable_step_count"].as_u64().unwrap_or(0) > 0);
    let _ = fs::remove_file(path);
}

#[test]
fn plans_next_round_from_evidence_wrapper_json() {
    let dir = std::env::temp_dir();
    let path = dir.join(format!(
        "kyuubiki-material-exploration-wrapper-{}.json",
        std::process::id()
    ));
    let exploration = run_material_exploration("structural-panel").expect("exploration");
    let wrapper = serde_json::json!({
        "schema_version": "kyuubiki.automated-material-research-example/v1",
        "exploration": exploration
    });
    fs::write(&path, serde_json::to_vec(&wrapper).expect("json")).expect("write");

    let plan = plan_next_round(path.to_str().expect("utf8 path")).expect("plan");

    assert_eq!(
        plan["schema_version"].as_str(),
        Some("kyuubiki.material-exploration-next-round-execution/v1")
    );
    assert!(plan["runnable_step_count"].as_u64().unwrap_or(0) > 0);
    let _ = fs::remove_file(path);
}

#[test]
fn runs_next_round_from_previous_exploration_json() {
    let dir = std::env::temp_dir();
    let path = dir.join(format!(
        "kyuubiki-material-exploration-run-next-{}.json",
        std::process::id()
    ));
    let exploration = run_material_exploration("dielectric-screening").expect("exploration");
    fs::write(&path, serde_json::to_vec(&exploration).expect("json")).expect("write");

    let next = run_next_round(path.to_str().expect("utf8 path")).expect("next run");

    assert_eq!(
        next["schema_version"].as_str(),
        Some("kyuubiki.material-exploration-run/v1")
    );
    assert_eq!(next["mode"].as_str(), Some("local_solver_next_round"));
    assert_eq!(next["iteration"].as_u64(), Some(2));
    assert_eq!(next["next_round"]["iteration"].as_u64(), Some(3));
    assert_eq!(next["candidate_count"].as_u64(), Some(3));
    assert!(next["report"]["winner_candidate_id"].is_string());
    let _ = fs::remove_file(path);
}

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
    assert_eq!(rerun["step_count"].as_u64(), Some(1));
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
fn chains_next_rounds_from_previous_exploration_json() {
    let dir = std::env::temp_dir();
    let path = dir.join(format!(
        "kyuubiki-material-exploration-chain-{}.json",
        std::process::id()
    ));
    let exploration = run_material_exploration("heat-spreader").expect("exploration");
    fs::write(&path, serde_json::to_vec(&exploration).expect("json")).expect("write");

    let chain = chain_next_rounds(path.to_str().expect("utf8 path"), 2).expect("chain");

    assert_eq!(
        chain["schema_version"].as_str(),
        Some("kyuubiki.material-exploration-chain/v1")
    );
    assert_eq!(chain["round_count"].as_u64(), Some(2));
    assert_eq!(
        chain["stop_reason"].as_str(),
        Some("risk_mitigation_required")
    );
    assert_eq!(chain["all_winners_stable"].as_bool(), Some(true));
    assert_eq!(
        chain["decision_counts"]["mitigate_design_risk"].as_u64(),
        Some(2)
    );
    assert_eq!(chain["repair_summary"]["required"].as_bool(), Some(true));
    assert!(
        chain["repair_summary"]["violated_gate_ids"]
            .as_array()
            .is_some_and(|ids| !ids.is_empty())
    );
    assert!(
        chain["repair_summary"]["focus_candidate_ids"]
            .as_array()
            .is_some_and(|ids| !ids.is_empty())
    );
    assert_eq!(chain["repair_plan"]["required"].as_bool(), Some(true));
    assert_eq!(
        chain["repair_plan"]["priority"].as_str(),
        Some("before_expansion")
    );
    assert!(
        chain["repair_plan"]["actions"]
            .as_array()
            .is_some_and(|actions| actions.iter().any(|action| {
                action["id"].as_str() == Some("generate_lower_risk_neighbor_candidates")
            }))
    );
    assert_eq!(chain["final_iteration"].as_u64(), Some(3));
    assert_eq!(chain["summaries"].as_array().map(Vec::len), Some(2));
    assert_eq!(chain["summaries"][0]["iteration"].as_u64(), Some(2));
    assert_eq!(chain["runs"].as_array().map(Vec::len), Some(2));
    let _ = fs::remove_file(path);
}
