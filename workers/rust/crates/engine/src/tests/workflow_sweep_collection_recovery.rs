use super::workflow_parameter_sweep_graph_fixtures::{
    parameter_sweep_result_scoring_graph, sweep_result_inputs,
};
use crate::run_workflow_graph;
use kyuubiki_protocol::WorkflowGraphRunRequest;
use serde_json::{Value, json};

#[test]
fn collection_failures_cannot_publish_rankings_and_explicit_fresh_graphs_recover() {
    for policy in ["fail", "skip"] {
        for scenario in [
            "result_state",
            "summary_state",
            "case_state",
            "reserved_metric",
        ] {
            let mut inputs = sweep_result_inputs();
            let mut graph = parameter_sweep_result_scoring_graph(5.0);
            let failing = match scenario {
                "result_state" => {
                    inputs.get_mut("agent_results").unwrap()[1]["converged"] = json!(false);
                    "join_results"
                }
                "summary_state" => {
                    inputs.get_mut("agent_results").unwrap()[1]["summary"]["stability_result"] =
                        json!({"converged":false});
                    "join_results"
                }
                "case_state" => {
                    inputs.get_mut("sweep_cases").unwrap()[1]["converged"] = json!(false);
                    "summarize_results"
                }
                "reserved_metric" => {
                    for result in inputs
                        .get_mut("agent_results")
                        .unwrap()
                        .as_array_mut()
                        .unwrap()
                    {
                        result["summary"]["id"] = json!("foreign");
                    }
                    graph
                        .nodes
                        .iter_mut()
                        .find(|node| node.id == "summarize_results")
                        .unwrap()
                        .config = Some(json!({"fields":["mass","max_stress","id"]}));
                    "summarize_results"
                }
                _ => unreachable!(),
            };
            graph
                .nodes
                .iter_mut()
                .find(|node| node.id == failing)
                .unwrap()
                .config
                .as_mut()
                .unwrap()["on_error"] = json!(policy);
            let result = run_workflow_graph(WorkflowGraphRunRequest {
                graph,
                input_artifacts: inputs,
            });
            if policy == "fail" {
                let error = result.unwrap_err();
                assert!(
                    error.contains("nonconverged") || error.contains("reserved"),
                    "{scenario}: {error}"
                );
            } else {
                let rejected = result.unwrap();
                assert_eq!(rejected.failed_nodes, [failing], "{scenario}");
                for node in [
                    "score_results",
                    "map_quality_candidates",
                    "rank_quality_candidates",
                    "prepare_next_round",
                    "expand_next_cases",
                    "lineage_output",
                ] {
                    assert!(
                        !rejected.completed_nodes.iter().any(|id| id == node),
                        "{scenario}: {node}"
                    );
                }
                assert!(!rejected.artifacts.contains_key("score_results.scored"));
                assert!(!rejected.artifacts.contains_key("expand_next_cases.cases"));
            }
            let fresh = run_workflow_graph(WorkflowGraphRunRequest {
                graph: parameter_sweep_result_scoring_graph(5.0),
                input_artifacts: sweep_result_inputs(),
            })
            .unwrap();
            let exchanged: Value =
                serde_json::from_slice(&serde_json::to_vec(&fresh.artifacts).unwrap()).unwrap();
            assert_eq!(
                exchanged["rank_quality_candidates.ranking"]["best_candidate_id"],
                "material_panel_1"
            );
            assert_eq!(
                exchanged["prepare_next_round.request"]["action"],
                "continue"
            );
            assert_eq!(exchanged["expand_next_cases.cases"]["case_count"], 4);
        }
    }
}
