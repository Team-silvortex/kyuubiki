use super::workflow_quality_sweep_plan_admission::fixtures;
use super::workflow_quality_sweep_request_recovery;
use crate::{run_workflow_graph, workflow_executor::run_transform_operator};
use kyuubiki_protocol::{WorkflowGraph, WorkflowGraphRunRequest};
use serde_json::{Value, json};
use std::collections::BTreeMap;

fn graph(policy: &str) -> WorkflowGraph {
    let mut graph = workflow_quality_sweep_request_recovery::graph(policy);
    graph.nodes.retain(|node| node.id != "build_plan");
    graph.edges.retain(|edge| edge.to.node != "build_plan");
    let edge = graph
        .edges
        .iter_mut()
        .find(|edge| edge.to.node == "materialize")
        .unwrap();
    edge.from.node = "quality_candidates".into();
    edge.from.port = "payload".into();
    graph
        .nodes
        .iter_mut()
        .find(|node| node.id == "materialize")
        .unwrap()
        .config = Some(json!({"on_error":policy}));
    graph
}

fn request(graph: WorkflowGraph, payload: Value) -> WorkflowGraphRunRequest {
    WorkflowGraphRunRequest {
        graph,
        input_artifacts: BTreeMap::from([("quality_candidates".into(), payload)]),
    }
}

#[test]
fn direct_plan_errors_block_dependents_and_fresh_repairs_preserve_metadata() {
    let fixtures = fixtures();
    for id in [
        "enabled_stop",
        "wrong_strategy",
        "wrong_focus_state",
        "rejected_enabled",
        "declared_count_stale",
        "report_failed",
    ] {
        for policy in ["fail", "skip"] {
            let graph = graph(policy);
            let bad = fixtures.iter().find(|row| row["id"] == id).unwrap()["payload"].clone();
            let result = run_workflow_graph(request(graph.clone(), bad));
            if policy == "fail" {
                assert!(result.is_err(), "{id}");
            } else {
                let rejected = result.unwrap();
                assert_eq!(rejected.failed_nodes, ["materialize"], "{id}");
                assert_eq!(rejected.completed_nodes, ["quality_candidates"]);
                for key in ["materialize.expansion", "expand_cases.cases"] {
                    assert!(!rejected.artifacts.contains_key(key));
                }
            }
            let repaired =
                fixtures.iter().find(|row| row["id"] == "repair").unwrap()["payload"].clone();
            let fresh = run_workflow_graph(request(graph, repaired.clone())).unwrap();
            assert!(fresh.failed_nodes.is_empty());
            let cases = &fresh.artifacts["expand_cases.cases"];
            assert_eq!(cases["case_count"], 2);
            for case in cases["cases"].as_array().unwrap() {
                for field in [
                    "coupled_readiness",
                    "optimization_hint",
                    "repair_strategy",
                    "repair_focus",
                ] {
                    assert_eq!(case["metadata"][field], repaired[field]);
                }
            }
        }
    }
}

#[test]
fn generated_plans_reject_single_field_corruption_without_losing_valid_repair_paths() {
    for state in ["ready", "warn", "block"] {
        let report = json!({"coupled_readiness_state":state});
        let decision = run_transform_operator("transform.prepare_quality_next_round_request",
            json!({"ranking":[{"candidate_id":"a","score":1,"ready":true,"coupled_readiness":report}]}),
            json!({"target_score":0.1,"search_space":{"temperature_delta":[120,160]}})).unwrap();
        let plan = run_transform_operator(
            "transform.build_quality_parameter_sweep_plan",
            decision,
            json!({}),
        )
        .unwrap();
        for (field, value) in [
            ("sweep_action", json!("stop")),
            (
                "source_rejected_candidates",
                json!([{"candidate_id":"bad"}]),
            ),
            ("source_ranking_complete", json!(false)),
            ("converged", json!(false)),
            ("repair_strategy", json!("unknown")),
            ("repair_focus", json!({"domain":"mismatched"})),
            ("case_count_estimate", json!(3)),
        ] {
            let mut changed = plan.clone();
            changed[field] = value;
            let exchanged = serde_json::from_slice(&serde_json::to_vec(&changed).unwrap()).unwrap();
            assert!(
                run_transform_operator(
                    "transform.materialize_quality_sweep_expansion",
                    exchanged,
                    json!({})
                )
                .is_err(),
                "{state}:{field}"
            );
        }
        let materialized = run_transform_operator(
            "transform.materialize_quality_sweep_expansion",
            plan.clone(),
            json!({}),
        )
        .unwrap();
        let cases =
            run_transform_operator("transform.expand_parameter_sweep", materialized, json!({}))
                .unwrap();
        assert_eq!(cases["case_count"], 2);
        for case in cases["cases"].as_array().unwrap() {
            assert_eq!(case["metadata"]["coupled_readiness"], report);
            assert_eq!(case["metadata"]["repair_strategy"], plan["repair_strategy"]);
        }
    }
}

#[test]
fn disabled_and_billion_sample_deferred_plans_never_expand_cases() {
    for id in [
        "stop",
        "disabled_repair",
        "disabled_incomplete",
        "deferred_limit_raise",
    ] {
        let case = fixtures().into_iter().find(|row| row["id"] == id).unwrap();
        let materialized = run_transform_operator(
            "transform.materialize_quality_sweep_expansion",
            case["payload"].clone(),
            case["config"].clone(),
        )
        .unwrap();
        let expanded =
            run_transform_operator("transform.expand_parameter_sweep", materialized, json!({}))
                .unwrap();
        assert_eq!(expanded["case_count"], 0, "{id}");
        assert_eq!(expanded["cases"], json!([]));
    }
}
