use super::workflow_quality_decision_recovery::next_round_graph;
use super::workflow_quality_sweep_request_admission::fixtures;
use crate::run_workflow_graph;
use kyuubiki_protocol::{WorkflowGraph, WorkflowGraphRunRequest};
use serde_json::{Value, json};
use std::collections::BTreeMap;

pub(super) fn graph(policy: &str) -> WorkflowGraph {
    let mut graph = next_round_graph();
    graph.nodes.retain(|node| node.id != "prepare_request");
    graph.edges.retain(|edge| edge.to.node != "prepare_request");
    let edge = graph
        .edges
        .iter_mut()
        .find(|edge| edge.to.node == "build_plan")
        .unwrap();
    edge.from.node = "quality_candidates".into();
    edge.from.port = "payload".into();
    let node = graph
        .nodes
        .iter_mut()
        .find(|node| node.id == "build_plan")
        .unwrap();
    node.config = Some(json!({"on_error":policy,"base":{"temperature":0}}));
    graph
}

fn request(graph: WorkflowGraph, payload: Value) -> WorkflowGraphRunRequest {
    WorkflowGraphRunRequest {
        graph,
        input_artifacts: BTreeMap::from([("quality_candidates".into(), payload)]),
    }
}

#[test]
fn direct_sweep_failures_stop_dependents_and_explicit_repairs_recover() {
    let fixtures = fixtures();
    for id in [
        "stop_block",
        "request_ready_masks_block",
        "hint_conflict",
        "continue_block",
        "hidden_failed_convergence",
        "hint_warning_count_conflict",
    ] {
        let bad = fixtures.iter().find(|case| case["id"] == id).unwrap()["payload"].clone();
        for policy in ["fail", "skip"] {
            let graph = graph(policy);
            let result = run_workflow_graph(request(graph.clone(), bad.clone()));
            if policy == "fail" {
                assert!(result.is_err(), "{id}");
            } else {
                let rejected = result.unwrap();
                assert_eq!(rejected.failed_nodes, ["build_plan"], "{id}");
                assert_eq!(rejected.completed_nodes, ["quality_candidates"]);
                for artifact in [
                    "build_plan.plan",
                    "materialize.expansion",
                    "expand_cases.cases",
                ] {
                    assert!(
                        !rejected.artifacts.contains_key(artifact),
                        "{id}: {artifact}"
                    );
                }
            }
            let corrected =
                fixtures.iter().find(|case| case["id"] == "repair").unwrap()["payload"].clone();
            let fresh = run_workflow_graph(request(graph, corrected)).unwrap();
            assert!(fresh.failed_nodes.is_empty());
            assert_eq!(fresh.artifacts["build_plan.plan"]["sweep_action"], "replan");
            let cases = &fresh.artifacts["expand_cases.cases"];
            assert_eq!(cases["case_count"], 2);
            for case in cases["cases"].as_array().unwrap() {
                assert_eq!(
                    case["metadata"]["repair_strategy"],
                    "repair_coupled_readiness_sweep"
                );
                assert_eq!(
                    case["metadata"]["coupled_readiness"]["coupled_readiness_ready"],
                    false
                );
            }
        }
    }
}

#[test]
fn rejected_candidates_alone_disable_the_entire_sweep_chain() {
    let payload = fixtures()
        .into_iter()
        .find(|case| case["id"] == "rejected_list_alone")
        .unwrap()["payload"]
        .clone();
    let run = run_workflow_graph(request(graph("fail"), payload)).unwrap();
    assert!(run.failed_nodes.is_empty());
    assert_eq!(run.artifacts["build_plan.plan"]["sweep_enabled"], false);
    assert_eq!(run.artifacts["expand_cases.cases"]["case_count"], 0);
    assert_eq!(run.artifacts["expand_cases.cases"]["cases"], json!([]));
}
