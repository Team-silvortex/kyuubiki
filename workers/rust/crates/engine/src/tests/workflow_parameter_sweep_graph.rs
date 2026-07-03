use crate::run_workflow_graph;
use kyuubiki_protocol::WorkflowGraphRunRequest;

use super::workflow_parameter_sweep_graph_fixtures::{
    material_study_envelope_graph, material_study_envelope_inputs,
    parameter_sweep_result_scoring_graph, sweep_result_inputs,
};

#[test]
fn runs_parameter_sweep_result_scoring_workflow_graph() {
    let run = run_workflow_graph(WorkflowGraphRunRequest {
        graph: parameter_sweep_result_scoring_graph(5.0),
        input_artifacts: sweep_result_inputs(),
    })
    .expect("parameter sweep result scoring workflow should run");

    let joined = run
        .artifacts
        .get("join_results.joined")
        .expect("joined sweep results should exist");
    assert_eq!(joined["joined_summary_count"].as_u64(), Some(2));
    assert_eq!(joined["missing_summary_count"].as_u64(), Some(0));

    let summarized = run
        .artifacts
        .get("summarize_results.summary")
        .expect("summarized sweep rows should exist");
    assert_eq!(summarized["row_count"].as_u64(), Some(2));
    assert_eq!(
        summarized["numeric_columns"]["max_stress"]["min"].as_f64(),
        Some(84.0)
    );
    assert_eq!(
        summarized["rows"][1]["metadata"]["round"].as_str(),
        Some("seed")
    );
    assert_eq!(
        summarized["rows"][1]["metadata"]["source_candidate_id"].as_str(),
        Some("baseline")
    );

    let scored = run
        .artifacts
        .get("score_results.scored")
        .expect("scored sweep rows should exist");
    assert_eq!(scored["scored_count"].as_u64(), Some(2));
    assert_eq!(scored["best"]["case_id"].as_str(), Some("material_panel_1"));
    assert_eq!(scored["best"]["objective_feasible"].as_bool(), Some(true));
    assert_eq!(
        scored["scored_rows"][1]["case_id"].as_str(),
        Some("material_panel_0")
    );
    assert_eq!(
        scored["scored_rows"][1]["objective_feasible"].as_bool(),
        Some(false)
    );
    assert_eq!(scored["best"]["metadata"]["round"].as_str(), Some("seed"));

    let candidates = run
        .artifacts
        .get("map_quality_candidates.candidates")
        .expect("quality candidates should exist");
    assert_eq!(candidates["candidate_count"].as_u64(), Some(2));
    assert_eq!(
        candidates["source_best_case_id"].as_str(),
        Some("material_panel_1")
    );
    assert_eq!(
        candidates["candidates"]["material_panel_1"]["source_row"]["metadata"]
            ["source_candidate_id"]
            .as_str(),
        Some("baseline")
    );

    let ranking = run
        .artifacts
        .get("rank_quality_candidates.ranking")
        .expect("quality ranking should exist");
    assert_eq!(
        ranking["best_candidate_id"].as_str(),
        Some("material_panel_1")
    );
    assert_eq!(ranking["ready_candidate_count"].as_u64(), Some(1));

    let request = run
        .artifacts
        .get("prepare_next_round.request")
        .expect("next round request should exist");
    assert_eq!(request["action"].as_str(), Some("continue"));
    assert_eq!(
        request["selected_candidate_id"].as_str(),
        Some("material_panel_1")
    );

    let plan = run
        .artifacts
        .get("build_next_plan.plan")
        .expect("next sweep plan should exist");
    assert_eq!(plan["sweep_enabled"].as_bool(), Some(true));
    assert_eq!(
        plan["source_candidate_id"].as_str(),
        Some("material_panel_1")
    );
    assert_eq!(plan["case_count_estimate"].as_u64(), Some(4));

    let next_cases = run
        .artifacts
        .get("expand_next_cases.cases")
        .expect("next sweep cases should exist");
    assert_eq!(next_cases["case_count"].as_u64(), Some(4));
    assert_eq!(
        next_cases["cases"][0]["id"].as_str(),
        Some("material_next_round_0")
    );
    assert_eq!(
        next_cases["cases"][3]["model"]["material"]["density"].as_f64(),
        Some(7800.0)
    );
    assert_eq!(
        next_cases["cases"][0]["metadata"]["source_candidate_id"].as_str(),
        Some("material_panel_1")
    );
    assert_eq!(
        next_cases["cases"][0]["metadata"]["source_plan_contract"].as_str(),
        Some("kyuubiki.quality_parameter_sweep_plan/v1")
    );
}

#[test]
fn stops_parameter_sweep_result_scoring_workflow_graph_when_target_is_met() {
    let run = run_workflow_graph(WorkflowGraphRunRequest {
        graph: parameter_sweep_result_scoring_graph(20.0),
        input_artifacts: sweep_result_inputs(),
    })
    .expect("parameter sweep result scoring workflow should stop cleanly");

    let request = run
        .artifacts
        .get("prepare_next_round.request")
        .expect("next round request should exist");
    assert_eq!(request["action"].as_str(), Some("stop"));
    assert_eq!(
        request["selected_candidate_id"].as_str(),
        Some("material_panel_1")
    );

    let plan = run
        .artifacts
        .get("build_next_plan.plan")
        .expect("disabled next sweep plan should exist");
    assert_eq!(plan["sweep_enabled"].as_bool(), Some(false));
    assert_eq!(plan["case_count_estimate"].as_u64(), Some(0));
    assert_eq!(plan["sweep_action"].as_str(), Some("stop"));

    let next_cases = run
        .artifacts
        .get("expand_next_cases.cases")
        .expect("empty next cases should exist");
    assert_eq!(next_cases["case_count"].as_u64(), Some(0));
    assert_eq!(next_cases["sweep_enabled"].as_bool(), Some(false));
    assert_eq!(next_cases["sweep_action"].as_str(), Some("stop"));
}

#[test]
fn runs_material_study_envelope_ranking_workflow_graph() {
    let run = run_workflow_graph(WorkflowGraphRunRequest {
        graph: material_study_envelope_graph(),
        input_artifacts: material_study_envelope_inputs(),
    })
    .expect("material study envelope workflow should run");

    let envelopes = run
        .artifacts
        .get("compose_envelopes.envelopes")
        .expect("material envelopes should exist");
    assert_eq!(
        envelopes["material_envelope_batch_contract"].as_str(),
        Some("kyuubiki.material_study_envelope_batch/v1")
    );
    assert_eq!(
        envelopes["material_envelope_best_candidate_id"].as_str(),
        Some("cool_stiff")
    );
    assert_eq!(
        envelopes["material_envelope_candidate_count"].as_u64(),
        Some(3)
    );

    let ranking = run
        .artifacts
        .get("rank_envelopes.ranking")
        .expect("material envelope ranking should exist");
    assert_eq!(
        ranking["material_best_candidate_id"].as_str(),
        Some("cool_stiff")
    );
    assert_eq!(ranking["material_feasible_count"].as_u64(), Some(2));

    let pareto = run
        .artifacts
        .get("pareto_envelopes.pareto")
        .expect("material envelope pareto should exist");
    assert_eq!(pareto["material_pareto_candidate_count"].as_u64(), Some(3));
    assert_eq!(pareto["material_pareto_feasible_count"].as_u64(), Some(2));
    assert!(
        pareto["material_pareto_dominated"]
            .as_array()
            .expect("dominated array")
            .iter()
            .any(|entry| entry["candidate_id"].as_str() == Some("hot_light")
                && entry["dominated_by"].as_str() == Some("infeasible"))
    );
}
