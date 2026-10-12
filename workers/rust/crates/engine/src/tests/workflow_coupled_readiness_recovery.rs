use super::workflow_coupled_readiness_admission::fixtures;
use super::workflow_quality_decision_recovery::next_round_graph;
use crate::run_workflow_graph;
use kyuubiki_protocol::{WorkflowGraph, WorkflowGraphRunRequest};
use serde_json::{Value, json};
use std::collections::BTreeMap;

fn request(graph: WorkflowGraph, payload: Value) -> WorkflowGraphRunRequest {
    WorkflowGraphRunRequest {
        graph,
        input_artifacts: BTreeMap::from([("quality_candidates".into(), payload)]),
    }
}

#[test]
fn conflicting_coupled_reports_stop_dependents_and_fresh_recovery_is_explicit() {
    let fixtures = fixtures();
    for id in [
        "row_objective_conflict",
        "row_payload_conflict",
        "hidden_nonconverged",
        "tail_malformed",
    ] {
        let bad = &fixtures.iter().find(|case| case["id"] == id).unwrap()["payload"];
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
            let result = run_workflow_graph(request(graph.clone(), bad.clone()));
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
            let corrected = fixtures
                .iter()
                .find(|case| case["id"] == "row_ready")
                .unwrap()["payload"]
                .clone();
            let fresh = run_workflow_graph(request(graph, corrected)).unwrap();
            assert!(fresh.failed_nodes.is_empty());
            assert_eq!(
                fresh.artifacts["prepare_request.request"]["action"],
                "continue"
            );
            assert_eq!(fresh.artifacts["expand_cases.cases"]["case_count"], 4);
        }
    }
}

#[test]
fn partial_coupled_evidence_keeps_repair_and_review_intent_through_sweep_expansion() {
    let fixtures = fixtures();
    for (id, action, strategy) in [
        (
            "null_row_preserves_inner_block",
            "replan",
            "repair_coupled_readiness_sweep",
        ),
        ("row_false_only", "replan", "repair_coupled_readiness_sweep"),
        (
            "row_required_missing_only",
            "replan",
            "repair_coupled_readiness_sweep",
        ),
        (
            "row_warning_domains_only",
            "continue",
            "review_coupled_readiness_sweep",
        ),
    ] {
        let payload = fixtures.iter().find(|case| case["id"] == id).unwrap()["payload"].clone();
        let run = run_workflow_graph(request(next_round_graph(), payload)).unwrap();
        assert!(run.failed_nodes.is_empty(), "{id}");
        let decision = &run.artifacts["prepare_request.request"];
        assert_eq!(decision["action"], action, "{id}");
        assert_eq!(decision["selected_candidate_ready"], action != "replan");
        let plan = &run.artifacts["build_plan.plan"];
        assert_eq!(plan["repair_strategy"], strategy);
        assert_eq!(
            plan["coupled_readiness"],
            decision["selected_coupled_readiness"]
        );
        let cases = &run.artifacts["expand_cases.cases"];
        assert_eq!(cases["case_count"], 4);
        for case in cases["cases"].as_array().unwrap() {
            assert_eq!(case["metadata"]["repair_strategy"], strategy);
            assert_eq!(
                case["metadata"]["coupled_readiness"],
                decision["selected_coupled_readiness"]
            );
        }
    }
}
