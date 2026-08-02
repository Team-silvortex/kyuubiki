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
    assert_eq!(
        exploration["execution_authority"]["execution_class"].as_str(),
        Some("real_solver")
    );
    assert_eq!(
        exploration["execution_authority"]["mock_execution"].as_bool(),
        Some(false)
    );
    assert_eq!(
        exploration["execution_authority"]["fallback_used"].as_bool(),
        Some(false)
    );
    assert_eq!(
        exploration["execution_authority"]["production_eligible"].as_bool(),
        Some(true)
    );
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
fn material_catalog_exposes_study_entrypoints() {
    let catalog = material_catalog_payload().expect("catalog");

    assert_eq!(
        catalog["schema_version"].as_str(),
        Some("kyuubiki.material-study-catalog/v1")
    );
    assert_eq!(catalog["study_count"].as_u64(), Some(5));
    assert!(
        catalog["studies"]
            .as_array()
            .is_some_and(|studies| studies.iter().any(|study| {
                study["id"].as_str() == Some("material_composite_thermo_electric_panel")
                    && study["material_card_contract_required"].as_bool() == Some(true)
                    && study["material_card_schema_version"].as_str()
                        == Some("kyuubiki.material-card/v1")
                    && study["material_card_ref_count"].as_u64() == Some(3)
            }))
    );
    assert!(
        catalog["next_steps"]
            .as_array()
            .is_some_and(|steps| !steps.is_empty())
    );
}

#[test]
fn material_study_description_resolves_aliases_and_metrics() {
    let study = material_study_payload("heat-spreader").expect("study");

    assert_eq!(
        study["schema_version"].as_str(),
        Some("kyuubiki.material-study-description/v1")
    );
    assert_eq!(
        study["id"].as_str(),
        Some("material_heat_spreader_screening")
    );
    assert_eq!(
        study["report_schema_version"].as_str(),
        Some("kyuubiki.material-research-report/v1")
    );
    assert_eq!(study["metric_count"].as_u64(), Some(4));
    assert_eq!(
        study["study"]["material_card_schema_version"].as_str(),
        Some("kyuubiki.material-card/v1")
    );
    assert_eq!(
        study["study"]["material_card_contract_required"].as_bool(),
        Some(true)
    );
    assert!(
        study["recommended_flow"]
            .as_array()
            .is_some_and(|steps| steps.iter().any(|step| {
                step.as_str()
                    .is_some_and(|text| text.contains("--plan-next"))
            }))
    );
}

#[test]
fn material_study_plan_previews_steps_without_running_solver() {
    let plan = material_study_plan_payload("composite-thermo-electric-panel").expect("plan");

    assert_eq!(
        plan["schema_version"].as_str(),
        Some("kyuubiki.material-study-execution-plan/v1")
    );
    assert_eq!(
        plan["study_id"].as_str(),
        Some("material_composite_thermo_electric_panel")
    );
    assert_eq!(plan["solve_step_count"].as_u64(), Some(3));
    assert_eq!(plan["candidate_count"].as_u64(), Some(3));
    assert_eq!(
        plan["material_card_schema_version"].as_str(),
        Some("kyuubiki.material-card/v1")
    );
    assert_eq!(plan["material_card_ref_count"].as_u64(), Some(3));
    assert!(plan["candidate_ids"].as_array().is_some_and(|ids| {
        ids.iter()
            .any(|id| id.as_str() == Some("copper_ptfe_glass_epoxy"))
    }));
    assert!(
        plan["actions"]
            .as_array()
            .is_some_and(|actions| actions.iter().all(|action| action.is_string()))
    );
    assert!(
        plan["recommended_command"]
            .as_str()
            .is_some_and(|command| command.contains("composite-thermo-electric-panel"))
    );
}

fn assert_composite_candidate_evidence(row: &Value) {
    let id = row["candidate_id"].as_str().unwrap_or("unknown");
    assert!(row["interface_risk_score"].is_number(), "{id}: risk");
    assert!(row["weakest_interface"].is_object(), "{id}: interface");
    assert_eq!(
        row["electrostatic_cross_validation"]["status"].as_str(),
        Some("pass"),
        "{id}: electrostatic cross-validation"
    );
    assert!(
        row["electrostatic_cross_validation"]["relative_error"]
            .as_f64()
            .is_some_and(|error| error <= 1.0e-9),
        "{id}: electrostatic relative error"
    );
    assert_eq!(
        row["electrostatic_mesh_convergence"]["status"].as_str(),
        Some("pass"),
        "{id}: electrostatic mesh"
    );
    assert_eq!(
        row["electrostatic_mesh_convergence"]["samples"]
            .as_array()
            .map(Vec::len),
        Some(4),
        "{id}: electrostatic mesh samples"
    );
    assert_eq!(
        row["electrothermal_loss_projection"]["schema_version"].as_str(),
        Some("kyuubiki.composite-electrothermal-loss-projection/v1"),
        "{id}: electrothermal loss projection"
    );
    assert_eq!(
        row["electrothermal_loss_projection"]["energy_balance_relative_error"].as_f64(),
        Some(0.0),
        "{id}: electrothermal energy balance"
    );
    assert_eq!(
        row["heat_cross_validation"]["status"].as_str(),
        Some("pass"),
        "{id}: heat cross-validation"
    );
    assert_eq!(
        row["heat_mesh_convergence"]["status"].as_str(),
        Some("pass"),
        "{id}: heat mesh"
    );
    assert_eq!(
        row["heat_mesh_convergence"]["samples"]
            .as_array()
            .map(Vec::len),
        Some(4),
        "{id}: heat mesh samples"
    );
    assert_eq!(
        row["heat_to_thermal_projection"]["mapped_node_count"].as_u64(),
        Some(8),
        "{id}: thermal projection nodes"
    );
    assert_eq!(
        row["heat_to_thermal_projection"]["maximum_coordinate_error_m"].as_f64(),
        Some(0.0),
        "{id}: thermal projection coordinates"
    );

    let thermal = &row["thermal_mesh_convergence"];
    assert_eq!(
        thermal["status"].as_str(),
        Some("fail"),
        "{id}: thermal mesh"
    );
    assert_eq!(
        thermal["samples"].as_array().map(Vec::len),
        Some(4),
        "{id}: thermal mesh samples"
    );
    assert_eq!(
        thermal["regime_assessment"]["diagnosis"].as_str(),
        Some("pass_metrics_lack_qualified_discretization_uncertainty"),
        "{id}: thermal regime"
    );
    assert!(
        thermal["regime_assessment"]["metrics"][0]["fine_grid_gci"].is_null(),
        "{id}: displacement GCI should be unknown"
    );
    assert!(
        thermal["regime_assessment"]["metrics"][1]["fine_grid_gci"]
            .as_f64()
            .is_some_and(|value| value > 0.1),
        "{id}: energy GCI should remain above tolerance"
    );
    assert_eq!(
        thermal["regime_assessment"]["metrics"][2]["regime"].as_str(),
        Some("monotonic_diverging"),
        "{id}: peak stress regime"
    );
    assert_eq!(
        thermal["algebraic_validation"]["status"].as_str(),
        Some("pass"),
        "{id}: algebraic validation"
    );
    let series = thermal["algebraic_validation"]["series"]
        .as_array()
        .expect("algebraic series");
    assert_eq!(series.len(), 3, "{id}: algebraic series");
    assert!(
        series.iter().all(|entry| entry["samples"]
            .as_array()
            .is_some_and(|samples| samples.len() == 4)),
        "{id}: algebraic samples"
    );

    assert_eq!(
        row["thermal_constraint_regularized_mesh_convergence"]["status"].as_str(),
        Some("fail"),
        "{id}: regularized thermal mesh"
    );
    assert_eq!(
        row["thermal_constraint_sensitivity"]["diagnosis"].as_str(),
        Some("mixed_restraint_sensitivity_and_persistent_energy_nonconvergence"),
        "{id}: restraint sensitivity"
    );
    assert_eq!(
        row["thermal_stress_recovery"]["status"].as_str(),
        Some("fail"),
        "{id}: stress recovery"
    );
    let grading = &row["thermal_interface_grading_assessment"];
    assert_eq!(
        grading["diagnosis"].as_str(),
        Some("graded_mesh_did_not_resolve_nonconvergence"),
        "{id}: interface grading"
    );
    assert!(
        grading["p95_change_ratio_graded_to_uniform"]
            .as_f64()
            .is_some_and(|ratio| ratio > 1.0),
        "{id}: graded P95 should not claim improvement"
    );
    assert!(
        grading["max_change_ratio_graded_to_uniform"]
            .as_f64()
            .is_some_and(|ratio| ratio > 1.0),
        "{id}: graded peak should not claim improvement"
    );
    assert_eq!(
        grading["qualification_effect"].as_str(),
        Some("diagnostic_only_does_not_override_uniform_mesh_gates"),
        "{id}: grading qualification"
    );
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
    let candidates = exploration["report"]["candidates"]
        .as_array()
        .expect("candidate rows");
    for row in candidates {
        assert_composite_candidate_evidence(row);
    }
    assert!(
        exploration["report"]["reliability"]["quality_gates"]
            .as_array()
            .is_some_and(|gates| gates.iter().any(|gate| {
                gate["id"].as_str() == Some("gate.electrostatic_closed_form.relative_error")
                    && gate["status"].as_str() == Some("pass")
            }))
    );
    assert!(
        exploration["report"]["reliability"]["quality_gates"]
            .as_array()
            .is_some_and(|gates| gates.iter().any(|gate| {
                gate["id"].as_str() == Some("gate.thermal_solver.relative_residual")
                    && gate["status"].as_str() == Some("pass")
                    && gate["actual_value"]
                        .as_f64()
                        .is_some_and(|value| value <= 1.0e-10)
            }))
    );
    assert!(
        exploration["report"]["reliability"]["quality_gates"]
            .as_array()
            .is_some_and(|gates| gates
                .iter()
                .filter(|gate| {
                    gate["id"]
                        .as_str()
                        .is_some_and(|id| id.starts_with("gate.heat_"))
                        && gate["status"].as_str() == Some("pass")
                })
                .count()
                == 4)
    );
    assert!(
        exploration["report"]["reliability"]["quality_gates"]
            .as_array()
            .is_some_and(|gates| gates
                .iter()
                .filter(|gate| {
                    gate["id"]
                        .as_str()
                        .is_some_and(|id| id.starts_with("gate.electrostatic_mesh_convergence."))
                        && gate["status"].as_str() == Some("pass")
                })
                .count()
                == 2)
    );
    assert!(
        exploration["report"]["reliability"]["quality_gates"]
            .as_array()
            .is_some_and(|gates| gates.iter().any(|gate| {
                gate["id"].as_str() == Some("gate.thermal_mesh_gci.displacement")
                    && gate["status"].as_str() == Some("unknown")
            }))
    );
    assert!(
        exploration["report"]["reliability"]["quality_gates"]
            .as_array()
            .is_some_and(|gates| gates.iter().any(|gate| {
                gate["id"].as_str() == Some("gate.thermal_mesh_gci.strain_energy")
                    && gate["status"].as_str() == Some("violate")
            }))
    );
    assert!(
        exploration["report"]["reliability"]["quality_gates"]
            .as_array()
            .is_some_and(|gates| gates
                .iter()
                .filter(|gate| {
                    gate["id"]
                        .as_str()
                        .is_some_and(|id| id.starts_with("gate.thermal_stress_recovery."))
                        && gate["status"].as_str() == Some("violate")
                })
                .count()
                == 1)
    );
    assert!(
        exploration["report"]["reliability"]["quality_gates"]
            .as_array()
            .is_some_and(|gates| gates
                .iter()
                .filter(|gate| {
                    gate["id"]
                        .as_str()
                        .is_some_and(|id| id.starts_with("gate.thermal_mesh_convergence."))
                        && gate["status"].as_str() == Some("violate")
                })
                .count()
                == 1)
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
    assert!(
        exploration["result_payloads"][0]["electrothermal_loss_projection"]["total_loss_w"]
            .as_f64()
            .is_some_and(|value| value > 0.0)
    );
    assert!(
        exploration["result_payloads"][0]["electrostatic_mesh_convergence"]["samples"]
            .as_array()
            .is_some_and(|samples| samples.len() == 4)
    );
    assert!(exploration["result_payloads"][0]["heat"].is_object());
    assert_eq!(
        exploration["result_payloads"][0]["heat_cross_validation"]["status"].as_str(),
        Some("pass")
    );
    assert!(
        exploration["result_payloads"][0]["heat_mesh_convergence"]["samples"]
            .as_array()
            .is_some_and(|samples| samples.len() == 4)
    );
    assert!(exploration["result_payloads"][0]["thermal"].is_object());
    assert_eq!(
        exploration["result_payloads"][0]["heat_to_thermal_projection"]["mapped_node_count"]
            .as_u64(),
        Some(8)
    );
    assert_eq!(
        exploration["result_payloads"][0]["thermal_mesh_convergence"]["status"].as_str(),
        Some("fail")
    );
    assert_eq!(
        exploration["result_payloads"][0]["thermal_constraint_sensitivity"]["qualification_effect"]
            .as_str(),
        Some("diagnostic_only_does_not_override_primary_quality_gates")
    );
    assert_eq!(
        exploration["result_payloads"][0]["thermal_interface_grading_assessment"]["diagnosis"]
            .as_str(),
        Some("graded_mesh_did_not_resolve_nonconvergence")
    );
}

#[test]
fn plans_next_round_from_previous_exploration_json() {
    let dir = std::env::temp_dir();
    let path = dir.join(format!(
        "kyuubiki-material-exploration-{}.json",
        std::process::id()
    ));
    let exploration = run_material_exploration("heat-spreader").expect("exploration");
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
    let exploration = run_material_exploration("heat-spreader").expect("exploration");
    fs::write(&path, serde_json::to_vec(&exploration).expect("json")).expect("write");

    let next = run_next_round(path.to_str().expect("utf8 path")).expect("next run");

    assert_eq!(
        next["schema_version"].as_str(),
        Some("kyuubiki.material-exploration-run/v1")
    );
    assert_eq!(next["mode"].as_str(), Some("local_solver_next_round"));
    assert_eq!(
        next["execution_authority"]["executor_id"].as_str(),
        Some("kyuubiki.rust.local-solver")
    );
    assert_eq!(next["iteration"].as_u64(), Some(2));
    assert_eq!(next["next_round"]["iteration"].as_u64(), Some(3));
    assert_eq!(next["candidate_count"].as_u64(), Some(3));
    assert!(next["report"]["winner_candidate_id"].is_string());
    assert_eq!(
        next["lineage"]["schema_version"].as_str(),
        Some("kyuubiki.material-next-round-lineage/v1")
    );
    assert_eq!(next["lineage"]["source_iteration"].as_u64(), Some(1));
    assert_eq!(
        next["lineage"]["decision"].as_str(),
        exploration["next_round"]["decision"].as_str()
    );
    assert_eq!(
        next["lineage"]["optimization_objectives"]["schema_version"].as_str(),
        Some("kyuubiki.material-next-round-optimization-objectives/v1")
    );
    assert!(
        next["lineage"]["material_card_refs"]
            .as_array()
            .is_some_and(|refs| !refs.is_empty())
    );
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
    assert_eq!(
        chain["convergence_assessment"]["schema_version"].as_str(),
        Some("kyuubiki.material-chain-convergence-assessment/v1")
    );
    assert_eq!(
        chain["convergence_assessment"]["state"].as_str(),
        Some("blocked_by_quality_gates")
    );
    assert_eq!(
        chain["convergence_assessment"]["winner_stable"].as_bool(),
        Some(true)
    );
    assert!(
        chain["convergence_assessment"]["winner_score_delta"]
            .as_f64()
            .is_some_and(|delta| delta <= 0.001)
    );
    assert_eq!(chain["repair_summary"]["required"].as_bool(), Some(true));
    assert!(
        chain["repair_summary"]["violated_gate_ids"]
            .as_array()
            .is_some_and(|ids| !ids.is_empty())
    );
    assert!(
        chain["repair_summary"]["violated_gate_ids"]
            .as_array()
            .is_some_and(|ids| ids
                .iter()
                .any(|id| id.as_str() == Some("gate.areal_mass.warning")))
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
    assert!(chain["summaries"][0]["winner_score"].is_number());
    assert_eq!(chain["summaries"][0]["source_iteration"].as_u64(), Some(1));
    assert_eq!(
        chain["summaries"][0]["lineage_schema_version"].as_str(),
        Some("kyuubiki.material-next-round-lineage/v1")
    );
    assert_eq!(
        chain["summaries"][0]["optimization_objectives"]["schema_version"].as_str(),
        Some("kyuubiki.material-next-round-optimization-objectives/v1")
    );
    assert!(
        chain["summaries"][0]["material_card_refs"]
            .as_array()
            .is_some_and(|refs| !refs.is_empty())
    );
    assert_eq!(
        chain["optimization_trace"].as_array().map(Vec::len),
        Some(2)
    );
    assert_eq!(
        chain["optimization_trace"][0]["mode"].as_str(),
        Some("risk_constrained_search")
    );
    assert!(
        chain["optimization_trace"][0]["primary_metric_ids"]
            .as_array()
            .is_some_and(|ids| !ids.is_empty())
    );
    assert_eq!(chain["runs"].as_array().map(Vec::len), Some(2));
    let _ = fs::remove_file(path);
}

#[test]
fn shared_chain_schema_fixture_matches_cli_chain_shape() {
    let fixture: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../../../../../schemas/examples.material-exploration-chain.json"
    ))
    .expect("fixture json");

    assert_eq!(
        fixture["schema_version"].as_str(),
        Some("kyuubiki.material-exploration-chain/v1")
    );
    assert_eq!(
        fixture["convergence_assessment"]["schema_version"].as_str(),
        Some("kyuubiki.material-chain-convergence-assessment/v1")
    );
    assert_eq!(
        fixture["summaries"][0]["optimization_objectives"]["schema_version"].as_str(),
        Some("kyuubiki.material-next-round-optimization-objectives/v1")
    );
    assert_eq!(
        fixture["optimization_trace"].as_array().map(Vec::len),
        fixture["summaries"].as_array().map(Vec::len)
    );
    assert_eq!(
        fixture["summaries"].as_array().map(Vec::len),
        fixture["runs"].as_array().map(Vec::len)
    );
}
