use super::workflow_parameter_sweep_graph_fixtures::{
    parameter_sweep_result_scoring_graph, sweep_result_inputs,
};
use crate::run_workflow_graph;
use kyuubiki_protocol::WorkflowGraphRunRequest;
use serde_json::{Value, json};

#[test]
fn native_scoring_and_mapping_failures_isolate_downstream_and_fresh_runs_recover() {
    for policy in ["fail", "skip"] {
        for scenario in ["bad_objective", "bad_mapping", "uncalibrated_maximum"] {
            let mut graph = parameter_sweep_result_scoring_graph(5.0);
            let failing = if scenario == "bad_objective" {
                "score_results"
            } else {
                "map_quality_candidates"
            };
            for node in &mut graph.nodes {
                if node.id == "score_results" && scenario != "bad_mapping" {
                    node.config = Some(if scenario == "bad_objective" {
                        json!({"objectives":[{"field":"mass","weight":-1}]})
                    } else {
                        json!({"objectives":[{"field":"mass","goal":"max"}]})
                    });
                }
                if node.id == "map_quality_candidates" && scenario == "bad_mapping" {
                    node.config = Some(json!({"quality_domain":null}));
                }
                if node.id == failing {
                    node.config.as_mut().unwrap()["on_error"] = json!(policy);
                }
            }
            let failed = run_workflow_graph(WorkflowGraphRunRequest {
                graph,
                input_artifacts: sweep_result_inputs(),
            });
            if policy == "fail" {
                assert!(failed.is_err(), "{scenario}");
            } else {
                let run = failed.unwrap();
                assert_eq!(run.failed_nodes, [failing], "{scenario}");
                for id in [
                    "rank_quality_candidates",
                    "prepare_next_round",
                    "expand_next_cases",
                    "lineage_output",
                ] {
                    assert!(
                        !run.completed_nodes.iter().any(|node| node == id),
                        "{scenario}: {id}"
                    );
                }
                assert!(
                    !run.artifacts
                        .contains_key("rank_quality_candidates.ranking")
                );
                assert!(!run.artifacts.contains_key("expand_next_cases.cases"));
            }
            let recovered = run_workflow_graph(WorkflowGraphRunRequest {
                graph: parameter_sweep_result_scoring_graph(5.0),
                input_artifacts: sweep_result_inputs(),
            })
            .unwrap();
            assert_eq!(
                recovered.artifacts["prepare_next_round.request"]["action"],
                "continue"
            );
            assert_eq!(
                recovered.artifacts["rank_quality_candidates.ranking"]["best_candidate_id"],
                "material_panel_1"
            );
            assert_eq!(
                recovered.artifacts["expand_next_cases.cases"]["case_count"],
                4
            );
            let encoded = serde_json::to_vec(&recovered.artifacts).unwrap();
            let restored: Value = serde_json::from_slice(&encoded).unwrap();
            assert_eq!(
                restored["map_quality_candidates.candidates"]["candidate_count"],
                2
            );
        }
    }
}

#[test]
fn native_all_infeasible_graph_retains_blockers_and_replans_instead_of_stopping() {
    let mut graph = parameter_sweep_result_scoring_graph(1e15);
    graph
        .nodes
        .iter_mut()
        .find(|node| node.id == "score_results")
        .unwrap()
        .config = Some(json!({
        "objectives":[{"field":"mass","max_allowed":0},{"field":"max_stress","weight":0,"max_allowed":0}]
    }));
    let run = run_workflow_graph(WorkflowGraphRunRequest {
        graph,
        input_artifacts: sweep_result_inputs(),
    })
    .unwrap();
    assert_eq!(run.artifacts["score_results.scored"]["best"], Value::Null);
    assert_eq!(
        run.artifacts["score_results.scored"]["selection_ready"],
        false
    );
    assert_eq!(
        run.artifacts["map_quality_candidates.candidates"]["source_best_case_id"],
        Value::Null
    );
    assert_eq!(
        run.artifacts["rank_quality_candidates.ranking"]["ready_candidate_count"],
        0
    );
    assert_eq!(
        run.artifacts["prepare_next_round.request"]["action"],
        "replan"
    );
    let candidates = &run.artifacts["map_quality_candidates.candidates"]["candidates"];
    for row in candidates.as_object().unwrap().values() {
        assert_eq!(
            row["qualities"]["material_sweep"]["material_sweep_quality_blocking_terms"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
    }
}

#[test]
fn native_fixed_reference_maximization_continues_until_absolute_policy_is_met() {
    let mut graph = parameter_sweep_result_scoring_graph(5.0);
    graph
        .nodes
        .iter_mut()
        .find(|node| node.id == "score_results")
        .unwrap()
        .config = Some(json!({"objectives":[{"field":"mass","goal":"max"}]}));
    graph
        .nodes
        .iter_mut()
        .find(|node| node.id == "map_quality_candidates")
        .unwrap()
        .config = Some(json!({"score_reference":20.0}));
    let blocked = run_workflow_graph(WorkflowGraphRunRequest {
        graph: graph.clone(),
        input_artifacts: sweep_result_inputs(),
    })
    .unwrap();
    assert_eq!(
        blocked.artifacts["rank_quality_candidates.ranking"]["best_candidate_ready"],
        false
    );
    assert_eq!(
        blocked.artifacts["prepare_next_round.request"]["action"],
        "replan"
    );
    // Readiness is separate from the stop target: the fixed cost is 12.2,
    // above the default ready limit of 12 but below this explicit study limit.
    graph
        .nodes
        .iter_mut()
        .find(|node| node.id == "rank_quality_candidates")
        .unwrap()
        .config = Some(json!({"max_ready_score":20.0}));
    let run = run_workflow_graph(WorkflowGraphRunRequest {
        graph,
        input_artifacts: sweep_result_inputs(),
    })
    .unwrap();
    assert_eq!(
        run.artifacts["prepare_next_round.request"]["action"],
        "continue"
    );
    assert_eq!(
        run.artifacts["rank_quality_candidates.ranking"]["best_candidate_ready"],
        true
    );
    assert_eq!(
        run.artifacts["rank_quality_candidates.ranking"]["best_candidate_id"],
        "material_panel_1"
    );
    assert_eq!(
        run.artifacts["rank_quality_candidates.ranking"]["best_candidate_score"],
        12.2
    );
}
