use super::workflow_quality_ranking_graph::quality_ranking_to_sweep_graph;
use crate::run_workflow_graph;
use kyuubiki_protocol::{WorkflowGraph, WorkflowGraphRunRequest};
use serde_json::{Value, json};
use std::collections::BTreeMap;

fn candidates() -> Value {
    json!({"candidates":{"a":{"qualities":{"thermal":{
        "thermal_quality_score":1.0,"thermal_quality_ready":true
    }}}}})
}

fn ranking() -> Value {
    json!({"ranking":[{"candidate_id":"a","score":1.0,"ready":true}]})
}

pub(super) fn next_round_graph() -> WorkflowGraph {
    let mut graph = quality_ranking_to_sweep_graph(0.5);
    graph.nodes.retain(|node| node.id != "rank_candidates");
    graph.edges.retain(|edge| edge.to.node != "rank_candidates");
    let edge = graph
        .edges
        .iter_mut()
        .find(|edge| edge.to.node == "prepare_request")
        .unwrap();
    edge.from.node = "quality_candidates".to_string();
    edge.from.port = "payload".to_string();
    graph
}

fn request(graph: WorkflowGraph, payload: Value) -> WorkflowGraphRunRequest {
    WorkflowGraphRunRequest {
        graph,
        input_artifacts: BTreeMap::from([("quality_candidates".to_string(), payload)]),
    }
}

#[test]
fn invalid_quality_decisions_stop_dependents_and_fresh_inputs_recover() {
    for scenario in [
        "mixed_domain",
        "bad_flag",
        "bad_id",
        "failed_envelope",
        "duplicate_id",
        "bad_tail",
    ] {
        for policy in ["fail", "skip"] {
            let at_rank = matches!(scenario, "mixed_domain" | "bad_flag" | "bad_id");
            let mut payload = if at_rank { candidates() } else { ranking() };
            match scenario {
                "mixed_domain" => {
                    payload["candidates"]["a"]["qualities"]["thermal"]["structural_quality_ready"] =
                        json!(true)
                }
                "bad_flag" => {
                    payload["candidates"]["a"]["qualities"]["thermal"]["thermal_quality_ready"] =
                        Value::Null
                }
                "bad_id" => {
                    payload = json!({"candidates":[{"id":null,"qualities":candidates()["candidates"]["a"]["qualities"]}]})
                }
                "failed_envelope" => payload["converged"] = json!(false),
                "duplicate_id" => payload["ranking"]
                    .as_array_mut()
                    .unwrap()
                    .push(ranking()["ranking"][0].clone()),
                "bad_tail" => payload["ranking"].as_array_mut().unwrap().push(Value::Null),
                _ => unreachable!(),
            }
            let mut graph = if at_rank {
                quality_ranking_to_sweep_graph(0.5)
            } else {
                next_round_graph()
            };
            let failing = if at_rank {
                "rank_candidates"
            } else {
                "prepare_request"
            };
            graph
                .nodes
                .iter_mut()
                .find(|node| node.id == failing)
                .unwrap()
                .config
                .as_mut()
                .unwrap()["on_error"] = json!(policy);
            let result = run_workflow_graph(request(graph, payload));
            if policy == "fail" {
                assert!(result.is_err(), "{scenario}");
            } else {
                let run = result.unwrap();
                assert_eq!(run.failed_nodes, [failing], "{scenario}");
                for node in ["build_plan", "materialize", "expand_cases", "sweep_output"] {
                    assert!(
                        !run.completed_nodes.iter().any(|id| id == node),
                        "{scenario}: {node}"
                    );
                }
                assert!(!run.artifacts.contains_key("prepare_request.request"));
                assert!(!run.artifacts.contains_key("expand_cases.cases"));
            }
            let fresh = if at_rank {
                request(quality_ranking_to_sweep_graph(0.5), candidates())
            } else {
                request(next_round_graph(), ranking())
            };
            let recovered = run_workflow_graph(fresh).unwrap();
            assert!(recovered.failed_nodes.is_empty());
            let exchanged: Value =
                serde_json::from_slice(&serde_json::to_vec(&recovered.artifacts).unwrap()).unwrap();
            assert_eq!(exchanged["prepare_request.request"]["action"], "continue");
            assert_eq!(exchanged["expand_cases.cases"]["case_count"], 4);
        }
    }
}

#[test]
fn partial_rankings_pause_while_known_quality_blocks_replan_repairs() {
    for scenario in ["partial", "objective_blocker", "objective_not_ready"] {
        let (graph, payload) = if scenario == "partial" {
            let mut payload = candidates();
            payload["candidates"]["a"]["qualities"]["thermal"]["thermal_quality_score"] =
                json!(0.0);
            payload["candidates"]["broken"] = json!({"qualities":{"thermal":{
                "thermal_quality_score":0.0,"structural_quality_ready":true
            }}});
            (quality_ranking_to_sweep_graph(0.5), payload)
        } else {
            let mut payload = ranking();
            payload["ranking"][0]["blocking_terms"] = json!([]);
            payload["ranking"][0]["objective"] = if scenario == "objective_blocker" {
                json!({"composite_quality_blocking_terms":[{"domain":"thermal","source_blocking_terms":[{"field":"flux"}]}]})
            } else {
                json!({"composite_quality_ready":false})
            };
            (next_round_graph(), payload)
        };
        let run = run_workflow_graph(request(graph, payload)).unwrap();
        assert!(run.failed_nodes.is_empty());
        let next = &run.artifacts["prepare_request.request"];
        assert_eq!(next["action"], "replan", "{scenario}");
        assert_eq!(run.artifacts["build_plan.plan"]["sweep_action"], "replan");
        match scenario {
            "partial" => {
                assert_eq!(run.artifacts["expand_cases.cases"]["case_count"], 0);
                assert_eq!(run.artifacts["expand_cases.cases"]["sweep_enabled"], false);
                assert_eq!(
                    next["source_rejected_candidates"][0]["candidate_id"],
                    "broken"
                );
                assert_eq!(
                    next["selected_iteration_hint"]["action"],
                    "repair_rejected_candidates"
                );
            }
            "objective_blocker" => {
                assert_eq!(next["selected_iteration_hint"]["focus_field"], "flux");
                assert_eq!(
                    run.artifacts["build_plan.plan"]["repair_strategy"],
                    "repair_blocking_term_sweep"
                );
                assert_eq!(run.artifacts["expand_cases.cases"]["case_count"], 4);
            }
            _ => {
                assert_eq!(next["selected_candidate_ready"], false);
                assert_eq!(run.artifacts["expand_cases.cases"]["case_count"], 4);
            }
        }
        let fresh = run_workflow_graph(request(next_round_graph(), ranking())).unwrap();
        assert_eq!(fresh.artifacts["expand_cases.cases"]["case_count"], 4);
    }
}
