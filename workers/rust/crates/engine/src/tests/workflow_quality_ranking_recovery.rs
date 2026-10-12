use super::workflow_quality_decision_recovery::next_round_graph;
use crate::run_workflow_graph;
use kyuubiki_protocol::WorkflowGraphRunRequest;
use serde_json::{Value, json};
use std::collections::BTreeMap;

#[test]
fn incoherent_rankings_cannot_publish_sweeps_and_corrected_fresh_runs_recover() {
    let fixtures: Vec<Value> = serde_json::from_str(include_str!(
        "../../../../../../tests/fixtures/workflow-quality-ranking-integrity.json"
    ))
    .unwrap();
    for id in [
        "descending_score",
        "selected_objective_score_mismatch",
        "tail_objective_score_mismatch",
        "duplicate_rank",
    ] {
        let bad = fixtures.iter().find(|case| case["id"] == id).unwrap()["payload"].clone();
        for policy in ["fail", "skip"] {
            let mut graph = next_round_graph();
            graph
                .nodes
                .iter_mut()
                .find(|node| node.id == "prepare_request")
                .unwrap()
                .config
                .as_mut()
                .unwrap()["on_error"] = json!(policy);
            let result = run_workflow_graph(WorkflowGraphRunRequest {
                graph: graph.clone(),
                input_artifacts: BTreeMap::from([("quality_candidates".into(), bad.clone())]),
            });
            if policy == "fail" {
                assert!(result.is_err(), "{id}");
            } else {
                let rejected = result.unwrap();
                assert_eq!(rejected.failed_nodes, ["prepare_request"], "{id}");
                assert_eq!(rejected.completed_nodes, ["quality_candidates"]);
                for artifact in [
                    "prepare_request.request",
                    "build_plan.plan",
                    "expand_cases.cases",
                ] {
                    assert!(
                        !rejected.artifacts.contains_key(artifact),
                        "{id}: {artifact}"
                    );
                }
            }
            let corrected = json!({"ranking":[
                {"candidate_id":"a","rank":1,"score":1,"ready":true,"objective":{"composite_quality_score":1}},
                {"candidate_id":"b","rank":2,"score":2,"ready":true,"objective":{"composite_quality_score":2}}
            ]});
            let fresh = run_workflow_graph(WorkflowGraphRunRequest {
                graph,
                input_artifacts: BTreeMap::from([("quality_candidates".into(), corrected)]),
            })
            .unwrap();
            assert!(fresh.failed_nodes.is_empty());
            assert_eq!(
                fresh.artifacts["prepare_request.request"]["selected_candidate_id"],
                "a"
            );
            assert_eq!(
                fresh.artifacts["prepare_request.request"]["action"],
                "continue"
            );
            assert_eq!(fresh.artifacts["expand_cases.cases"]["case_count"], 4);
        }
    }
}
